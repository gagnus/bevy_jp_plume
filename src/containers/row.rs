//! BSN scene function for a horizontal flex container.
use bevy::scene::{Scene, bsn};
use bevy::ui::{AlignItems, Display, FlexDirection, Node, Val};

use crate::{
    constants::size, font_styles::TextStyleRelay, theme::InheritableThemeTextColor, tokens,
};

/// Horizontal container that vertically centers mixed-height children; content
/// goes in `Children`. Carries the standard text color and relays the font.
pub fn row() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: size::GAP,
            // Flex defaults `min_height` to the content size, which stops any
            // nested scrolling region from bounding — a scroll area only engages
            // once every container between it and the fixed height can shrink.
            // Floor a container that must not be crushed with `.min_height()`.
            min_height: Val::ZERO,
        }
        InheritableThemeTextColor(tokens::TEXT_DIM)
        TextStyleRelay
    }
}
