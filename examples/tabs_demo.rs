//! Demonstrates `ui.tabs`: a value-keyed tab container whose strip animates an
//! accent underline to the selected tab, showing one tab's body at a time.
use bevy::prelude::*;
use bevy_jp_plume::prelude::*;

#[path = "common/mod.rs"]
mod common;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Tab {
    #[default]
    General,
    Video,
    Audio,
    Advanced,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Quality {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum WindowMode {
    #[default]
    Windowed,
    Borderless,
    Fullscreen,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Accent {
    #[default]
    Crimson,
    Teal,
    Amber,
}

impl Accent {
    fn color(self) -> Color {
        match self {
            Accent::Crimson => Color::srgb(0.82, 0.24, 0.29),
            Accent::Teal => Color::srgb(0.16, 0.60, 0.58),
            Accent::Amber => Color::srgb(0.88, 0.66, 0.20),
        }
    }
}

#[derive(Resource, Debug, Clone, Default, PartialEq)]
struct DemoSettings {
    tab: Tab,
    hints: bool,
    player_name: String,
    accent: Accent,
    window_mode: WindowMode,
    gamma: f32,
    quality: Quality,
    muted: bool,
    volume: f32,
    audio_advanced: bool,
    buffer_ms: u32,
}

impl DemoSettings {
    fn initial() -> Self {
        Self {
            hints: true,
            player_name: "Player One".to_owned(),
            gamma: 1.0,
            volume: 0.8,
            audio_advanced: true,
            buffer_ms: 256,
            ..Default::default()
        }
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, PlumePlugins))
        .insert_resource(DemoSettings::initial())
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(
            Update,
            (tabs_demo_ui, common::log_on_change::<DemoSettings>),
        );
    common::apply_args(&mut app, false);
    app.run();
}

fn tabs_demo_ui(mut root: PlumeRoot, mut settings: ResMut<DemoSettings>) {
    let mut s = settings.clone();
    root.screen(|ui| {
        ui.caption("A tab container on screen — one body at a time.");
        ui.vertical(|ui| {
            ui.tabs(&mut s.tab, |tabs| {
                tabs.tab(Tab::General, "General", |ui| {
                    ui.checkbox(&mut s.hints, "Show tutorial hints");
                    ui.horizontal(|ui| {
                        ui.caption("Player name");
                        ui.text_edit(&mut s.player_name).grow();
                    });
                    ui.horizontal(|ui| {
                        ui.caption("Player color");
                        ui.select(&mut s.accent, |select| {
                            select.option(Accent::Crimson, "Crimson");
                            select.option(Accent::Teal, "Teal");
                            select.option(Accent::Amber, "Amber");
                        })
                        .grow();
                        // The swatch is keyed by call site, not by color, so it
                        // recolors in place as the select changes.
                        ui.color_swatch(s.accent.color());
                    });
                })
                .icon(font_awesome::solid::GEAR);
                tabs.tab(Tab::Video, "Video", |ui| {
                    ui.caption("Window mode");
                    ui.radio(&mut s.window_mode, WindowMode::Windowed, "Windowed");
                    ui.radio(&mut s.window_mode, WindowMode::Borderless, "Borderless");
                    ui.radio(&mut s.window_mode, WindowMode::Fullscreen, "Fullscreen");
                    // In a column the rule draws horizontally; in the footer row below
                    // the same call draws vertically.
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.caption("Gamma");
                        ui.slider(&mut s.gamma, 0.0..=2.0).grow();
                    });
                    ui.horizontal(|ui| {
                        ui.caption("Quality");
                        ui.select(&mut s.quality, |select| {
                            select.option(Quality::Low, "Low");
                            select.option(Quality::Medium, "Medium");
                            select.option(Quality::High, "High");
                        });
                    });
                });
                tabs.tab(Tab::Audio, "Audio", |ui| {
                    ui.horizontal(|ui| {
                        ui.caption("Muted");
                        ui.toggle(&mut s.muted);
                    });
                    ui.horizontal(|ui| {
                        ui.caption("Volume");
                        ui.slider(&mut s.volume, 0.0..=1.0).grow();
                    });
                    ui.horizontal(|ui| {
                        ui.disclosure(&mut s.audio_advanced);
                        ui.caption("Advanced");
                    });
                    if s.audio_advanced {
                        ui.horizontal(|ui| {
                            ui.caption("Buffer");
                            ui.number(&mut s.buffer_ms).step(64u32).suffix("ms").grow();
                        });
                    }
                });
                tabs.tab(Tab::Advanced, "Advanced", |ui| {
                    ui.caption("Unreachable while the tab is disabled.");
                })
                .enabled(false);
            })
            .height(px(300));
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .button("Reset to defaults")
                    .variant(ButtonVariant::Outline)
                    .clicked
                {
                    s = DemoSettings {
                        tab: s.tab,
                        ..DemoSettings::initial()
                    };
                }
                ui.separator();
                ui.flex_spacer();
                ui.button("Cancel");
                ui.button("Apply").primary();
            });
        })
        .width(px(420));
    })
    .background_slot(ThemeSlot::Neutral0);
    settings.set_if_neq(s);
}
