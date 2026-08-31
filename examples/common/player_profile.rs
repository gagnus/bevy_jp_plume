//! Player-profile dialog as a self-contained feature plugin: text/choice fields laid
//! out on a single label gutter, grouped under flat section headers.
use bevy::prelude::*;
use bevy_jp_plume::prelude::*;

use super::debug_hub::{AddDebugDialog, DebugDialogRegistry};
use super::log_on_change;

const TITLE: &str = "Player Profile";

const GUTTER: Val = Val::Em(6.0);

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Class {
    #[default]
    Warrior,
    Mage,
    Rogue,
}

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
pub struct ProfileSettings {
    name: String,
    motto: String,
    class: Class,
    difficulty: Difficulty,
    team: Team,
    permadeath: bool,
    music: u8,
}

impl Default for ProfileSettings {
    fn default() -> Self {
        Self {
            name: "Player One".into(),
            motto: String::new(),
            class: Class::default(),
            difficulty: Difficulty::default(),
            team: Team::default(),
            permadeath: false,
            music: 70,
        }
    }
}

/// Adds the player-profile dialog: its resource, its dialog system + hub entry, and its log.
pub struct PlayerProfilePlugin(pub bool);

impl Plugin for PlayerProfilePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProfileSettings>()
            .add_debug_dialog(
                TITLE,
                lucide::PERSON_STANDING,
                self.0,
                player_profile_dialog,
            )
            .add_systems(Update, log_on_change::<ProfileSettings>);
    }
}

fn field(ui: &mut Ui, label: &str, f: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.caption(label).width(GUTTER);
        f(ui);
    });
}

fn player_profile_dialog(
    mut root: PlumeRoot,
    mut registry: ResMut<DebugDialogRegistry>,
    mut settings: ResMut<ProfileSettings>,
) {
    let mut s = settings.clone();
    let mut open = registry.is_open(TITLE);
    // `done` closes from inside the body (the dialog holds `open` until `show` ends).
    let mut done = false;
    root.dialog(TITLE, &mut open)
        .width(em(27))
        .at(px(450), px(60))
        .show(|ui| {
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
                    ui.select(&mut s.class, |select| {
                        select.option(Class::Warrior, "Warrior");
                        select.option(Class::Mage, "Mage");
                        select.option(Class::Rogue, "Rogue").enabled(false);
                    })
                    .grow();
                });
            })
            .collapsible(false);

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

            ui.section("Options", |ui| {
                field(ui, "Permadeath", |ui| {
                    ui.flex_spacer();
                    ui.toggle(&mut s.permadeath);
                });
                field(ui, "Music", |ui| {
                    ui.slider(&mut s.music, 0..=100).grow();
                    ui.caption(&format!("{}%", s.music)).width(em(2.5));
                });
            })
            .collapsible(false);

            ui.separator();

            ui.horizontal(|ui| {
                if ui
                    .icon_button(lucide::UNDO_2, "Reset")
                    .variant(ButtonVariant::Outline)
                    .clicked
                {
                    s = ProfileSettings::default();
                }
                ui.flex_spacer();
                done = ui.button("Done").primary().clicked;
            });
        });
    open = open && !done;
    if open != registry.is_open(TITLE) {
        registry.set_open(TITLE, open);
    }
    settings.set_if_neq(s);
}
