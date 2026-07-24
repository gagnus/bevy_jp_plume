//! BSN scene function for a fixed-size invisible gap.
use bevy::scene::{Scene, bsn};
use bevy::ui::{Node, Val};

/// An invisible node `length` along its container's main axis — the fixed-size
/// counterpart to [`flex_spacer`](crate::containers::flex_spacer).
pub fn space(length: Val) -> impl Scene {
    bsn! {
        Node {
            flex_basis: {length},
            flex_grow: 0.0,
            flex_shrink: 0.0,
        }
    }
}
