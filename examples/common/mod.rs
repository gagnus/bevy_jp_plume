//! Scaffolding shared by the imm acceptance examples: resource change logging,
//! headless screenshot verification and theme selection.
use std::path::PathBuf;

use bevy::prelude::*;
use bevy_jp_plume::{light_theme, theme::UiTheme};

/// Command-line options accepted by every example.
#[derive(argh::FromArgs)]
struct ExampleArgs {
    /// save a screenshot here once the UI has settled, then exit
    #[argh(option)]
    screenshot: Option<PathBuf>,

    /// build the UI with the light theme instead of the default dark one
    #[argh(switch)]
    light: bool,
}

/// Print the backing resource whenever it changes, to confirm every control
/// round-trips (and that nothing writes while idle).
pub fn log_on_change<R: Resource + core::fmt::Debug>(res: Res<R>) {
    if res.is_changed() {
        info!("{:?}", *res);
    }
}

/// Apply the example command line:
///
/// - `--screenshot shot.png` (or `--screenshot=shot.png`): headless verification —
///   save a PNG once the UI has settled, then exit. Without it the app runs normally.
/// - `--light`: swap in the light palette. `PlumePlugins` installs the dark one, so
///   this overwrites [`UiTheme`] and must be called after the plugins are added.
pub fn apply_args(app: &mut App) {
    let args: ExampleArgs = argh::from_env();

    if args.light {
        let mut theme = UiTheme::default();
        theme.set_palette(&light_theme::default_light_palette());
        app.insert_resource(theme);
    }

    if let Some(path) = args.screenshot {
        app.add_systems(Update, screenshot_and_exit(path));
    }
}

fn screenshot_and_exit(path: PathBuf) -> impl FnMut(Local<u32>, Commands, MessageWriter<AppExit>) {
    move |mut frames: Local<u32>, mut commands: Commands, mut exit: MessageWriter<AppExit>| {
        use bevy::render::view::screenshot::{Screenshot, save_to_disk};
        *frames += 1;
        if *frames == 200 {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path.clone()));
        }
        if *frames == 260 {
            exit.write(AppExit::Success);
        }
    }
}
