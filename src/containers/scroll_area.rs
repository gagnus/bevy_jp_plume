//! Generic vertical scroll region: a gutter-reserving frame, its scrolling
//! viewport, and the scrollbar that drives it. Shared by the dialog body and the
//! imm `scroll_area` widget.
use bevy::ecs::{entity::Entity, template::EntityTemplate};
use bevy::scene::{Scene, bsn};
use bevy::ui::{AlignItems, Display, FlexDirection, Node, Overflow, PositionType, Val};
use bevy::ui_widgets::{ControlOrientation, ScrollArea};

use crate::{
    constants::size,
    controls::{PlumeScrollbar, ScrollbarGutter},
};

// Bounded frame holding the scrolling viewport and its scrollbar. Distinct from a
// plain column because `ScrollbarGutter` assigns `padding.right`, which elsewhere
// would eat the container's own padding.
pub(crate) fn scroll_frame() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            flex_grow: 1.0,
            min_height: Val::ZERO,
        }
        ScrollbarGutter(size::SCROLLBAR_GUTTER)
    }
}

// The scrolling viewport itself. Vertical only — a scroll area is never allowed to
// scroll sideways. Content goes in the [`scroll_content`] child, not here.
pub(crate) fn scroll_viewport() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            flex_grow: 1.0,
            min_height: Val::ZERO,
            overflow: Overflow::scroll_y(),
        }
        ScrollArea
    }
}

// Content column inside a [`scroll_viewport`]. Flex shrinks items to fit their
// container even when it scrolls, and `row()`/`column()` floor `min_height` at zero,
// so without a `flex_shrink: 0` wrapper to absorb that pressure every row the caller
// writes gets crushed — centered content spilling out of the clip — instead of
// overflowing into the scroll.
pub(crate) fn scroll_content() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: size::GAP,
            flex_shrink: 0.0,
        }
    }
}

// Placement shared by every scroll region's scrollbar: pinned down the trailing
// edge of the [`scroll_frame`].
pub(crate) fn scrollbar_node() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            right: Val::ZERO,
            top: Val::ZERO,
            bottom: Val::ZERO,
            width: size::SCROLLBAR_WIDTH,
        }
    }
}

// Vertical scrollbar driving the viewport at `target`. Hidden, and its gutter
// reclaimed, whenever the content fits — see `update_scrollbar_visibility`.
pub(crate) fn scrollbar(target: Entity) -> impl Scene {
    bsn! {
        @PlumeScrollbar {
            @target: {EntityTemplate::from(target)},
            @orientation: {ControlOrientation::Vertical},
        }
        scrollbar_node()
    }
}
