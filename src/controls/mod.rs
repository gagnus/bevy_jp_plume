//! Meta-module containing all controls (widgets that are interactive).

mod button;
mod checkbox;
mod color_swatch;
mod listview;
mod menu;
mod radio;
mod scrollbar;
mod select;
mod slider;
mod text_input;
mod toggle_switch;

pub use button::*;
pub use checkbox::*;
pub use color_swatch::*;
pub use listview::PlumeListRow;
pub use radio::*;
pub use scrollbar::*;
pub use select::*;
pub use slider::*;
pub use text_input::*;
pub use toggle_switch::*;

use listview::ListViewPlugin;

use crate::alpha_pattern::AlphaPatternPlugin;
use bevy_app::Plugin;

/// Plugin which registers all controls.
pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins((
            AlphaPatternPlugin,
            ButtonPlugin,
            CheckboxPlugin,
            ColorSwatchPlugin,
            ListViewPlugin,
            RadioPlugin,
            ScrollbarPlugin,
            SelectPlugin,
            SliderPlugin,
            TextInputPlugin,
            ToggleSwitchPlugin,
        ));
    }
}
