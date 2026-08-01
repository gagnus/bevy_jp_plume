//! Meta-module containing all controls (widgets that are interactive).

mod button;
mod checkbox;
mod color_edit;
mod color_picker;
mod color_swatch;
mod default_width;
mod disclosure;
mod number_input;
mod radio;
mod scrollbar;
mod select;
mod set_value;
mod slider;
mod text_input;
mod toggle_switch;
mod xy_pad;

pub use button::{ButtonVariant, PlumeButton, PlumeButtonProps, PlumeToolButton};
pub use checkbox::{PlumeCheckbox, PlumeCheckboxProps};
pub use color_edit::{PlumeColorEdit, PlumeColorEditProps};
pub use color_picker::{ColorPickerValue, PlumeColorPicker, PlumeColorPickerProps};
pub use color_swatch::{ColorSwatchValue, PlumeColorSwatch};
pub use default_width::DefaultWidth;
pub use disclosure::PlumeDisclosure;
pub use number_input::{PlumeNumberInput, PlumeNumberInputProps};
pub use radio::{PlumeRadio, PlumeRadioGroup, PlumeRadioProps};
pub use scrollbar::{PlumeScrollbar, PlumeScrollbarProps, ScrollbarGutter};
pub use select::{PlumeSelect, PlumeSelectProps, SelectedIndex, select_options};
pub use set_value::SetValue;
pub(crate) use set_value::SetValuePlugin;
pub use slider::{PlumeSlider, PlumeSliderProps};
pub use text_input::{NoSelectAllOnFocus, PlumeTextInput, PlumeTextInputProps, TextInputValue};
pub use toggle_switch::PlumeToggleSwitch;
pub(crate) use xy_pad::{PlumeXyPad, XyPadLock, XyPadValue};

pub(crate) use button::ButtonOutline;
pub(crate) use select::set_select_max_visible;
pub(crate) use text_input::{
    TextInputField, set_editable_text, text_input_field, text_input_frame, text_input_placeholder,
    text_input_suffix,
};

// Prop type on `PlumeTextInputProps`; re-exported so apps stay on the plume surface.
pub use bevy::text::EditableTextFilter;

use bevy::app::Plugin;
use button::ButtonPlugin;
use checkbox::CheckboxPlugin;
use color_edit::ColorEditPlugin;
use color_picker::ColorPickerPlugin;
use color_swatch::ColorSwatchPlugin;
use default_width::DefaultWidthPlugin;
use disclosure::DisclosurePlugin;
use number_input::NumberInputPlugin;
use radio::RadioPlugin;
use scrollbar::ScrollbarPlugin;
use select::SelectPlugin;
use slider::SliderPlugin;
use text_input::TextInputPlugin;
use toggle_switch::ToggleSwitchPlugin;
use xy_pad::XyPadPlugin;

// Plugin which registers all controls.
pub(crate) struct ControlsPlugin;

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
        ));
        app.add_plugins((
            NumberInputPlugin,
            RadioPlugin,
            ScrollbarPlugin,
            SelectPlugin,
            SetValuePlugin,
            SliderPlugin,
            TextInputPlugin,
            ToggleSwitchPlugin,
            XyPadPlugin,
        ));
    }
}
