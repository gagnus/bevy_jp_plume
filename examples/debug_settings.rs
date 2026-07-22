//! The debug-menu dialog on its own — a thin mount of `common::debug_settings`. The
//! same plugin is one of several the combined `showcase` example brings together.
#[path = "common/mod.rs"]
mod common;

fn main() {
    let mut app = common::demo_app(false);
    app.add_plugins(common::debug_settings::DebugSettingsPlugin(true));
    app.run();
}
