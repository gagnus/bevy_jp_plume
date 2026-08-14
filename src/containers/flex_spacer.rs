//! BSN scene function for a growing invisible gap.
use bevy::picking::Pickable;
use bevy::scene::{Scene, bsn};
use bevy::ui::Node;

/// An invisible node that absorbs the container's leftover main-axis space.
///
/// [`Pickable::IGNORE`] for the same reason as [`space`](crate::containers::space),
/// and it matters more here: this is the one that spans the leftover width.
pub fn flex_spacer() -> impl Scene {
    bsn! {
        Node {
            flex_grow: 1.0,
        }
        Pickable::IGNORE
    }
}
