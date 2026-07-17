use bevy_scene::{Scene, bsn};
use bevy_ui::{Node, px};

use crate::{theme::ThemeBackgroundColor, tokens};

/// A hairline horizontal rule; relies on the container's `align_items: Stretch`
/// (e.g. [`column`](crate::containers::column)) for its width.
pub fn separator() -> impl Scene {
    bsn! {
        Node {
            height: px(1),
        }
        ThemeBackgroundColor(tokens::SEPARATOR)
    }
}
