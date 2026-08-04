//! Fill-fn editing a [`ThemeEditablePalette`]: preset buttons plus a slider per
//! ramp parameter.

use core::ops::RangeInclusive;

use bevy::color::Color;
use bevy::ui::Val;

use crate::constants::size;
use crate::controls::ButtonVariant;
use crate::imm::{PlumeImm, Ui};
use crate::theme::dark_theme::default_dark_palette;
use crate::theme::light_theme::default_light_palette;
use crate::theme::{OklchaArray, ThemeEditablePalette};

/// Palette editor (presets, neutral/accent/text ramps) as a fill-fn for your own
/// container. Mutates `palette` in place; bake it into the live theme with
/// [`UiTheme::set_palette`](crate::theme::UiTheme::set_palette) when it changes.
pub fn theme_editor(ui: &mut Ui, palette: &mut ThemeEditablePalette) {
    ui.horizontal(|ui| {
        if ui
            .button("Reset Dark")
            .variant(ButtonVariant::Outline)
            .grow()
            .clicked
        {
            *palette = default_dark_palette();
        }
        if ui
            .button("Reset Light")
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

    ui.section("Neutrals", |ui| ramp_rows(ui, &mut palette.neutrals, 0.2))
        .collapsible(false);
    ui.section("Accent", |ui| ramp_rows(ui, &mut palette.accent, 0.5))
        .collapsible(false);
    ui.section("Danger", |ui| danger_row(ui, palette))
        .collapsible(false);
    ui.section("Text", |ui| {
        ramp_rows(ui, &mut palette.text, 0.2);
        param_row(
            ui,
            "Disable \u{3b1}",
            &mut palette.disabled_text_alpha_modifier,
            0.0..=1.0,
            3,
            None,
            None,
        );
    })
    .collapsible(false);
}

// The danger ramp has no editable inputs — fixed hue and chroma over the accent's
// lightnesses — so it gets swatches only, to show it tracking the accent sliders above.
fn danger_row(ui: &mut Ui, palette: &ThemeEditablePalette) {
    ui.horizontal(|ui| {
        ui.caption("Derived");
        for stop in 0..3 {
            ui.color_swatch(palette.danger(stop))
                .square(size::TEXT_HEIGHT);
        }
    });
}

// Hue, chroma, then one lightness row per stop. `chroma_max` keeps neutral/text
// ramps near-gray while the accent ramp reaches full saturation. Each lightness row
// previews its stop color; hue/chroma rows reserve the same slot so sliders line up.
fn ramp_rows<const N: usize>(ui: &mut Ui, ramp: &mut OklchaArray<N>, chroma_max: f32) {
    param_row(
        ui,
        "Hue",
        &mut ramp.hue,
        0.0..=360.0,
        0,
        Some("\u{b0}"),
        None,
    );
    param_row(
        ui,
        "Chroma",
        &mut ramp.chroma,
        0.0..=chroma_max,
        3,
        None,
        None,
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
) {
    let gutter_width = size::TEXT_HEIGHT * 3.5;

    ui.horizontal(|ui| {
        ui.caption(label).width(if swatch.is_none() {
            gutter_width.try_add(size::TEXT_HEIGHT).expect("Add Val")
        } else {
            gutter_width
        });
        match swatch {
            Some(color) => {
                ui.color_swatch(color).square(size::TEXT_HEIGHT);
            }
            None => ui.space(Val::ZERO), // keeps the child/gap count
        }
        ui.slider(value, range.clone()).grow().precision(precision);
        let number = ui.number(value).range(range).precision(precision);
        if let Some(suffix) = suffix {
            number.suffix(suffix);
        }
    });
}
