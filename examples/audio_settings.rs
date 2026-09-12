//! The audio-settings dialog on its own - a thin mount of `common::audio`. The same
//! plugin is one of several the combined `showcase` example brings together.
#[path = "common/mod.rs"]
mod common;

fn main() {
    let mut app = common::demo_app(false);
    app.add_plugins(common::audio_settings::AudioSettingsPlugin(true));
    app.run();
}
