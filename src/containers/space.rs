use bevy_scene::{Scene, bsn};
use bevy_ui::{Node, Val};

/// An invisible node of fixed size along its container's main axis: `length`
/// wide in a [`row`](crate::containers::row), `length` tall in a
/// [`column`](crate::containers::column).
///
/// The fixed-size counterpart to [`flex_spacer`](crate::containers::flex_spacer),
/// for opening a deliberate gap wider than the container's own.
pub fn space(length: Val) -> impl Scene {
    bsn! {
        Node {
            flex_basis: {length},
            flex_grow: 0.0,
            flex_shrink: 0.0,
        }
    }
}
