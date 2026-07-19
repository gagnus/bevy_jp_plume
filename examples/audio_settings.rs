//! Pass-1 acceptance example: one immediate-mode system drives an audio-settings
//! dialog with zero app-side wiring, backed by one resource, printed live, with
//! Reset wired to `Default`.
use bevy::prelude::*;
use bevy_jp_plume::{
    PlumePlugins,
    imm::{PlumeImm, PlumeUi},
};

#[path = "common/mod.rs"]
mod common;

#[derive(Resource, Debug, Clone, PartialEq)]
struct AudioSettings {
    open: bool,
    volume: f32,
    muted: bool,
    output: usize,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            open: true,
            volume: 0.5,
            muted: false,
            output: 0,
        }
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, PlumePlugins))
        .init_resource::<AudioSettings>()
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(
            Update,
            (audio_settings_ui, common::log_on_change::<AudioSettings>),
        );
    common::screenshot_on_arg(&mut app);
    app.run();
}

fn audio_settings_ui(mut ui: PlumeUi, mut settings: ResMut<AudioSettings>) {
    // Build the UI against a local clone and write back with `set_if_neq`, so the
    // resource only registers as changed when a control actually changed it.
    let mut s = settings.clone();

    if ui.button("Audio Settings").enabled(!s.open).clicked {
        s.open = true;
    }

    // Local copy dodges the borrow conflict between `&mut open` and the fields.
    let mut open = s.open;
    ui.dialog("Audio", &mut open).show(|ui| {
        ui.horizontal(|ui| {
            ui.caption("Volume");
            ui.slider(&mut s.volume, 0.0..=1.0).enabled(!s.muted);
            ui.number(&mut s.volume).enabled(!s.muted);
        });

        ui.checkbox(&mut s.muted, "Mute");
        ui.select(&mut s.output, &["Speakers", "Headphones"]);
        ui.separator();

        ui.horizontal(|ui| {
            ui.flex_spacer();
            if ui.button("Reset").clicked {
                let open = s.open;
                s = AudioSettings { open, ..default() };
            }
        });
    });
    s.open = open;
    settings.set_if_neq(s);
}
