//! Scaffolding shared by the imm acceptance examples: resource change logging,
//! headless screenshot verification, theme selection, and the feature modules each
//! example (and the combined `showcase`) mounts as a plugin.
// Every example bin compiles all of `common` but mounts only some feature modules, so
// the rest are unavoidably dead code in that bin.
#![allow(dead_code)]
use std::path::PathBuf;

use bevy::prelude::*;
use bevy_jp_plume::{PlumePlugins, light_theme, theme::UiTheme};

use crate::common::gallery::GalleryPlugin;

pub mod audio_settings;
pub mod debug_hub;
pub mod debug_settings;
pub mod gallery;
pub mod player_profile;
pub mod theme_editor;
pub mod tree_view;

/// Base app every example builds on: default + plume plugins, a camera, the dialog hub,
/// and the shared command-line handling. Callers add their feature plugin(s) and `run`.
pub fn demo_app(default_gallery: bool) -> App {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, PlumePlugins))
        .add_plugins(debug_hub::DebugDialogHubPlugin)
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        });
    apply_args(&mut app, default_gallery);
    app
}

/// Command-line options accepted by every example.
#[derive(argh::FromArgs, Debug)]
struct ExampleArgs {
    /// save a screenshot here once the UI has settled, then exit
    #[argh(option)]
    screenshot: Option<PathBuf>,

    /// build the UI with the light theme instead of the default dark one
    #[argh(switch)]
    light: bool,

    /// show the gallery
    #[argh(switch)]
    gallery: bool,

    /// don't show the gallery
    #[argh(switch)]
    no_gallery: bool,
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
pub fn apply_args(app: &mut App, default_gallery: bool) {
    let args: ExampleArgs = argh::from_env();

    if (default_gallery && !args.no_gallery) || args.gallery {
        app.add_plugins(GalleryPlugin);
    }

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
