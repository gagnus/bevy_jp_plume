//! Demonstrates `ui.screen`: a full-screen root surface that gives top-level
//! widgets the standard font/text-color context, so bare `caption`/text works at
//! root scope without wrapping everything in a dialog.
use bevy::prelude::*;
use bevy_jp_plume::{
    PlumePlugins,
    imm::{PlumeImm, PlumeRoot},
};

#[path = "common/mod.rs"]
mod common;

#[derive(Resource, Debug, Clone, Default, PartialEq)]
struct DemoSettings {
    enabled: bool,
}

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, PlumePlugins))
        .init_resource::<DemoSettings>()
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(
            Update,
            (screen_demo_ui, common::log_on_change::<DemoSettings>),
        );
    common::apply_args(&mut app);
    app.run();
}

fn screen_demo_ui(mut root: PlumeRoot, mut settings: ResMut<DemoSettings>) {
    let mut s = settings.clone();
    root.screen(|ui| {
        // Bare text at root scope: renders in the themed caption font because the
        // screen surface establishes the inheritable font/color context.
        ui.caption("Top-level caption — no dialog, no wrapper column.");
        // In this column the separator draws horizontally.
        ui.separator();
        ui.horizontal(|ui| {
            ui.button("Bare button");
            // The same call in a row draws vertically, spanning the row height.
            ui.separator();
            ui.button("Another").primary();
            // Fixed main-axis gap: width here, height in the column above.
            ui.space(px(40));
            ui.button("After a 40px space");
        });
        ui.checkbox(&mut s.enabled, "A checkbox at root scope");
        // Without align_self the column's Stretch would run this to full width.
        ui.button("Hugs its label").align_self(AlignSelf::Start);
    });
    settings.set_if_neq(s);
}
