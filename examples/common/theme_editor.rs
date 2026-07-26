//! Theme-editor dialog as a self-contained feature plugin: each control binds straight
//! to a palette field; edits bake into the live `UiTheme` every frame the palette moves.
use bevy::prelude::*;
use bevy_jp_plume::{
    constants::{font_awesome, size},
    controls::ButtonVariant,
    dark_theme::default_dark_palette,
    imm::{PlumeImm, PlumeRoot, Ui},
    light_theme::default_light_palette,
    theme::{OklchaArray, ThemeEditablePalette, UiTheme},
};

use super::debug_hub::{AddDebugDialog, DebugDialogRegistry};
use super::log_on_change;

const TITLE: &str = "Theme Editor";

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ThemeEditor {
    palette: ThemeEditablePalette,
}

impl ThemeEditor {
    fn new(light: bool) -> Self {
        let palette = if light {
            default_light_palette()
        } else {
            default_dark_palette()
        };
        Self { palette }
    }
}

/// Adds the theme-editor dialog: seeds its palette resource (matching `--light`), bakes
/// edits into `UiTheme`, and registers its dialog + hub entry and log.
pub struct ThemeEditorPlugin(pub bool);

impl Plugin for ThemeEditorPlugin {
    fn build(&self, app: &mut App) {
        // The editor seeds the theme, so `--light` has to seed the editor, not just `UiTheme`.
        let light = std::env::args().any(|arg| arg == "--light");
        app.insert_resource(ThemeEditor::new(light))
            .add_debug_dialog(
                TITLE,
                font_awesome::solid::PALETTE,
                self.0,
                theme_editor_dialog,
            )
            .add_systems(Update, rebuild_theme.after(theme_editor_dialog))
            .add_systems(Update, log_on_change::<ThemeEditor>);
    }
}

fn rebuild_theme(editor: Res<ThemeEditor>, mut theme: ResMut<UiTheme>) {
    if editor.is_changed() {
        theme.set_palette(&editor.palette);
    }
}

fn theme_editor_dialog(
    mut root: PlumeRoot,
    mut registry: ResMut<DebugDialogRegistry>,
    mut editor: ResMut<ThemeEditor>,
) {
    // Build against a clone and write back with `set_if_neq`, so the resource only
    // registers as changed when a control actually moved.
    let mut s = editor.clone();
    let mut open = registry.is_open(TITLE);
    root.dialog(TITLE, &mut open)
        .width(px(320))
        .at(px(900), px(90))
        .icon(font_awesome::solid::PALETTE)
        .show(|ui| {
            ui.horizontal(|ui| {
                if ui
                    .button("Reset Dark")
                    .variant(ButtonVariant::Outline)
                    .grow()
                    .clicked
                {
                    s.palette = default_dark_palette();
                }
                if ui
                    .button("Reset Light")
                    .variant(ButtonVariant::Outline)
                    .grow()
                    .clicked
                {
                    s.palette = default_light_palette();
                }
                if ui
                    .button("Random")
                    .variant(ButtonVariant::Outline)
                    .grow()
                    .clicked
                {
                    s.palette = ThemeEditablePalette::random();
                }
            });

            ui.scroll_area(|ui| {
                ui.section("Neutrals", |ui| ramp_rows(ui, &mut s.palette.neutrals, 0.2))
                    .collapsible(false);
                ui.section("Accent", |ui| ramp_rows(ui, &mut s.palette.accent, 0.5))
                    .collapsible(false);
                ui.section("Text", |ui| {
                    ramp_rows(ui, &mut s.palette.text, 0.2);
                    param_row(
                        ui,
                        "Disable \u{3b1}",
                        &mut s.palette.disabled_text_alpha_modifier,
                        0.0..=1.0,
                        3,
                        None,
                        None,
                    );
                })
                .collapsible(false);
            })
            .max_height(px(500));
        });
    if open != registry.is_open(TITLE) {
        registry.set_open(TITLE, open);
    }
    editor.set_if_neq(s);
}

/// Hue, chroma, then one lightness row per stop. `chroma_max` keeps neutral/text
/// ramps near-grey while the accent ramp reaches full saturation. Each lightness row
/// previews its stop color; hue/chroma rows reserve the same slot so sliders line up.
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
    for (i, l) in ramp.l.iter_mut().enumerate() {
        let swatch = Color::oklcha(*l, chroma, hue, 1.0);
        param_row(ui, &format!("L{i}"), l, 0.0..=1.0, 3, None, Some(swatch));
    }
}

/// A color swatch (or a reserved gap), a label, a slider and a number input, all bound
/// to one palette scalar.
fn param_row(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
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
