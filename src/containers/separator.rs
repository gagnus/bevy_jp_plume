use bevy_scene::{Scene, bsn};
use bevy_ui::{AlignSelf, Node};

use crate::{constants::size, theme::ThemeBackgroundColor, tokens};

/// A hairline rule that takes its orientation from the container it sits in:
/// a horizontal line in a [`column`](crate::containers::column), a vertical one
/// in a [`row`](crate::containers::row).
///
/// `flex_basis` sizes the main axis whichever way the parent flows, and
/// `align_self: Stretch` spans the cross axis even when the parent centers its
/// children, so neither dimension needs to name a direction.
pub fn separator() -> impl Scene {
    bsn! {
        Node {
            flex_basis: size::CONTAINER_BORDER,
            flex_grow: 0.0,
            flex_shrink: 0.0,
            align_self: AlignSelf::Stretch,
        }
        ThemeBackgroundColor(tokens::SEPARATOR)
    }
}
