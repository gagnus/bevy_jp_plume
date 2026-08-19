//! Theme-editor dialog as a self-contained feature plugin: hosts plume's
//! tabbed `theme_editor_tabs`, editing the live `UiTheme` in place.
use bevy::prelude::*;
use bevy_jp_plume::prelude::*;
use bevy_jp_plume::theme::theme_editor_tabs;

use super::debug_hub::{AddDebugDialog, DebugDialogRegistry};

const TITLE: &str = "Theme Editor";

/// Adds the theme-editor dialog and registers its dialog + hub entry.
pub struct ThemeEditorPlugin(pub bool);

impl Plugin for ThemeEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_debug_dialog(
            TITLE,
            font_awesome::solid::PALETTE,
            self.0,
            theme_editor_dialog,
        );
    }
}

// Which theme's tab the editor shows.
#[derive(Resource, Default)]
struct SelectedTheme(ThemeId);

fn theme_editor_dialog(
    mut root: PlumeRoot,
    mut registry: ResMut<DebugDialogRegistry>,
    mut theme: ResMut<UiTheme>,
    mut selected: Local<SelectedTheme>,
) {
    let mut open = registry.is_open(TITLE);
    let mut edited = false;
    root.dialog(TITLE, &mut open)
        .width(em(23))
        .max_height(px(500))
        .at_corner(Corner::BottomRight, px(50), px(50))
        .show(|ui| {
            edited = theme_editor_tabs(ui, theme.bypass_change_detection(), &mut selected.0);
        });
    if edited {
        theme.set_changed();
    }
    if open != registry.is_open(TITLE) {
        registry.set_open(TITLE, open);
    }
}
