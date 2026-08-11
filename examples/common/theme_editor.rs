//! Theme-editor dialog as a self-contained feature plugin: hosts plume's own
//! `theme_editor` fill-fn and bakes its palette into the live `UiTheme`.
use bevy::prelude::*;
use bevy_jp_plume::prelude::*;
use bevy_jp_plume::theme::{ThemeEditablePalette, theme_editor};

use super::debug_hub::{AddDebugDialog, DebugDialogRegistry};
use super::log_on_change;

const TITLE: &str = "Theme Editor";

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ThemePaletteEditor(ThemeEditablePalette);

/// Adds the theme-editor dialog: seeds its palette resource from `UiTheme`, bakes
/// edits back into `UiTheme`, and registers its dialog + hub entry and log.
pub struct ThemeEditorPlugin(pub bool);

impl Plugin for ThemeEditorPlugin {
    fn build(&self, app: &mut App) {
        let editable_palette = app.world().resource::<UiTheme>().editable().clone();

        app.insert_resource(ThemePaletteEditor(editable_palette))
            .add_debug_dialog(
                TITLE,
                font_awesome::solid::PALETTE,
                self.0,
                theme_editor_dialog,
            )
            .add_systems(Update, rebuild_theme.after(theme_editor_dialog))
            .add_systems(Update, log_on_change::<ThemePaletteEditor>);
    }
}

fn rebuild_theme(editor: Res<ThemePaletteEditor>, mut theme: ResMut<UiTheme>) {
    if editor.is_changed() {
        theme.set_palette(&editor.0);
    }
}

fn theme_editor_dialog(
    mut root: PlumeRoot,
    mut registry: ResMut<DebugDialogRegistry>,
    mut editor: ResMut<ThemePaletteEditor>,
) {
    let mut local = editor.clone();
    let mut open = registry.is_open(TITLE);
    root.dialog(TITLE, &mut open)
        .width(em(23))
        .max_height(px(500))
        .at_corner(Corner::BottomRight, px(50), px(50))
        .show(|ui| theme_editor(ui, &mut local.0));
    if open != registry.is_open(TITLE) {
        registry.set_open(TITLE, open);
    }
    editor.set_if_neq(local);
}
