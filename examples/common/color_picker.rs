//! Colour-picker demo: the retained `PlumeColorPicker` control driven through the
//! immediate-mode `ui.color_picker(&mut Color)` wrapper. The picker keeps its own
//! HSV working-truth, so the app only holds a plain `Color`.
use bevy::color::Color;
use bevy::prelude::*;
use bevy_jp_plume::imm::{PlumeImm, PlumeRoot};

use super::log_on_change;

/// The demo's colour, round-tripped through the picker each frame.
#[derive(Resource, Debug, Clone, Copy)]
pub struct ColorPickerState {
    color: Color,
}

impl Default for ColorPickerState {
    fn default() -> Self {
        Self {
            color: Color::hsv(210.0, 0.7, 0.9),
        }
    }
}

/// Adds the inline colour-picker demo.
pub struct ColorPickerPlugin;

impl Plugin for ColorPickerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ColorPickerState>()
            .add_systems(Update, color_picker_ui)
            .add_systems(Update, log_on_change::<ColorPickerState>);
    }
}

fn color_picker_ui(mut root: PlumeRoot, mut state: ResMut<ColorPickerState>) {
    let mut color = state.color;
    root.screen(|ui| {
        ui.flex_spacer();
        ui.horizontal(|ui| {
            ui.flex_spacer();
            ui.color_picker(&mut color);
            ui.flex_spacer();
        });
        ui.horizontal(|ui| {
            ui.flex_spacer();
            ui.caption("Swatch");
            ui.color_edit(&mut color);
            ui.flex_spacer();
        });
        ui.flex_spacer();
    });
    if state.color != color {
        state.color = color;
    }
}
