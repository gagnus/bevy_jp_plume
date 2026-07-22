//! Design tokens used by Plume themes.
//!
//! The term "design token" is commonly used in UX design to mean the smallest unit of a theme,
//! similar in concept to a CSS variable. Each token names one visual aspect of a widget
//! (background, border, ...) and maps to a [`ThemeSlot`](crate::theme::ThemeSlot) for its color.

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

// Outline buttons (transparent background, visible border)

/// Outline button background
pub const BUTTON_OUTLINE_BG: ThemeToken = ThemeToken::new_static("plume.button.outline.bg");
/// Outline button background (hovered)
pub const BUTTON_OUTLINE_BG_HOVER: ThemeToken =
    ThemeToken::new_static("plume.button.outline.bg.hover");
/// Outline button background (disabled)
pub const BUTTON_OUTLINE_BG_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.button.outline.bg.disabled");
/// Outline button background (pressed)
pub const BUTTON_OUTLINE_BG_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.button.outline.bg.pressed");
/// Outline button border
pub const BUTTON_OUTLINE_BORDER: ThemeToken = ThemeToken::new_static("plume.button.outline.border");
/// Outline button border (hovered)
pub const BUTTON_OUTLINE_BORDER_HOVER: ThemeToken =
    ThemeToken::new_static("plume.button.outline.border.hover");
/// Outline button border (disabled)
pub const BUTTON_OUTLINE_BORDER_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.button.outline.border.disabled");
/// Outline button border (pressed)
pub const BUTTON_OUTLINE_BORDER_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.button.outline.border.pressed");
/// Border color for the button variants that paint no outline
pub const BUTTON_BORDER_NONE: ThemeToken = ThemeToken::new_static("plume.button.border.none");

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
pub const SLIDER_THUMB: ThemeToken = ThemeToken::new_static("plume.slider.thumb.border");
/// Slider thumb (hover)
pub const SLIDER_THUMB_HOVER: ThemeToken =
    ThemeToken::new_static("plume.slider.thumb.border.hover");
/// Slider thumb (pressed)
pub const SLIDER_THUMB_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.slider.thumb.border.pressed");
/// Slider thumb (disabled)
pub const SLIDER_THUMB_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.slider.thumb.border.disabled");

// Scrollbar

/// Background for scrollbar
pub const SCROLLBAR_BG: ThemeToken = ThemeToken::new_static("plume.scrollbar.bg");
/// Background for scrollbar moving bar
pub const SCROLLBAR_THUMB: ThemeToken = ThemeToken::new_static("plume.scrollbar.thumb");
/// Background for scrollbar moving bar (hovered)
pub const SCROLLBAR_THUMB_HOVER: ThemeToken = ThemeToken::new_static("plume.scrollbar.thumb.hover");
/// Background for scrollbar moving bar (pressed)
pub const SCROLLBAR_THUMB_PRESSED: ThemeToken =
    ThemeToken::new_static("plume.scrollbar.thumb.pressed");

// Checkbox

/// Checkbox background around the checkmark
pub const CHECKBOX_BG: ThemeToken = ThemeToken::new_static("plume.checkbox.bg");
/// Checkbox border around the checkmark (disabled)
pub const CHECKBOX_BG_DISABLED: ThemeToken = ThemeToken::new_static("plume.checkbox.bg.disabled");
/// Checkbox background around the checkmark (checked)
pub const CHECKBOX_BG_CHECKED: ThemeToken = ThemeToken::new_static("plume.checkbox.bg.checked");
/// Checkbox border around the checkmark (checked+disabled)
pub const CHECKBOX_BG_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.checkbox.bg.checked.disabled");
/// Checkbox border around the checkmark
pub const CHECKBOX_BORDER: ThemeToken = ThemeToken::new_static("plume.checkbox.border");
/// Checkbox border around the checkmark (disabled)
pub const CHECKBOX_BORDER_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.checkbox.border.disabled");
/// Checkbox border around the checkmark (checked)
pub const CHECKBOX_BORDER_CHECKED: ThemeToken =
    ThemeToken::new_static("plume.checkbox.border.checked");
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
/// Background of the radio button (disabled)
pub const RADIO_BG_DISABLED: ThemeToken = ThemeToken::new_static("plume.radio.bg.disabled");
/// Background of the radio button (checked)
pub const RADIO_BG_CHECKED: ThemeToken = ThemeToken::new_static("plume.radio.bg.checked");
/// Background of the radio button (checked+disabled)
pub const RADIO_BG_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.radio.bg.checked.disabled");
/// Border around the radio button
pub const RADIO_BORDER: ThemeToken = ThemeToken::new_static("plume.radio.border");
/// Border around the radio button (disabled)
pub const RADIO_BORDER_DISABLED: ThemeToken = ThemeToken::new_static("plume.radio.border.disabled");
/// Border around the radio button (checked)
pub const RADIO_BORDER_CHECKED: ThemeToken = ThemeToken::new_static("plume.radio.border.checked");
/// Border around the radio button (checked+disabled)
pub const RADIO_BORDER_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.radio.border.checked.disabled");
/// Radio check mark
pub const RADIO_MARK: ThemeToken = ThemeToken::new_static("plume.radio.mark");
/// Radio check mark (disabled)
pub const RADIO_MARK_DISABLED: ThemeToken = ThemeToken::new_static("plume.radio.mark.disabled");
/// Radio label text
pub const RADIO_TEXT: ThemeToken = ThemeToken::new_static("plume.radio.text");
/// Radio label text (disabled)
pub const RADIO_TEXT_DISABLED: ThemeToken = ThemeToken::new_static("plume.radio.text.disabled");

// Toggle Switch

/// Switch background around the switch
pub const SWITCH_BG: ThemeToken = ThemeToken::new_static("plume.switch.bg");
/// Switch background around the switch (disabled)
pub const SWITCH_BG_DISABLED: ThemeToken = ThemeToken::new_static("plume.switch.bg.disabled");
/// Switch background around the switch (checked)
pub const SWITCH_BG_CHECKED: ThemeToken = ThemeToken::new_static("plume.switch.bg.checked");
/// Switch background around the switch (checked+disabled)
pub const SWITCH_BG_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.switch.bg.checked.disabled");
/// Switch border around the switch
pub const SWITCH_BORDER: ThemeToken = ThemeToken::new_static("plume.switch.border");
/// Switch border around the switch (disabled)
pub const SWITCH_BORDER_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.switch.border.disabled");
/// Switch border around the switch (checked)
pub const SWITCH_BORDER_CHECKED: ThemeToken = ThemeToken::new_static("plume.switch.border.checked");
/// Switch border around the switch (checked+disabled)
pub const SWITCH_BORDER_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.switch.border.checked.disabled");
/// Switch slide background
pub const SWITCH_SLIDE_BG: ThemeToken = ThemeToken::new_static("plume.switch.slide.bg");
/// Switch slide background (disabled)
pub const SWITCH_SLIDE_BG_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.switch.slide.bg.disabled");
/// Switch slide background (checked)
pub const SWITCH_SLIDE_BG_CHECKED: ThemeToken =
    ThemeToken::new_static("plume.switch.slide.bg.checked");
/// Switch slide background (checked+disabled)
pub const SWITCH_SLIDE_BG_CHECKED_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.switch.slide.bg.checked.disabled");

// Menus

/// Menu background
pub const MENU_BG: ThemeToken = ThemeToken::new_static("plume.menu.bg");
/// Menu border
pub const MENU_BORDER: ThemeToken = ThemeToken::new_static("plume.menu.border");

// Text Input

/// Background for text input
pub const TEXT_INPUT_BG: ThemeToken = ThemeToken::new_static("plume.textinput.bg");
/// Background for text input
pub const TEXT_INPUT_BG_ACTIVE: ThemeToken = ThemeToken::new_static("plume.textinput.bg.active");
/// Background for text input (disabled)
pub const TEXT_INPUT_BG_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.textinput.bg.disabled");
/// Text color for text input
pub const TEXT_INPUT_TEXT: ThemeToken = ThemeToken::new_static("plume.textinput.text");
/// Text color for text input when editing
pub const TEXT_INPUT_TEXT_ACTIVE: ThemeToken =
    ThemeToken::new_static("plume.textinput.text.active");
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
/// Border for text input while focused (being edited)
pub const TEXT_INPUT_BORDER_ACTIVE: ThemeToken =
    ThemeToken::new_static("plume.textinput.border.active");
/// Border for text input when disabled
pub const TEXT_INPUT_BORDER_DISABLED: ThemeToken =
    ThemeToken::new_static("plume.textinput.border.disabled");

// Color swatch

/// Border around a color swatch
pub const COLOR_SWATCH_BORDER: ThemeToken = ThemeToken::new_static("plume.swatch.border");

// Section

/// Section background
pub const SECTION_HEADER_BG: ThemeToken = ThemeToken::new_static("plume.section.header.bg");
/// Section header text color
pub const SECTION_HEADER_TEXT: ThemeToken = ThemeToken::new_static("plume.section.header.text");
/// Muted header text color, for the flat header a non-collapsible section wears
pub const SECTION_HEADER_MUTED_TEXT: ThemeToken =
    ThemeToken::new_static("plume.section.header.muted_text");
/// Section body background
pub const SECTION_BODY_BG: ThemeToken = ThemeToken::new_static("plume.section.body.bg");

// Group

/// Group background
pub const GROUP_BG: ThemeToken = ThemeToken::new_static("plume.group.bg");

// Listview

/// Listview row background
pub const LISTROW_BG: ThemeToken = ThemeToken::new_static("plume.listrow.bg");
/// Listview row background (hovered)
pub const LISTROW_BG_HOVER: ThemeToken = ThemeToken::new_static("plume.listrow.bg.hover");
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

// Separator

/// Separator hairline
pub const SEPARATOR: ThemeToken = ThemeToken::new_static("plume.separator");

/// Focus ring outline
pub const FOCUS_RING: ThemeToken = ThemeToken::new_static("plume.focus-ring");

/// State groups over the constants above, for
/// [`InteractionTokens::pick`](crate::theme::InteractionTokens::pick) and
/// [`CheckedTokens::pick`](crate::theme::CheckedTokens::pick).
pub mod sets {
    use crate::theme::{CheckedTokens, InteractionTokens};

    /// Regular button background
    pub const BUTTON_BG: InteractionTokens = InteractionTokens {
        base: super::BUTTON_BG,
        hover: super::BUTTON_BG_HOVER,
        pressed: super::BUTTON_BG_PRESSED,
        disabled: super::BUTTON_BG_DISABLED,
    };
    /// Primary button background
    pub const BUTTON_PRIMARY_BG: InteractionTokens = InteractionTokens {
        base: super::BUTTON_PRIMARY_BG,
        hover: super::BUTTON_PRIMARY_BG_HOVER,
        pressed: super::BUTTON_PRIMARY_BG_PRESSED,
        disabled: super::BUTTON_PRIMARY_BG_DISABLED,
    };
    /// Plain button background
    pub const BUTTON_PLAIN_BG: InteractionTokens = InteractionTokens {
        base: super::BUTTON_PLAIN_BG,
        hover: super::BUTTON_PLAIN_BG_HOVER,
        pressed: super::BUTTON_PLAIN_BG_PRESSED,
        disabled: super::BUTTON_PLAIN_BG_DISABLED,
    };
    /// Outline button background
    pub const BUTTON_OUTLINE_BG: InteractionTokens = InteractionTokens {
        base: super::BUTTON_OUTLINE_BG,
        hover: super::BUTTON_OUTLINE_BG_HOVER,
        pressed: super::BUTTON_OUTLINE_BG_PRESSED,
        disabled: super::BUTTON_OUTLINE_BG_DISABLED,
    };
    /// Outline button border
    pub const BUTTON_OUTLINE_BORDER: InteractionTokens = InteractionTokens {
        base: super::BUTTON_OUTLINE_BORDER,
        hover: super::BUTTON_OUTLINE_BORDER_HOVER,
        pressed: super::BUTTON_OUTLINE_BORDER_PRESSED,
        disabled: super::BUTTON_OUTLINE_BORDER_DISABLED,
    };
    /// Slider track background
    pub const SLIDER_BG: InteractionTokens = InteractionTokens {
        base: super::SLIDER_BG,
        hover: super::SLIDER_BG_HOVER,
        pressed: super::SLIDER_BG_PRESSED,
        disabled: super::SLIDER_BG_DISABLED,
    };
    /// Slider fill bar
    pub const SLIDER_BAR: InteractionTokens = InteractionTokens {
        base: super::SLIDER_BAR,
        hover: super::SLIDER_BAR_HOVER,
        pressed: super::SLIDER_BAR_PRESSED,
        disabled: super::SLIDER_BAR_DISABLED,
    };
    /// Slider thumb
    pub const SLIDER_THUMB: InteractionTokens = InteractionTokens {
        base: super::SLIDER_THUMB,
        hover: super::SLIDER_THUMB_HOVER,
        pressed: super::SLIDER_THUMB_PRESSED,
        disabled: super::SLIDER_THUMB_DISABLED,
    };
    /// Scrollbar thumb (never disabled; `disabled` falls back to `base`).
    pub const SCROLLBAR_THUMB: InteractionTokens = InteractionTokens {
        base: super::SCROLLBAR_THUMB,
        hover: super::SCROLLBAR_THUMB_HOVER,
        pressed: super::SCROLLBAR_THUMB_PRESSED,
        disabled: super::SCROLLBAR_THUMB,
    };

    /// Checkbox background fill
    pub const CHECKBOX_BG: CheckedTokens = CheckedTokens {
        base: super::CHECKBOX_BG,
        checked: super::CHECKBOX_BG_CHECKED,
        disabled: super::CHECKBOX_BG_DISABLED,
        checked_disabled: super::CHECKBOX_BG_CHECKED_DISABLED,
    };
    /// Checkbox border
    pub const CHECKBOX_BORDER: CheckedTokens = CheckedTokens {
        base: super::CHECKBOX_BORDER,
        checked: super::CHECKBOX_BORDER_CHECKED,
        disabled: super::CHECKBOX_BORDER_DISABLED,
        checked_disabled: super::CHECKBOX_BORDER_CHECKED_DISABLED,
    };
    /// Radio disc background
    pub const RADIO_BG: CheckedTokens = CheckedTokens {
        base: super::RADIO_BG,
        checked: super::RADIO_BG_CHECKED,
        disabled: super::RADIO_BG_DISABLED,
        checked_disabled: super::RADIO_BG_CHECKED_DISABLED,
    };
    /// Radio border ring
    pub const RADIO_BORDER: CheckedTokens = CheckedTokens {
        base: super::RADIO_BORDER,
        checked: super::RADIO_BORDER_CHECKED,
        disabled: super::RADIO_BORDER_DISABLED,
        checked_disabled: super::RADIO_BORDER_CHECKED_DISABLED,
    };
    /// Toggle switch pill background
    pub const SWITCH_BG: CheckedTokens = CheckedTokens {
        base: super::SWITCH_BG,
        checked: super::SWITCH_BG_CHECKED,
        disabled: super::SWITCH_BG_DISABLED,
        checked_disabled: super::SWITCH_BG_CHECKED_DISABLED,
    };
    /// Toggle switch border ring
    pub const SWITCH_BORDER: CheckedTokens = CheckedTokens {
        base: super::SWITCH_BORDER,
        checked: super::SWITCH_BORDER_CHECKED,
        disabled: super::SWITCH_BORDER_DISABLED,
        checked_disabled: super::SWITCH_BORDER_CHECKED_DISABLED,
    };
    /// Toggle switch knob background
    pub const SWITCH_SLIDE_BG: CheckedTokens = CheckedTokens {
        base: super::SWITCH_SLIDE_BG,
        checked: super::SWITCH_SLIDE_BG_CHECKED,
        disabled: super::SWITCH_SLIDE_BG_DISABLED,
        checked_disabled: super::SWITCH_SLIDE_BG_CHECKED_DISABLED,
    };
}
