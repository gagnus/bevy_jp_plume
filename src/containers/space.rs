//! BSN scene function for a fixed-size invisible gap.
use bevy::picking::Pickable;
use bevy::scene::{Scene, bsn};
use bevy::ui::{Node, Val};

use crate::font_styles::TextStyleRelay;

/// An invisible node `length` along its container's main axis — the fixed-size
/// counterpart to [`flex_spacer`](crate::containers::flex_spacer).
///
/// [`Pickable::IGNORE`], like [`screen`](crate::containers::screen): a gap that draws
/// nothing has no business swallowing a pick.
pub fn space(length: Val) -> impl Scene {
    bsn! {
        Node {
            flex_basis: length,
            flex_grow: 0.0,
            flex_shrink: 0.0,
        }
        Pickable::IGNORE
        // `length` is routinely an em size, which needs the chain's `EmSize`.
        TextStyleRelay
    }
}
