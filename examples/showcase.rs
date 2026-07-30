//! Combined showcase: the control gallery as a full-screen backdrop with every feature
//! dialog floating over it, each toggled from the hub toolbar. Every dialog is the same
//! plugin its standalone example mounts — this one just mounts them all at once.
#[path = "common/mod.rs"]
mod common;

use common::{
    audio_settings::AudioSettingsPlugin, debug_settings::DebugSettingsPlugin,
    font_scaling::FontScalingPlugin, player_profile::PlayerProfilePlugin,
    theme_editor::ThemeEditorPlugin, tree_view::TreeViewPlugin,
};

fn main() {
    let mut app = common::demo_app(true);
    app.add_plugins((
        AudioSettingsPlugin(false),
        DebugSettingsPlugin(false),
        FontScalingPlugin(false),
        PlayerProfilePlugin(false),
        ThemeEditorPlugin(false),
        TreeViewPlugin(false),
    ));
    app.run();
}
