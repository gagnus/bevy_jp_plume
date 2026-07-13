use bevy_scene::{Scene, bsn};
use bevy_ui::{AlignItems, Display, FlexDirection, Node};

use crate::constants::size;

/// Horizontal container that vertically centers mixed-height children,
/// e.g. a label beside a button; content goes in `Children`.
pub fn row() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: size::GAP,
        }
    }
}
