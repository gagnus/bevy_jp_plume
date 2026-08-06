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
use bevy::ecs::observer::On;
use bevy::ecs::query::{Added, Changed, Has, Or, With, Without};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::math::Vec2;
use bevy::picking::events::{Drag, Pointer};
use bevy::picking::hover::Hovered;
use bevy::picking::{Pickable, PickingSystems};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::text::{EmSize, RemSize};
use bevy::ui::{
    AlignItems, AlignSelf, ComputedNode, ComputedUiRenderTargetInfo, FlexDirection, JustifyContent,
    Node, PositionType, Pressed, UiSystems, Val,
};
use bevy::ui_widgets::ValueChange;
use bevy::window::SystemCursorIcon;

use crate::constants::size;
use crate::cursor::EntityCursor;
use crate::font_styles::TextStyleRelay;
use crate::theme::ThemeBackgroundToken;
use crate::tokens;

// Grab width of the divider. The visible line is a hairline; this is the strip
// the pointer has to land in, which has to be usable without being a gutter.
const DIVIDER_GRAB: Val = size::em_from_px(7.0);

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

/// Two panes with a divider the user drags to re-proportion them.
///
/// The first pane is sized from [`SplitFraction`]; the second takes what is
/// left, so the two always fill the splitter. [`SplitMin`] gives each a floor.
///
/// A floor stated as a length is enforced twice over: the divider clamps
/// against it while dragging, and the pane's own `min_width`/`min_height` stops
/// the layout regardless. A `Val::Auto` floor can only be enforced the second
/// way — a content minimum is not known until the layout has run — so the
/// stored fraction is corrected from the layout afterwards instead.
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
            first: Box::new(bsn_list!()),
            second: Box::new(bsn_list!()),
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
            first,
            second,
        } = props;
        bsn! {
            splitter_frame(axis, fraction)
            template_value(SplitMin { first: min_first, second: min_second })
            Children [
                (splitter_pane(SplitPane::First) Children [ {first} ]),
                splitter_divider(axis),
                (splitter_pane(SplitPane::Second) Children [ {second} ]),
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
            flex_direction: {direction},
            align_items: AlignItems::Stretch,
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
            flex_grow: {grow},
            flex_shrink: {shrink},
            flex_basis: {basis},
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

// Marks the grab strip between the panes.
#[derive(Component, Clone, Copy, Default, Reflect)]
#[reflect(Component, Clone, Default)]
struct SplitDivider;

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
        SplitAxis::Horizontal => (size::CONTAINER_BORDER, Val::Auto),
        SplitAxis::Vertical => (Val::Auto, size::CONTAINER_BORDER),
    };
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            width: {width},
            height: {height},
            top: {top},
            bottom: {bottom},
            left: {left},
            right: {right},
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        SplitDivider
        Hovered
        TextStyleRelay
        EntityCursor::System({cursor})
        on(drag_divider)
        Children [(
            Node {
                width: {line_width},
                height: {line_height},
                align_self: AlignSelf::Stretch,
            }
            TextStyleRelay
            Pickable::IGNORE
            ThemeBackgroundToken(tokens::SEPARATOR)
        )]
    }
}

// Pushes the model onto the panes: the first pane's length is the fraction,
// each pane's floor is its `SplitMin`, and the divider centres on the seam
// between them. The second pane grows into the rest, so only the first is sized.
fn apply_split(
    q_splitters: Query<
        (&Children, &SplitAxis, &SplitFraction, &SplitMin),
        Or<(
            Added<SplitterRoot>,
            Changed<SplitFraction>,
            Changed<SplitMin>,
            Changed<Children>,
        )>,
    >,
    mut q_panes: Query<(&SplitPane, &mut Node)>,
    mut q_dividers: Query<&mut Node, (With<SplitDivider>, Without<SplitPane>)>,
) {
    for (children, axis, fraction, min) in q_splitters.iter() {
        let share = Val::Percent(fraction.0.clamp(0.0, 1.0) * 100.0);
        for child in children.iter() {
            if let Ok((pane, mut node)) = q_panes.get_mut(*child) {
                let floor = match pane {
                    SplitPane::First => min.first,
                    SplitPane::Second => min.second,
                };
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
            // Half its own width back, so the grab strip straddles the seam
            // rather than starting at it.
            if let Ok(mut node) = q_dividers.get_mut(*child) {
                let inset = -(DIVIDER_GRAB / 2.0);
                match axis {
                    SplitAxis::Horizontal => {
                        if node.left != share {
                            node.left = share;
                            node.margin.left = inset;
                        }
                    }
                    SplitAxis::Vertical => {
                        if node.top != share {
                            node.top = share;
                            node.margin.top = inset;
                        }
                    }
                }
            }
        }
    }
}

// Drag the divider: the pointer's travel along the axis, as a share of the
// splitter's length, clamped to what the two minimums leave.
fn drag_divider(
    mut drag: On<Pointer<Drag>>,
    q_child_of: Query<&ChildOf>,
    mut q_splitters: Query<(
        &SplitAxis,
        &mut SplitFraction,
        &SplitMin,
        &ComputedNode,
        &ComputedUiRenderTargetInfo,
        Option<&EmSize>,
    )>,
    rem_size: Res<RemSize>,
    mut commands: Commands,
) {
    let Ok(parent) = q_child_of.get(drag.event_target()) else {
        return;
    };
    let splitter = parent.parent();
    let Ok((axis, mut fraction, min, node, target, em_size)) = q_splitters.get_mut(splitter) else {
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
    // An over-constrained splitter cannot honour both floors; the first pane
    // keeps its own and the second gives, which is what flexbox does anyway.
    let wanted = (fraction.0 + travel).clamp(low, high.max(low));

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

// The hairline brightens under the pointer and while dragging: the divider is
// chrome you have to find before you can use it. Borrows the scrollbar thumb's
// tokens, the theme's existing voice for a draggable neutral strip.
fn update_divider_styles(
    q_dividers: Query<
        (&Children, Has<Pressed>, &Hovered),
        (
            With<SplitDivider>,
            Or<(Added<SplitDivider>, Added<Pressed>, Changed<Hovered>)>,
        ),
    >,
    mut commands: Commands,
) {
    for (children, pressed, hovered) in q_dividers.iter() {
        let token = if pressed || hovered.get() {
            tokens::sets::SCROLLBAR_THUMB.pick(false, pressed, hovered.get())
        } else {
            tokens::SEPARATOR
        };
        for child in children.iter() {
            commands
                .entity(*child)
                .insert(ThemeBackgroundToken(token.clone()));
        }
    }
}

// Registers the splitter's layout, drag and style systems.
pub(crate) struct SplitterPlugin;

impl Plugin for SplitterPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            update_divider_styles.in_set(PickingSystems::Last),
        )
        .add_systems(PostUpdate, apply_split.before(UiSystems::Layout))
        // After the layout it is measuring.
        .add_systems(PostUpdate, snap_split_to_layout.after(UiSystems::Layout));
    }
}
