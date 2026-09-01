//! Debug-menu dialog as a self-contained feature plugin: two columns of rendering,
//! physics, interface, diagnostics and cheats controls, backed by one resource.
use bevy::prelude::*;
use bevy_jp_plume::imm::{ImmResponse, ImmSelect, kind};
use bevy_jp_plume::prelude::*;

use super::debug_hub::{AddDebugDialog, DebugDialogRegistry};
use super::{Options, log_on_change};

const TITLE: &str = "Debug Options";

/// Backing state for both the imm dialog and its retained twin.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct DebugSettings {
    pub wireframe: bool,
    pub show_colliders: bool,
    pub freeze_culling: bool,
    pub view_mode: ViewMode,
    pub gamma: f32,
    pub pause_sim: bool,
    pub time_scale: f32,
    pub fps_overlay: bool,
    pub entity_inspector: bool,
    pub overlay: OverlayCorner,
    pub log_level: LogLevel,
    pub capture_dir: String,
    pub noclip: bool,
    pub infinite_health: bool,
    pub move_speed: f32,
    pub ui_scale: f32,
}

/// Base font size the dialog scales from.
pub const BASE_FONT_PX: f32 = 14.0;

impl Default for DebugSettings {
    fn default() -> Self {
        Self {
            wireframe: false,
            show_colliders: true,
            freeze_culling: false,
            view_mode: ViewMode::default(),
            gamma: 2.2,
            pause_sim: false,
            time_scale: 1.0,
            fps_overlay: true,
            entity_inspector: false,
            overlay: OverlayCorner::default(),
            log_level: LogLevel::default(),
            capture_dir: "captures/latest".into(),
            noclip: false,
            infinite_health: false,
            move_speed: 6.0,
            ui_scale: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ViewMode {
    #[default]
    Lit,
    Albedo,
    Normals,
    Depth,
    Overdraw,
}

impl Options for ViewMode {
    const ALL: &'static [Self] = &[
        Self::Lit,
        Self::Albedo,
        Self::Normals,
        Self::Depth,
        Self::Overdraw,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Lit => "Lit",
            Self::Albedo => "Albedo",
            Self::Normals => "Normals",
            Self::Depth => "Depth",
            Self::Overdraw => "Overdraw",
        }
    }

    fn enabled(self) -> bool {
        self != Self::Overdraw
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum OverlayCorner {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Options for OverlayCorner {
    const ALL: &'static [Self] = &[
        Self::TopLeft,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomRight,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::TopLeft => "Top left",
            Self::TopRight => "Top right",
            Self::BottomLeft => "Bottom left",
            Self::BottomRight => "Bottom right",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl Options for LogLevel {
    const ALL: &'static [Self] = &[
        Self::Error,
        Self::Warn,
        Self::Info,
        Self::Debug,
        Self::Trace,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Error => "Error",
            Self::Warn => "Warn",
            Self::Info => "Info",
            Self::Debug => "Debug",
            Self::Trace => "Trace",
        }
    }
}

/// Adds the debug-menu dialog: its resource, its dialog system + hub entry, and its log.
pub struct DebugSettingsPlugin(pub bool);

impl Plugin for DebugSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugSettings>()
            .add_debug_dialog(TITLE, lucide::BUG, self.0, debug_settings_dialog)
            .add_systems(Update, log_on_change::<DebugSettings>);
    }
}

fn debug_settings_dialog(
    mut root: PlumeRoot,
    mut registry: ResMut<DebugDialogRegistry>,
    mut settings: ResMut<DebugSettings>,
    mut reset_confirm_open: Local<bool>,
) {
    let mut s = settings.clone();
    let mut open = registry.is_open(TITLE);

    root.dialog(TITLE, &mut open)
        .width(em(600.0 / BASE_FONT_PX))
        .at_corner(Corner::BottomLeft, px(20), px(20))
        .show(|ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    rendering_pane(ui, &mut s);
                    physics_pane(ui, &mut s);
                    interface_pane(ui, &mut s);
                })
                .grow();
                ui.vertical(|ui| {
                    diagnostics_pane(ui, &mut s);
                    cheats_pane(ui, &mut s);
                })
                .grow();
            })
            .align_items(AlignItems::Start);

            ui.separator();

            ui.horizontal(|ui| {
                let (mut do_reset, mut keep) = (false, false);
                ui.icon_button(lucide::UNDO_2, "Reset to defaults")
                    .variant(ButtonVariant::Outline)
                    .tooltip_container(|ui| {
                        ui.horizontal(|ui| {
                            ui.icon(lucide::UNDO_2);
                            ui.caption("Reset to defaults")
                                .text_color_slot(ThemeSlot::Text0);
                        });
                        ui.caption("Every debug option returns to its default value");
                    })
                    .popup(&mut reset_confirm_open)
                    .toggle_on_click()
                    .show(|ui| {
                        ui.caption("Reset all settings to defaults?").no_wrap();
                        ui.horizontal(|ui| {
                            ui.flex_spacer();
                            do_reset = ui.button("Reset").primary().clicked;
                            keep = ui.button("Keep").variant(ButtonVariant::Outline).clicked;
                        });
                    });
                if do_reset {
                    s = DebugSettings::default();
                }
                if do_reset || keep {
                    *reset_confirm_open = false;
                }
                ui.flex_spacer();
                ui.button("Cancel")
                    .variant(ButtonVariant::Outline)
                    .tooltip("Discard changes and close");
                ui.button("Apply")
                    .primary()
                    .tooltip("Apply changes and close");
            });
        })
        .map(|r| r.font_size(BASE_FONT_PX * s.ui_scale));

    if open != registry.is_open(TITLE) {
        registry.set_open(TITLE, open);
    }
    settings.set_if_neq(s);
}

fn rendering_pane(ui: &mut Ui, s: &mut DebugSettings) {
    ui.section("Rendering", |ui| {
        ui.checkbox(&mut s.wireframe, "Wireframe")
            .tooltip("Draw all meshes as wireframe");
        ui.checkbox(&mut s.show_colliders, "Show colliders");
        ui.checkbox(&mut s.freeze_culling, "Freeze frustum culling");
        select_row(ui, "View mode", &mut s.view_mode)
            .tooltip("Which render pass fills the viewport");
        slider_row(ui, "Gamma", &mut s.gamma, 0.5..=3.0, 0.1, 2, None)
            .tooltip("Display gamma correction");
    });
}

fn physics_pane(ui: &mut Ui, s: &mut DebugSettings) {
    ui.section("Physics", |ui| {
        ui.checkbox(&mut s.pause_sim, "Pause simulation")
            .tooltip("Halt the physics clock; rendering keeps running");
        slider_row(
            ui,
            "Time scale",
            &mut s.time_scale,
            0.0..=2.0,
            0.05,
            2,
            None,
        );
    });
}

fn interface_pane(ui: &mut Ui, s: &mut DebugSettings) {
    ui.section("Interface", |ui| {
        slider_row(ui, "UI scale", &mut s.ui_scale, 0.5..=2.0, 0.05, 2, None)
            .tooltip("Scales this dialog's text and everything sized from it");
    });
}

fn diagnostics_pane(ui: &mut Ui, s: &mut DebugSettings) {
    ui.section("Diagnostics", |ui| {
        toggle_row(ui, "FPS overlay", &mut s.fps_overlay);
        toggle_row(ui, "Entity inspector", &mut s.entity_inspector);
        select_row(ui, "Overlay", &mut s.overlay);
        ui.horizontal(|ui| {
            ui.caption("Log level").width(em(84.0 / BASE_FONT_PX));
            ui.select(&mut s.log_level, options_of::<LogLevel>)
                .grow()
                .max_visible(3);
        });
        // A path-style field: clear tucked inside the leading edge, a warning
        // inside the trailing one — `prefix_container` / `suffix_container` at work.
        ui.horizontal(|ui| {
            ui.caption("Capture to").width(em(84.0 / BASE_FONT_PX));
            let mut clear = false;
            let text_edit = ui
                .text_edit(&mut s.capture_dir)
                .grow()
                .placeholder("Beside the app")
                .prefix_container(|ui| {
                    clear = ui
                        .tool_button(lucide::X)
                        .flat()
                        .variant(ButtonVariant::Plain)
                        .font_scale(0.8)
                        .tooltip("Clear")
                        .clicked;
                });

            // After the edit and clear fold in, so the warning tracks this
            // frame's text rather than lagging it by one.
            if clear {
                s.capture_dir.clear();
            }
            if !s.capture_dir.is_empty() {
                text_edit.suffix_container(|ui| {
                    ui.horizontal(|ui| {
                        ui.icon(lucide::TRIANGLE_ALERT)
                            .text_color_slot(ThemeSlot::Danger0)
                            .tooltip("Should leave it blank!");
                    })
                    .padding(UiRect::right(size::SPACE_TIGHT));
                });
            }
        });
    });
}

fn cheats_pane(ui: &mut Ui, s: &mut DebugSettings) {
    ui.section("Cheats", |ui| {
        ui.checkbox(&mut s.noclip, "Noclip");
        ui.checkbox(&mut s.infinite_health, "Infinite health");
        slider_row(
            ui,
            "Move speed",
            &mut s.move_speed,
            1.0..=40.0,
            1.0,
            0,
            Some("m/s"),
        );
    });
}

fn slider_row<'r, 'w, 's>(
    ui: &'r mut Ui<'w, 's>,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
    precision: usize,
    suffix: Option<&str>,
) -> ImmResponse<'r, 'w, 's, kind::Row> {
    ui.horizontal(|ui| {
        ui.caption(label).width(em(84.0 / BASE_FONT_PX));
        ui.slider(value, range.clone())
            .grow()
            .step(step)
            .precision(precision);
        let number = ui
            .number(value)
            .range(range)
            .step(step)
            .precision(precision);
        if let Some(suffix) = suffix {
            number.suffix(suffix);
        }
    })
}

fn options_of<T: Options>(select: &mut ImmSelect<T>) {
    for &option in T::ALL {
        select
            .option(option, option.label())
            .enabled(option.enabled());
    }
}

fn select_row<'r, 'w, 's, T: Options>(
    ui: &'r mut Ui<'w, 's>,
    label: &str,
    selected: &mut T,
) -> ImmResponse<'r, 'w, 's, kind::Row> {
    ui.horizontal(|ui| {
        ui.caption(label).width(em(84.0 / BASE_FONT_PX));
        ui.select(selected, options_of::<T>).grow();
    })
}

fn toggle_row(ui: &mut Ui, label: &str, value: &mut bool) {
    ui.horizontal(|ui| {
        ui.caption(label);
        ui.flex_spacer();
        ui.toggle(value);
    });
}
