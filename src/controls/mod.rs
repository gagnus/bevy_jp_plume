//! Meta-module containing all controls (widgets that are interactive).

mod button;
mod checkbox;
mod color_edit;
mod color_picker;
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
mod xy_pad;

pub use button::*;
pub use checkbox::*;
pub use color_edit::*;
pub use color_picker::*;
pub use color_swatch::*;
pub use default_width::DefaultWidth;
pub use disclosure::*;
pub use listview::{PlumeSelectOption, SelectOptionIndex, options_from_strings};
pub use number_input::*;
pub use radio::*;
pub use scrollbar::*;
pub use select::*;
pub use slider::*;
pub use text_input::*;
pub use toggle_switch::*;
pub use xy_pad::*;

// Prop type on `PlumeTextInputProps`; re-exported so apps stay on the plume surface.
pub use bevy::text::EditableTextFilter;

use listview::SelectOptionsPlugin;

use bevy::app::Plugin;
use default_width::DefaultWidthPlugin;

/// Plugin which registers all controls.
pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        // Split into two tuples: `add_plugins` accepts tuples up to arity 15.
        app.add_plugins((
            ButtonPlugin,
            CheckboxPlugin,
            ColorEditPlugin,
            ColorPickerPlugin,
            ColorSwatchPlugin,
            DefaultWidthPlugin,
            DisclosurePlugin,
            SelectOptionsPlugin,
        ));
        app.add_plugins((
            NumberInputPlugin,
            RadioPlugin,
            ScrollbarPlugin,
            SelectPlugin,
            SliderPlugin,
            TextInputPlugin,
            ToggleSwitchPlugin,
            XyPadPlugin,
        ));
    }
}
