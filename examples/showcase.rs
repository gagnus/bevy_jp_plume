//! Combined showcase: the gallery as a full-screen backdrop with every feature dialog
//! floating over it, each the same plugin its standalone example mounts.
#[path = "common/mod.rs"]
mod common;

use common::audio_settings::AudioSettingsPlugin;
use common::debug_settings::DebugSettingsPlugin;
use common::player_profile::PlayerProfilePlugin;
use common::theme_editor::ThemeEditorPlugin;

fn main() {
    let mut app = common::demo_app(true);
    app.add_plugins((
        AudioSettingsPlugin(false),
        DebugSettingsPlugin(false),
        PlayerProfilePlugin(false),
        ThemeEditorPlugin(false),
    ));
    app.run();
}
