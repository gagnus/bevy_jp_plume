//! Debug-menu dialog as a self-contained feature plugin: two panes of rendering /
//! physics / diagnostics / cheats controls, backed by one resource.
use bevy::prelude::*;
use bevy_jp_plume::{
    constants::font_awesome,
    controls::ButtonVariant,
    imm::{ImmSelect, PlumeImm, PlumeRoot, Ui},
};

use super::debug_hub::{AddDebugDialog, DebugDialogRegistry};
use super::log_on_change;

const TITLE: &str = "Debug Options";

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct DebugSettings {
    // Rendering
    wireframe: bool,
    show_colliders: bool,
    freeze_culling: bool,
    view_mode: ViewMode,
    gamma: f32,
    // Physics
    pause_sim: bool,
    time_scale: f32,
    // Diagnostics
    fps_overlay: bool,
    entity_inspector: bool,
    overlay: OverlayCorner,
    log_level: LogLevel,
    // Cheats
    noclip: bool,
    infinite_health: bool,
    move_speed: f32,
}

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
            noclip: false,
            infinite_health: false,
            move_speed: 6.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum ViewMode {
    #[default]
    Lit,
    Albedo,
    Normals,
    Depth,
    Overdraw,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum OverlayCorner {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

/// Adds the debug-menu dialog: its resource, its dialog system + hub entry, and its log.
pub struct DebugSettingsPlugin(pub bool);

impl Plugin for DebugSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugSettings>()
            .add_debug_dialog(
                TITLE,
                font_awesome::solid::BUG,
                self.0,
                debug_settings_dialog,
            )
            .add_systems(Update, log_on_change::<DebugSettings>);
    }
}

fn debug_settings_dialog(
    mut root: PlumeRoot,
    mut registry: ResMut<DebugDialogRegistry>,
    mut settings: ResMut<DebugSettings>,
) {
    // Build against a clone and write back with `set_if_neq`, so the resource only
    // registers as changed when a control actually changed it.
    let mut s = settings.clone();
    let mut open = registry.is_open(TITLE);
    // Fixed width so the equal-.grow() columns have something to resolve against.
    root.dialog(TITLE, &mut open)
        .width(px(600))
        .at(px(20), px(320))
        .icon(font_awesome::solid::BUG)
        .show(|ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    rendering_pane(ui, &mut s);
                    physics_pane(ui, &mut s);
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
                if ui
                    .icon_button(font_awesome::solid::ARROW_ROTATE_LEFT, "Reset to defaults")
                    .variant(ButtonVariant::Outline)
                    .clicked
                {
                    s = DebugSettings::default();
                }
                ui.flex_spacer();
                ui.button("Cancel").variant(ButtonVariant::Outline);
                ui.button("Apply").primary();
            });
        });
    if open != registry.is_open(TITLE) {
        registry.set_open(TITLE, open);
    }
    settings.set_if_neq(s);
}

fn rendering_pane(ui: &mut Ui, s: &mut DebugSettings) {
    ui.section("Rendering", |ui| {
        ui.checkbox(&mut s.wireframe, "Wireframe");
        ui.checkbox(&mut s.show_colliders, "Show colliders");
        ui.checkbox(&mut s.freeze_culling, "Freeze frustum culling");
        select_row(ui, "View mode", &mut s.view_mode, |select| {
            select.option(ViewMode::Lit, "Lit");
            select.option(ViewMode::Albedo, "Albedo");
            select.option(ViewMode::Normals, "Normals");
            select.option(ViewMode::Depth, "Depth");
            select.option(ViewMode::Overdraw, "Overdraw");
        });
        slider_row(ui, "Gamma", &mut s.gamma, 0.5..=3.0, 0.1, 2, None);
    });
}

fn physics_pane(ui: &mut Ui, s: &mut DebugSettings) {
    ui.section("Physics", |ui| {
        ui.checkbox(&mut s.pause_sim, "Pause simulation");
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

fn diagnostics_pane(ui: &mut Ui, s: &mut DebugSettings) {
    ui.section("Diagnostics", |ui| {
        toggle_row(ui, "FPS overlay", &mut s.fps_overlay);
        toggle_row(ui, "Entity inspector", &mut s.entity_inspector);
        select_row(ui, "Overlay", &mut s.overlay, |select| {
            select.option(OverlayCorner::TopLeft, "Top left");
            select.option(OverlayCorner::TopRight, "Top right");
            select.option(OverlayCorner::BottomLeft, "Bottom left");
            select.option(OverlayCorner::BottomRight, "Bottom right");
        });
        // Inline (not select_row) to chain .max_visible(): the popup caps at
        // three rows and scrolls the remaining two.
        ui.horizontal(|ui| {
            ui.caption("Log level").width(px(84));
            ui.select(&mut s.log_level, |select| {
                select.option(LogLevel::Error, "Error");
                select.option(LogLevel::Warn, "Warn");
                select.option(LogLevel::Info, "Info");
                select.option(LogLevel::Debug, "Debug");
                select.option(LogLevel::Trace, "Trace");
            })
            .grow()
            .max_visible(3);
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

// Reusable rows: repeated calls under one parent need no `push_id` — same-id
// repeats are disambiguated by occurrence index.
fn slider_row(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
    precision: usize,
    suffix: Option<&str>,
) {
    ui.horizontal(|ui| {
        ui.caption(label).width(px(84));
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
    });
}

fn select_row<T: PartialEq>(
    ui: &mut Ui,
    label: &str,
    selected: &mut T,
    options: impl FnOnce(&mut ImmSelect<T>),
) {
    ui.horizontal(|ui| {
        ui.caption(label).width(px(84));
        ui.select(selected, options).grow();
    });
}

fn toggle_row(ui: &mut Ui, label: &str, value: &mut bool) {
    ui.horizontal(|ui| {
        ui.caption(label);
        ui.flex_spacer();
        ui.toggle(value);
    });
}
