//! Combined showcase: the gallery as a full-screen backdrop with every feature dialog
//! floating over it, each the same plugin its standalone example mounts.
#[path = "common/mod.rs"]
mod common;

use common::{
    audio_settings::AudioSettingsPlugin, debug_settings::DebugSettingsPlugin,
    player_profile::PlayerProfilePlugin, theme_editor::ThemeEditorPlugin,
    tree_view::TreeViewPlugin,
};

fn main() {
    let mut app = common::demo_app(true);
    app.add_plugins((
        AudioSettingsPlugin(false),
        DebugSettingsPlugin(false),
        PlayerProfilePlugin(false),
        ThemeEditorPlugin(false),
        TreeViewPlugin(false),
    ));
    app.run();
}
