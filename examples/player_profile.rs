//! Pass-2 acceptance example for `text_edit` + `radio`: a player profile dialog
//! backed by one resource, printed live, with Reset wired to `Default`.
use bevy::prelude::*;
use bevy_jp_plume::{
    PlumePlugins,
    constants::font_awesome,
    imm::{PlumeImm, PlumeUi},
};

#[path = "common/mod.rs"]
mod common;

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
    difficulty: Difficulty,
    team: Team,
}

impl Default for ProfileSettings {
    fn default() -> Self {
        Self {
            open: true,
            name: "Player One".into(),
            motto: String::new(),
            difficulty: Difficulty::default(),
            team: Team::default(),
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
    common::screenshot_on_arg(&mut app);
    app.run();
}

fn profile_ui(mut ui: PlumeUi, mut settings: ResMut<ProfileSettings>) {
    // Build the UI against a local clone and write back with `set_if_neq`, so the
    // resource only registers as changed when a control actually changed it.
    let mut s = settings.clone();

    if ui.button("Player Profile").enabled(!s.open).clicked {
        s.open = true;
    }

    // Local copy dodges the borrow conflict between `&mut open` and the fields;
    // `done` closes from inside the body (the dialog holds `open` until `show` ends).
    let mut open = s.open;
    let mut done = false;
    ui.dialog("Player Profile", &mut open)
        .width(px(340))
        .at(px(160), px(100))
        .show(|ui| {
            ui.horizontal(|ui| {
                ui.caption("Name").width(px(56));
                ui.text_edit(&mut s.name).grow();
            });
            ui.horizontal(|ui| {
                ui.caption("Motto").width(px(56));
                ui.text_edit(&mut s.motto)
                    .grow()
                    .placeholder("A few words…");
            });

            // A radio group is just radios sharing one binding.
            ui.section("Difficulty", |ui| {
                ui.radio(&mut s.difficulty, Difficulty::Easy, "Easy");
                ui.radio(&mut s.difficulty, Difficulty::Normal, "Normal");
                ui.radio(&mut s.difficulty, Difficulty::Hard, "Hard");
            });
            ui.section("Team", |ui| {
                ui.radio(&mut s.team, Team::Red, "Red");
                ui.radio(&mut s.team, Team::Blue, "Blue");
            });

            ui.separator();

            ui.horizontal(|ui| {
                if ui
                    .icon_button(font_awesome::solid::ARROW_ROTATE_LEFT, "Reset")
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
