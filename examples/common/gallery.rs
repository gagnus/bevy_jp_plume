//! Control gallery feature plugin: a 3-by-2 grid of "one of each control" cards over
//! the three neutral surface slots, enabled and disabled. The `showcase` backdrop.
use bevy::prelude::*;
use bevy_jp_plume::prelude::*;

use super::log_on_change;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum SelectChoice {
    #[default]
    Alpha,
    Beta,
    Gamma,
    Delta,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum RadioChoice {
    #[default]
    A,
    B,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum TabChoice {
    #[default]
    First,
    Second,
    Third,
}

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct GalleryState {
    checkbox_a: bool,
    checkbox_b: bool,
    toggle_normal_a: bool,
    toggle_normal_b: bool,
    toggle_plain_a: bool,
    toggle_plain_b: bool,
    toggle_outline_a: bool,
    toggle_outline_b: bool,
    disclosure_a: bool,
    disclosure_b: bool,
    toggle_a: bool,
    toggle_b: bool,
    radio: RadioChoice,
    slider: f32,
    select: SelectChoice,
    text: String,
    number: f32,
    tab: TabChoice,
    autosave: bool,
    color: Color,
}

impl Default for GalleryState {
    fn default() -> Self {
        Self {
            checkbox_a: false,
            checkbox_b: true,
            toggle_normal_a: false,
            toggle_normal_b: true,
            toggle_plain_a: false,
            toggle_plain_b: true,
            toggle_outline_a: false,
            toggle_outline_b: true,
            disclosure_a: false,
            disclosure_b: true,
            toggle_a: false,
            toggle_b: true,
            radio: RadioChoice::default(),
            slider: 30.0,
            select: SelectChoice::default(),
            text: String::new(),
            number: 4.0,
            tab: TabChoice::default(),
            autosave: true,
            color: Color::srgb(0.62, 0.31, 0.24),
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

fn gallery_ui(mut root: PlumeRoot, mut state: ResMut<GalleryState>) {
    let mut s = state.clone();
    let surfaces = [
        (ThemeSlot::Neutral0, "Neutral0 — Window"),
        (ThemeSlot::Neutral1, "Neutral1 — Dialog"),
        (ThemeSlot::Neutral2, "Neutral2 — Section Header"),
    ];
    root.screen(|ui| {
        ui.flex_spacer();
        ui.horizontal(|ui| {
            for (slot, name) in surfaces {
                gallery_column(ui, &mut s, slot, name);
            }
        })
        .align_items(AlignItems::Stretch);
        ui.flex_spacer();
    })
    .background_slot(ThemeSlot::Neutral0)
    .padding(size::SPACE * 4.0);
    state.set_if_neq(s);
}

fn gallery_column(ui: &mut Ui, state: &mut GalleryState, bg_slot: ThemeSlot, name: &str) {
    ui.vertical(|ui| {
        ui.caption(name);
        gallery_card(ui, state, bg_slot, false);
        gallery_card(ui, state, bg_slot, true);
    })
    .grow();
}

fn gallery_card(ui: &mut Ui, state: &mut GalleryState, bg_slot: ThemeSlot, disabled: bool) {
    ui.vertical(|ui| {
        ui.caption(if disabled { "Disabled" } else { "Enabled" });

        ui.horizontal(|ui| {
            ui.menu_bar(|bar| {
                bar.menu("File", |menu| {
                    menu.item("New").shortcut("Ctrl+N");
                    menu.item("Save").enabled(false);
                    menu.separator();
                    menu.submenu("Recent", |menu| {
                        menu.item("alpha.scn");
                        menu.item("beta.scn");
                    });
                })
                .enabled(!disabled);
                bar.menu("Edit", |menu| {
                    menu.item("Undo").shortcut("Ctrl+Z");
                    menu.item_toggle("Autosave", &mut state.autosave);
                })
                .enabled(!disabled);
            });
            ui.flex_spacer();
            ui.tabs(&mut state.tab, |tabs| {
                tabs.tab(TabChoice::First, "First")
                    .enabled(!disabled)
                    .no_body();
                tabs.tab(TabChoice::Second, "Second")
                    .enabled(!disabled)
                    .no_body();
                tabs.tab(TabChoice::Third, "Third")
                    .enabled(!disabled)
                    .no_body();
            });
        });

        ui.horizontal(|ui| {
            ui.button("Button").grow().enabled(!disabled);
            ui.button("Primary").primary().grow().enabled(!disabled);
            ui.button("Danger").danger().grow().enabled(!disabled);
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
            ui.tool_button(font_awesome::solid::BOLD)
                .checkable(&mut state.toggle_normal_a)
                .tooltip("Normal toggle")
                .enabled(!disabled);
            ui.tool_button(font_awesome::solid::BOLD)
                .checkable(&mut state.toggle_normal_b)
                .tooltip("Normal toggle")
                .enabled(!disabled);
            ui.tool_button(font_awesome::solid::MAGNET)
                .checkable(&mut state.toggle_plain_a)
                .variant(ButtonCheckableVariant::Plain)
                .tooltip("Plain toggle")
                .enabled(!disabled);
            ui.tool_button(font_awesome::solid::MAGNET)
                .checkable(&mut state.toggle_plain_b)
                .variant(ButtonCheckableVariant::Plain)
                .tooltip("Plain toggle")
                .enabled(!disabled);
            ui.tool_button(font_awesome::solid::BORDER_ALL)
                .checkable(&mut state.toggle_outline_a)
                .variant(ButtonCheckableVariant::Outline)
                .tooltip("Outline toggle")
                .enabled(!disabled);
            ui.tool_button(font_awesome::solid::BORDER_ALL)
                .checkable(&mut state.toggle_outline_b)
                .variant(ButtonCheckableVariant::Outline)
                .tooltip("Outline toggle")
                .enabled(!disabled);
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
            ui.separator();
            ui.color_edit(&mut state.color).enabled(!disabled);
            ui.select(&mut state.select, |select| {
                select.option(SelectChoice::Alpha, "Alpha");
                select.option(SelectChoice::Beta, "Beta");
                select.option(SelectChoice::Gamma, "Gamma");
                select.option(SelectChoice::Delta, "Delta").enabled(false);
            })
            .max_visible(3)
            .enabled(!disabled);
        });

        ui.slider(&mut state.slider, 0.0..=100.0).enabled(!disabled);
        ui.horizontal(|ui| {
            ui.text_edit(&mut state.text)
                .placeholder("Type here")
                .enabled(!disabled)
                .grow();
            ui.text_edit(&mut state.text)
                .select_on_focus(false)
                .placeholder("Caret on click")
                .enabled(!disabled)
                .grow();
        });
        ui.horizontal(|ui| {
            ui.number(&mut state.number).enabled(!disabled).grow();
            ui.number(&mut state.number)
                .suffix("px")
                .enabled(!disabled)
                .grow();
        });
    })
    .background_slot(bg_slot)
    .padding(size::SPACE * 2.0)
    .border_radius(size::CORNER_RADIUS);
}
