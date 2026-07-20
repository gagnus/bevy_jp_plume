//! BSN scene function for a growing invisible gap.
use bevy_scene::{Scene, bsn};
use bevy_ui::Node;

/// An invisible node that absorbs the container's leftover main-axis space.
pub fn flex_spacer() -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
        }
    }
}
