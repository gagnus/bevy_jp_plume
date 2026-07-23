//! Player-profile dialog as a self-contained feature plugin: text/choice fields laid
//! out on a single label gutter, grouped under flat section headers.
use bevy::prelude::*;
use bevy_jp_plume::{
    constants::font_awesome,
    controls::ButtonVariant,
    imm::{PlumeImm, PlumeRoot, Ui},
};

use super::debug_hub::{AddDebugDialog, DebugDialogRegistry};
use super::log_on_change;

const TITLE: &str = "Player Profile";

/// Width of the label gutter every row aligns to.
const GUTTER: f32 = 76.0;

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
    music: f32,
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
            music: 70.0,
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
                font_awesome::solid::PERSON,
                self.0,
                player_profile_dialog,
            )
            .add_systems(Update, log_on_change::<ProfileSettings>);
    }
}

/// One gutter row: a caption pinned to [`GUTTER`], then whatever `f` builds.
fn field(ui: &mut Ui, label: &str, f: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.caption(label).width(px(GUTTER));
        f(ui);
    });
}

fn player_profile_dialog(
    mut root: PlumeRoot,
    mut registry: ResMut<DebugDialogRegistry>,
    mut settings: ResMut<ProfileSettings>,
) {
    // Build against a clone and write back with `set_if_neq`, so the resource only
    // registers as changed when a control actually changed it.
    let mut s = settings.clone();
    let mut open = registry.is_open(TITLE);
    // `done` closes from inside the body (the dialog holds `open` until `show` ends).
    let mut done = false;
    root.dialog(TITLE, &mut open)
        .width(px(380))
        .at(px(450), px(60))
        .icon(font_awesome::solid::PERSON)
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
                    ui.select(&mut s.class, |select| {
                        select.option(Class::Warrior, "Warrior");
                        select.option(Class::Mage, "Mage");
                        select.option(Class::Rogue, "Rogue");
                    })
                    .grow();
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
