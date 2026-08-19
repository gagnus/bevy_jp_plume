//! Fill-fn editing a [`ThemeEditablePalette`]: preset buttons plus a slider per
//! ramp parameter.

use core::cell::Cell;
use core::ops::RangeInclusive;

use bevy::color::Color;
use bevy::ui::{Val, em};

use crate::controls::ButtonVariant;
use crate::imm::{PlumeImm, Ui};
use crate::style::font_awesome;
use crate::theme::dark_theme::default_dark_palette;
use crate::theme::light_theme::default_light_palette;
use crate::theme::{OklchaArray, ThemeEditablePalette, ThemeId, ThemeSlot, UiTheme};

/// Palette editor (presets, neutral/accent/text ramps) as a fill-fn for your own
/// container. Mutates `palette` in place; bake it into the live theme with
/// [`UiTheme::set_palette`](crate::theme::UiTheme::set_palette) when it changes.
pub fn theme_editor(ui: &mut Ui, palette: &mut ThemeEditablePalette) {
    ui.horizontal(|ui| {
        if ui
            .button("Dark")
            .variant(ButtonVariant::Outline)
            .grow()
            .clicked
        {
            *palette = default_dark_palette();
        }
        if ui
            .button("Light")
            .variant(ButtonVariant::Outline)
            .grow()
            .clicked
        {
            *palette = default_light_palette();
        }
        if ui
            .button("Random")
            .variant(ButtonVariant::Outline)
            .grow()
            .clicked
        {
            *palette = ThemeEditablePalette::random();
        }
    });

    ui.separator().full_bleed();

    ui.section("Neutrals", |ui| {
        ramp_rows(ui, &mut palette.neutrals, 0.2, false)
    })
    .collapsible(false);
    let is_dangerous_accent = palette.is_accent_close_to_danger();
    ui.section("Accent", |ui| {
        ramp_rows(ui, &mut palette.accent, 0.5, is_dangerous_accent)
    })
    .collapsible(false);
    ui.section("Text", |ui| {
        ramp_rows(ui, &mut palette.text, 0.2, false);
    })
    .collapsible(false);
    ui.section("Disabled", |ui| {
        ramp_rows(ui, &mut palette.disabled, 0.2, false)
    })
    .collapsible(false);
}

/// [`theme_editor`] for every registered theme, behind a tab bar: the default
/// theme labeled "Default", the rest by their id. Edits bake straight into
/// `theme`; returns true when one did.
// Takes plain `&mut UiTheme` so a `ResMut` caller can pass
// `bypass_change_detection()` and `set_changed()` on true — deref-muting the
// resource every open frame would repaint the whole UI continuously.
pub fn theme_editor_tabs(ui: &mut Ui, theme: &mut UiTheme, selected: &mut ThemeId) -> bool {
    let ids: Vec<ThemeId> = theme.theme_ids().cloned().collect();
    // Every tab's body closure is built each frame, so the edit reports through
    // a shared cell rather than a `&mut` each.
    let edited = Cell::new(None);
    ui.tabs(selected, |tabs| {
        for id in &ids {
            let label = if *id == ThemeId::default() {
                "Default"
            } else {
                id.name()
            };
            tabs.tab(id.clone(), label).body(|ui| {
                let mut palette = theme.editable(Some(id)).clone();
                theme_editor(ui, &mut palette);
                if palette != *theme.editable(Some(id)) {
                    edited.set(Some((id.clone(), palette)));
                }
            });
        }
    });
    match edited.take() {
        Some((id, palette)) => {
            theme.set_palette(id, &palette);
            true
        }
        None => false,
    }
}

// Hue, chroma, then one lightness row per stop. `chroma_max` keeps neutral/text
// ramps near-gray while the accent ramp reaches full saturation. Each lightness row
// previews its stop color; hue/chroma rows reserve the same slot so sliders line up.
fn ramp_rows<const N: usize>(
    ui: &mut Ui,
    ramp: &mut OklchaArray<N>,
    chroma_max: f32,
    is_dangerous: bool,
) {
    param_row(
        ui,
        "Hue",
        &mut ramp.hue,
        0.0..=360.0,
        0,
        Some("\u{b0}"),
        None,
        is_dangerous,
    );
    param_row(
        ui,
        "Chroma",
        &mut ramp.chroma,
        0.0..=chroma_max,
        3,
        None,
        None,
        is_dangerous,
    );
    let (hue, chroma) = (ramp.hue, ramp.chroma);
    for (stop, lightness) in ramp.l.iter_mut().enumerate() {
        let swatch = Color::oklcha(*lightness, chroma, hue, 1.0);
        param_row(
            ui,
            &format!("L{stop}"),
            lightness,
            0.0..=1.0,
            3,
            None,
            Some(swatch),
            false,
        );
    }
}

// A color swatch (or a reserved gap), a label, a slider and a number input, all bound
// to one palette scalar.
fn param_row(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    precision: usize,
    suffix: Option<&str>,
    swatch: Option<Color>,
    is_dangerous: bool,
) {
    let gutter_width = em(3.5);

    ui.horizontal(|ui| {
        ui.caption(label)
            .width(if swatch.is_none() && !is_dangerous {
                gutter_width.try_add(em(1)).expect("Add Val")
            } else {
                gutter_width
            });
        match swatch {
            Some(color) => {
                ui.color_swatch(color).size(em(1).into());
            }
            None => {
                if is_dangerous {
                    ui.icon(font_awesome::solid::TRIANGLE_EXCLAMATION)
                        .text_color_slot(ThemeSlot::Danger0)
                        .width(em(1))
                        .tooltip("This hue/chroma is very close to the 'Danger' color");
                } else {
                    ui.space(Val::ZERO); // keeps the child/gap count
                }
            }
        }
        ui.slider(value, range.clone()).grow().precision(precision);
        let number = ui.number(value).range(range).precision(precision);
        if let Some(suffix) = suffix {
            number.suffix(suffix);
        }
    });
}
