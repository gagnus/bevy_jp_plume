//! BSN scene function for a horizontal flex container.
use bevy::scene::{Scene, bsn};
use bevy::ui::{AlignItems, Display, FlexDirection, Node, Val};

use crate::{constants::size, font_styles::TextStyleRelay};

/// Horizontal container that vertically centers mixed-height children; content
/// goes in `Children`. 
pub fn row() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: size::GAP,
            // Flex's `auto` minimum refuses to shrink below content, so a nested scroll
            // area only bounds once every container above it can give. Re-floor with `.min_height()`.
            min_height: Val::ZERO,
        }
        TextStyleRelay
    }
}
