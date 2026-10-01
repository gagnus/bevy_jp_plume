//! Theme-editor dialog as a self-contained feature plugin: hosts plume's
//! tabbed `theme_editor_tabs`, editing the live `UiTheme` in place.
use bevy::prelude::*;
use bevy_plume::prelude::*;
use bevy_plume::theme::theme_editor_tabs;

use super::debug_hub::{AddDebugDialog, DebugDialogRegistry};

const TITLE: &str = "Theme Editor";

/// Adds the theme-editor dialog and registers its dialog + hub entry.
pub struct ThemeEditorPlugin(pub bool);

impl Plugin for ThemeEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_debug_dialog(TITLE, lucide::PALETTE, self.0, theme_editor_dialog);
    }
}

fn theme_editor_dialog(
    mut root: PlumeRoot,
    mut registry: ResMut<DebugDialogRegistry>,
    mut theme: ResMut<UiTheme>,
    mut selected: Local<ThemeId>,
) {
    let mut open = registry.is_open(TITLE);
    root.dialog(TITLE, &mut open)
        .width(em(23))
        .max_height(px(500))
        .at_corner(Corner::BottomRight, em(1), em(1))
        .show(|ui| theme_editor_tabs(ui, &mut theme, &mut selected));
    if open != registry.is_open(TITLE) {
        registry.set_open(TITLE, open);
    }
}
