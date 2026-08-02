//! Generic vertical scroll region: a gutter-reserving frame, its scrolling
//! viewport, and the scrollbar that drives it. Shared by the dialog body and the
//! imm `scroll_area` widget.
use bevy::app::{App, Plugin, PostUpdate};
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::Children;
use bevy::ecs::query::With;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Local, Query};
use bevy::ecs::template::EntityTemplate;
use bevy::log::warn_once;
use bevy::math::Rect;
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::ui::{
    AlignItems, CalculatedClip, ComputedNode, Display, FlexDirection, Node, Overflow, PositionType,
    UiGlobalTransform, UiSystems, Val,
};
use bevy::ui_widgets::{ControlOrientation, ScrollArea};

use crate::constants::size;
use crate::controls::{PlumeScrollbar, ScrollbarGutter};
use crate::font_styles::TextStyleRelay;

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
        TextStyleRelay
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
        TextStyleRelay
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
        TextStyleRelay
    }
}

// Installs the check for a scroll region that was never given a height to
// scroll within.
pub(crate) struct ScrollAreaPlugin;

impl Plugin for ScrollAreaPlugin {
    fn build(&self, app: &mut App) {
        // `PostLayout` is where `CalculatedClip` is written.
        app.add_systems(
            PostUpdate,
            warn_unbounded_scroll_area.after(UiSystems::PostLayout),
        );
    }
}

// Layout rounds, so a rect flush with its clip must not read as past it.
const CLIP_EPSILON: f32 = 0.5;

// An unbounded scroll region is as tall as its content, so it never scrolls and an
// ancestor cuts it off instead — which looks correct from inside the region.
fn warn_unbounded_scroll_area(
    query_areas: Query<
        (&ComputedNode, &UiGlobalTransform, Option<&CalculatedClip>),
        With<ScrollArea>,
    >,
    mut suspect_last_frame: Local<bool>,
) {
    let clipped_without_scrolling = query_areas.iter().any(|(node, transform, clip)| {
        // A hidden region (an unselected tab body) lays out at zero size, which
        // reads as clipped without ever being a problem.
        if node.size.y <= CLIP_EPSILON {
            return false;
        }
        // Overflowing its viewport means it is scrolling, which is the point.
        if node.content_size.y > node.size.y + CLIP_EPSILON {
            return false;
        }
        let Some(clip) = clip else {
            return false;
        };
        let rect = Rect::from_center_size(transform.translation, node.size);
        rect.max.y > clip.clip.max.y + CLIP_EPSILON || rect.min.y < clip.clip.min.y - CLIP_EPSILON
    });

    // Layout settles over a frame or two, so a single-frame reading proves nothing.
    let confirmed = clipped_without_scrolling && *suspect_last_frame;
    *suspect_last_frame = clipped_without_scrolling;
    if confirmed {
        warn_once!(
            "A scroll area is clipped but not scrolling — its content is cut off and no \
             scrollbar reaches it. Every container between it and a fixed height needs \
             `min_height: Val::ZERO` to shrink."
        );
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
        // An em width needs the chain's `EmSize`.
        TextStyleRelay
    }
}

/// Vertically scrolling region: `contents` scroll inside a managed viewport with a
/// self-hiding scrollbar, once they outgrow the height the area is given.
///
/// Give it a bounded height (a `max_height`, or a `flex_grow` inside a bounded
/// parent) — an unbounded one just grows and never scrolls.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeScrollAreaProps)]
#[reflect(Component, Default, Clone)]
pub struct PlumeScrollArea;

/// Props for a [`PlumeScrollArea`].
pub struct PlumeScrollAreaProps {
    /// The scrolling content.
    pub contents: Box<dyn SceneList>,
}

impl Default for PlumeScrollAreaProps {
    fn default() -> Self {
        Self {
            contents: Box::new(bsn_list!()),
        }
    }
}

impl PlumeScrollArea {
    fn scene(props: PlumeScrollAreaProps) -> impl Scene {
        let contents = props.contents;
        bsn! {
            scroll_frame()
            Children [
                (
                    #viewport
                    scroll_viewport()
                    Children [
                        (scroll_content() Children [ {contents} ])
                    ]
                ),
                (
                    @PlumeScrollbar {
                        @target: #viewport,
                        @orientation: {ControlOrientation::Vertical},
                    }
                    scrollbar_node()
                ),
            ]
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
