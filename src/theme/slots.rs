//! ThemeSlot provides contextual names for the palette entries.

use bevy::ecs::component::Component;
use bevy::reflect::Reflect;

use crate::tokens::{self, ThemeToken};

/// A single semantic color role, these are all the colors that make up
/// a theme.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Reflect)]
pub enum ThemeSlot {
    /// Window background.
    /// - `SELECT_BG`
    /// - `SCROLLBAR_BG`
    /// - `TEXT_INPUT_BG`
    /// - `TEXT_INPUT_BG_ACTIVE`
    /// - `WINDOW_BG`
    /// - `TAB_BAR_BG`
    #[default]
    Neutral0,

    /// Dialog background.
    /// - `MENU_BG`
    /// - `DIALOG_BG`
    /// - `DIALOG_HEADER_BG`
    /// - `POPUP_BG`
    /// - `SECTION_BODY_BG`
    /// - `TOOLTIP_BG`
    /// - `TAB_BODY_BG`
    /// - `TAB_BG_SELECTED`
    Neutral1,

    /// Raised header background.
    /// - `SECTION_HEADER_BG`
    Neutral2,

    /// Hover for controls with no background.
    /// - `SELECT_OPTION_BG_HOVER`
    /// - `BUTTON_OUTLINE_BG_HOVER`
    /// - `BUTTON_PLAIN_BG_HOVER`
    /// - `TAB_BG_HOVERED`
    Neutral3,

    /// Control body (+ borders + slider track + pressed for Neutral3 hovers).
    /// - `BUTTON_BG`
    /// - `BUTTON_OUTLINE_BORDER`
    /// - `CHECKBOX_BORDER`
    /// - `DIALOG_BORDER`
    /// - `POPUP_BORDER`
    /// - `RADIO_BORDER`
    /// - `MENU_BORDER`
    /// - `TEXT_INPUT_BORDER`
    /// - `TOOLTIP_BORDER`
    /// - `SELECT_BORDER`
    /// - `COLOR_SWATCH_BORDER`
    /// - `SCROLLBAR_THUMB`
    /// - `SEPARATOR`
    /// - `SWITCH_BG`
    /// - `SLIDER_BG`
    /// - `SLIDER_BG_HOVER`
    /// - `SLIDER_BG_PRESSED`
    /// - `BUTTON_OUTLINE_BG_PRESSED`
    /// - `BUTTON_PLAIN_BG_PRESSED`
    /// - `TAB_BG_PRESSED`
    Neutral4,

    /// Control hover
    /// - `BUTTON_BG_HOVER`
    /// - `BUTTON_OUTLINE_BORDER_HOVER`
    /// - `MENU_ITEM_BG_HOVER`
    /// - `MENU_BUTTON_BG_HOVER`
    Neutral5,

    /// Control pressed
    /// - `BUTTON_BG_PRESSED`
    /// - `BUTTON_OUTLINE_BORDER_PRESSED`
    Neutral6,

    /// Bright text.
    /// - `BUTTON_TEXT`
    /// - `DIALOG_HEADER_TEXT`
    /// - `MENU_BUTTON_TEXT`
    /// - `MENU_ITEM_TEXT`
    /// - `SELECT_OPTION_TEXT`
    /// - `SECTION_HEADER_TEXT`
    /// - `TAB_TEXT_SELECTED`
    /// - `TEXT_INPUT_TEXT`
    /// - `TEXT_INPUT_TEXT_ACTIVE`
    /// - `TEXT_MAIN`
    Text0,

    /// Body text.
    /// - `BUTTON_OUTLINE_TEXT_UNCHECKED`
    /// - `BUTTON_PLAIN_TEXT_UNCHECKED`
    /// - `CHECKBOX_TEXT`
    /// - `DIALOG_TEXT`
    /// - `RADIO_TEXT`
    /// - `SECTION_HEADER_MUTED_TEXT`
    /// - `TAB_TEXT`
    /// - `TEXT_DIM`
    /// - `TOOLTIP_TEXT`
    Text1,

    /// Disabled text over a [`Disabled1`](Self::Disabled1) control background.
    /// - `BUTTON_CHECKED_TEXT_DISABLED`
    /// - `BUTTON_DANGER_TEXT_DISABLED`
    /// - `BUTTON_PRIMARY_TEXT_DISABLED`
    /// - `BUTTON_TEXT_DISABLED`
    /// - `TEXT_INPUT_TEXT_DISABLED`
    Disabled0,

    /// Disabled control and disabled text sitting straight on background.
    /// - `BUTTON_BG_DISABLED`
    /// - `BUTTON_CHECKED_BG_DISABLED`
    /// - `BUTTON_DANGER_BG_DISABLED`
    /// - `BUTTON_OUTLINE_BORDER_DISABLED`
    /// - `BUTTON_PRIMARY_BG_DISABLED`
    /// - `CHECKBOX_BORDER_CHECKED_DISABLED`
    /// - `CHECKBOX_BORDER_DISABLED`
    /// - `CHECKBOX_MARK_DISABLED`
    /// - `CHECKBOX_TEXT_DISABLED`
    /// - `MENU_BUTTON_TEXT_DISABLED`
    /// - `MENU_ITEM_TEXT_DISABLED`
    /// - `RADIO_BORDER_CHECKED_DISABLED`
    /// - `RADIO_BORDER_DISABLED`
    /// - `RADIO_MARK_DISABLED`
    /// - `RADIO_TEXT_DISABLED`
    /// - `SELECT_OPTION_TEXT_DISABLED`
    /// - `SLIDER_BAR_DISABLED`
    /// - `SLIDER_BG_DISABLED`
    /// - `SLIDER_THUMB_DISABLED`
    /// - `SWITCH_BORDER_CHECKED_DISABLED`
    /// - `SWITCH_BORDER_DISABLED`
    /// - `SWITCH_SLIDE_BG_CHECKED_DISABLED`
    /// - `SWITCH_SLIDE_BG_DISABLED`
    /// - `TAB_INDICATOR_DISABLED`
    /// - `TAB_TEXT_DISABLED`
    /// - `TEXT_DISABLED`
    /// - `TEXT_INPUT_BG_DISABLED`
    /// - `TEXT_INPUT_BORDER_DISABLED`
    Disabled1,

    /// Base call-to-action and checked-state color.
    /// - `BUTTON_CHECKED_BG`
    /// - `BUTTON_OUTLINE_BORDER_CHECKED`
    /// - `BUTTON_OUTLINE_TEXT_CHECKED`
    /// - `BUTTON_PLAIN_TEXT_CHECKED`
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
    /// - `BUTTON_CHECKED_BG_HOVER`
    /// - `BUTTON_OUTLINE_BORDER_CHECKED_HOVER`
    /// - `BUTTON_PRIMARY_BG_HOVER`
    /// - `SLIDER_BAR_HOVER`
    /// - `SLIDER_THUMB_HOVER`
    /// - `SCROLLBAR_THUMB_HOVER`
    Accent1,

    /// Call-to-action pressed.
    /// - `BUTTON_CHECKED_BG_PRESSED`
    /// - `BUTTON_OUTLINE_BORDER_CHECKED_PRESSED`
    /// - `BUTTON_PRIMARY_BG_PRESSED`
    /// - `SLIDER_BAR_PRESSED`
    /// - `SLIDER_THUMB_PRESSED`
    /// - `SCROLLBAR_THUMB_PRESSED`
    Accent2,

    /// Brightest accent.
    /// - `TEXT_INPUT_CURSOR`
    Accent3,

    /// Base destructive-action color: the red counterpart to [`Accent0`](Self::Accent0).
    /// - `BUTTON_DANGER_BG`
    Danger0,

    /// Destructive-action hover.
    /// - `BUTTON_DANGER_BG_HOVER`
    Danger1,

    /// Destructive-action pressed.
    /// - `BUTTON_DANGER_BG_PRESSED`
    Danger2,

    /// Foreground over accent-filled components, and the switch knob.
    /// - `BUTTON_CHECKED_TEXT`
    /// - `BUTTON_DANGER_TEXT`
    /// - `BUTTON_PRIMARY_TEXT`
    /// - `CHECKBOX_MARK`
    /// - `RADIO_MARK`
    /// - `SWITCH_SLIDE_BG`
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
    /// - `RADIO_BG`
    /// - `RADIO_BG_CHECKED_DISABLED`
    /// - `RADIO_BG_DISABLED`
    /// - `SEPARATOR_HIDDEN`
    /// - `SWITCH_BG_CHECKED_DISABLED`
    /// - `SWITCH_BG_DISABLED`
    /// - `TEXT_INPUT_SELECTION_UNFOCUSED`
    /// - `CHECKBOX_BORDER_CHECKED`
    /// - `RADIO_BORDER_CHECKED`
    /// - `SWITCH_BORDER_CHECKED`
    /// - `SWITCH_BORDER`
    /// - `TAB_BG_DISABLED`
    /// - `TAB_BG`
    Transparent,
}

impl ThemeSlot {
    /// Every slot, in discriminant order (matches the resolved palette's storage).
    pub const ALL: [ThemeSlot; 24] = [
        ThemeSlot::Neutral0,
        ThemeSlot::Neutral1,
        ThemeSlot::Neutral2,
        ThemeSlot::Neutral3,
        ThemeSlot::Neutral4,
        ThemeSlot::Neutral5,
        ThemeSlot::Neutral6,
        ThemeSlot::Text0,
        ThemeSlot::Text1,
        ThemeSlot::Disabled0,
        ThemeSlot::Disabled1,
        ThemeSlot::Accent0,
        ThemeSlot::Accent1,
        ThemeSlot::Accent2,
        ThemeSlot::Accent3,
        ThemeSlot::Danger0,
        ThemeSlot::Danger1,
        ThemeSlot::Danger2,
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
            ThemeSlot::Disabled0 => "Disabled 0",
            ThemeSlot::Disabled1 => "Disabled 1",
            ThemeSlot::Accent0 => "Accent 0",
            ThemeSlot::Accent1 => "Accent 1",
            ThemeSlot::Accent2 => "Accent 2",
            ThemeSlot::Accent3 => "Accent 3",
            ThemeSlot::Danger0 => "Danger 0",
            ThemeSlot::Danger1 => "Danger 1",
            ThemeSlot::Danger2 => "Danger 2",
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
    (tokens::TEXT_DISABLED, ThemeSlot::Disabled1),
    (tokens::BUTTON_BG, ThemeSlot::Neutral4),
    (tokens::BUTTON_BG_HOVER, ThemeSlot::Neutral5),
    (tokens::BUTTON_BG_PRESSED, ThemeSlot::Neutral6),
    (tokens::BUTTON_BG_DISABLED, ThemeSlot::Disabled1),
    (tokens::BUTTON_PRIMARY_BG, ThemeSlot::Accent0),
    (tokens::BUTTON_PRIMARY_BG_HOVER, ThemeSlot::Accent1),
    (tokens::BUTTON_PRIMARY_BG_PRESSED, ThemeSlot::Accent2),
    (tokens::BUTTON_PRIMARY_BG_DISABLED, ThemeSlot::Disabled1),
    (tokens::BUTTON_DANGER_BG, ThemeSlot::Danger0),
    (tokens::BUTTON_DANGER_BG_HOVER, ThemeSlot::Danger1),
    (tokens::BUTTON_DANGER_BG_PRESSED, ThemeSlot::Danger2),
    (tokens::BUTTON_DANGER_BG_DISABLED, ThemeSlot::Disabled1),
    (tokens::BUTTON_DANGER_TEXT, ThemeSlot::Contrast),
    (tokens::BUTTON_DANGER_TEXT_DISABLED, ThemeSlot::Disabled0),
    (tokens::BUTTON_PLAIN_BG, ThemeSlot::Transparent),
    (tokens::BUTTON_PLAIN_BG_HOVER, ThemeSlot::Neutral3),
    (tokens::BUTTON_PLAIN_BG_PRESSED, ThemeSlot::Neutral4),
    (tokens::BUTTON_PLAIN_BG_DISABLED, ThemeSlot::Transparent),
    (tokens::BUTTON_OUTLINE_BG, ThemeSlot::Transparent),
    (tokens::BUTTON_OUTLINE_BG_HOVER, ThemeSlot::Neutral3),
    (tokens::BUTTON_OUTLINE_BG_PRESSED, ThemeSlot::Neutral4),
    (tokens::BUTTON_OUTLINE_BG_DISABLED, ThemeSlot::Transparent),
    (tokens::BUTTON_OUTLINE_BORDER, ThemeSlot::Neutral4),
    (tokens::BUTTON_OUTLINE_BORDER_HOVER, ThemeSlot::Neutral5),
    (tokens::BUTTON_OUTLINE_BORDER_PRESSED, ThemeSlot::Neutral6),
    (tokens::BUTTON_OUTLINE_BORDER_DISABLED, ThemeSlot::Disabled1),
    (tokens::BUTTON_BORDER_NONE, ThemeSlot::Transparent),
    (tokens::BUTTON_CHECKED_BG, ThemeSlot::Accent0),
    (tokens::BUTTON_CHECKED_BG_HOVER, ThemeSlot::Accent1),
    (tokens::BUTTON_CHECKED_BG_PRESSED, ThemeSlot::Accent2),
    (tokens::BUTTON_CHECKED_BG_DISABLED, ThemeSlot::Disabled1),
    (tokens::BUTTON_CHECKED_TEXT, ThemeSlot::Contrast),
    (tokens::BUTTON_CHECKED_TEXT_DISABLED, ThemeSlot::Disabled0),
    (tokens::BUTTON_PLAIN_TEXT_UNCHECKED, ThemeSlot::Text1),
    (tokens::BUTTON_PLAIN_TEXT_CHECKED, ThemeSlot::Accent0),
    (tokens::BUTTON_OUTLINE_TEXT_UNCHECKED, ThemeSlot::Text1),
    (tokens::BUTTON_OUTLINE_TEXT_CHECKED, ThemeSlot::Accent0),
    (tokens::BUTTON_OUTLINE_BORDER_CHECKED, ThemeSlot::Accent0),
    (
        tokens::BUTTON_OUTLINE_BORDER_CHECKED_HOVER,
        ThemeSlot::Accent1,
    ),
    (
        tokens::BUTTON_OUTLINE_BORDER_CHECKED_PRESSED,
        ThemeSlot::Accent2,
    ),
    (tokens::BUTTON_TEXT, ThemeSlot::Text0),
    (tokens::BUTTON_TEXT_DISABLED, ThemeSlot::Disabled0),
    (tokens::BUTTON_PRIMARY_TEXT, ThemeSlot::Contrast),
    (tokens::BUTTON_PRIMARY_TEXT_DISABLED, ThemeSlot::Disabled0),
    (tokens::SLIDER_BG, ThemeSlot::Neutral4),
    (tokens::SLIDER_BG_HOVER, ThemeSlot::Neutral4),
    (tokens::SLIDER_BG_PRESSED, ThemeSlot::Neutral4),
    (tokens::SLIDER_BG_DISABLED, ThemeSlot::Disabled1),
    (tokens::SLIDER_BAR, ThemeSlot::Accent0),
    (tokens::SLIDER_BAR_HOVER, ThemeSlot::Accent1),
    (tokens::SLIDER_BAR_PRESSED, ThemeSlot::Accent2),
    (tokens::SLIDER_BAR_DISABLED, ThemeSlot::Disabled1),
    (tokens::SLIDER_THUMB, ThemeSlot::Accent0),
    (tokens::SLIDER_THUMB_HOVER, ThemeSlot::Accent1),
    (tokens::SLIDER_THUMB_PRESSED, ThemeSlot::Accent2),
    (tokens::SLIDER_THUMB_DISABLED, ThemeSlot::Disabled1),
    (tokens::SCROLLBAR_BG, ThemeSlot::Neutral0),
    (tokens::SCROLLBAR_THUMB, ThemeSlot::Neutral4),
    (tokens::SCROLLBAR_THUMB_HOVER, ThemeSlot::Accent1),
    (tokens::SCROLLBAR_THUMB_PRESSED, ThemeSlot::Accent2),
    (tokens::CHECKBOX_BG, ThemeSlot::Transparent),
    (tokens::CHECKBOX_BG_DISABLED, ThemeSlot::Transparent),
    (tokens::CHECKBOX_BG_CHECKED, ThemeSlot::Accent0),
    (tokens::CHECKBOX_BG_CHECKED_DISABLED, ThemeSlot::Transparent),
    (tokens::CHECKBOX_BORDER, ThemeSlot::Neutral4),
    (tokens::CHECKBOX_BORDER_DISABLED, ThemeSlot::Disabled1),
    (tokens::CHECKBOX_BORDER_CHECKED, ThemeSlot::Transparent),
    (
        tokens::CHECKBOX_BORDER_CHECKED_DISABLED,
        ThemeSlot::Disabled1,
    ),
    (tokens::CHECKBOX_MARK, ThemeSlot::Contrast),
    (tokens::CHECKBOX_MARK_DISABLED, ThemeSlot::Disabled1),
    (tokens::CHECKBOX_TEXT, ThemeSlot::Text1),
    (tokens::CHECKBOX_TEXT_DISABLED, ThemeSlot::Disabled1),
    (tokens::RADIO_BG, ThemeSlot::Transparent),
    (tokens::RADIO_BG_DISABLED, ThemeSlot::Transparent),
    (tokens::RADIO_BG_CHECKED, ThemeSlot::Accent0),
    (tokens::RADIO_BG_CHECKED_DISABLED, ThemeSlot::Transparent),
    (tokens::RADIO_BORDER, ThemeSlot::Neutral4),
    (tokens::RADIO_BORDER_DISABLED, ThemeSlot::Disabled1),
    (tokens::RADIO_BORDER_CHECKED, ThemeSlot::Transparent),
    (tokens::RADIO_BORDER_CHECKED_DISABLED, ThemeSlot::Disabled1),
    (tokens::RADIO_MARK, ThemeSlot::Contrast),
    (tokens::RADIO_MARK_DISABLED, ThemeSlot::Disabled1),
    (tokens::RADIO_TEXT, ThemeSlot::Text1),
    (tokens::RADIO_TEXT_DISABLED, ThemeSlot::Disabled1),
    (tokens::SWITCH_BG, ThemeSlot::Neutral4),
    (tokens::SWITCH_BG_DISABLED, ThemeSlot::Transparent),
    (tokens::SWITCH_BG_CHECKED, ThemeSlot::Accent0),
    (tokens::SWITCH_BG_CHECKED_DISABLED, ThemeSlot::Transparent),
    (tokens::SWITCH_BORDER, ThemeSlot::Transparent),
    (tokens::SWITCH_BORDER_DISABLED, ThemeSlot::Disabled1),
    (tokens::SWITCH_BORDER_CHECKED, ThemeSlot::Transparent),
    (tokens::SWITCH_BORDER_CHECKED_DISABLED, ThemeSlot::Disabled1),
    (tokens::SWITCH_SLIDE_BG, ThemeSlot::Contrast),
    (tokens::SWITCH_SLIDE_BG_DISABLED, ThemeSlot::Disabled1),
    (tokens::SWITCH_SLIDE_BG_CHECKED, ThemeSlot::Contrast),
    (
        tokens::SWITCH_SLIDE_BG_CHECKED_DISABLED,
        ThemeSlot::Disabled1,
    ),
    (tokens::MENU_BG, ThemeSlot::Neutral1),
    (tokens::MENU_BORDER, ThemeSlot::Neutral4),
    (tokens::MENU_BUTTON_BG_HOVER, ThemeSlot::Neutral5),
    (tokens::MENU_BUTTON_TEXT, ThemeSlot::Text0),
    (tokens::MENU_BUTTON_TEXT_DISABLED, ThemeSlot::Disabled1),
    (tokens::MENU_ITEM_BG_HOVER, ThemeSlot::Neutral5),
    (tokens::MENU_ITEM_TEXT, ThemeSlot::Text0),
    (tokens::MENU_ITEM_TEXT_DISABLED, ThemeSlot::Disabled1),
    (tokens::POPUP_BG, ThemeSlot::Neutral1),
    (tokens::POPUP_BORDER, ThemeSlot::Neutral4),
    (tokens::TOOLTIP_BG, ThemeSlot::Neutral1),
    (tokens::TOOLTIP_BORDER, ThemeSlot::Neutral4),
    (tokens::TOOLTIP_TEXT, ThemeSlot::Text1),
    (tokens::TEXT_INPUT_BG, ThemeSlot::Neutral0),
    (tokens::TEXT_INPUT_BG_ACTIVE, ThemeSlot::Neutral0),
    (tokens::TEXT_INPUT_BG_DISABLED, ThemeSlot::Disabled1),
    (tokens::TEXT_INPUT_TEXT, ThemeSlot::Text0),
    (tokens::TEXT_INPUT_TEXT_ACTIVE, ThemeSlot::Text0),
    (tokens::TEXT_INPUT_TEXT_DISABLED, ThemeSlot::Disabled0),
    (tokens::TEXT_INPUT_CURSOR, ThemeSlot::Accent3),
    (tokens::TEXT_INPUT_SELECTION, ThemeSlot::Accent0),
    (
        tokens::TEXT_INPUT_SELECTION_UNFOCUSED,
        ThemeSlot::Transparent,
    ),
    (tokens::TEXT_INPUT_BORDER, ThemeSlot::Neutral4),
    (tokens::TEXT_INPUT_BORDER_ACTIVE, ThemeSlot::Accent0),
    (tokens::TEXT_INPUT_BORDER_DISABLED, ThemeSlot::Disabled1),
    (tokens::COLOR_SWATCH_BORDER, ThemeSlot::Neutral4),
    (tokens::SECTION_HEADER_BG, ThemeSlot::Neutral2),
    (tokens::SECTION_HEADER_TEXT, ThemeSlot::Text0),
    (tokens::SECTION_HEADER_MUTED_TEXT, ThemeSlot::Text1),
    (tokens::SECTION_BODY_BG, ThemeSlot::Neutral1),
    (tokens::TAB_TEXT, ThemeSlot::Text1),
    (tokens::TAB_TEXT_SELECTED, ThemeSlot::Text0),
    (tokens::TAB_TEXT_DISABLED, ThemeSlot::Disabled1),
    (tokens::TAB_INDICATOR, ThemeSlot::Accent0),
    (tokens::TAB_INDICATOR_DISABLED, ThemeSlot::Disabled1),
    (tokens::TAB_BAR_BG, ThemeSlot::Neutral0),
    (tokens::TAB_BG, ThemeSlot::Transparent),
    (tokens::TAB_BG_HOVERED, ThemeSlot::Neutral3),
    (tokens::TAB_BG_PRESSED, ThemeSlot::Neutral4),
    (tokens::TAB_BG_DISABLED, ThemeSlot::Transparent),
    (tokens::TAB_BG_SELECTED, ThemeSlot::Neutral1),
    (tokens::TAB_BODY_BG, ThemeSlot::Neutral1),
    (tokens::SELECT_BG, ThemeSlot::Neutral0),
    (tokens::SELECT_BORDER, ThemeSlot::Neutral4),
    (tokens::SELECT_OPTION_BG_HOVER, ThemeSlot::Neutral3),
    (tokens::SELECT_OPTION_TEXT, ThemeSlot::Text0),
    (tokens::SELECT_OPTION_TEXT_DISABLED, ThemeSlot::Disabled1),
    (tokens::DIALOG_BG, ThemeSlot::Neutral1),
    (tokens::DIALOG_BORDER, ThemeSlot::Neutral4),
    (tokens::DIALOG_HEADER_BG, ThemeSlot::Neutral1),
    (tokens::DIALOG_TEXT, ThemeSlot::Text1),
    (tokens::DIALOG_HEADER_TEXT, ThemeSlot::Text0),
    (tokens::SEPARATOR, ThemeSlot::Neutral4),
    (tokens::SEPARATOR_HIDDEN, ThemeSlot::Transparent),
    (tokens::FOCUS_RING, ThemeSlot::FocusRing),
];

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    // `pub const NAME: ThemeToken = ThemeToken::new_static("id")` pairs out of
    // tokens.rs, tolerating multiline declarations.
    fn token_const_names() -> HashMap<String, String> {
        let mut out = HashMap::new();
        let mut rest = include_str!("tokens.rs");
        while let Some(pos) = rest.find("pub const ") {
            rest = &rest[pos + "pub const ".len()..];
            let Some((name, after_name)) = rest.split_once(':') else {
                break;
            };
            let Some((ty, _)) = after_name.split_once('=') else {
                continue;
            };
            if ty.trim() != "ThemeToken" {
                continue;
            }
            let Some((_, after_open)) = after_name.split_once("new_static(\"") else {
                continue;
            };
            let Some((id, after_id)) = after_open.split_once('"') else {
                continue;
            };
            out.insert(id.to_string(), name.trim().to_string());
            rest = after_id;
        }
        out
    }

    // `- \`TOKEN\`` doc lines per variant out of this file's enum block.
    fn doc_listed() -> Vec<(String, String)> {
        let src = include_str!("slots.rs");
        let body = src
            .split_once("pub enum ThemeSlot {")
            .expect("enum start")
            .1
            .split_once("\n}")
            .expect("enum end")
            .0;
        let mut out = Vec::new();
        let mut pending: Vec<String> = Vec::new();
        for line in body.lines() {
            let line = line.trim();
            if let Some(token) = line
                .strip_prefix("/// - `")
                .and_then(|rest| rest.split_once('`'))
                .map(|(token, _)| token)
            {
                pending.push(token.to_string());
            } else if !line.is_empty()
                && !line.starts_with("///")
                && !line.starts_with("#[")
                && let Some(variant) = line.strip_suffix(',')
            {
                out.extend(pending.drain(..).map(|token| (token, variant.to_string())));
            }
        }
        out
    }

    /// Every slot doc comment's token list agrees with [`DEFAULT_TOKEN_SLOTS`]:
    /// each mapped token is listed exactly once, under the slot it maps to.
    #[test]
    fn slot_doc_lists_match_mapping() {
        let names = token_const_names();
        assert!(
            names.len() > 100,
            "token parser broke: {} tokens",
            names.len()
        );
        let listed = doc_listed();
        assert!(
            listed.len() > 100,
            "doc parser broke: {} entries",
            listed.len()
        );

        let mapping: HashMap<String, String> = DEFAULT_TOKEN_SLOTS
            .iter()
            .map(|(token, slot)| {
                let name = names
                    .get(&token.to_string())
                    .unwrap_or_else(|| panic!("no const named for token `{token}`"));
                (name.clone(), format!("{slot:?}"))
            })
            .collect();
        assert_eq!(
            mapping.len(),
            DEFAULT_TOKEN_SLOTS.len(),
            "duplicate token in DEFAULT_TOKEN_SLOTS"
        );

        let mut problems = Vec::new();
        let mut seen: HashMap<&str, &str> = HashMap::new();
        for (token, variant) in &listed {
            if let Some(earlier) = seen.insert(token, variant) {
                problems.push(format!(
                    "`{token}` listed under both {earlier} and {variant}"
                ));
            }
            match mapping.get(token) {
                None => problems.push(format!("{variant} lists `{token}`, which maps to no slot")),
                Some(actual) if actual != variant => {
                    problems.push(format!("{variant} lists `{token}`, which maps to {actual}"));
                }
                _ => {}
            }
        }
        for (token, variant) in &mapping {
            if !seen.contains_key(token.as_str()) {
                problems.push(format!(
                    "`{token}` maps to {variant} but no doc list names it"
                ));
            }
        }
        assert!(problems.is_empty(), "\n{}", problems.join("\n"));
    }

    /// Every token declared in tokens.rs has a slot in [`DEFAULT_TOKEN_SLOTS`].
    #[test]
    fn every_token_has_a_slot() {
        let names = token_const_names();
        assert!(
            names.len() > 100,
            "token parser broke: {} tokens",
            names.len()
        );

        let mapped: std::collections::HashSet<String> = DEFAULT_TOKEN_SLOTS
            .iter()
            .map(|(token, _)| token.to_string())
            .collect();
        let mut missing: Vec<&str> = names
            .iter()
            .filter(|(id, _)| !mapped.contains(*id))
            .map(|(_, name)| name.as_str())
            .collect();
        missing.sort_unstable();
        assert!(
            missing.is_empty(),
            "tokens with no slot in DEFAULT_TOKEN_SLOTS:\n{}",
            missing.join("\n")
        );
    }
}
