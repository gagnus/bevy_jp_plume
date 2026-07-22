//! Combined showcase: the control gallery as a full-screen backdrop with every feature
//! dialog floating over it, each toggled from the hub toolbar. Every dialog is the same
//! plugin its standalone example mounts — this one just mounts them all at once.
#[path = "common/mod.rs"]
mod common;

use common::{
    audio_settings::AudioSettingsPlugin, debug_settings::DebugSettingsPlugin,
    gallery::GalleryPlugin, player_profile::PlayerProfilePlugin, theme_editor::ThemeEditorPlugin,
};

fn main() {
    let mut app = common::demo_app();
    app.add_plugins((
        GalleryPlugin,
        AudioSettingsPlugin,
        DebugSettingsPlugin,
        PlayerProfilePlugin,
        ThemeEditorPlugin,
    ));
    app.run();
}
