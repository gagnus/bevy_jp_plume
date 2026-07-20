//! BSN scene function for a vertical flex container.
use bevy_scene::{Scene, bsn};
use bevy_text::FontWeight;
use bevy_ui::{AlignItems, Display, FlexDirection, Node};

use crate::{
    constants::{fonts, size},
    font_styles::InheritableFont,
    theme::InheritableThemeTextColor,
    tokens,
};

/// Vertical container that stretches children to its own width; content goes in
/// `Children`. Carries the standard font and text color, so bare text works.
pub fn column() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: size::GAP,
        }
        InheritableThemeTextColor(tokens::TEXT_MAIN)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
    }
}
