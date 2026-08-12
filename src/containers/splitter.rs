//! Two panes with a draggable divider between them.
//!
//! The split is stored as a *fraction* of the splitter's length, so both panes
//! keep their share as the window resizes. Each pane may state a minimum; the
//! divider stops there, and so does the stored fraction — a value that kept
//! travelling past what the layout will do would leave the drag with dead
//! travel to unwind before the divider moved again.
use bevy::app::{Plugin, PostUpdate, PreUpdate};
use bevy::ecs::change_detection::DetectChangesMut;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EntityEvent;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::lifecycle::RemovedComponents;
use bevy::ecs::observer::On;
use bevy::ecs::query::{Added, Changed, Has, Or, With, Without};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::math::Vec2;
use bevy::picking::events::{Cancel, Drag, DragEnd, DragStart, Pointer, Press, Release};
use bevy::picking::hover::Hovered;
use bevy::picking::{Pickable, PickingSystems};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::text::{EmSize, RemSize};
use bevy::ui::{
    AlignItems, AlignSelf, ComputedNode, ComputedUiRenderTargetInfo, FlexDirection, JustifyContent,
    Node, Overflow, PositionType, Pressed, UiSystems, Val,
};
use bevy::ui_widgets::ValueChange;
use bevy::window::SystemCursorIcon;

use crate::constants::size;
use crate::cursor::{CursorLock, EntityCursor};
use crate::font_styles::TextStyleRelay;
use crate::theme::ThemeBackgroundToken;
use crate::tokens;

// Grab width of the divider. The visible line is a hairline; this is the strip
// the pointer has to land in, which has to be usable without being a gutter.
const DIVIDER_GRAB: Val = size::em_from_px(7.0);

// The hairline while hovered or dragged: thick enough to read as grabbable. It
// grows inside the grab strip, so neither pane moves.
const DIVIDER_LINE_ACTIVE: Val = size::em_from_px(3.0);

// A fraction this far from what the layout actually produced is taken as the
// layout having clamped it — see `snap_split_to_layout`. Half a physical pixel,
// as a fraction, is below anything the eye or the next drag can tell apart.
const SNAP_EPSILON_PX: f32 = 0.5;

/// Which way a [`PlumeSplitter`]'s panes are arranged.
#[derive(Component, Clone, Copy, PartialEq, Eq, Default, Debug, Reflect)]
#[reflect(Component, Clone, Default)]
pub enum SplitAxis {
    /// Panes side by side with a vertical divider, like a
    /// [`row`](crate::containers::row).
    #[default]
    Horizontal,
    /// Panes stacked with a horizontal divider, like a
    /// [`column`](crate::containers::column).
    Vertical,
}

impl SplitAxis {
    // The drag component and the measurement that move the divider.
    fn of(self, value: Vec2) -> f32 {
        match self {
            SplitAxis::Horizontal => value.x,
            SplitAxis::Vertical => value.y,
        }
    }

    fn cursor(self) -> SystemCursorIcon {
        match self {
            SplitAxis::Horizontal => SystemCursorIcon::ColResize,
            SplitAxis::Vertical => SystemCursorIcon::RowResize,
        }
    }
}

/// The first pane's share of the splitter's length, `0.0..=1.0`. The divider
/// writes it as it is dragged (emitting [`ValueChange<f32>`]); write it to move
/// the divider from code.
#[derive(Component, Clone, Copy, PartialEq, Debug, Reflect)]
#[reflect(Component, Clone)]
pub struct SplitFraction(pub f32);

impl Default for SplitFraction {
    fn default() -> Self {
        SplitFraction(0.5)
    }
}

/// How small each pane may get.
///
/// [`Val::Auto`] means the pane's own content minimum — flexbox enforces it, and
/// the divider discovers it by being stopped (see [`PlumeSplitter`]). Any other
/// `Val` is resolved against the splitter, so `Px`, `Percent` and `Em` all say
/// what you would expect.
#[derive(Component, Clone, Copy, PartialEq, Debug, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct SplitMin {
    /// Smallest the first pane may be.
    pub first: Val,
    /// Smallest the second pane may be.
    pub second: Val,
}

impl Default for SplitMin {
    fn default() -> Self {
        SplitMin {
            first: Val::Auto,
            second: Val::Auto,
        }
    }
}

/// Paint the divider only while it is in use.
#[derive(Component, Clone, Copy, PartialEq, Eq, Default, Debug, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct SplitDividerAutoHide(pub bool);

/// Opt-in, per pane: dragging the divider well past the pane's floor snaps it
/// fully closed ([`SplitFraction`] exactly `0.0` or `1.0`).
#[derive(Component, Clone, Copy, PartialEq, Eq, Default, Debug, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct SplitCollapsible {
    /// The first pane may collapse, closing to the left/top edge.
    pub first: bool,
    /// The second pane may collapse, closing to the right/bottom edge.
    pub second: bool,
}

/// Two panes with a divider the user drags to re-proportion them.
///
/// The first pane is sized from [`SplitFraction`]; the second takes what is
/// left, so the two always fill the splitter. [`SplitMin`] gives each a floor.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeSplitterProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeSplitter;

/// Props used to construct the [`PlumeSplitter`] scene.
pub struct PlumeSplitterProps {
    /// Which way the panes are arranged.
    pub axis: SplitAxis,
    /// The first pane's starting share, `0.0..=1.0`.
    pub fraction: f32,
    /// Smallest the first pane may be; [`Val::Auto`] for its content minimum.
    pub min_first: Val,
    /// Smallest the second pane may be; [`Val::Auto`] for its content minimum.
    pub min_second: Val,
    /// Paint the divider only while hovered or dragged.
    pub auto_hide: bool,
    /// Let a drag snap the first pane fully closed (see [`SplitCollapsible`]).
    pub collapsible_first: bool,
    /// Let a drag snap the second pane fully closed (see [`SplitCollapsible`]).
    pub collapsible_second: bool,
    /// Contents of the first pane — left, or top.
    pub first: Box<dyn SceneList>,
    /// Contents of the second pane — right, or bottom.
    pub second: Box<dyn SceneList>,
}

impl Default for PlumeSplitterProps {
    fn default() -> Self {
        Self {
            axis: SplitAxis::default(),
            fraction: 0.5,
            min_first: Val::Auto,
            min_second: Val::Auto,
            auto_hide: false,
            collapsible_first: false,
            collapsible_second: false,
            first: Box::new(bsn_list![]),
            second: Box::new(bsn_list![]),
        }
    }
}

impl PlumeSplitter {
    fn scene(props: PlumeSplitterProps) -> impl Scene {
        let PlumeSplitterProps {
            axis,
            fraction,
            min_first,
            min_second,
            auto_hide,
            collapsible_first,
            collapsible_second,
            first,
            second,
        } = props;
        bsn! {
            splitter_frame(axis, fraction)
            template_value(SplitMin { first: min_first, second: min_second })
            template_value(SplitDividerAutoHide(auto_hide))
            template_value(SplitCollapsible { first: collapsible_first, second: collapsible_second })
            Children [
                (
                    splitter_pane(SplitPane::First)
                    Children [
                        {first},
                    ]
                ),
                splitter_divider(axis),
                (
                    splitter_pane(SplitPane::Second)
                    Children [
                        {second},
                    ]
                ),
            ]
        }
    }
}

// The splitter's root, as a plain component. `PlumeSplitter` is a scene
// component — spawning it bare is an error, and the scene machinery adds it —
// so the systems below key off this instead.
#[derive(Component, Clone, Copy, Default, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct SplitterRoot;

// Which pane of its splitter a node is.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Reflect)]
#[reflect(Component, Clone)]
pub(crate) enum SplitPane {
    First,
    Second,
}

// The splitter's own node, with no panes in it. Split out from the scene above
// so the immediate layer can build the same structure a piece at a time — its
// panes hold imm children, which a prop taking a finished `SceneList` cannot.
pub(crate) fn splitter_frame(axis: SplitAxis, fraction: f32) -> impl Scene {
    let direction = match axis {
        SplitAxis::Horizontal => FlexDirection::Row,
        SplitAxis::Vertical => FlexDirection::Column,
    };
    bsn! {
        Node {
            flex_direction: direction,
            align_items: AlignItems::Stretch,
            // Flex's `auto` minimum refuses to shrink below content, so a scroll
            // area anywhere in a pane only bounds once the splitter can give too.
            min_width: Val::ZERO,
            min_height: Val::ZERO,
        }
        SplitterRoot
        template_value(axis)
        template_value(SplitFraction(fraction))
        SplitMin
        // `Val::Em` minimums need the chain's `EmSize` to resolve.
        TextStyleRelay
    }
}

// One pane. The first is sized by `apply_split` and must not shrink below its
// share to make room for the other's content — the fraction is the authority,
// and `min_*` the only override; the second takes whatever is left.
pub(crate) fn splitter_pane(pane: SplitPane) -> impl Scene {
    let (grow, shrink, basis) = match pane {
        SplitPane::First => (0.0, 0.0, Val::Auto),
        SplitPane::Second => (1.0, 1.0, Val::ZERO),
    };
    bsn! {
        Node {
            flex_grow: grow,
            flex_shrink: shrink,
            flex_basis: basis,
            // The pane's own axis is floored by `apply_split` from `SplitMin`;
            // this is the cross axis, which must give so the pane can't be
            // propped open by content taller (or wider) than the splitter.
            min_width: Val::ZERO,
            min_height: Val::ZERO,
        }
        template_value(pane)
        TextStyleRelay
        // A pane is pure layout, like `screen`, so it lets picks fall through
        // its empty parts — whatever the app puts in it does its own blocking.
        // It has to default this way round: the app can reach the splitter to
        // say `.pickable()`, but never the panes inside it, so a pane that
        // blocked would be a pane nothing could stop blocking. Content over a
        // 3d viewport is the case that needs it.
        Pickable::IGNORE
    }
}

// Marks the grab strip between the panes. `drag_share` is the pointer's
// position in share space during a drag.
#[derive(Component, Clone, Copy, Default, Reflect)]
#[reflect(Component, Clone, Default)]
struct SplitDivider {
    drag_share: f32,
}

// The grab strip, with the hairline centred inside it. The strip is wider than
// the line it draws, so the divider can be grabbed without being a gutter: it
// is positioned *over* the seam rather than taking a place in the flow, which
// leaves the two panes flush and their shares summing to the whole. In the flow
// it would both push the panes apart by its own width and overflow the splitter
// once the fraction left less room than it takes.
pub(crate) fn splitter_divider(axis: SplitAxis) -> impl Scene {
    let cursor = axis.cursor();
    // Along the axis: the grab width, centred on the seam by `apply_split`.
    // Across it: pinned to both edges, so the strip spans the splitter.
    let (width, height) = match axis {
        SplitAxis::Horizontal => (DIVIDER_GRAB, Val::Auto),
        SplitAxis::Vertical => (Val::Auto, DIVIDER_GRAB),
    };
    let (top, bottom, left, right) = match axis {
        SplitAxis::Horizontal => (Val::ZERO, Val::ZERO, Val::Auto, Val::Auto),
        SplitAxis::Vertical => (Val::Auto, Val::Auto, Val::ZERO, Val::ZERO),
    };
    let (line_width, line_height) = match axis {
        SplitAxis::Horizontal => (size::HAIRLINE, Val::Auto),
        SplitAxis::Vertical => (Val::Auto, size::HAIRLINE),
    };
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            width: width,
            height: height,
            top: top,
            bottom: bottom,
            left: left,
            right: right,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        SplitDivider
        Hovered
        TextStyleRelay
        EntityCursor::System(cursor)
        CursorLock
        on(drag_divider)
        on(drag_start_divider)
        on(press_divider)
        on(release_divider)
        on(drag_end_divider)
        on(cancel_divider)
        Children [
            (
                Node {
                    width: line_width,
                    height: line_height,
                    align_self: AlignSelf::Stretch,
                }
                TextStyleRelay
                Pickable::IGNORE
                ThemeBackgroundToken(tokens::SEPARATOR)
            ),
        ]
    }
}

// Pushes the model onto the panes: the first pane's length is the fraction,
// each pane's floor is its `SplitMin`, and the divider centres on the seam
// between them. The second pane grows into the rest, so only the first is sized.
fn apply_split(
    q_splitters: Query<
        (
            &Children,
            &SplitAxis,
            &SplitFraction,
            &SplitMin,
            Option<&SplitCollapsible>,
        ),
        Or<(
            Added<SplitterRoot>,
            Changed<SplitFraction>,
            Changed<SplitMin>,
            Changed<SplitCollapsible>,
            Changed<Children>,
        )>,
    >,
    mut q_panes: Query<(&SplitPane, &mut Node)>,
    mut q_dividers: Query<&mut Node, (With<SplitDivider>, Without<SplitPane>)>,
) {
    for (children, axis, fraction, min, collapsible) in q_splitters.iter() {
        let f = fraction.0.clamp(0.0, 1.0);
        let share = Val::Percent(f * 100.0);
        let collapse = collapsible.copied().unwrap_or_default();
        for child in children.iter() {
            if let Ok((pane, mut node)) = q_panes.get_mut(*child) {
                // A collapsed pane must actually reach zero: its floor is
                // lifted and whatever contents remain are clipped.
                let collapsed = match pane {
                    SplitPane::First => collapse.first && f == 0.0,
                    SplitPane::Second => collapse.second && f == 1.0,
                };
                let floor = match (collapsed, pane) {
                    (true, _) => Val::ZERO,
                    (false, SplitPane::First) => min.first,
                    (false, SplitPane::Second) => min.second,
                };
                let overflow = if collapsed {
                    Overflow::clip()
                } else {
                    Overflow::visible()
                };
                if node.overflow != overflow {
                    node.overflow = overflow;
                }
                match axis {
                    SplitAxis::Horizontal => {
                        // Only the first pane is sized; the second grows.
                        if *pane == SplitPane::First && node.width != share {
                            node.width = share;
                        }
                        if node.min_width != floor {
                            node.min_width = floor;
                        }
                    }
                    SplitAxis::Vertical => {
                        if *pane == SplitPane::First && node.height != share {
                            node.height = share;
                        }
                        if node.min_height != floor {
                            node.min_height = floor;
                        }
                    }
                }
            }

            // Splitter position (accounting for edges)
            if let Ok(mut node) = q_dividers.get_mut(*child) {
                let inset = if f == 0.0 {
                    Val::ZERO
                } else if f == 1.0 {
                    -DIVIDER_GRAB
                } else {
                    -(DIVIDER_GRAB / 2.0)
                };
                match axis {
                    SplitAxis::Horizontal => {
                        if node.left != share || node.margin.left != inset {
                            node.left = share;
                            node.margin.left = inset;
                        }
                    }
                    SplitAxis::Vertical => {
                        if node.top != share || node.margin.top != inset {
                            node.top = share;
                            node.margin.top = inset;
                        }
                    }
                }
            }
        }
    }
}

// The divider has no headless widget behind it, so it keeps its own `Pressed`:
// press marks the gesture, release/drag-end/cancel end it. While it is on, the
// active style and the resize cursor hold even when the pointer outruns the
// strip mid-drag.
fn press_divider(press: On<Pointer<Press>>, mut commands: Commands) {
    commands.entity(press.event_target()).insert(Pressed);
}

fn release_divider(release: On<Pointer<Release>>, mut commands: Commands) {
    commands.entity(release.event_target()).remove::<Pressed>();
}

fn drag_end_divider(drag_end: On<Pointer<DragEnd>>, mut commands: Commands) {
    commands.entity(drag_end.event_target()).remove::<Pressed>();
}

fn cancel_divider(cancel: On<Pointer<Cancel>>, mut commands: Commands) {
    commands.entity(cancel.event_target()).remove::<Pressed>();
}

// A fresh gesture starts from the stored fraction, not whatever share the last
// drag's pointer ran ahead to.
fn drag_start_divider(
    drag_start: On<Pointer<DragStart>>,
    mut q_dividers: Query<&mut SplitDivider>,
    q_child_of: Query<&ChildOf>,
    q_fractions: Query<&SplitFraction>,
) {
    let Ok(mut divider) = q_dividers.get_mut(drag_start.event_target()) else {
        return;
    };
    let Ok(parent) = q_child_of.get(drag_start.event_target()) else {
        return;
    };
    if let Ok(fraction) = q_fractions.get(parent.parent()) {
        divider.drag_share = fraction.0;
    }
}

// Drag the divider: the pointer's travel along the axis, as a share of the
// splitter's length, clamped to what the two minimums leave. A collapsible
// pane adds two zones past its floor: down to half the floor the divider holds
// at the floor, further and it snaps fully closed — and the same halfway
// point, crossed outward, is where a closed pane snaps back to its floor.
fn drag_divider(
    mut drag: On<Pointer<Drag>>,
    mut q_dividers: Query<&mut SplitDivider>,
    q_child_of: Query<&ChildOf>,
    mut q_splitters: Query<(
        &SplitAxis,
        &mut SplitFraction,
        &SplitMin,
        Option<&SplitCollapsible>,
        &ComputedNode,
        &ComputedUiRenderTargetInfo,
        Option<&EmSize>,
    )>,
    rem_size: Res<RemSize>,
    mut commands: Commands,
) {
    let Ok(mut divider) = q_dividers.get_mut(drag.event_target()) else {
        return;
    };
    let Ok(parent) = q_child_of.get(drag.event_target()) else {
        return;
    };
    let splitter = parent.parent();
    let Ok((axis, mut fraction, min, collapsible, node, target, em_size)) =
        q_splitters.get_mut(splitter)
    else {
        return;
    };
    // The splitter owns this gesture; a drag on the divider must never also pan
    // or move whatever the splitter happens to sit in.
    drag.propagate(false);

    let length = axis.of(node.size());
    if length <= 0.0 {
        return;
    }
    // Both in the pointer's own space: the node measures physical, the drag
    // arrives logical.
    let travel = axis.of(drag.delta) / (length * node.inverse_scale_factor());

    // Minimums resolve to physical px, so dividing by the physical length gives
    // the share each one reserves. `Val::Auto` does not resolve — that floor is
    // the pane's content minimum, which only the layout knows; `snap_split_to_layout`
    // corrects the fraction once it has run.
    let share_of = |floor: Val| {
        floor
            .resolve(
                target.scale_factor(),
                length,
                target.physical_size().as_vec2(),
                em_size.copied().unwrap_or_default(),
                *rem_size,
            )
            .map_or(0.0, |px| px / length)
    };
    let low = share_of(min.first);
    let high = 1.0 - share_of(min.second);
    let collapse = collapsible.copied().unwrap_or_default();

    // Within the floors the gesture is based on the live fraction, which is
    // what keeps the drag free of dead travel; past a collapsible floor it is
    // based on the pointer's own accumulated share, which the hysteresis needs.
    let base = if (collapse.first && divider.drag_share < low)
        || (collapse.second && divider.drag_share > high)
    {
        divider.drag_share
    } else {
        fraction.0
    };
    let pointer = base + travel;
    divider.drag_share = pointer;
    // An over-constrained splitter cannot honour both floors; the first pane
    // keeps its own and the second gives, which is what flexbox does anyway.
    let wanted = if collapse.first && pointer < low * 0.5 {
        0.0
    } else if collapse.second && pointer > (high + 1.0) * 0.5 {
        1.0
    } else if collapse.first && pointer < low {
        low
    } else if collapse.second && pointer > high {
        high.max(low)
    } else {
        pointer.clamp(low, high.max(low))
    };

    fraction.set_if_neq(SplitFraction(wanted));
    commands.trigger(ValueChange {
        source: splitter,
        value: wanted,
        is_final: false,
    });
}

// Adopts what the layout actually did, so the stored fraction never drifts from
// what is on screen. That happens whenever a floor the drag could not resolve
// stopped a pane — a `Val::Auto` minimum, or content that refuses to shrink —
// and without it the divider would develop dead travel: the fraction would keep
// moving while the pane could not, and dragging back would do nothing until the
// slack unwound. Writing the measured value back converges in one frame, since
// asking for exactly what the layout already gives changes nothing.
fn snap_split_to_layout(
    mut q_splitters: Query<(
        Entity,
        &Children,
        &SplitAxis,
        &mut SplitFraction,
        &ComputedNode,
    )>,
    q_panes: Query<(&SplitPane, &ComputedNode)>,
    mut commands: Commands,
) {
    for (splitter, children, axis, mut fraction, node) in q_splitters.iter_mut() {
        let length = axis.of(node.size());
        if length <= 0.0 {
            continue;
        }
        let Some(measured) = children.iter().find_map(|child| {
            q_panes
                .get(*child)
                .ok()
                .filter(|(pane, _)| **pane == SplitPane::First)
                .map(|(_, pane_node)| axis.of(pane_node.size()))
        }) else {
            continue;
        };
        let actual = measured / length;
        if (actual - fraction.0).abs() * length <= SNAP_EPSILON_PX {
            continue;
        }
        fraction.0 = actual;
        commands.trigger(ValueChange {
            source: splitter,
            value: actual,
            is_final: false,
        });
    }
}

// The hairline brightens and thickens under the pointer and while dragging: the
// divider is chrome you have to find before you can use it. Borrows the
// scrollbar thumb's tokens, the theme's existing voice for a draggable neutral
// strip.
fn style_divider(
    children: &Children,
    axis: SplitAxis,
    auto_hide: bool,
    pressed: bool,
    hovered: bool,
    q_lines: &mut Query<&mut Node, Without<SplitDivider>>,
    commands: &mut Commands,
) {
    let active = pressed || hovered;
    let token = if active {
        tokens::sets::SCROLLBAR_THUMB.pick(false, pressed, hovered)
    } else if auto_hide {
        tokens::SEPARATOR_HIDDEN
    } else {
        tokens::SEPARATOR
    };
    let line = if active {
        DIVIDER_LINE_ACTIVE
    } else {
        size::HAIRLINE
    };
    for child in children.iter() {
        if let Ok(mut node) = q_lines.get_mut(*child) {
            match axis {
                SplitAxis::Horizontal => {
                    if node.width != line {
                        node.width = line;
                    }
                }
                SplitAxis::Vertical => {
                    if node.height != line {
                        node.height = line;
                    }
                }
            }
        }
        commands
            .entity(*child)
            .insert(ThemeBackgroundToken(token.clone()));
    }
}

fn update_divider_styles(
    q_dividers: Query<
        (Entity, &Children, Has<Pressed>, &Hovered),
        (
            With<SplitDivider>,
            Or<(Added<SplitDivider>, Added<Pressed>, Changed<Hovered>)>,
        ),
    >,
    q_child_of: Query<&ChildOf>,
    q_splitters: Query<(&SplitAxis, Option<&SplitDividerAutoHide>), With<SplitterRoot>>,
    mut q_lines: Query<&mut Node, Without<SplitDivider>>,
    mut commands: Commands,
) {
    for (divider, children, pressed, hovered) in q_dividers.iter() {
        let Ok(child_of) = q_child_of.get(divider) else {
            continue;
        };
        let Ok((axis, auto_hide)) = q_splitters.get(child_of.parent()) else {
            continue;
        };
        style_divider(
            children,
            *axis,
            auto_hide.is_some_and(|hide| hide.0),
            pressed,
            hovered.get(),
            &mut q_lines,
            &mut commands,
        );
    }
}

// Releasing away from the strip removes `Pressed` without a `Hovered` change,
// which only removal detection sees.
fn update_divider_styles_remove(
    q_dividers: Query<(&Children, Has<Pressed>, &Hovered), With<SplitDivider>>,
    mut removed_pressed: RemovedComponents<Pressed>,
    q_child_of: Query<&ChildOf>,
    q_splitters: Query<(&SplitAxis, Option<&SplitDividerAutoHide>), With<SplitterRoot>>,
    mut q_lines: Query<&mut Node, Without<SplitDivider>>,
    mut commands: Commands,
) {
    for divider in removed_pressed.read() {
        let Ok((children, pressed, hovered)) = q_dividers.get(divider) else {
            continue;
        };
        let Ok(child_of) = q_child_of.get(divider) else {
            continue;
        };
        let Ok((axis, auto_hide)) = q_splitters.get(child_of.parent()) else {
            continue;
        };
        style_divider(
            children,
            *axis,
            auto_hide.is_some_and(|hide| hide.0),
            pressed,
            hovered.get(),
            &mut q_lines,
            &mut commands,
        );
    }
}

// The flag lives on the splitter root, out of the divider-driven triggers' sight.
fn update_divider_styles_auto_hide(
    q_splitters: Query<
        (&Children, &SplitAxis, &SplitDividerAutoHide),
        (With<SplitterRoot>, Changed<SplitDividerAutoHide>),
    >,
    q_dividers: Query<(&Children, Has<Pressed>, &Hovered), With<SplitDivider>>,
    mut q_lines: Query<&mut Node, Without<SplitDivider>>,
    mut commands: Commands,
) {
    for (children, axis, auto_hide) in q_splitters.iter() {
        for child in children.iter() {
            let Ok((divider_children, pressed, hovered)) = q_dividers.get(*child) else {
                continue;
            };
            style_divider(
                divider_children,
                *axis,
                auto_hide.0,
                pressed,
                hovered.get(),
                &mut q_lines,
                &mut commands,
            );
        }
    }
}

// Registers the splitter's layout, drag and style systems.
pub(crate) struct SplitterPlugin;

impl Plugin for SplitterPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            (
                update_divider_styles,
                update_divider_styles_remove,
                update_divider_styles_auto_hide,
            )
                .in_set(PickingSystems::Last),
        )
        .add_systems(PostUpdate, apply_split.before(UiSystems::Layout))
        // After the layout it is measuring.
        .add_systems(PostUpdate, snap_split_to_layout.after(UiSystems::Layout));
    }
}
