//! BSN scene function for a horizontal flex container.
use bevy_scene::{Scene, bsn};
use bevy_text::FontWeight;
use bevy_ui::{AlignItems, Display, FlexDirection, Node};

use crate::{
    constants::{fonts, size},
    font_styles::InheritableFont,
    theme::InheritableThemeTextColor,
    tokens,
};

/// Horizontal container that vertically centers mixed-height children; content
/// goes in `Children`. Carries the standard font and text color.
pub fn row() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: size::GAP,
        }
        InheritableThemeTextColor(tokens::TEXT_DIM)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
    }
}
