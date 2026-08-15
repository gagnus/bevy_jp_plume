//! Meta-module containing all controls (widgets that are interactive).

mod button;
mod checkbox;
mod color_edit;
mod color_picker;
mod color_swatch;
mod default_width;
mod disclosure;
mod menu;
mod number_input;
mod radio;
mod scrollbar;
mod select;
mod set_value;
mod slider;
mod text_input;
mod toggle_switch;
mod xy_pad;

use bevy::app::Plugin;
// Prop type on `PlumeTextInputProps`; re-exported so apps stay on the plume surface.
pub use bevy::text::EditableTextFilter;
use button::ButtonPlugin;
pub use button::{
    ButtonCheckableVariant, ButtonVariant, PlumeButton, PlumeButtonProps, PlumeToolButton,
};
pub(crate) use button::{ButtonOutline, set_icon_glyph};
use checkbox::CheckboxPlugin;
pub use checkbox::{PlumeCheckbox, PlumeCheckboxProps};
use color_edit::ColorEditPlugin;
pub use color_edit::{PlumeColorEdit, PlumeColorEditProps};
use color_picker::ColorPickerPlugin;
pub use color_picker::{ColorPickerValue, PlumeColorPicker, PlumeColorPickerProps};
use color_swatch::ColorSwatchPlugin;
pub use color_swatch::{ColorSwatchValue, PlumeColorSwatch};
pub use default_width::DefaultWidth;
use default_width::DefaultWidthPlugin;
use disclosure::DisclosurePlugin;
pub use disclosure::PlumeDisclosure;
use menu::MenuPlugin;
pub(crate) use menu::{
    MenuButtonRole, MenuOpen, MenuShortcutText, imm_menu_anchor, imm_menu_frame, menu_anchor_base,
};
pub use menu::{PlumeMenuBar, PlumeMenuButton, PlumeMenuButtonProps, menu_anchor};
pub(crate) use number_input::NumberInputFrame;
use number_input::NumberInputPlugin;
pub use number_input::{NoDrag, PlumeNumberInput, PlumeNumberInputProps};
use radio::RadioPlugin;
pub use radio::{PlumeRadio, PlumeRadioGroup, PlumeRadioProps};
use scrollbar::ScrollbarPlugin;
pub use scrollbar::{PlumeScrollbar, PlumeScrollbarProps, ScrollbarGutter, ScrollbarHidden};
use select::SelectPlugin;
pub(crate) use select::set_select_max_visible;
pub use select::{PlumeSelect, PlumeSelectProps, SelectedIndex, select_options};
pub use set_value::SetValue;
pub(crate) use set_value::SetValuePlugin;
use slider::SliderPlugin;
pub use slider::{PlumeSlider, PlumeSliderProps};
use text_input::TextInputPlugin;
pub use text_input::{NoSelectAllOnFocus, PlumeTextInput, PlumeTextInputProps, TextInputValue};
pub(crate) use text_input::{
    TextInputField, set_editable_text, text_input_field, text_input_frame, text_input_outline,
    text_input_placeholder, text_input_suffix,
};
pub use toggle_switch::PlumeToggleSwitch;
use toggle_switch::ToggleSwitchPlugin;
use xy_pad::XyPadPlugin;
pub(crate) use xy_pad::{PlumeXyPad, XyPadLock, XyPadValue};

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
            MenuPlugin,
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
