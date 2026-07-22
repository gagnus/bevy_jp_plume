//! Imm-mode theme editor: the smoke test's retained palette dialog, rebuilt on the
//! immediate-mode surface. Each control binds straight to a palette field; edits bake
//! into the live `UiTheme` every frame the palette changes.
use bevy::prelude::*;
use bevy_jp_plume::{
    PlumePlugins,
    controls::ButtonVariant,
    dark_theme::default_dark_palette,
    imm::{PlumeImm, PlumeRoot, Ui},
    light_theme::default_light_palette,
    theme::{OklchaArray, ThemeEditablePalette, UiTheme},
};

#[path = "common/mod.rs"]
mod common;

/// Width of the label gutter every ramp row aligns to.
const GUTTER: f32 = 64.0;
/// Side of the lightness-stop swatch, and of the gap that keeps swatchless rows aligned.
const SWATCH: f32 = 16.0;

#[derive(Resource, Clone, Debug, PartialEq)]
struct ThemeEditor {
    open: bool,
    palette: ThemeEditablePalette,
}

impl ThemeEditor {
    fn new(light: bool) -> Self {
        let palette = if light {
            default_light_palette()
        } else {
            default_dark_palette()
        };
        Self {
            open: true,
            palette,
        }
    }
}

fn main() {
    let mut app = App::new();
    // The editor seeds the theme, so `--light` has to seed the editor, not just `UiTheme`.
    let light = std::env::args().any(|arg| arg == "--light");
    app.add_plugins((DefaultPlugins, PlumePlugins))
        .insert_resource(ThemeEditor::new(light))
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(
            Update,
            (
                theme_editor_ui,
                rebuild_theme,
                common::log_on_change::<ThemeEditor>,
            )
                .chain(),
        );
    common::apply_args(&mut app);
    app.run();
}

fn rebuild_theme(editor: Res<ThemeEditor>, mut theme: ResMut<UiTheme>) {
    if editor.is_changed() {
        theme.set_palette(&editor.palette);
    }
}

fn theme_editor_ui(mut root: PlumeRoot, mut editor: ResMut<ThemeEditor>) {
    // Build against a clone and write back with `set_if_neq`, so the resource only
    // registers as changed when a control actually moved.
    let mut s = editor.clone();

    root.screen(|ui| {
        ui.horizontal(|ui| {
            if ui.button("Theme Editor").enabled(!s.open).clicked {
                s.open = true;
            }
        });
    });

    let mut open = s.open;
    root.dialog("Theme Editor", &mut open)
        .width(px(320))
        .max_height(px(600))
        .at(px(320), px(40))
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
            });

            ui.section("Neutrals", |ui| ramp_rows(ui, &mut s.palette.neutrals, 0.2))
                .collapsible(false);
            ui.section("Accent", |ui| ramp_rows(ui, &mut s.palette.accent, 0.5))
                .collapsible(false);
            ui.section("Text", |ui| {
                ramp_rows(ui, &mut s.palette.text, 0.2);
                param_row(
                    ui,
                    "Disabled \u{3b1}",
                    &mut s.palette.disabled_text_alpha_modifier,
                    0.0..=1.0,
                    3,
                    None,
                    None,
                );
            })
            .collapsible(false);
        });
    s.open = open;
    editor.set_if_neq(s);
}

/// Hue, chroma, then one lightness row per stop. `chroma_max` keeps neutral/text
/// ramps near-grey while the accent ramp reaches full saturation. Each lightness row
/// previews its stop colour; hue/chroma rows reserve the same slot so sliders line up.
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

/// A colour swatch (or a reserved gap), a label, a slider and a number input, all bound
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
    ui.horizontal(|ui| {
        ui.caption(label)
            .width(px(GUTTER + if swatch.is_none() { SWATCH } else { 0.0 }));
        match swatch {
            Some(color) => {
                ui.color_swatch(color).square(px(SWATCH));
            }
            None => ui.space(px(0)), // keeps the child/gap count
        }
        ui.slider(value, range.clone()).grow().precision(precision);
        let number = ui.number(value).range(range).precision(precision);
        if let Some(suffix) = suffix {
            number.suffix(suffix);
        }
    });
}
