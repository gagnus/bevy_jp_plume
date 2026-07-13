//! Design tokens used by Plume themes.
//!
//! The term "design token" is commonly used in UX design to mean the smallest unit of a theme,
//! similar in concept to a CSS variable. Each token represents an assignment of a color or
//! value to a specific visual aspect of a widget, such as background or border.

use crate::theme::ThemeToken;

/// Window background
pub const WINDOW_BG: ThemeToken = ThemeToken::new_static("plume.window.bg");

/// Regular text
pub const TEXT_MAIN: ThemeToken = ThemeToken::new_static("plume.text.main");
/// Dim text
pub const TEXT_DIM: ThemeToken = ThemeToken::new_static("plume.text.dim");

// Normal buttons

/// Regular button background
pub const BUTTON_BG: ThemeToken = ThemeToken::new_static("plume.button.bg");
/// Regular button background (hovered)
pub const BUTTON_BG_HOVER: ThemeToken = ThemeToken::new_static("plume.button.bg.hover");
/// Regular button background (disabled)
pub const BUTTON_BG_DISABLED: ThemeToken = ThemeToken::new_static("plume.button.bg.disabled");
/// Regular button background (pressed)
pub const BUTTON_BG_PRESSED: ThemeToken = ThemeToken::new_static("plume.button.bg.pressed");
/// Regular button text
pub const BUTTON_TEXT: ThemeToken = ThemeToken::new_static("plume.button.txt");
/// Regular button text (disabled)
pub const BUTTON_TEXT_DISABLED: ThemeToken = ThemeToken::new_static("plume.button.txt.disabled");

// Primary ("default") buttons

/// Primary button background
pub const BUTTON_PRIMARY_BG: ThemeToken = ThemeToken::new_static("plume.button.primary.bg");
/// Primary button background (hovered)
pub const BUTTON_PRIMARY_BG_HOVER: ThemeToken =
    ThemeToken::new_static("plume.button.primary.bg.hover");
/// Primary button background (disabled)
pub const BUTTON_PRIMARY_BG_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.button.primary.bg.disabled");
/// Primary button background (pressed)
pub const BUTTON_PRIMARY_BG_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.button.primary.bg.pressed");
/// Primary button text
pub const BUTTON_PRIMARY_TEXT: ThemeToken = ThemeToken::new_static("plume.button.primary.txt");
/// Primary button text (disabled)
pub const BUTTON_PRIMARY_TEXT_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.button.primary.txt.disabled");

// Plain buttons (transparent background)

/// Plain button background
pub const BUTTON_PLAIN_BG: ThemeToken = ThemeToken::new_static("plume.button.plain.bg");
/// Plain button background (hovered)
pub const BUTTON_PLAIN_BG_HOVER: ThemeToken = ThemeToken::new_static("plume.button.plain.bg.hover");
/// Plain button background (disabled)
pub const BUTTON_PLAIN_BG_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.button.plain.bg.disabled");
/// Plain button background (pressed)
pub const BUTTON_PLAIN_BG_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.button.plain.bg.pressed");

// Slider

/// Background for slider
pub const SLIDER_BG: ThemeToken = ThemeToken::new_static("plume.slider.bg");
/// Background for slider (hovered)
pub const SLIDER_BG_HOVER: ThemeToken = ThemeToken::new_static("plume.slider.bg.hover");
/// Background for slider (pressed)
pub const SLIDER_BG_PRESSED: ThemeToken = ThemeToken::new_static("plume.slider.bg.pressed");
/// Background for slider (disabled)
pub const SLIDER_BG_DISABLED: ThemeToken = ThemeToken::new_static("plume.slider.bg.disabled");
/// Fill color for slider
pub const SLIDER_BAR: ThemeToken = ThemeToken::new_static("plume.slider.bar");
/// Fill color for slider (hovered)
pub const SLIDER_BAR_HOVER: ThemeToken = ThemeToken::new_static("plume.slider.bar.hover");
/// Fill color for slider (pressed)
pub const SLIDER_BAR_PRESSED: ThemeToken = ThemeToken::new_static("plume.slider.bar.pressed");
/// Fill color for slider (disabled)
pub const SLIDER_BAR_DISABLED: ThemeToken = ThemeToken::new_static("plume.slider.bar.disabled");
/// Slider thumb
pub const SLIDER_THUMB: ThemeToken = ThemeToken::new_static("plume.slider.thumb");

// Scrollbar

/// Background for scrollbar
pub const SCROLLBAR_BG: ThemeToken = ThemeToken::new_static("plume.scrollbar.bg");
/// Background for scrollbar moving bar
pub const SCROLLBAR_THUMB: ThemeToken = ThemeToken::new_static("plume.scrollbar.thumb");
/// Background for scrollbar moving bar (hovered)
pub const SCROLLBAR_THUMB_HOVER: ThemeToken = ThemeToken::new_static("plume.scrollbar.thumb.hover");

// Checkbox

/// Checkbox background around the checkmark
pub const CHECKBOX_BG: ThemeToken = ThemeToken::new_static("plume.checkbox.bg");
/// Checkbox background around the checkmark (hovered)
pub const CHECKBOX_BG_HOVER: ThemeToken = ThemeToken::new_static("plume.checkbox.bg.hover");
/// Checkbox background around the checkmark (pressed)
pub const CHECKBOX_BG_PRESSED: ThemeToken = ThemeToken::new_static("plume.checkbox.bg.pressed");
/// Checkbox border around the checkmark (disabled)
pub const CHECKBOX_BG_DISABLED: ThemeToken = ThemeToken::new_static("plume.checkbox.bg.disabled");
/// Checkbox background around the checkmark (checked)
pub const CHECKBOX_BG_CHECKED: ThemeToken = ThemeToken::new_static("plume.checkbox.bg.checked");
/// Checkbox background around the checkmark (checked+hover)
pub const CHECKBOX_BG_CHECKED_HOVER: ThemeToken =
    ThemeToken::new_static("plume.checkbox.bg.checked.hover");
/// Checkbox background around the checkmark (checked+pressed)
pub const CHECKBOX_BG_CHECKED_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.checkbox.bg.checked.pressed");
/// Checkbox border around the checkmark (checked+disabled)
pub const CHECKBOX_BG_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.checkbox.bg.checked.disabled");
/// Checkbox border around the checkmark
pub const CHECKBOX_BORDER: ThemeToken = ThemeToken::new_static("plume.checkbox.border");
/// Checkbox border around the checkmark (hovered)
pub const CHECKBOX_BORDER_HOVER: ThemeToken = ThemeToken::new_static("plume.checkbox.border.hover");
/// Checkbox border around the checkmark (pressed)
pub const CHECKBOX_BORDER_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.checkbox.border.pressed");
/// Checkbox border around the checkmark (disabled)
pub const CHECKBOX_BORDER_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.checkbox.border.disabled");
/// Checkbox border around the checkmark (checked)
pub const CHECKBOX_BORDER_CHECKED: ThemeToken =
    ThemeToken::new_static("plume.checkbox.border.checked");
/// Checkbox border around the checkmark (checked+hovered)
pub const CHECKBOX_BORDER_CHECKED_HOVER: ThemeToken =
    ThemeToken::new_static("plume.checkbox.border.checked.hover");
/// Checkbox border around the checkmark (checked+pressed)
pub const CHECKBOX_BORDER_CHECKED_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.checkbox.border.checked.pressed");
/// Checkbox border around the checkmark (checked+disabled)
pub const CHECKBOX_BORDER_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.checkbox.border.checked.disabled");
/// Checkbox check mark
pub const CHECKBOX_MARK: ThemeToken = ThemeToken::new_static("plume.checkbox.mark");
/// Checkbox check mark (disabled)
pub const CHECKBOX_MARK_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.checkbox.mark.disabled");
/// Checkbox label text
pub const CHECKBOX_TEXT: ThemeToken = ThemeToken::new_static("plume.checkbox.text");
/// Checkbox label text (disabled)
pub const CHECKBOX_TEXT_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.checkbox.text.disabled");

// Radio button

/// Background of the radio button
pub const RADIO_BG: ThemeToken = ThemeToken::new_static("plume.radio.bg");
/// Background of the radio button (hovered)
pub const RADIO_BG_HOVER: ThemeToken = ThemeToken::new_static("plume.radio.bg.hover");
/// Background of the radio button (pressed)
pub const RADIO_BG_PRESSED: ThemeToken = ThemeToken::new_static("plume.radio.bg.pressed");
/// Background of the radio button (disabled)
pub const RADIO_BG_DISABLED: ThemeToken = ThemeToken::new_static("plume.radio.bg.disabled");
/// Background of the radio button (checked)
pub const RADIO_BG_CHECKED: ThemeToken = ThemeToken::new_static("plume.radio.bg.checked");
/// Background of the radio button (checked+hovered)
pub const RADIO_BG_CHECKED_HOVER: ThemeToken =
    ThemeToken::new_static("plume.radio.bg.checked.hover");
/// Background of the radio button (checked+pressed)
pub const RADIO_BG_CHECKED_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.radio.bg.checked.pressed");
/// Background of the radio button (checked+disabled)
pub const RADIO_BG_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.radio.bg.checked.disabled");
/// Border around the radio button
pub const RADIO_BORDER: ThemeToken = ThemeToken::new_static("plume.radio.border");
/// Border around the radio button (hovered)
pub const RADIO_BORDER_HOVER: ThemeToken = ThemeToken::new_static("plume.radio.border.hover");
/// Border around the radio button (pressed)
pub const RADIO_BORDER_PRESSED: ThemeToken = ThemeToken::new_static("plume.radio.border.pressed");
/// Border around the radio button (disabled)
pub const RADIO_BORDER_DISABLED: ThemeToken = ThemeToken::new_static("plume.radio.border.disabled");
/// Border around the radio button (checked)
pub const RADIO_BORDER_CHECKED: ThemeToken = ThemeToken::new_static("plume.radio.border.checked");
/// Border around the radio button (checked+hovered)
pub const RADIO_BORDER_CHECKED_HOVER: ThemeToken =
    ThemeToken::new_static("plume.radio.border.checked.hover");
/// Border around the radio button (checked+pressed)
pub const RADIO_BORDER_CHECKED_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.radio.border.checked.pressed");
/// Border around the radio button (checked+disabled)
pub const RADIO_BORDER_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.radio.border.checked.disabled");
/// Radio check mark
pub const RADIO_MARK: ThemeToken = ThemeToken::new_static("plume.radio.mark");
/// Radio check mark (hovered)
pub const RADIO_MARK_HOVER: ThemeToken = ThemeToken::new_static("plume.radio.mark.hover");
/// Radio check mark (pressed)
pub const RADIO_MARK_PRESSED: ThemeToken = ThemeToken::new_static("plume.radio.mark.pressed");
/// Radio check mark (disabled)
pub const RADIO_MARK_DISABLED: ThemeToken = ThemeToken::new_static("plume.radio.mark.disabled");
/// Radio label text
pub const RADIO_TEXT: ThemeToken = ThemeToken::new_static("plume.radio.text");
/// Radio label text (disabled)
pub const RADIO_TEXT_DISABLED: ThemeToken = ThemeToken::new_static("plume.radio.text.disabled");

// Toggle Switch

/// Switch background around the switch
pub const SWITCH_BG: ThemeToken = ThemeToken::new_static("plume.switch.bg");
/// Switch background around the switch (hovered)
pub const SWITCH_BG_HOVER: ThemeToken = ThemeToken::new_static("plume.switch.bg.hover");
/// Switch background around the switch (pressed)
pub const SWITCH_BG_PRESSED: ThemeToken = ThemeToken::new_static("plume.switch.bg.pressed");
/// Switch background around the switch (disabled)
pub const SWITCH_BG_DISABLED: ThemeToken = ThemeToken::new_static("plume.switch.bg.disabled");
/// Switch background around the switch (checked)
pub const SWITCH_BG_CHECKED: ThemeToken = ThemeToken::new_static("plume.switch.bg.checked");
/// Switch background around the switch (checked+hover)
pub const SWITCH_BG_CHECKED_HOVER: ThemeToken =
    ThemeToken::new_static("plume.switch.bg.checked.hover");
/// Switch background around the switch (checked+pressed)
pub const SWITCH_BG_CHECKED_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.switch.bg.checked.pressed");
/// Switch background around the switch (checked+disabled)
pub const SWITCH_BG_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.switch.bg.checked.disabled");
/// Switch border around the switch
pub const SWITCH_BORDER: ThemeToken = ThemeToken::new_static("plume.switch.border");
/// Switch border around the switch (hovered)
pub const SWITCH_BORDER_HOVER: ThemeToken = ThemeToken::new_static("plume.switch.border.hover");
/// Switch border around the switch (pressed)
pub const SWITCH_BORDER_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.switch.border.hover.pressed");
/// Switch border around the switch (disabled)
pub const SWITCH_BORDER_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.switch.border.disabled");
/// Switch border around the switch (checked)
pub const SWITCH_BORDER_CHECKED: ThemeToken = ThemeToken::new_static("plume.switch.border.checked");
/// Switch border around the switch (checked+hovered)
pub const SWITCH_BORDER_CHECKED_HOVER: ThemeToken =
    ThemeToken::new_static("plume.switch.border.checked.hover");
/// Switch border around the switch (checked+pressed)
pub const SWITCH_BORDER_CHECKED_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.switch.border.checked.pressed");
/// Switch border around the switch (checked+disabled)
pub const SWITCH_BORDER_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.switch.border.checked.disabled");
/// Switch slide background
pub const SWITCH_SLIDE_BG: ThemeToken = ThemeToken::new_static("plume.switch.slide.bg");
/// Switch slide background (hovered)
pub const SWITCH_SLIDE_BG_HOVER: ThemeToken = ThemeToken::new_static("plume.switch.slide.bg.hover");
/// Switch slide background (pressed)
pub const SWITCH_SLIDE_BG_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.switch.slide.bg.pressed");
/// Switch slide background (disabled)
pub const SWITCH_SLIDE_BG_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.switch.slide.bg.disabled");
/// Switch slide background (checked)
pub const SWITCH_SLIDE_BG_CHECKED: ThemeToken =
    ThemeToken::new_static("plume.switch.slide.bg.checked");
/// Switch slide background (checked+hovered)
pub const SWITCH_SLIDE_BG_CHECKED_HOVER: ThemeToken =
    ThemeToken::new_static("plume.switch.slide.bg.checked.hover");
/// Switch slide background (checked+pressed)
pub const SWITCH_SLIDE_BG_CHECKED_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.switch.slide.bg.checked.pressed");
/// Switch slide background (checked+disabled)
pub const SWITCH_SLIDE_BG_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.switch.slide.bg.checked.disabled");

/// Switch slide border
pub const SWITCH_SLIDE_BORDER: ThemeToken = ThemeToken::new_static("plume.switch.slide.border");
/// Switch slide border (hovered)
pub const SWITCH_SLIDE_BORDER_HOVER: ThemeToken =
    ThemeToken::new_static("plume.switch.slide.border.hover");
/// Switch slide border (pressed)
pub const SWITCH_SLIDE_BORDER_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.switch.slide.border.pressed");
/// Switch slide border (disabled)
pub const SWITCH_SLIDE_BORDER_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.switch.slide.border.disabled");
/// Switch slide border (checked)
pub const SWITCH_SLIDE_BORDER_CHECKED: ThemeToken =
    ThemeToken::new_static("plume.switch.slide.border.checked");
/// Switch slide border (checked+hovered)
pub const SWITCH_SLIDE_BORDER_CHECKED_HOVER: ThemeToken =
    ThemeToken::new_static("plume.switch.slide.border.checked.hover");
/// Switch slide border (checked+pressed)
pub const SWITCH_SLIDE_BORDER_CHECKED_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.switch.slide.border.checked.pressed");
/// Switch slide border (checked+disabled)
pub const SWITCH_SLIDE_BORDER_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.switch.slide.border.checked.disabled");

// Menus

/// Menu background
pub const MENU_BG: ThemeToken = ThemeToken::new_static("plume.menu.bg");
/// Menu border
pub const MENU_BORDER: ThemeToken = ThemeToken::new_static("plume.menu.border");

// Text Input

/// Background for text input
pub const TEXT_INPUT_BG: ThemeToken = ThemeToken::new_static("plume.textinput.bg");
/// Text color for text input
pub const TEXT_INPUT_TEXT: ThemeToken = ThemeToken::new_static("plume.textinput.text");
/// Text color for text input (disabled)
pub const TEXT_INPUT_TEXT_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.textinput.text.disabled");
/// Cursor color for text input
pub const TEXT_INPUT_CURSOR: ThemeToken = ThemeToken::new_static("plume.textinput.cursor");
/// Selection color for text input
pub const TEXT_INPUT_SELECTION: ThemeToken = ThemeToken::new_static("plume.textinput.selection");
/// Selection color for unfocused text input
pub const TEXT_INPUT_SELECTION_UNFOCUSED: ThemeToken =
    ThemeToken::new_static("plume.textinput.selection.unfocused");
/// Border for text input
pub const TEXT_INPUT_BORDER: ThemeToken = ThemeToken::new_static("plume.textinput.border");

// Subpane

/// Subpane background
pub const SUBPANE_HEADER_BG: ThemeToken = ThemeToken::new_static("plume.subpane.header.bg");
/// Subpane header border
pub const SUBPANE_HEADER_BORDER: ThemeToken = ThemeToken::new_static("plume.subpane.header.border");
/// Subpane header text color
pub const SUBPANE_HEADER_TEXT: ThemeToken = ThemeToken::new_static("plume.subpane.header.text");
/// Subpane body background
pub const SUBPANE_BODY_BG: ThemeToken = ThemeToken::new_static("plume.subpane.body.bg");
/// Subpane body border
pub const SUBPANE_BODY_BORDER: ThemeToken = ThemeToken::new_static("plume.subpane.body.border");

// Group

/// Group background
pub const GROUP_BG: ThemeToken = ThemeToken::new_static("plume.group.bg");
/// Group border
pub const GROUP_BORDER: ThemeToken = ThemeToken::new_static("plume.group.border");

// Listview

/// Listview row background
pub const LISTROW_BG: ThemeToken = ThemeToken::new_static("plume.listrow.bg");
/// Listview row background (hovered)
pub const LISTROW_BG_HOVER: ThemeToken = ThemeToken::new_static("plume.listrow.bg.hover");
/// Listview row background (selected)
pub const LISTROW_BG_SELECTED: ThemeToken = ThemeToken::new_static("plume.listrow.bg.selected");
/// Listview row text
pub const LISTROW_TEXT: ThemeToken = ThemeToken::new_static("plume.listrow.text");
/// Listview row text (disabled)
pub const LISTROW_TEXT_DISABLED: ThemeToken = ThemeToken::new_static("plume.listrow.text.disabled");

// Modal Dialog

/// Dialog background
pub const DIALOG_BG: ThemeToken = ThemeToken::new_static("plume.dialog.bg");
/// Dialog border
pub const DIALOG_BORDER: ThemeToken = ThemeToken::new_static("plume.dialog.border");
/// Dialog header background
pub const DIALOG_HEADER_BG: ThemeToken = ThemeToken::new_static("plume.dialog.header.bg");
/// Dialog text
pub const DIALOG_TEXT: ThemeToken = ThemeToken::new_static("plume.dialog.text");
/// Dialog header text
pub const DIALOG_HEADER_TEXT: ThemeToken = ThemeToken::new_static("plume.dialog.header.text");
