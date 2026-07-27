//! Theme-editor dialog as a self-contained feature plugin: hosts plume's own
//! `theme_editor` fill-fn and bakes its palette into the live `UiTheme`.
use bevy::prelude::*;
use bevy_jp_plume::{
    constants::font_awesome,
    dark_theme::default_dark_palette,
    imm::PlumeRoot,
    light_theme::default_light_palette,
    theme::{ThemeEditablePalette, UiTheme, theme_editor},
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
    let mut local = editor.clone();
    let mut open = registry.is_open(TITLE);
    root.dialog(TITLE, &mut open)
        .width(px(320))
        .max_height(px(500))
        .at(px(900), px(90))
        .icon(font_awesome::solid::PALETTE)
        .show(|ui| theme_editor(ui, &mut local.palette));
    if open != registry.is_open(TITLE) {
        registry.set_open(TITLE, open);
    }
    editor.set_if_neq(local);
}
