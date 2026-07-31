//! Font-scaling acceptance dialog
use bevy::prelude::*;
use bevy_jp_plume::prelude::*;

use super::debug_hub::{AddDebugDialog, DebugDialogRegistry};
use super::log_on_change;

const TITLE: &str = "Font Scaling";

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct FontScalingDemo {
    enabled: bool,
    flavour: Flavour,
    volume: f32,
    name: String,
    color: Color,
}

impl Default for FontScalingDemo {
    fn default() -> Self {
        Self {
            enabled: true,
            flavour: Flavour::default(),
            volume: 0.5,
            name: "".into(),
            color: Color::srgb(0.3, 0.6, 0.9),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Flavour {
    #[default]
    Vanilla,
    Chocolate,
    Strawberry,
}

/// Adds the font-scaling dialog: its resource, its dialog system + hub entry, and its log.
pub struct FontScalingPlugin(pub bool);

impl Plugin for FontScalingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FontScalingDemo>()
            .add_debug_dialog(
                TITLE,
                font_awesome::solid::TEXT_HEIGHT,
                self.0,
                font_scaling_dialog,
            )
            .add_systems(Update, log_on_change::<FontScalingDemo>);
    }
}

fn font_scaling_dialog(
    mut root: PlumeRoot,
    mut registry: ResMut<DebugDialogRegistry>,
    mut demo: ResMut<FontScalingDemo>,
) {
    let mut s = demo.clone();
    let mut open = registry.is_open(TITLE);
    root.dialog(TITLE, &mut open)
        .width(px(640))
        .height(px(200))
        .at(px(640), px(60))
        .icon(font_awesome::solid::TEXT_HEIGHT)
        .show(|ui| {
            ui.horizontal(|ui| {
                // Shared backing fields: flipping a control moves its twin.
                ui.vertical(|ui| demo_column(ui, &mut s, "11 px"))
                    .grow()
                    .font_size(FontSize::Px(11.0));
                ui.vertical(|ui| demo_column(ui, &mut s, "18 px"))
                    .grow()
                    .font_size(FontSize::Px(18.0));
            })
            .align_items(AlignItems::Start);

            ui.separator();

            ui.horizontal(|ui| {
                ui.vertical(|ui| color_column(ui, "Raw orange column"))
                    .grow()
                    .text_color(Color::srgb(1.0, 0.6, 0.2));
                ui.vertical(|ui| color_column(ui, "Accent slot column"))
                    .grow()
                    .text_color_slot(ThemeSlot::Accent1);
            })
            .align_items(AlignItems::Start);
        })
        .map(|dialog_response| dialog_response.font_size(FontSize::Px(20.0)));
    if open != registry.is_open(TITLE) {
        registry.set_open(TITLE, open);
    }
    demo.set_if_neq(s);
}

fn demo_column(ui: &mut Ui, s: &mut FontScalingDemo, heading: &str) {
    ui.horizontal(|ui| {
        ui.caption(heading).text_color_slot(ThemeSlot::Text0);
        ui.icon(font_awesome::solid::FONT);
    });
    ui.section("Section", |ui| {
        ui.select(&mut s.flavour, |select| {
            select.option(Flavour::Vanilla, "Vanilla");
            select.option(Flavour::Chocolate, "Chocolate");
            select.option(Flavour::Strawberry, "Strawberry");
        });
        ui.slider(&mut s.volume, 0.0..=1.0);
        ui.number(&mut s.volume).precision(2).suffix("%");
        ui.text_edit(&mut s.name).placeholder("Name");
    });
    ui.horizontal(|ui| {
        ui.button("Button");
        ui.button("Primary").primary();
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.enabled, "A");
        ui.toggle(&mut s.enabled);
        ui.radio(&mut s.enabled, true, "B");
        // Click the swatch: the picker popup inherits the column's font too.
        ui.color_edit(&mut s.color);
    });
    // Per-widget override on a bare caption — a text leaf styling itself.
    ui.caption("28 px caption").font_size(FontSize::Px(28.0));
    // Nearest source wins: nested re-establishment back to the standard size.
    ui.vertical(|ui| {
        ui.caption("Nested 14 px again");
    })
    .font_size(FontSize::Px(14.0));
}

// The column color reaches captions and icons (the section body included);
// control roots keep their state colors; per-leaf establishment wins locally.
fn color_column(ui: &mut Ui, heading: &str) {
    ui.horizontal(|ui| {
        ui.caption(heading);
        ui.icon(font_awesome::solid::PALETTE);
    });
    ui.section("Colored Section", |ui| {
        ui.caption("Caption through a section body");
    });
    ui.caption("Per-leaf slot override")
        .text_color_slot(ThemeSlot::Text0);
    ui.button("State-styled button");
    // Nearest source wins for color too.
    ui.vertical(|ui| {
        ui.caption("Nested raw red");
    })
    .text_color(Color::srgb(0.9, 0.3, 0.3));
    ui.caption("back to inherited");
}
