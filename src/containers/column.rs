use bevy_scene::{Scene, bsn};
use bevy_ui::{AlignItems, Display, FlexDirection, Node};

use crate::constants::size;

/// Vertical container that stretches children to its own width,
/// e.g. a stack of rows or subpanes; content goes in `Children`.
pub fn column() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: size::GAP,
        }
    }
}
