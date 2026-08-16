//! Generic scroll region on either axis: a gutter-reserving frame, its scrolling
//! viewport, and the scrollbar that drives it. Shared by the dialog body and the
//! imm `scroll_area` widget.
use bevy::app::{App, Plugin, PostUpdate};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::Children;
use bevy::ecs::observer::On;
use bevy::ecs::query::{Or, With};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Local, Query};
use bevy::ecs::template::EntityTemplate;
use bevy::input::mouse::MouseScrollUnit;
use bevy::log::warn_once;
use bevy::math::Rect;
use bevy::picking::events::{Pointer, Scroll};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::ui::{
    AlignItems, CalculatedClip, ComputedNode, Display, FlexDirection, Node, Overflow, OverflowAxis,
    PositionType, ScrollPosition, UiGlobalTransform, UiSystems, Val,
};
use bevy::ui_widgets::{ControlOrientation, ScrollArea};

use crate::constants::size;
use crate::containers::{BodyGap, BodyPadding, apply_body_style};
use crate::controls::{PlumeScrollbar, ScrollbarGutter};
use crate::font_styles::TextStyleRelay;

/// The axis a scroll region scrolls along.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ScrollAxis {
    /// Scrolls up and down; content stacks in a column.
    #[default]
    Vertical,
    /// Scrolls left and right; content stacks in a row.
    Horizontal,
}

// The content stack, so [`BodyGap`] on the frame can find it.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct ScrollContent;

impl ScrollAxis {
    fn flex_direction(self) -> FlexDirection {
        match self {
            Self::Vertical => FlexDirection::Column,
            Self::Horizontal => FlexDirection::Row,
        }
    }

    fn orientation(self) -> ControlOrientation {
        match self {
            Self::Vertical => ControlOrientation::Vertical,
            Self::Horizontal => ControlOrientation::Horizontal,
        }
    }

    // `auto` resolves to the content size, which stops the region ever bounding.
    fn min_size(self) -> (Val, Val) {
        match self {
            Self::Vertical => (Val::Auto, Val::ZERO),
            Self::Horizontal => (Val::ZERO, Val::Auto),
        }
    }
}

// Bounded frame holding the scrolling viewport and its scrollbar. Distinct from a
// plain column because `ScrollbarGutter` assigns padding on the scrollbar's edge,
// which elsewhere would eat the container's own padding.
pub(crate) fn scroll_frame(axis: ScrollAxis) -> impl Scene {
    let (min_width, min_height) = axis.min_size();
    // A horizontal region hugs its height in the column it sits in; `.grow()` is
    // the caller's to add.
    let flex_grow = match axis {
        ScrollAxis::Vertical => 1.0,
        ScrollAxis::Horizontal => 0.0,
    };
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: {axis.flex_direction()},
            align_items: AlignItems::Stretch,
            flex_grow: flex_grow,
            min_width: min_width,
            min_height: min_height,
        }
        ScrollbarGutter(size::SCROLLBAR_GUTTER)
        TextStyleRelay
    }
}

// The scrolling viewport itself. Single-axis — a scroll area never scrolls both
// ways. Content goes in the [`scroll_content`] child, not here.
pub(crate) fn scroll_viewport(axis: ScrollAxis) -> impl Scene {
    let (min_width, min_height) = axis.min_size();
    let overflow = match axis {
        ScrollAxis::Vertical => Overflow::scroll_y(),
        ScrollAxis::Horizontal => Overflow::scroll_x(),
    };
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: {axis.flex_direction()},
            align_items: AlignItems::Stretch,
            flex_grow: 1.0,
            min_width: min_width,
            min_height: min_height,
            overflow: overflow,
        }
        ScrollArea
        TextStyleRelay
    }
}

// Content stack inside a [`scroll_viewport`]. Flex shrinks items to fit even when the
// container scrolls, and `row()`/`column()` floor their minimums at zero, so without a
// `flex_shrink: 0` wrapper every row gets crushed instead of overflowing into the scroll.
pub(crate) fn scroll_content(axis: ScrollAxis) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: {axis.flex_direction()},
            align_items: AlignItems::Stretch,
            row_gap: size::SPACE,
            column_gap: size::SPACE,
            flex_shrink: 0.0,
        }
        ScrollContent
        TextStyleRelay
    }
}

// The content stack under a scroll frame, two levels down past the viewport. Also
// how a dialog finds the stack its own body scrolls when a height knob bounds it.
pub(crate) fn scroll_content_of(
    frame: &Children,
    q_children: &Query<&Children>,
    q_content: &Query<(), With<ScrollContent>>,
) -> Option<Entity> {
    frame
        .iter()
        .filter_map(|viewport| q_children.get(*viewport).ok())
        .flat_map(|stacks| stacks.iter())
        .find(|entity| q_content.contains(**entity))
        .copied()
}

// `BodyGap` / `BodyPadding` are set on the frame, which is what a caller holds, but
// only the content stack lays the items out.
fn relay_scroll_content_style(
    q_frames: Query<
        (Option<&BodyGap>, Option<&BodyPadding>, &Children),
        Or<(With<BodyGap>, With<BodyPadding>)>,
    >,
    q_children: Query<&Children>,
    q_content: Query<(), With<ScrollContent>>,
    mut q_nodes: Query<&mut Node>,
) {
    for (gap, padding, children) in q_frames.iter() {
        let Some(content) = scroll_content_of(children, &q_children, &q_content) else {
            continue;
        };
        if let Ok(mut node) = q_nodes.get_mut(content) {
            apply_body_style(&mut node, gap, padding);
        }
    }
}

// A wheel reports its notches on `y`, so a region that only scrolls sideways would
// never move. Send that delta down its one axis instead, as a browser does.
fn scroll_sideways_on_wheel(
    scroll: On<Pointer<Scroll>>,
    mut query_areas: Query<(&Node, &ComputedNode, &mut ScrollPosition), With<ScrollArea>>,
) {
    let Ok((node, computed, mut position)) = query_areas.get_mut(scroll.entity) else {
        return;
    };
    // One that scrolls both ways already reads the wheel the way the user means it.
    if node.overflow.x != OverflowAxis::Scroll || node.overflow.y == OverflowAxis::Scroll {
        return;
    }
    let delta_y = if scroll.unit == MouseScrollUnit::Line {
        scroll.y * 16. // note hard coded for my setup, much smaller than the MouseScrollPixelsPerLine 100
    } else {
        scroll.y
    };

    // A tilt wheel or trackpad swipe already arrives on `x`, handled upstream.
    if delta_y == 0.0 {
        return;
    }
    let visible = computed.size() * computed.inverse_scale_factor;
    let content = computed.content_size() * computed.inverse_scale_factor;
    position.x = (position.x - delta_y).clamp(0.0, (content.x - visible.x).max(0.0));
}

// Installs the sideways wheel mapping and the check for a scroll region that was
// never given a height to scroll within.
pub(crate) struct ScrollAreaPlugin;

impl Plugin for ScrollAreaPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(scroll_sideways_on_wheel);
        app.add_systems(
            PostUpdate,
            relay_scroll_content_style.before(UiSystems::Layout),
        );
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

// Placement shared by every scroll region's scrollbar: pinned along the trailing
// edge of the [`scroll_frame`].
pub(crate) fn scrollbar_node(axis: ScrollAxis) -> impl Scene {
    // The unpinned side stays `Auto` so the scrollbar keeps its own thickness.
    let (left, top, width, height) = match axis {
        ScrollAxis::Vertical => (Val::Auto, Val::ZERO, size::SCROLLBAR_WIDTH, Val::Auto),
        ScrollAxis::Horizontal => (Val::ZERO, Val::Auto, Val::Auto, size::SCROLLBAR_WIDTH),
    };
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: left,
            right: Val::ZERO,
            top: top,
            bottom: Val::ZERO,
            width: width,
            height: height,
        }
        // An em width needs the chain's `EmSize`.
        TextStyleRelay
    }
}

/// Scrolling region: `contents` scroll inside a managed viewport with a
/// self-hiding scrollbar, once they outgrow the size the area is given.
///
/// Give it a bounded main axis (a `max_height`, or a `flex_grow` inside a bounded
/// parent) — an unbounded one just grows and never scrolls.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeScrollAreaProps)]
#[reflect(Component, Default, Clone)]
pub struct PlumeScrollArea;

/// Props for a [`PlumeScrollArea`].
pub struct PlumeScrollAreaProps {
    /// The scrolling content.
    pub contents: Box<dyn SceneList>,
    /// Which way the region scrolls.
    pub axis: ScrollAxis,
}

impl Default for PlumeScrollAreaProps {
    fn default() -> Self {
        Self {
            contents: Box::new(bsn_list![]),
            axis: ScrollAxis::default(),
        }
    }
}

impl PlumeScrollArea {
    fn scene(props: PlumeScrollAreaProps) -> impl Scene {
        let PlumeScrollAreaProps { contents, axis } = props;
        bsn! {
            scroll_frame(axis)
            Children [
                (
                    #viewport
                    scroll_viewport(axis)
                    Children [
                        (
                            scroll_content(axis)
                            Children [
                                {contents},
                            ]
                        ),
                    ]
                ),
                (
                    @PlumeScrollbar {
                        @target: #viewport,
                        @orientation: {axis.orientation()},
                    }
                    scrollbar_node(axis)
                ),
            ]
        }
    }
}

// Scrollbar driving the viewport at `target`. Hidden, and its gutter reclaimed,
// whenever the content fits — see `update_scrollbar_visibility`.
pub(crate) fn scrollbar(target: Entity, axis: ScrollAxis) -> impl Scene {
    bsn! {
        @PlumeScrollbar {
            @target: EntityTemplate::from(target),
            @orientation: {axis.orientation()},
        }
        scrollbar_node(axis)
    }
}
