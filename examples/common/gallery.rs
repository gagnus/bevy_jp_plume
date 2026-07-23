//! Control gallery as a self-contained feature plugin: a 3-by-2 grid of "one of each
//! control" cards previewing every control against the three neutral surface slots,
//! enabled and disabled. Drawn as a full-screen surface — the backdrop the `showcase`
//! floats its dialogs over — so it registers no hub entry.
use bevy::prelude::*;
use bevy_jp_plume::{
    controls::ButtonVariant,
    imm::{PlumeImm, PlumeRoot, Ui},
    theme::{ThemeSlot, UiTheme},
};

use super::log_on_change;

const SELECT_OPTIONS: [&str; 4] = ["Alpha", "Beta", "Gamma", "Delta"];

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum RadioChoice {
    #[default]
    A,
    B,
}

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct GalleryState {
    checkbox_a: bool,
    checkbox_b: bool,
    disclosure_a: bool,
    disclosure_b: bool,
    toggle_a: bool,
    toggle_b: bool,
    radio: RadioChoice,
    slider: f32,
    select: usize,
    text: String,
    number: f32,
}

impl Default for GalleryState {
    fn default() -> Self {
        Self {
            checkbox_a: false,
            checkbox_b: true,
            disclosure_a: false,
            disclosure_b: true,
            toggle_a: false,
            toggle_b: true,
            radio: RadioChoice::default(),
            slider: 30.0,
            select: 0,
            text: String::new(),
            number: 4.0,
        }
    }
}

/// Adds the control gallery: its resource, its full-screen system, and its log.
pub struct GalleryPlugin;

impl Plugin for GalleryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GalleryState>()
            .add_systems(Update, gallery_ui)
            .add_systems(Update, log_on_change::<GalleryState>);
    }
}

fn gallery_ui(mut root: PlumeRoot, mut state: ResMut<GalleryState>, theme: Res<UiTheme>) {
    // Build against a local clone and write back with `set_if_neq`, so the
    // resource only registers as changed when a control actually changed it.
    let mut s = state.clone();
    // The three neutral surface shades, read fresh each frame so palette edits land.
    let surfaces = [
        (ThemeSlot::Neutral0, "Neutral0 — window"),
        (ThemeSlot::Neutral1, "Neutral1 — section body, dialog"),
        (ThemeSlot::Neutral2, "Neutral2 — group, section header"),
    ];
    root.screen(|ui| {
        // Equal spacers above and below centre the grid in the full-height screen column.
        ui.flex_spacer();
        ui.horizontal(|ui| {
            for (slot, name) in surfaces {
                gallery_column(ui, &mut s, theme.palette[slot], name);
            }
        })
        .align_items(AlignItems::Stretch);
        ui.flex_spacer();
    });
    state.set_if_neq(s);
}

/// One surface's column: its name, then an enabled and a disabled card
/// stacked below it — the two rows of the 3-by-2 grid.
fn gallery_column(ui: &mut Ui, state: &mut GalleryState, bg: Color, name: &str) {
    ui.vertical(|ui| {
        ui.caption(name);
        gallery_card(ui, state, bg, false);
        gallery_card(ui, state, bg, true);
    })
    .grow();
}

/// One card: a control of every kind, tinted `bg`, all disabled together when
/// `disabled`. The single function all 6 cards are built from — nothing here
/// is copy-pasted per cell.
fn gallery_card(ui: &mut Ui, state: &mut GalleryState, bg: Color, disabled: bool) {
    ui.vertical(|ui| {
        ui.caption(if disabled { "Disabled" } else { "Enabled" });

        ui.horizontal(|ui| {
            ui.button("Button").grow().enabled(!disabled);
            ui.button("Primary").primary().grow().enabled(!disabled);
            ui.button("Outline")
                .variant(ButtonVariant::Outline)
                .grow()
                .enabled(!disabled);
            ui.button("Plain")
                .variant(ButtonVariant::Plain)
                .grow()
                .enabled(!disabled);
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.checkbox(&mut state.checkbox_a, "Off").enabled(!disabled);
            ui.checkbox(&mut state.checkbox_b, "On").enabled(!disabled);
            ui.separator();
            ui.disclosure(&mut state.disclosure_a).enabled(!disabled);
            ui.disclosure(&mut state.disclosure_b).enabled(!disabled);
        });
        ui.horizontal(|ui| {
            ui.toggle(&mut state.toggle_a).enabled(!disabled);
            ui.toggle(&mut state.toggle_b).enabled(!disabled);
            ui.separator();
            ui.radio(&mut state.radio, RadioChoice::A, "A")
                .enabled(!disabled);
            ui.radio(&mut state.radio, RadioChoice::B, "B")
                .enabled(!disabled);
        });

        ui.slider(&mut state.slider, 0.0..=100.0).enabled(!disabled);
        ui.select(&mut state.select, &SELECT_OPTIONS)
            .enabled(!disabled);
        ui.text_edit(&mut state.text)
            .placeholder("Type here")
            .enabled(!disabled);
        ui.number(&mut state.number).enabled(!disabled);
    })
    .background(bg)
    .pad(UiRect::all(px(16)));
}
