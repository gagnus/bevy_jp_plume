//! BSN scene function for a hairline rule.
use bevy_scene::{Scene, bsn};
use bevy_ui::{AlignSelf, Node};

use crate::{constants::size, theme::ThemeBackgroundColor, tokens};

/// A hairline rule taking its orientation from the container it sits in:
/// horizontal in a [`column`](crate::containers::column), vertical in a
/// [`row`](crate::containers::row).
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
