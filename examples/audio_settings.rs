//! Pass-1 acceptance example: one immediate-mode system drives an audio-settings
//! dialog with zero app-side wiring.
use bevy::prelude::*;
use bevy_jp_plume::{
    PlumePlugins,
    imm::{PlumeImm, PlumeUi},
};

#[derive(Resource, Debug)]
struct AudioSettings {
    open: bool,
    volume: f32,
    muted: bool,
    output: usize,
}

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, PlumePlugins))
        .insert_resource(AudioSettings {
            open: true,
            volume: 0.5,
            muted: false,
            output: 0,
        })
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(Update, audio_settings_ui);
    // AUDIO_SHOT=<path.png>: save a screenshot and exit (for headless verification).
    if std::env::var_os("AUDIO_SHOT").is_some() {
        app.add_systems(Update, screenshot_and_exit);
    }
    app.run();
}

fn screenshot_and_exit(
    mut frames: Local<u32>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    *frames += 1;
    if *frames == 200
        && let Some(path) = std::env::var_os("AUDIO_SHOT")
    {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(std::path::PathBuf::from(path)));
    }
    if *frames == 260 {
        exit.write(AppExit::Success);
    }
}

fn audio_settings_ui(mut ui: PlumeUi, mut settings: ResMut<AudioSettings>) {
    let settings_str = format!("{:?}", settings);

    let AudioSettings {
        open,
        volume,
        muted,
        output,
    } = &mut *settings;

    if ui.button("Audio Settings").enabled(!*open).clicked {
        *open = true;
    }

    // TODO: doesn't currently use font as global scope isn't complete
    ui.caption(&settings_str);

    ui.dialog("Audio", open, |ui| {
        ui.horizontal(|ui| {
            ui.caption("Volume");
            ui.slider(volume, 0.0..=1.0).enabled(!*muted);
            ui.number(volume).enabled(!*muted);
        });

        ui.checkbox(muted, "Mute");
        ui.select(output, &["Speakers", "Headphones"]);
        ui.separator();

        ui.horizontal(|ui| {
            ui.caption("Hello world").grow();
            if ui.button("Reset").clicked {
                *volume = 0.5;
                *muted = false;
                *output = 0;
            }
        });
    });
}
