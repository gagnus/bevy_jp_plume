//! Meta-module containing all controls (widgets that are interactive).

mod button;
mod checkbox;
mod color_swatch;
mod default_width;
mod disclosure;
mod listview;
mod menu;
mod number_input;
mod radio;
mod scrollbar;
mod select;
mod slider;
mod text_input;
mod toggle_switch;

pub use button::*;
pub use checkbox::*;
pub use color_swatch::*;
pub use default_width::DefaultWidth;
pub use disclosure::*;
pub use listview::{ListRowIndex, PlumeListRow, list_rows_from_strings};
pub use number_input::*;
pub use radio::*;
pub use scrollbar::*;
pub use select::*;
pub use slider::*;
pub use text_input::*;
pub use toggle_switch::*;

// Prop type on `PlumeTextInputProps`; re-exported so apps stay on the plume surface.
pub use bevy::text::EditableTextFilter;

use listview::ListViewPlugin;

use bevy::app::Plugin;
use default_width::DefaultWidthPlugin;

/// Plugin which registers all controls.
pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins((
            ButtonPlugin,
            CheckboxPlugin,
            ColorSwatchPlugin,
            DefaultWidthPlugin,
            DisclosurePlugin,
            ListViewPlugin,
            NumberInputPlugin,
            RadioPlugin,
            ScrollbarPlugin,
            SelectPlugin,
            SliderPlugin,
            TextInputPlugin,
            ToggleSwitchPlugin,
        ));
    }
}
