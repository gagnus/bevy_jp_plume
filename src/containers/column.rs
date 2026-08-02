//! BSN scene function for a vertical flex container.
use bevy::scene::{Scene, bsn};
use bevy::ui::{AlignItems, Display, FlexDirection, Node, Val};

use crate::constants::size;
use crate::font_styles::TextStyleRelay;

/// Vertical container that stretches children to its own width; content goes in
/// `Children`.
pub fn column() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: size::GAP,
            // Flex's `auto` minimum refuses to shrink below content, so a nested scroll
            // area only bounds once every container above it can give. Re-floor with `.min_height()`.
            min_height: Val::ZERO,
        }
        TextStyleRelay
    }
}
