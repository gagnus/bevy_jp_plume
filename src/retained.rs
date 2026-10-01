//! The retained layer [`imm`](crate::imm) builds on: scene functions and the
//! `PlumeX` scene components, spawned and driven by hand.
//!
//! Prefer `imm` - it drives all of this for you. Reach here to compose a widget
//! plume does not offer, or to read a control's state off its entity.

// The engine types a hand-built scene has to name: control state to read or
// seed, and the Tab-traversal scope for a root that is not `screen()`. Kept on
// plume's surface so an app never has to reach into `bevy_ui`/`bevy_ui_widgets`.
pub use bevy::app::Propagate;
pub use bevy::input_focus::tab_navigation::TabGroup;
pub use bevy::ui::{Checkable, Checked, InteractionDisabled, Selected};
pub use bevy::ui_widgets::{Activate, RequestClose, SliderValue, ValueChange};

pub use crate::body::{BodyGap, BodyPadding};
pub use crate::containers::{
    CloseRequested, PlumeDialog, PlumeDialogProps, PlumeModal, PlumeModalProps, PlumePopup,
    PlumePopupProps, PlumeReorderable, PlumeReorderableItem, PlumeReorderableItemProps,
    PlumeReorderableProps, PlumeScrollArea, PlumeScrollAreaProps, PlumeSection, PlumeSectionProps,
    PlumeSplitter, PlumeSplitterProps, PlumeTab, PlumeTabProps, PlumeTabs, PlumeTabsProps,
    PopupDismiss, PopupPlacement, PopupSocket, ReorderMove, Screen, ScrollAxis, SectionCollapsed,
    SplitAxis, SplitCollapsible, SplitDividerAutoHide, SplitMin, SplitPane, SplitSize, SplitSized,
    TabTarget, close_popup, column, modal_title, popup_socket, row, screen, tab_body, tab_label,
};
pub use crate::controls::{
    CheckerUnderlay, ColorPickerValue, ColorSwatchValue, EditableTextFilter, NoBlurOnEnter, NoDrag,
    NoSelectAllOnFocus, NoVerticalArrows, PlumeButton, PlumeButtonProps, PlumeCheckbox,
    PlumeCheckboxProps, PlumeColorEdit, PlumeColorEditProps, PlumeColorPicker,
    PlumeColorPickerProps, PlumeColorSwatch, PlumeColorSwatchProps, PlumeDisclosure, PlumeMenuBar,
    PlumeMenuButton, PlumeMenuButtonProps, PlumeNumberInput, PlumeNumberInputProps, PlumeRadio,
    PlumeRadioGroup, PlumeRadioProps, PlumeScrollbar, PlumeScrollbarProps, PlumeSelect,
    PlumeSelectProps, PlumeSlider, PlumeSliderProps, PlumeTextInput, PlumeTextInputProps,
    PlumeToggleSwitch, PlumeToolButton, ScrollbarGutter, ScrollbarHidden, SelectedIndex,
    SetSelectOptions, TextInputValue, menu_anchor, select_options,
};
pub use crate::default_width::DefaultWidth;
pub use crate::display::{
    SeparatorBleed, Tooltip, TooltipContent, TooltipSettings, TooltipWhenClipped, caption,
    flex_spacer, icon, separator, space,
};
pub use crate::set_value::SetValue;
// The propagation source a retained scene sets a subtree's [`ThemeId`] with.
pub use crate::theme::ThemeId;
pub use crate::theme::components::{
    Flat, GradientAmount, Inert, InheritableTextColor, InheritableThemeTextSlot,
    ThemeBackgroundSlot, ThemeBorderSlot, ThemeTextSlot, control_box_shadow, dialog_box_shadow,
};
pub use crate::utils::cursor::{CapturePointer, DefaultCursor, EntityCursor, OverrideCursor};
pub use crate::utils::focus::{FocusIndicator, FocusWithinIndicator, InsetFocusRing};
pub use crate::utils::font_styles::{FontStyleSystems, InheritableFont, PlumeFontSize, small_caps};
