//! Pass-2 acceptance example for `text_edit` + `radio`, grown into a design
//! showcase: one resource-backed dialog laid out on a single label gutter, its
//! controls grouped under flat (non-collapsible) section headers.
use bevy::prelude::*;
use bevy_jp_plume::{
    PlumePlugins,
    constants::font_awesome,
    controls::ButtonVariant,
    imm::{PlumeImm, PlumeRoot, Ui},
};

#[path = "common/mod.rs"]
mod common;

/// Width of the label gutter every row aligns to.
const GUTTER: f32 = 76.0;

const CLASSES: &[&str] = &["Warrior", "Mage", "Rogue"];

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Team {
    #[default]
    Red,
    Blue,
}

#[derive(Resource, Debug, Clone, PartialEq)]
struct ProfileSettings {
    open: bool,
    name: String,
    motto: String,
    class: usize,
    difficulty: Difficulty,
    team: Team,
    permadeath: bool,
    music: f32,
}

impl Default for ProfileSettings {
    fn default() -> Self {
        Self {
            open: true,
            name: "Player One".into(),
            motto: String::new(),
            class: 0,
            difficulty: Difficulty::default(),
            team: Team::default(),
            permadeath: false,
            music: 70.0,
        }
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, PlumePlugins))
        .init_resource::<ProfileSettings>()
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_systems(
            Update,
            (profile_ui, common::log_on_change::<ProfileSettings>),
        );
    common::apply_args(&mut app);
    app.run();
}

/// One gutter row: a caption pinned to [`GUTTER`], then whatever `f` builds.
fn field(ui: &mut Ui, label: &str, f: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.caption(label).width(px(GUTTER));
        f(ui);
    });
}

fn profile_ui(mut root: PlumeRoot, mut settings: ResMut<ProfileSettings>) {
    // Build the UI against a local clone and write back with `set_if_neq`, so the
    // resource only registers as changed when a control actually changed it.
    let mut s = settings.clone();

    root.screen(|ui| {
        ui.horizontal(|ui| {
            if ui.button("Player Profile").enabled(!s.open).clicked {
                s.open = true;
            }
        });
    });

    // Local copy dodges the borrow conflict between `&mut open` and the fields;
    // `done` closes from inside the body (the dialog holds `open` until `show` ends).
    let mut open = s.open;
    let mut done = false;
    root.dialog("Player Profile", &mut open)
        .width(px(380))
        .at(px(160), px(100))
        .show(|ui| {
            // Identity: three text/choice fields sharing the gutter.
            ui.section("Profile", |ui| {
                field(ui, "Name", |ui| {
                    ui.text_edit(&mut s.name).grow();
                });
                field(ui, "Motto", |ui| {
                    ui.text_edit(&mut s.motto)
                        .grow()
                        .placeholder("A few words…");
                });
                field(ui, "Class", |ui| {
                    ui.select(&mut s.class, CLASSES).grow();
                });
            })
            .collapsible(false);

            // Match settings: radio groups laid out along the same gutter.
            ui.section("Match", |ui| {
                field(ui, "Difficulty", |ui| {
                    ui.radio(&mut s.difficulty, Difficulty::Easy, "Easy");
                    ui.radio(&mut s.difficulty, Difficulty::Normal, "Normal");
                    ui.radio(&mut s.difficulty, Difficulty::Hard, "Hard");
                });
                field(ui, "Team", |ui| {
                    ui.radio(&mut s.team, Team::Red, "Red");
                    ui.radio(&mut s.team, Team::Blue, "Blue");
                });
            })
            .collapsible(false);

            // Options: a right-aligned toggle and a slider with a live read-out.
            ui.section("Options", |ui| {
                field(ui, "Permadeath", |ui| {
                    ui.flex_spacer();
                    ui.toggle(&mut s.permadeath);
                });
                field(ui, "Music", |ui| {
                    ui.slider(&mut s.music, 0.0..=100.0).grow().step(1.0);
                    ui.caption(&format!("{:.0}%", s.music)).width(px(36));
                });
            })
            .collapsible(false);

            ui.separator();

            ui.horizontal(|ui| {
                if ui
                    .icon_button(font_awesome::solid::ARROW_ROTATE_LEFT, "Reset")
                    .variant(ButtonVariant::Outline)
                    .clicked
                {
                    let open = s.open;
                    s = ProfileSettings { open, ..default() };
                }
                ui.flex_spacer();
                done = ui.button("Done").primary().clicked;
            });
        });
    s.open = open && !done;
    settings.set_if_neq(s);
}
