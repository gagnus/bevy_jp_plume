//! Two panes with a draggable divider between them. One pane owns the stored
//! size; the other flexes into the rest, absorbing resizes of the splitter.
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
use bevy::picking::events::{
    PointerCancel, PointerDrag, PointerDragEnd, PointerDragStart, PointerPress, PointerRelease,
};
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
use crate::theme::ThemeBackgroundToken;
use crate::tokens;

// Grab width of the divider. The visible line is a hairline; this is the strip
// the pointer has to land in, which has to be usable without being a gutter.
const DIVIDER_GRAB: Val = size::em_from_px(7.0);

// The hairline while hovered or dragged: thick enough to read as grabbable. It
// grows inside the grab strip, so neither pane moves.
const DIVIDER_LINE_ACTIVE: Val = size::em_from_px(3.0);

// A seam this far (physical px) from where the stored size puts it is taken as
// the layout having stopped a pane — see `snap_split_to_layout`. Half a pixel
// is below anything the eye or the next drag can tell apart.
const SNAP_EPSILON_PX: f32 = 0.5;

// Where between its floor and the edge a collapsible pane snaps closed (and a
// closed one reopens): the pointer's distance from the edge, as a share of the
// floor's. Lower means the pointer must get closer to the edge.
const COLLAPSE_POINT: f32 = 0.25;

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

/// One of a [`PlumeSplitter`]'s two panes.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Default, Debug, Reflect)]
#[reflect(Component, Clone, Default)]
pub enum SplitPane {
    /// Left, or top.
    #[default]
    First,
    /// Right, or bottom.
    Second,
}

impl SplitPane {
    fn other(self) -> Self {
        match self {
            SplitPane::First => SplitPane::Second,
            SplitPane::Second => SplitPane::First,
        }
    }
}

/// The sized pane's length, and whether a drag has snapped a pane closed.
///
/// The unit is kept as given: `Percent` re-proportions with the splitter, while
/// `Px`/`Em`/`Rem` anchor the sized pane so only the flexing pane absorbs
/// resizes. The divider writes this as it is dragged (emitting
/// [`ValueChange<SplitSize>`]); write it to move the divider from code.
#[derive(Component, Clone, Copy, PartialEq, Debug, Reflect)]
#[reflect(Component, Clone)]
pub struct SplitSize {
    /// The sized pane's main-axis length (see [`SplitSized`]).
    pub size: Val,
    /// The pane a drag has snapped closed, if any (see [`SplitCollapsible`]).
    /// While closed, `size` keeps its last open value.
    pub closed: Option<SplitPane>,
}

impl SplitSize {
    /// An open splitter with the sized pane at `size`.
    pub fn new(size: Val) -> Self {
        Self { size, closed: None }
    }
}

impl Default for SplitSize {
    fn default() -> Self {
        Self::new(Val::Percent(50.0))
    }
}

/// Which pane [`SplitSize`] describes. The other pane flexes into whatever is
/// left, so it alone gives and takes when the splitter itself resizes.
#[derive(Component, Clone, Copy, PartialEq, Eq, Default, Debug, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct SplitSized(pub SplitPane);

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
/// fully closed ([`SplitSize::closed`]).
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
/// The sized pane's length is [`SplitSize`]; the other flexes into what is
/// left, so the two always fill the splitter. [`SplitMin`] gives each a floor;
/// a splitter too small for both floors squeezes the sized pane down to its own.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeSplitterProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeSplitter;

/// Props used to construct the [`PlumeSplitter`] scene.
pub struct PlumeSplitterProps {
    /// Which way the panes are arranged.
    pub axis: SplitAxis,
    /// The sized pane's starting length (see [`SplitSize`]).
    pub size: Val,
    /// Which pane `size` describes; the other flexes (see [`SplitSized`]).
    pub sized_pane: SplitPane,
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
            size: Val::Percent(50.0),
            sized_pane: SplitPane::First,
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
            size,
            sized_pane,
            min_first,
            min_second,
            auto_hide,
            collapsible_first,
            collapsible_second,
            first,
            second,
        } = props;
        bsn! {
            splitter_frame(axis, SplitSize::new(size), sized_pane)
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

// The splitter's own node, with no panes in it. Split out from the scene above
// so the immediate layer can build the same structure a piece at a time — its
// panes hold imm children, which a prop taking a finished `SceneList` cannot.
pub(crate) fn splitter_frame(
    axis: SplitAxis,
    size: SplitSize,
    sized_pane: SplitPane,
) -> impl Scene {
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
        template_value(size)
        template_value(SplitSized(sized_pane))
        SplitMin
        // `Pickable` is per-entity, so the frame is its own hit over the same area as
        // the panes and must ignore picks too, or theirs buys nothing. The divider
        // sets none of its own and is unaffected, so drags survive.
        Pickable::IGNORE
    }
}

// One pane. Its flex behavior is written by `apply_split` from which pane is
// sized; here only the cross-axis floors, which must give so the pane can't be
// propped open by content taller (or wider) than the splitter.
pub(crate) fn splitter_pane(pane: SplitPane) -> impl Scene {
    bsn! {
        Node {
            min_width: Val::ZERO,
            min_height: Val::ZERO,
        }
        template_value(pane)
        // A pane is pure layout, like `screen`, so it lets picks fall through its empty
        // parts — content over a 3d viewport is the case that needs it.
        Pickable::IGNORE
    }
}

// Marks the grab strip between the panes. `drag_px` is the pointer's position
// during a drag: the seam it asks for, in physical px from the start edge.
#[derive(Component, Clone, Copy, Default, Reflect)]
#[reflect(Component, Clone, Default)]
struct SplitDivider {
    drag_px: f32,
}

// The grab strip, with the hairline centred inside it. Wider than the line it draws,
// so it can be grabbed without being a gutter: positioned *over* the seam rather than
// in the flow, leaving the panes flush and their shares summing to the whole.
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
                Pickable::IGNORE
                ThemeBackgroundToken(tokens::SEPARATOR)
            ),
        ]
    }
}

// A physical-px length converted back into `unit`'s own unit, so a drag hands
// the stored `Val` a new number without changing what kind of `Val` it is.
// `Val::Auto` (unresolvable) falls back to a share of the splitter.
fn px_as_unit(
    px: f32,
    unit: Val,
    scale_factor: f32,
    length: f32,
    viewport: Vec2,
    em_size: EmSize,
    rem_size: RemSize,
) -> Val {
    let percent = |px: f32| {
        Val::Percent(if length > 0.0 {
            px / length * 100.0
        } else {
            0.0
        })
    };
    let in_unit = |make: fn(f32) -> Val| {
        make(1.0)
            .resolve(scale_factor, length, viewport, em_size, rem_size)
            .ok()
            .filter(|px_per_unit| *px_per_unit > 0.0)
            .map_or_else(|| percent(px), |px_per_unit| make(px / px_per_unit))
    };
    match unit {
        Val::Auto | Val::Percent(_) => percent(px),
        Val::Px(_) => in_unit(Val::Px),
        Val::Vw(_) => in_unit(Val::Vw),
        Val::Vh(_) => in_unit(Val::Vh),
        Val::VMin(_) => in_unit(Val::VMin),
        Val::VMax(_) => in_unit(Val::VMax),
        Val::Em(_) => in_unit(Val::Em),
        Val::Rem(_) => in_unit(Val::Rem),
    }
}

// The seam's distance from the start edge in physical px, as the stored state
// asks for it — `None` when the size is `Val::Auto`, which only layout can place.
fn stored_seam_px(
    size: &SplitSize,
    sized: SplitPane,
    scale_factor: f32,
    length: f32,
    viewport: Vec2,
    em_size: EmSize,
    rem_size: RemSize,
) -> Option<f32> {
    match size.closed {
        Some(SplitPane::First) => Some(0.0),
        Some(SplitPane::Second) => Some(length),
        None => size
            .size
            .resolve(scale_factor, length, viewport, em_size, rem_size)
            .ok()
            .map(|sized_px| match sized {
                SplitPane::First => sized_px,
                SplitPane::Second => length - sized_px,
            }),
    }
}

// Pushes the model onto the panes: the sized pane's length is `SplitSize`, the
// other flexes into the rest, and each pane's floor is its `SplitMin`. The
// divider is seeded onto the seam; `position_divider` then tracks the layout.
#[allow(clippy::too_many_arguments)]
fn apply_split(
    q_splitters: Query<
        (
            &Children,
            &SplitAxis,
            &SplitSize,
            &SplitSized,
            &SplitMin,
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            Option<&EmSize>,
        ),
        Or<(
            Added<SplitterRoot>,
            Changed<SplitSize>,
            Changed<SplitSized>,
            Changed<SplitMin>,
            Changed<Children>,
        )>,
    >,
    mut q_panes: Query<(&SplitPane, &mut Node)>,
    mut q_dividers: Query<&mut Node, (With<SplitDivider>, Without<SplitPane>)>,
    rem_size: Res<RemSize>,
) {
    for (children, axis, size, sized, min, root_node, target, em_size) in q_splitters.iter() {
        for child in children.iter() {
            if let Ok((pane, mut node)) = q_panes.get_mut(*child) {
                let is_sized = *pane == sized.0;
                let closed_here = size.closed == Some(*pane);
                let other_closed = size.closed == Some(pane.other());
                let main = match (closed_here, is_sized) {
                    (true, _) => Val::ZERO,
                    (false, true) => size.size,
                    (false, false) => Val::Auto,
                };
                // The survivor of a collapse fills the splitter whichever pane
                // it is; otherwise only the flexing pane grows.
                let grow = if closed_here || (is_sized && !other_closed) {
                    0.0
                } else {
                    1.0
                };
                // The sized pane may shrink: when the splitter is too small for
                // its length plus the other pane's floor, it gives — down to its
                // own floor — and takes its stored length back when room returns.
                let shrink = if closed_here { 0.0 } else { 1.0 };
                let basis = if is_sized { Val::Auto } else { Val::ZERO };
                let floor = match (closed_here, pane) {
                    (true, _) => Val::ZERO,
                    (false, SplitPane::First) => min.first,
                    (false, SplitPane::Second) => min.second,
                };
                // A collapsed pane must actually reach zero: its floor is
                // lifted and whatever contents remain are clipped.
                let overflow = if closed_here {
                    Overflow::clip()
                } else {
                    Overflow::visible()
                };
                if node.flex_grow != grow {
                    node.flex_grow = grow;
                }
                if node.flex_shrink != shrink {
                    node.flex_shrink = shrink;
                }
                if node.flex_basis != basis {
                    node.flex_basis = basis;
                }
                if node.overflow != overflow {
                    node.overflow = overflow;
                }
                match axis {
                    SplitAxis::Horizontal => {
                        if node.width != main {
                            node.width = main;
                        }
                        if node.min_width != floor {
                            node.min_width = floor;
                        }
                    }
                    SplitAxis::Vertical => {
                        if node.height != main {
                            node.height = main;
                        }
                        if node.min_height != floor {
                            node.min_height = floor;
                        }
                    }
                }
            }

            if let Ok(mut node) = q_dividers.get_mut(*child) {
                let seam = match (size.closed, sized.0) {
                    (Some(SplitPane::First), _) => Val::ZERO,
                    (Some(SplitPane::Second), _) => Val::Percent(100.0),
                    (None, SplitPane::First) => size.size,
                    (None, SplitPane::Second) => match size.size {
                        Val::Percent(percent) => Val::Percent(100.0 - percent),
                        // No `Val` can say "the length minus this"; seed from the
                        // last measured length and let `position_divider` correct.
                        sized_size => {
                            let length = axis.of(root_node.size());
                            let sized_px = sized_size
                                .resolve(
                                    target.scale_factor(),
                                    length,
                                    target.physical_size().as_vec2(),
                                    em_size.copied().unwrap_or_default(),
                                    *rem_size,
                                )
                                .unwrap_or(0.0);
                            Val::Px((length - sized_px).max(0.0) * root_node.inverse_scale_factor())
                        }
                    },
                };
                let inset = match size.closed {
                    Some(SplitPane::First) => Val::ZERO,
                    Some(SplitPane::Second) => -DIVIDER_GRAB,
                    None => -(DIVIDER_GRAB / 2.0),
                };
                match axis {
                    SplitAxis::Horizontal => {
                        if node.left != seam || node.margin.left != inset {
                            node.left = seam;
                            node.margin.left = inset;
                        }
                    }
                    SplitAxis::Vertical => {
                        if node.top != seam || node.margin.top != inset {
                            node.top = seam;
                            node.margin.top = inset;
                        }
                    }
                }
            }
        }
    }
}

// Keeps the divider on the seam the layout actually produced, which the seeded
// `Val` cannot know once flexbox has squeezed the sized pane below its stored
// length — a floor met, or a splitter too small for both panes.
fn position_divider(
    q_splitters: Query<
        (
            &Children,
            &SplitAxis,
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            Option<&EmSize>,
        ),
        With<SplitterRoot>,
    >,
    q_panes: Query<(&SplitPane, &ComputedNode)>,
    mut q_dividers: Query<&mut Node, With<SplitDivider>>,
    rem_size: Res<RemSize>,
) {
    for (children, axis, root_node, target, em_size) in q_splitters.iter() {
        let length = axis.of(root_node.size());
        if length <= 0.0 {
            continue;
        }
        let Some(seam) = children.iter().find_map(|child| {
            q_panes
                .get(*child)
                .ok()
                .filter(|(pane, _)| **pane == SplitPane::First)
                .map(|(_, pane_node)| axis.of(pane_node.size()))
        }) else {
            continue;
        };
        for child in children.iter() {
            let Ok(mut node) = q_dividers.get_mut(*child) else {
                continue;
            };
            let stored = match axis {
                SplitAxis::Horizontal => node.left,
                SplitAxis::Vertical => node.top,
            }
            .resolve(
                target.scale_factor(),
                length,
                target.physical_size().as_vec2(),
                em_size.copied().unwrap_or_default(),
                *rem_size,
            )
            .ok();
            // A `Percent` seed that already matches is left alone: it keeps
            // tracking a proportional resize with no measurement lag.
            let stale = stored.is_none_or(|px| (px - seam).abs() > SNAP_EPSILON_PX);
            let inset = if seam <= SNAP_EPSILON_PX {
                Val::ZERO
            } else if seam >= length - SNAP_EPSILON_PX {
                -DIVIDER_GRAB
            } else {
                -(DIVIDER_GRAB / 2.0)
            };
            let seam_val = Val::Px(seam * root_node.inverse_scale_factor());
            match axis {
                SplitAxis::Horizontal => {
                    if stale {
                        node.left = seam_val;
                    }
                    if node.margin.left != inset {
                        node.margin.left = inset;
                    }
                }
                SplitAxis::Vertical => {
                    if stale {
                        node.top = seam_val;
                    }
                    if node.margin.top != inset {
                        node.margin.top = inset;
                    }
                }
            }
        }
    }
}

// The divider has no headless widget behind it, so it keeps its own `Pressed`, which
// holds the active style and cursor when the pointer outruns the strip mid-drag.
fn press_divider(press: On<PointerPress>, mut commands: Commands) {
    commands.entity(press.event_target()).insert(Pressed);
}

fn release_divider(release: On<PointerRelease>, mut commands: Commands) {
    commands.entity(release.event_target()).remove::<Pressed>();
}

fn drag_end_divider(drag_end: On<PointerDragEnd>, mut commands: Commands) {
    commands.entity(drag_end.event_target()).remove::<Pressed>();
}

fn cancel_divider(cancel: On<PointerCancel>, mut commands: Commands) {
    commands.entity(cancel.event_target()).remove::<Pressed>();
}

// A fresh gesture starts from the seam as laid out, not wherever the last
// drag's pointer ran ahead to.
fn drag_start_divider(
    drag_start: On<PointerDragStart>,
    mut q_dividers: Query<&mut SplitDivider>,
    q_child_of: Query<&ChildOf>,
    q_splitters: Query<(&SplitAxis, &Children), With<SplitterRoot>>,
    q_panes: Query<(&SplitPane, &ComputedNode)>,
) {
    let Ok(mut divider) = q_dividers.get_mut(drag_start.event_target()) else {
        return;
    };
    let Ok(parent) = q_child_of.get(drag_start.event_target()) else {
        return;
    };
    let Ok((axis, children)) = q_splitters.get(parent.parent()) else {
        return;
    };
    if let Some(seam) = children.iter().find_map(|child| {
        q_panes
            .get(*child)
            .ok()
            .filter(|(pane, _)| **pane == SplitPane::First)
            .map(|(_, pane_node)| axis.of(pane_node.size()))
    }) {
        divider.drag_px = seam;
    }
}

// Drag the divider: pointer travel along the axis moves the seam, clamped to
// what the two floors leave, and written back in the stored size's own unit.
// Past a collapsible pane's floor, `COLLAPSE_POINT` of that floor is the
// hysteresis point — inward it snaps closed, outward it reopens.
fn drag_divider(
    mut drag: On<PointerDrag>,
    mut q_dividers: Query<&mut SplitDivider>,
    q_child_of: Query<&ChildOf>,
    mut q_splitters: Query<(
        &SplitAxis,
        &SplitSized,
        &mut SplitSize,
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
    let Ok((axis, sized, mut size, min, collapsible, node, target, em_size)) =
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
    let scale_factor = target.scale_factor();
    let viewport = target.physical_size().as_vec2();
    let em = em_size.copied().unwrap_or_default();
    // Both in the pointer's own space: the node measures physical, the drag
    // arrives logical.
    let travel = axis.of(drag.delta) / node.inverse_scale_factor();

    // Floors resolve to physical px; `Val::Auto` does not resolve — that floor
    // is the pane's content minimum, which only the layout knows, and
    // `snap_split_to_layout` adopts it once it has stopped the pane.
    let floor_px = |floor: Val| {
        floor
            .resolve(scale_factor, length, viewport, em, *rem_size)
            .unwrap_or(0.0)
    };
    let low = floor_px(min.first);
    let high = length - floor_px(min.second);
    let collapse = collapsible.copied().unwrap_or_default();

    // Within the floors the gesture is based on the stored seam, which
    // `snap_split_to_layout` keeps in step with the layout mid-drag — that is
    // what keeps the drag free of dead travel. Past a collapsible floor it is
    // based on the pointer's own accumulated position, which the hysteresis needs.
    let base = if (collapse.first && divider.drag_px < low)
        || (collapse.second && divider.drag_px > high)
    {
        divider.drag_px
    } else {
        stored_seam_px(
            &size,
            sized.0,
            scale_factor,
            length,
            viewport,
            em,
            *rem_size,
        )
        .unwrap_or(divider.drag_px)
    };
    let pointer = base + travel;
    divider.drag_px = pointer;

    // An over-constrained splitter cannot honour both floors; the first pane
    // keeps its own and the second gives, which is what flexbox does anyway.
    let (seam, closed) = if collapse.first && pointer < low * COLLAPSE_POINT {
        (0.0, Some(SplitPane::First))
    } else if collapse.second && pointer > length - (length - high) * COLLAPSE_POINT {
        (length, Some(SplitPane::Second))
    } else if collapse.first && pointer < low {
        (low, None)
    } else if collapse.second && pointer > high {
        (high.max(low), None)
    } else {
        (pointer.clamp(low, high.max(low)), None)
    };

    // Closing keeps the stored size, so reopening restores it.
    let new_size = if closed.is_some() {
        size.size
    } else {
        let sized_px = match sized.0 {
            SplitPane::First => seam,
            SplitPane::Second => length - seam,
        };
        px_as_unit(
            sized_px,
            size.size,
            scale_factor,
            length,
            viewport,
            em,
            *rem_size,
        )
    };
    let wanted = SplitSize {
        size: new_size,
        closed,
    };
    size.set_if_neq(wanted);
    commands.trigger(ValueChange {
        source: splitter,
        value: wanted,
        is_final: false,
    });
}

// Adopts what the layout actually did whenever a floor the drag could not resolve
// stopped a pane — a `Val::Auto` minimum, or content that refuses to shrink.
// Without it the stored size keeps moving while the pane cannot, and dragging
// back does nothing until that dead travel unwinds. Gated on the gesture: at
// rest a squeezed pane keeps its stored size, so a too-small window does not
// overwrite the length the pane returns to when room comes back.
fn snap_split_to_layout(
    mut q_splitters: Query<(
        Entity,
        &Children,
        &SplitAxis,
        &SplitSized,
        &mut SplitSize,
        &ComputedNode,
        &ComputedUiRenderTargetInfo,
        Option<&EmSize>,
    )>,
    q_panes: Query<(&SplitPane, &ComputedNode)>,
    q_dividers: Query<Has<Pressed>, With<SplitDivider>>,
    rem_size: Res<RemSize>,
    mut commands: Commands,
) {
    for (splitter, children, axis, sized, mut size, root_node, target, em_size) in
        q_splitters.iter_mut()
    {
        if size.closed.is_some() {
            continue;
        }
        let dragging = children
            .iter()
            .any(|child| q_dividers.get(*child).is_ok_and(|pressed| pressed));
        if !dragging {
            continue;
        }
        let length = axis.of(root_node.size());
        if length <= 0.0 {
            continue;
        }
        let Some(measured_seam) = children.iter().find_map(|child| {
            q_panes
                .get(*child)
                .ok()
                .filter(|(pane, _)| **pane == SplitPane::First)
                .map(|(_, pane_node)| axis.of(pane_node.size()))
        }) else {
            continue;
        };
        let scale_factor = target.scale_factor();
        let viewport = target.physical_size().as_vec2();
        let em = em_size.copied().unwrap_or_default();
        let stored = stored_seam_px(
            &size,
            sized.0,
            scale_factor,
            length,
            viewport,
            em,
            *rem_size,
        );
        if stored.is_some_and(|seam| (seam - measured_seam).abs() <= SNAP_EPSILON_PX) {
            continue;
        }
        let sized_px = match sized.0 {
            SplitPane::First => measured_seam,
            SplitPane::Second => length - measured_seam,
        };
        size.size = px_as_unit(
            sized_px,
            size.size,
            scale_factor,
            length,
            viewport,
            em,
            *rem_size,
        );
        commands.trigger(ValueChange {
            source: splitter,
            value: *size,
            is_final: false,
        });
    }
}

// The hairline brightens and thickens under the pointer and while dragging, on the
// scrollbar thumb's tokens — the theme's voice for a draggable neutral strip.
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
        // After the layout they are measuring.
        .add_systems(
            PostUpdate,
            (snap_split_to_layout, position_divider).after(UiSystems::Layout),
        );
    }
}
