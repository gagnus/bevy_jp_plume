//! Pass-2 acceptance example: an imm-mode replica of the smoke debug menu, backed
//! by one resource, printed live, with Reset wired to `Default`.
use bevy::prelude::*;
use bevy_jp_plume::{
    PlumePlugins,
    constants::font_awesome,
    imm::{PlumeImm, PlumeUi, Ui},
};

#[derive(Resource, Debug, Clone, PartialEq)]
struct DebugSettings {
    open: bool,
    // Rendering
    wireframe: bool,
    show_colliders: bool,
    freeze_culling: bool,
    view_mode: usize,
    gamma: f32,
    // Physics
    pause_sim: bool,
    time_scale: f32,
    // Diagnostics
    fps_overlay: bool,
    entity_inspector: bool,
    overlay: usize,
    log_level: usize,
    // Cheats
    noclip: bool,
    infinite_health: bool,
    move_speed: f32,
}

impl Default for DebugSettings {
    fn default() -> Self {
        Self {
            open: true,
            wireframe: false,
            show_colliders: true,
            freeze_culling: false,
            view_mode: 0,
            gamma: 2.2,
            pause_sim: false,
            time_scale: 1.0,
            fps_overlay: true,
            entity_inspector: false,
            overlay: 0,
            log_level: 0,
            noclip: false,
            infinite_health: false,
            move_speed: 6.0,
        }
    }
}

const VIEW_MODES: [&str; 5] = ["Lit", "Albedo", "Normals", "Depth", "Overdraw"];
const OVERLAY_CORNERS: [&str; 4] = ["Top left", "Top right", "Bottom left", "Bottom right"];
const LOG_LEVELS: [&str; 5] = ["Error", "Warn", "Info", "Debug", "Trace"];

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, PlumePlugins))
        .init_resource::<DebugSettings>()
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(Update, (debug_settings_ui, log_on_change));
    if std::env::var_os("DEBUG_SHOT").is_some() {
        app.add_systems(Update, screenshot_and_exit);
    }
    app.run();
}

// Prints the backing whenever it changes, to confirm every control round-trips.
fn log_on_change(settings: Res<DebugSettings>) {
    if settings.is_changed() {
        info!("{:?}", *settings);
    }
}

fn debug_settings_ui(mut ui: PlumeUi, mut settings: ResMut<DebugSettings>) {
    if ui.button("Debug Options").enabled(!settings.open).clicked {
        settings.open = true;
    }

    // Local clone dodges the borrow conflict between `&mut open` and the fields,
    // and lets Reset diff against Default cheaply.
    let mut open = settings.open;
    ui.dialog("Debug Options", &mut open, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.push_id("rendering", |ui| rendering_pane(ui, &mut settings));
                ui.push_id("physics", |ui| physics_pane(ui, &mut settings));
            })
            .width(Val::Px(280.0));
            ui.vertical(|ui| {
                ui.push_id("diagnostics", |ui| diagnostics_pane(ui, &mut settings));
                ui.push_id("cheats", |ui| cheats_pane(ui, &mut settings));
            })
            .width(Val::Px(280.0));
        })
        .align_top();

        ui.separator();

        ui.horizontal(|ui| {
            if ui
                .icon_button(font_awesome::solid::ARROW_ROTATE_LEFT, "Reset to defaults")
                .clicked
            {
                let open = settings.open;
                *settings = DebugSettings { open, ..default() };
            }
            ui.flex_spacer();
            ui.button("Cancel");
            ui.button("Apply").primary();
        });
    });
    settings.open = open;
}

fn rendering_pane(ui: &mut Ui, s: &mut DebugSettings) {
    ui.section("Rendering", |ui| {
        ui.checkbox(&mut s.wireframe, "Wireframe");
        ui.checkbox(&mut s.show_colliders, "Show colliders");
        ui.checkbox(&mut s.freeze_culling, "Freeze frustum culling");
        select_row(ui, "View mode", &mut s.view_mode, &VIEW_MODES);
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
        select_row(ui, "Overlay", &mut s.overlay, &OVERLAY_CORNERS);
        select_row(ui, "Log level", &mut s.log_level, &LOG_LEVELS);
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

// Reusable rows: distinct `id` scoping per label keeps repeated calls collision-free.
fn slider_row(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
    precision: usize,
    suffix: Option<&str>,
) {
    ui.push_id(label, |ui| {
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
    });
}

fn select_row(ui: &mut Ui, label: &str, index: &mut usize, options: &[&str]) {
    ui.push_id(label, |ui| {
        ui.horizontal(|ui| {
            ui.caption(label).width(px(84));
            ui.select(index, options).grow();
        });
    });
}

fn toggle_row(ui: &mut Ui, label: &str, value: &mut bool) {
    ui.push_id(label, |ui| {
        ui.horizontal(|ui| {
            ui.caption(label);
            ui.flex_spacer();
            ui.toggle(value);
        });
    });
}

fn screenshot_and_exit(
    mut frames: Local<u32>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    *frames += 1;
    if *frames == 200
        && let Some(path) = std::env::var_os("DEBUG_SHOT")
    {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(std::path::PathBuf::from(path)));
    }
    if *frames == 260 {
        exit.write(AppExit::Success);
    }
}
