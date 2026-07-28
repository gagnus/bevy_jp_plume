//! Audio-settings dialog as a self-contained feature plugin: registers its resource,
//! its dialog (with the hub), and its live change log.
use bevy::prelude::*;
use bevy_jp_plume::prelude::*;

use super::debug_hub::{AddDebugDialog, DebugDialogRegistry};
use super::log_on_change;

const TITLE: &str = "Audio Settings";

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct AudioSettings {
    volume: f32,
    muted: bool,
    output: Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Output {
    #[default]
    Speakers,
    Headphones,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            volume: 0.5,
            muted: false,
            output: Output::default(),
        }
    }
}

/// Adds the audio dialog: its resource, its dialog system + hub entry, and its log.
pub struct AudioSettingsPlugin(pub bool);

impl Plugin for AudioSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioSettings>()
            .add_debug_dialog(
                TITLE,
                font_awesome::solid::VOLUME,
                self.0,
                audio_settings_dialog,
            )
            .add_systems(Update, log_on_change::<AudioSettings>);
    }
}

fn audio_settings_dialog(
    mut root: PlumeRoot,
    mut registry: ResMut<DebugDialogRegistry>,
    mut settings: ResMut<AudioSettings>,
) {
    // Build against a clone and write back with `set_if_neq`, so the resource only
    // registers as changed when a control actually changed it.
    let mut s = settings.clone();
    let mut open = registry.is_open(TITLE);
    root.dialog(TITLE, &mut open)
        .at(px(60), px(80))
        .icon(font_awesome::solid::VOLUME)
        .show(|ui| {
            ui.horizontal(|ui| {
                ui.caption("Volume");
                ui.slider(&mut s.volume, 0.0..=1.0).enabled(!s.muted);
                ui.number(&mut s.volume).enabled(!s.muted);
            });

            ui.checkbox(&mut s.muted, "Mute");
            ui.select(&mut s.output, |select| {
                select.option(Output::Speakers, "Speakers");
                select.option(Output::Headphones, "Headphones");
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.flex_spacer();
                if ui.button("Reset").clicked {
                    s = AudioSettings::default();
                }
            });
        });
    if open != registry.is_open(TITLE) {
        registry.set_open(TITLE, open);
    }
    settings.set_if_neq(s);
}
