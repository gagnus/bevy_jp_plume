//! Scaffolding shared by the imm acceptance examples: resource change logging
//! and headless screenshot verification.
use bevy::prelude::*;

/// Print the backing resource whenever it changes, to confirm every control
/// round-trips (and that nothing writes while idle).
pub fn log_on_change<R: Resource + core::fmt::Debug>(res: Res<R>) {
    if res.is_changed() {
        info!("{:?}", *res);
    }
}

/// Headless verification: with a PNG path as the first program argument
/// (`cargo run --example … -- shot.png`), save a screenshot once the UI has
/// settled and exit. Without the argument the app runs normally.
pub fn screenshot_on_arg(app: &mut App) {
    if screenshot_path().is_some() {
        app.add_systems(Update, screenshot_and_exit);
    }
}

fn screenshot_path() -> Option<String> {
    std::env::args().nth(1)
}

fn screenshot_and_exit(
    mut frames: Local<u32>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    *frames += 1;
    if *frames == 200
        && let Some(path) = screenshot_path()
    {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(std::path::PathBuf::from(path)));
    }
    if *frames == 260 {
        exit.write(AppExit::Success);
    }
}
