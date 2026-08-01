//! ThemeSlot provides contextual names for the palette entries

use bevy::{ecs::component::Component, reflect::Reflect};

use crate::tokens::{self, ThemeToken};

/// A single semantic color role, these are all the colors that make up
/// a theme.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Reflect)]
pub enum ThemeSlot {
    /// Deepest background: the window, the scrollbar track, and the active text input.
    /// - `SCROLLBAR_BG`
    /// - `TABS_STRIP_BG`
    /// - `TEXT_INPUT_BG_ACTIVE`
    /// - `WINDOW_BG`
    #[default]
    Neutral0,

    /// Surface bodies sitting on the window.
    /// - `DIALOG_BG`
    /// - `DIALOG_HEADER_BG`
    /// - `MENU_BG`
    /// - `SECTION_BODY_BG`
    /// - `TABS_BODY_BG`
    /// - `TEXT_INPUT_BG`
    /// - `TOOLTIP_BG`
    Neutral1,

    /// Raised container chrome: section headers.
    /// - `SECTION_HEADER_BG`
    /// - `TAB_BG_HOVER`
    Neutral2,

    /// Disabled control chrome.
    /// - `BUTTON_BG_DISABLED`
    /// - `BUTTON_OUTLINE_BORDER_DISABLED`
    /// - `BUTTON_PRIMARY_BG_DISABLED`
    /// - `CHECKBOX_BORDER_CHECKED_DISABLED`
    /// - `CHECKBOX_BORDER_DISABLED`
    /// - `CHECKBOX_MARK_DISABLED`
    /// - `RADIO_BORDER_CHECKED_DISABLED`
    /// - `RADIO_BORDER_DISABLED`
    /// - `RADIO_MARK_DISABLED`
    /// - `SLIDER_BG_DISABLED`
    /// - `SLIDER_BAR_DISABLED`
    /// - `SLIDER_THUMB_DISABLED`
    /// - `SWITCH_BORDER_CHECKED_DISABLED`
    /// - `SWITCH_BORDER_DISABLED`
    /// - `SWITCH_SLIDE_BG_CHECKED_DISABLED`
    /// - `SWITCH_SLIDE_BG_DISABLED`
    /// - `TEXT_INPUT_BG_DISABLED`
    /// - `TEXT_INPUT_BORDER`
    /// - `TEXT_INPUT_BORDER_DISABLED`
    Neutral3,

    /// Control rest backgrounds and borders.
    /// - `BUTTON_BG`
    /// - `BUTTON_OUTLINE_BORDER`
    /// - `CHECKBOX_BORDER`
    /// - `DIALOG_BORDER`
    /// - `MENU_BORDER`
    /// - `RADIO_BORDER`
    /// - `SCROLLBAR_THUMB`
    /// - `SEPARATOR`
    /// - `SLIDER_BG`
    /// - `SLIDER_BG_HOVER`
    /// - `SLIDER_BG_PRESSED`
    /// - `SWITCH_BG`
    Neutral4,

    /// Neutral control hover.
    /// - `BUTTON_BG_HOVER`
    /// - `BUTTON_OUTLINE_BG_HOVER`
    /// - `BUTTON_OUTLINE_BORDER_HOVER`
    /// - `BUTTON_PLAIN_BG_HOVER`
    /// - `OPTION_BG_HOVER`
    /// - `SCROLLBAR_THUMB_HOVER`
    Neutral5,

    /// Neutral control pressed.
    /// - `BUTTON_BG_PRESSED`
    /// - `BUTTON_OUTLINE_BG_PRESSED`
    /// - `BUTTON_OUTLINE_BORDER_PRESSED`
    /// - `BUTTON_PLAIN_BG_PRESSED`
    /// - `SCROLLBAR_THUMB_PRESSED`
    Neutral6,

    /// Bright on-surface text and the unchecked switch knob.
    /// - `BUTTON_TEXT`
    /// - `DIALOG_HEADER_TEXT`
    /// - `OPTION_TEXT`
    /// - `SECTION_HEADER_TEXT`
    /// - `SWITCH_SLIDE_BG`
    /// - `TAB_TEXT_SELECTED`
    /// - `TEXT_INPUT_TEXT_ACTIVE`
    /// - `TEXT_MAIN`
    Text0,

    /// Body text.
    /// - `CHECKBOX_TEXT`
    /// - `DIALOG_TEXT`
    /// - `RADIO_TEXT`
    /// - `SECTION_HEADER_MUTED_TEXT`
    /// - `TAB_TEXT`
    /// - `TEXT_DIM`
    /// - `TEXT_INPUT_TEXT`
    /// - `TOOLTIP_TEXT`
    Text1,

    /// Disabled bright text.
    /// - `BUTTON_PRIMARY_TEXT_DISABLED`
    /// - `BUTTON_TEXT_DISABLED`
    /// - `OPTION_TEXT_DISABLED`
    TextDisabled0,

    /// Disabled body text.
    /// - `CHECKBOX_TEXT_DISABLED`
    /// - `RADIO_TEXT_DISABLED`
    /// - `TAB_TEXT_DISABLED`
    /// - `TEXT_INPUT_TEXT_DISABLED`
    TextDisabled1,

    /// Base call-to-action and checked-state color.
    /// - `BUTTON_PRIMARY_BG`
    /// - `CHECKBOX_BG_CHECKED`
    /// - `RADIO_BG_CHECKED`
    /// - `SLIDER_BAR`
    /// - `SLIDER_THUMB`
    /// - `SWITCH_BG_CHECKED`
    /// - `TAB_INDICATOR`
    /// - `TEXT_INPUT_BORDER_ACTIVE`
    /// - `TEXT_INPUT_SELECTION`
    Accent0,

    /// Call-to-action hover.
    /// - `BUTTON_PRIMARY_BG_HOVER`
    /// - `SLIDER_BAR_HOVER`
    /// - `SLIDER_THUMB_HOVER`
    Accent1,

    /// Call-to-action pressed.
    /// - `BUTTON_PRIMARY_BG_PRESSED`
    /// - `SLIDER_BAR_PRESSED`
    /// - `SLIDER_THUMB_PRESSED`
    Accent2,

    /// Brightest accent.
    /// - `TEXT_INPUT_CURSOR`
    Accent3,

    /// Foreground over accent-filled components.
    /// - `BUTTON_PRIMARY_TEXT`
    /// - `CHECKBOX_MARK`
    /// - `RADIO_MARK`
    /// - `SWITCH_SLIDE_BG_CHECKED`
    Contrast,

    /// Focus/selection ring color (derived: accent 0 at half alpha).
    /// - `FOCUS_RING`
    FocusRing,

    /// Red axis (reserved for axis-colored widgets).
    XAxis,

    /// Green axis (reserved for axis-colored widgets).
    YAxis,

    /// Blue axis (reserved for axis-colored widgets).
    ZAxis,

    /// Always [`Color::NONE`](bevy::color::Color::NONE); for tokens that paint nothing.
    /// - `BUTTON_BORDER_NONE`
    /// - `BUTTON_OUTLINE_BG`
    /// - `BUTTON_OUTLINE_BG_DISABLED`
    /// - `BUTTON_PLAIN_BG`
    /// - `BUTTON_PLAIN_BG_DISABLED`
    /// - `CHECKBOX_BG`
    /// - `CHECKBOX_BG_CHECKED_DISABLED`
    /// - `CHECKBOX_BG_DISABLED`
    /// - `OPTION_BG`
    /// - `RADIO_BG`
    /// - `RADIO_BG_CHECKED_DISABLED`
    /// - `RADIO_BG_DISABLED`
    /// - `SWITCH_BG_CHECKED_DISABLED`
    /// - `SWITCH_BG_DISABLED`
    /// - `TAB_BG`
    /// - `TEXT_INPUT_SELECTION_UNFOCUSED`
    /// - `CHECKBOX_BORDER_CHECKED`
    /// - `RADIO_BORDER_CHECKED`
    /// - `SWITCH_BORDER_CHECKED`
    /// - `SWITCH_BORDER`
    Transparent,
}

impl ThemeSlot {
    /// Every slot, in discriminant order (matches the resolved palette's storage).
    pub const ALL: [ThemeSlot; 21] = [
        ThemeSlot::Neutral0,
        ThemeSlot::Neutral1,
        ThemeSlot::Neutral2,
        ThemeSlot::Neutral3,
        ThemeSlot::Neutral4,
        ThemeSlot::Neutral5,
        ThemeSlot::Neutral6,
        ThemeSlot::Text0,
        ThemeSlot::Text1,
        ThemeSlot::TextDisabled0,
        ThemeSlot::TextDisabled1,
        ThemeSlot::Accent0,
        ThemeSlot::Accent1,
        ThemeSlot::Accent2,
        ThemeSlot::Accent3,
        ThemeSlot::Contrast,
        ThemeSlot::FocusRing,
        ThemeSlot::XAxis,
        ThemeSlot::YAxis,
        ThemeSlot::ZAxis,
        ThemeSlot::Transparent,
    ];

    /// Number of slots — the backing size of the resolved palette.
    pub const COUNT: usize = Self::ALL.len();

    /// Human-readable name, for editor UI / pickers.
    pub fn label(self) -> &'static str {
        match self {
            ThemeSlot::Neutral0 => "Neutral 0",
            ThemeSlot::Neutral1 => "Neutral 1",
            ThemeSlot::Neutral2 => "Neutral 2",
            ThemeSlot::Neutral3 => "Neutral 3",
            ThemeSlot::Neutral4 => "Neutral 4",
            ThemeSlot::Neutral5 => "Neutral 5",
            ThemeSlot::Neutral6 => "Neutral 6",
            ThemeSlot::Text0 => "Text 0",
            ThemeSlot::Text1 => "Text 1",
            ThemeSlot::TextDisabled0 => "Text Disabled 0",
            ThemeSlot::TextDisabled1 => "Text Disabled 1",
            ThemeSlot::Accent0 => "Accent 0",
            ThemeSlot::Accent1 => "Accent 1",
            ThemeSlot::Accent2 => "Accent 2",
            ThemeSlot::Accent3 => "Accent 3",
            ThemeSlot::Contrast => "Contrast",
            ThemeSlot::FocusRing => "Focus Ring",
            ThemeSlot::XAxis => "X Axis",
            ThemeSlot::YAxis => "Y Axis",
            ThemeSlot::ZAxis => "Z Axis",
            ThemeSlot::Transparent => "Transparent",
        }
    }
}

pub(crate) static DEFAULT_TOKEN_SLOTS: &[(ThemeToken, ThemeSlot)] = &[
    (tokens::WINDOW_BG, ThemeSlot::Neutral0),
    (tokens::TEXT_MAIN, ThemeSlot::Text0),
    (tokens::TEXT_DIM, ThemeSlot::Text1),
    (tokens::BUTTON_BG, ThemeSlot::Neutral4),
    (tokens::BUTTON_BG_HOVER, ThemeSlot::Neutral5),
    (tokens::BUTTON_BG_PRESSED, ThemeSlot::Neutral6),
    (tokens::BUTTON_BG_DISABLED, ThemeSlot::Neutral3),
    (tokens::BUTTON_PRIMARY_BG, ThemeSlot::Accent0),
    (tokens::BUTTON_PRIMARY_BG_HOVER, ThemeSlot::Accent1),
    (tokens::BUTTON_PRIMARY_BG_PRESSED, ThemeSlot::Accent2),
    (tokens::BUTTON_PRIMARY_BG_DISABLED, ThemeSlot::Neutral3),
    (tokens::BUTTON_PLAIN_BG, ThemeSlot::Transparent),
    (tokens::BUTTON_PLAIN_BG_HOVER, ThemeSlot::Neutral5),
    (tokens::BUTTON_PLAIN_BG_PRESSED, ThemeSlot::Neutral6),
    (tokens::BUTTON_PLAIN_BG_DISABLED, ThemeSlot::Transparent),
    (tokens::BUTTON_OUTLINE_BG, ThemeSlot::Transparent),
    (tokens::BUTTON_OUTLINE_BG_HOVER, ThemeSlot::Neutral5),
    (tokens::BUTTON_OUTLINE_BG_PRESSED, ThemeSlot::Neutral6),
    (tokens::BUTTON_OUTLINE_BG_DISABLED, ThemeSlot::Transparent),
    (tokens::BUTTON_OUTLINE_BORDER, ThemeSlot::Neutral4),
    (tokens::BUTTON_OUTLINE_BORDER_HOVER, ThemeSlot::Neutral5),
    (tokens::BUTTON_OUTLINE_BORDER_PRESSED, ThemeSlot::Neutral6),
    (tokens::BUTTON_OUTLINE_BORDER_DISABLED, ThemeSlot::Neutral3),
    (tokens::BUTTON_BORDER_NONE, ThemeSlot::Transparent),
    (tokens::BUTTON_TEXT, ThemeSlot::Text0),
    (tokens::BUTTON_TEXT_DISABLED, ThemeSlot::TextDisabled0),
    (tokens::BUTTON_PRIMARY_TEXT, ThemeSlot::Contrast),
    (
        tokens::BUTTON_PRIMARY_TEXT_DISABLED,
        ThemeSlot::TextDisabled0,
    ),
    (tokens::SLIDER_BG, ThemeSlot::Neutral4),
    (tokens::SLIDER_BG_HOVER, ThemeSlot::Neutral4),
    (tokens::SLIDER_BG_PRESSED, ThemeSlot::Neutral4),
    (tokens::SLIDER_BG_DISABLED, ThemeSlot::Neutral3),
    (tokens::SLIDER_BAR, ThemeSlot::Accent0),
    (tokens::SLIDER_BAR_HOVER, ThemeSlot::Accent1),
    (tokens::SLIDER_BAR_PRESSED, ThemeSlot::Accent2),
    (tokens::SLIDER_BAR_DISABLED, ThemeSlot::Neutral3),
    (tokens::SLIDER_THUMB, ThemeSlot::Accent0),
    (tokens::SLIDER_THUMB_HOVER, ThemeSlot::Accent1),
    (tokens::SLIDER_THUMB_PRESSED, ThemeSlot::Accent2),
    (tokens::SLIDER_THUMB_DISABLED, ThemeSlot::Neutral3),
    (tokens::SCROLLBAR_BG, ThemeSlot::Neutral0),
    (tokens::SCROLLBAR_THUMB, ThemeSlot::Neutral4),
    (tokens::SCROLLBAR_THUMB_HOVER, ThemeSlot::Neutral5),
    (tokens::SCROLLBAR_THUMB_PRESSED, ThemeSlot::Neutral6),
    (tokens::CHECKBOX_BG, ThemeSlot::Transparent),
    (tokens::CHECKBOX_BG_DISABLED, ThemeSlot::Transparent),
    (tokens::CHECKBOX_BG_CHECKED, ThemeSlot::Accent0),
    (tokens::CHECKBOX_BG_CHECKED_DISABLED, ThemeSlot::Transparent),
    (tokens::CHECKBOX_BORDER, ThemeSlot::Neutral4),
    (tokens::CHECKBOX_BORDER_DISABLED, ThemeSlot::Neutral3),
    (tokens::CHECKBOX_BORDER_CHECKED, ThemeSlot::Transparent),
    (
        tokens::CHECKBOX_BORDER_CHECKED_DISABLED,
        ThemeSlot::Neutral3,
    ),
    (tokens::CHECKBOX_MARK, ThemeSlot::Contrast),
    (tokens::CHECKBOX_MARK_DISABLED, ThemeSlot::Neutral3),
    (tokens::CHECKBOX_TEXT, ThemeSlot::Text1),
    (tokens::CHECKBOX_TEXT_DISABLED, ThemeSlot::TextDisabled1),
    (tokens::RADIO_BG, ThemeSlot::Transparent),
    (tokens::RADIO_BG_DISABLED, ThemeSlot::Transparent),
    (tokens::RADIO_BG_CHECKED, ThemeSlot::Accent0),
    (tokens::RADIO_BG_CHECKED_DISABLED, ThemeSlot::Transparent),
    (tokens::RADIO_BORDER, ThemeSlot::Neutral4),
    (tokens::RADIO_BORDER_DISABLED, ThemeSlot::Neutral3),
    (tokens::RADIO_BORDER_CHECKED, ThemeSlot::Transparent),
    (tokens::RADIO_BORDER_CHECKED_DISABLED, ThemeSlot::Neutral3),
    (tokens::RADIO_MARK, ThemeSlot::Contrast),
    (tokens::RADIO_MARK_DISABLED, ThemeSlot::Neutral3),
    (tokens::RADIO_TEXT, ThemeSlot::Text1),
    (tokens::RADIO_TEXT_DISABLED, ThemeSlot::TextDisabled1),
    (tokens::SWITCH_BG, ThemeSlot::Neutral4),
    (tokens::SWITCH_BG_DISABLED, ThemeSlot::Transparent),
    (tokens::SWITCH_BG_CHECKED, ThemeSlot::Accent0),
    (tokens::SWITCH_BG_CHECKED_DISABLED, ThemeSlot::Transparent),
    (tokens::SWITCH_BORDER, ThemeSlot::Transparent),
    (tokens::SWITCH_BORDER_DISABLED, ThemeSlot::Neutral3),
    (tokens::SWITCH_BORDER_CHECKED, ThemeSlot::Transparent),
    (tokens::SWITCH_BORDER_CHECKED_DISABLED, ThemeSlot::Neutral3),
    (tokens::SWITCH_SLIDE_BG, ThemeSlot::Contrast),
    (tokens::SWITCH_SLIDE_BG_DISABLED, ThemeSlot::Neutral3),
    (tokens::SWITCH_SLIDE_BG_CHECKED, ThemeSlot::Contrast),
    (
        tokens::SWITCH_SLIDE_BG_CHECKED_DISABLED,
        ThemeSlot::Neutral3,
    ),
    (tokens::MENU_BG, ThemeSlot::Neutral1),
    (tokens::MENU_BORDER, ThemeSlot::Neutral4),
    (tokens::TOOLTIP_BG, ThemeSlot::Neutral1),
    (tokens::TOOLTIP_TEXT, ThemeSlot::Text1),
    (tokens::TEXT_INPUT_BG, ThemeSlot::Neutral0),
    (tokens::TEXT_INPUT_BG_ACTIVE, ThemeSlot::Neutral0),
    (tokens::TEXT_INPUT_BG_DISABLED, ThemeSlot::Neutral3),
    (tokens::TEXT_INPUT_TEXT, ThemeSlot::Text1),
    (tokens::TEXT_INPUT_TEXT_ACTIVE, ThemeSlot::Text0),
    (tokens::TEXT_INPUT_TEXT_DISABLED, ThemeSlot::TextDisabled1),
    (tokens::TEXT_INPUT_CURSOR, ThemeSlot::Accent3),
    (tokens::TEXT_INPUT_SELECTION, ThemeSlot::Accent0),
    (
        tokens::TEXT_INPUT_SELECTION_UNFOCUSED,
        ThemeSlot::Transparent,
    ),
    (tokens::TEXT_INPUT_BORDER, ThemeSlot::Neutral3),
    (tokens::TEXT_INPUT_BORDER_ACTIVE, ThemeSlot::Accent0),
    (tokens::TEXT_INPUT_BORDER_DISABLED, ThemeSlot::Neutral3),
    (tokens::COLOR_SWATCH_BORDER, ThemeSlot::Neutral4),
    (tokens::SECTION_HEADER_BG, ThemeSlot::Neutral2),
    (tokens::SECTION_HEADER_TEXT, ThemeSlot::Text0),
    (tokens::SECTION_HEADER_MUTED_TEXT, ThemeSlot::Text1),
    (tokens::SECTION_BODY_BG, ThemeSlot::Neutral1),
    (tokens::TABS_STRIP_BG, ThemeSlot::Neutral0),
    (tokens::TABS_BODY_BG, ThemeSlot::Neutral1),
    (tokens::TAB_BG, ThemeSlot::Transparent),
    (tokens::TAB_BG_SELECTED, ThemeSlot::Neutral1),
    (tokens::TAB_BG_HOVER, ThemeSlot::Neutral5),
    (tokens::TAB_TEXT, ThemeSlot::Text1),
    (tokens::TAB_TEXT_SELECTED, ThemeSlot::Text0),
    (tokens::TAB_TEXT_DISABLED, ThemeSlot::TextDisabled1),
    (tokens::TAB_INDICATOR, ThemeSlot::Accent0),
    (tokens::OPTION_BG, ThemeSlot::Transparent),
    (tokens::OPTION_BG_HOVER, ThemeSlot::Neutral5),
    (tokens::OPTION_TEXT, ThemeSlot::Text0),
    (tokens::OPTION_TEXT_DISABLED, ThemeSlot::TextDisabled0),
    (tokens::DIALOG_BG, ThemeSlot::Neutral1),
    (tokens::DIALOG_BORDER, ThemeSlot::Neutral4),
    (tokens::DIALOG_HEADER_BG, ThemeSlot::Neutral1),
    (tokens::DIALOG_TEXT, ThemeSlot::Text1),
    (tokens::DIALOG_HEADER_TEXT, ThemeSlot::Text0),
    (tokens::SEPARATOR, ThemeSlot::Neutral4),
    (tokens::FOCUS_RING, ThemeSlot::FocusRing),
];
