//! The retained layer [`imm`](crate::imm) builds on: scene functions and the
//! `PlumeX` scene components, spawned and driven by hand.
//!
//! Prefer `imm` — it drives all of this for you. Reach here to compose a widget
//! plume does not offer, or to read a control's state off its entity.

pub use crate::containers::{
    PlumeDialog, PlumeDialogBody, PlumeDialogBodyProps, PlumeDialogClose, PlumeDialogProps,
    PlumeSection, PlumeSectionProps, PlumeTab, PlumeTabProps, PlumeTabs, PlumeTabsProps,
    SectionCollapsed, TabTarget, column, flex_spacer, row, screen, separator, space, tab_body,
};
pub use crate::controls::{
    ColorPickerValue, ColorSwatchValue, DefaultWidth, EditableTextFilter, PlumeButton,
    PlumeButtonProps, PlumeCheckbox, PlumeCheckboxProps, PlumeColorEdit, PlumeColorEditProps,
    PlumeColorPicker, PlumeColorPickerProps, PlumeColorSwatch, PlumeDisclosure, PlumeNumberInput,
    PlumeNumberInputProps, PlumeRadio, PlumeRadioGroup, PlumeRadioProps, PlumeScrollbar,
    PlumeScrollbarProps, PlumeSelect, PlumeSelectProps, PlumeSlider, PlumeSliderProps,
    PlumeTextInput, PlumeTextInputProps, PlumeToggleSwitch, PlumeToolButton, PlumeXyPad,
    PlumeXyPadProps, ScrollbarGutter, SelectedIndex, SetSelectedIndex, SetTextInputValue,
    TextInputValue, XyPadDragging, XyPadLock, XyPadValue, select_options,
};
pub use crate::display::{caption, caption_color, caption_small_caps, fa_icon};
pub use crate::theme::components::{
    Flat, GRADIENT_AMOUNT, Inert, InheritableThemeTextColor, ThemeBackgroundColor,
    ThemeBackgroundGradient, ThemeBackgroundSlot, ThemeBorderColor, ThemeBorderSlot,
    ThemeTextColor, ThemedText, control_box_shadow,
};
pub use crate::utils::cursor::{DefaultCursor, EntityCursor, OverrideCursor};
pub use crate::utils::focus::{FocusIndicator, FocusWithinIndicator};
pub use crate::utils::font_styles::InheritableFont;

// The engine types a hand-built scene has to name: control state to read or
// seed, and the Tab-traversal scope for a root that is not `screen()`. Kept on
// plume's surface so an app never has to reach into `bevy_ui`/`bevy_ui_widgets`
// (see the Plume-only surface rule in CLAUDE.md).
pub use bevy::input_focus::tab_navigation::TabGroup;
pub use bevy::ui::{Checked, InteractionDisabled, Selected};
pub use bevy::ui_widgets::{Activate, SliderValue, ValueChange};

/// Design tokens: the names a retained scene styles itself by, resolved through
/// [`UiTheme`](crate::theme::UiTheme) to a palette slot and then a color.
pub mod tokens {
    pub use crate::theme::tokens::*;
}
