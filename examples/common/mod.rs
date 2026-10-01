//! Scaffolding shared by the imm acceptance examples: change logging, screenshot
//! verification, theme selection, and the feature modules each example mounts.
// Every example bin compiles all of `common` but mounts only some feature modules, so
// the rest are unavoidably dead code in that bin.
#![allow(dead_code)]
use std::path::PathBuf;

use bevy::prelude::*;
use bevy_plume::prelude::*;
use bevy_plume::theme::palettes;

use crate::common::debug_hub::DebugDialogRegistry;
use crate::common::gallery::GalleryPlugin;

pub mod audio_settings;
pub mod debug_hub;
pub mod debug_settings;
pub mod gallery;
pub mod inspector_panel;
pub mod player_profile;
pub mod theme_editor;

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
// `argh` reads the description and every flag's help text from `///`, so these
// stay doc comments despite the type being private.
#[derive(argh::FromArgs, Debug)]
struct ExampleArgs {
    /// save a screenshot here once the UI has settled, then exit
    #[argh(option)]
    screenshot: Option<PathBuf>,

    /// show the gallery
    #[argh(switch)]
    gallery: bool,

    /// don't show the gallery
    #[argh(switch)]
    no_gallery: bool,

    /// open all dialogs at start, ignoring the examples preference
    #[argh(switch)]
    all_open: bool,

    /// default RemSize
    #[argh(option)]
    rem_size: Option<f32>,

    /// build the UI with the light theme instead of the default dark one
    #[argh(switch)]
    light: bool,

    /// set theme hue
    #[argh(option)]
    theme_hue: Option<f32>,

    /// set theme to have a complementary neutral hue
    #[argh(switch)]
    theme_complementary_neutral: bool,

    /// set theme to NOT have a complementary neutral hue
    #[argh(switch)]
    no_theme_complementary_neutral: bool,

    /// set theme to have a boosted neutral chroma
    #[argh(switch)]
    theme_boosted_neutral_chroma: bool,

    /// set theme to NOT have a boosted neutral chroma
    #[argh(switch)]
    no_theme_boosted_neutral_chroma: bool,
}

/// Print the backing resource whenever it changes, to confirm every control
/// round-trips (and that nothing writes while idle).
pub fn log_on_change<R: Resource + core::fmt::Debug>(res: Res<R>) {
    if res.is_changed() {
        info!("{:?}", *res);
    }
}

/// An enum offered as select options: imm binds the value, retained binds the index.
pub trait Options: Copy + PartialEq + Sized + 'static {
    /// Every option, in the order they appear in the dropdown.
    const ALL: &'static [Self];

    /// The option's dropdown label.
    fn label(self) -> &'static str;

    /// Whether the option can be picked.
    fn enabled(self) -> bool {
        true
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|&o| o == self).unwrap_or(0)
    }

    fn from_index(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or(Self::ALL[0])
    }

    /// Labels paired with their enabled flag, as [`PlumeSelect`] takes them.
    fn select_options() -> Vec<(String, bool)> {
        Self::ALL
            .iter()
            .map(|&o| (o.label().to_owned(), o.enabled()))
            .collect()
    }
}

/// Apply the example command line. `--light` overwrites the [`UiTheme`] that
/// `PlumePlugins` installed, so this has to run after the plugins are added.
pub fn apply_args(app: &mut App, default_gallery: bool) {
    fn reconcile(yes: bool, no: bool, default: bool) -> bool {
        (default && !no) || yes
    }

    let args: ExampleArgs = argh::from_env();

    if reconcile(args.gallery, args.no_gallery, default_gallery) {
        app.add_plugins(GalleryPlugin);
    }

    if args.light {
        app.insert_resource(UiTheme::from(palettes::light_palette(
            args.theme_hue.unwrap_or(palettes::DEFAULT_LIGHT_HUE),
            reconcile(
                args.theme_complementary_neutral,
                args.no_theme_complementary_neutral,
                palettes::DEFAULT_LIGHT_COMPLEMENTARY_NEUTRAL,
            ),
            reconcile(
                args.theme_boosted_neutral_chroma,
                args.no_theme_boosted_neutral_chroma,
                false,
            ),
        )));
    } else {
        app.insert_resource(UiTheme::from(palettes::dark_palette(
            args.theme_hue.unwrap_or(palettes::DEFAULT_DARK_HUE),
            reconcile(
                args.theme_complementary_neutral,
                args.no_theme_complementary_neutral,
                palettes::DEFAULT_DARK_COMPLEMENTARY_NEUTRAL,
            ),
            reconcile(
                args.theme_boosted_neutral_chroma,
                args.no_theme_boosted_neutral_chroma,
                false,
            ),
        )));
    }

    if let Some(path) = args.screenshot {
        app.add_systems(Update, screenshot_and_exit(path));
    }

    if args.all_open {
        app.world_mut()
            .resource_scope(|_, mut registry: Mut<DebugDialogRegistry>| {
                registry.set_all_open();
            });
    }

    if let Some(rem_size) = args.rem_size {
        app.insert_resource(RemSize(rem_size));
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
