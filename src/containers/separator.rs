use bevy_scene::{Scene, bsn};
use bevy_ui::Node;

use crate::{constants::size, theme::ThemeBackgroundColor, tokens};

/// A hairline horizontal rule; relies on the container's `align_items: Stretch`
/// (e.g. [`column`](crate::containers::column)) for its width.
pub fn separator() -> impl Scene {
    bsn! {
        Node {
            height: size::CONTAINER_BORDER,
        }
        ThemeBackgroundColor(tokens::SEPARATOR)
    }
}
