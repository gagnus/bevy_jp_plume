//! 2D value pad: a rectangular field with a draggable reticle, reporting a
//! [`Vec2`] in `0..=1` per axis (`x` left→right, `y` top→bottom). Background is
//! caller-owned — drop a [`BackgroundColor`](bevy::ui::BackgroundColor) or
//! [`BackgroundGradient`](bevy::ui::BackgroundGradient) on it — so the same pad
//! backs a saturation/value color plane or any other two-axis picker.
use bevy::app::{Plugin, PostUpdate};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::observer::On;
use bevy::ecs::query::{Changed, Has, With};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::math::Vec2;
use bevy::picking::Pickable;
use bevy::picking::events::{
    PointerCancel, PointerDrag, PointerDragEnd, PointerDragStart, PointerPress,
};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::ui::{
    AlignSelf, BorderRadius, ComputedNode, ComputedUiRenderTargetInfo, InteractionDisabled, Node,
    PositionType, Pressed, UiGlobalTransform, UiRect, UiScale, Val, Val2, em, percent,
};

use crate::constants::size;
use crate::cursor::{CursorLock, EntityCursor};
use crate::theme::ThemeBorderToken;
use crate::tokens;

// Ring thickness, proportioned to the reticle so it keeps its weight at any font
// size — deliberately not a [`size::HAIRLINE`], which is its own rule.
const RETICLE_BORDER: Val = size::em_from_px(2.0);

/// Props used to construct a [`PlumeXyPad`] scene.
pub struct PlumeXyPadProps {
    /// Reticle size.
    pub reticle_size: Val2,
    /// Reticle border radius for shaping.
    pub reticle_border_radius: BorderRadius,
}

impl Default for PlumeXyPadProps {
    fn default() -> Self {
        Self {
            reticle_size: em(1).into(),
            reticle_border_radius: size::CORNER_RADIUS.into(),
        }
    }
}

/// A 2D value pad. Spawnable as a scene component; size it by inserting a
/// [`Node`](bevy::ui::Node) beside it and give it a background of your choosing.
///
/// Reports [`XyPadValue`] (`0..=1` each axis) and self-updates it on drag, so it
/// works dropped straight into a scene; watch `Changed<XyPadValue>` to react.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
#[scene(PlumeXyPadProps)]
pub struct PlumeXyPad;

/// The pad's current value: `x` left→right, `y` top→bottom, each `0..=1`.
#[derive(Component, Clone, Copy, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct XyPadValue(pub Vec2);

impl Default for XyPadValue {
    fn default() -> Self {
        // Center, so a freshly spawned pad puts its reticle in the middle rather
        // than pinned to the top-left corner.
        Self(Vec2::splat(0.5))
    }
}

/// True while the user is dragging the reticle.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct XyPadDragging(pub bool);

/// Optionally pins an axis to a fixed value, turning the pad into a 1D strip
/// (a value bar). `Some(v)` locks that axis at `v`; `None` leaves it free. A hue
/// bar is `XyPadLock { x: Some(0.5), y: None }` (vertical) or the reverse
/// (horizontal). Absent component means both axes are free.
#[derive(Component, Default, Clone, Copy, Debug, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct XyPadLock {
    /// Fixed value for the horizontal axis, or `None` if free.
    pub x: Option<f32>,
    /// Fixed value for the vertical axis, or `None` if free.
    pub y: Option<f32>,
}

/// Constrains the pad to a ring: pointer input is projected onto the circle of
/// `radius` around the center, so the reticle rides the ring wherever the drag sits.
#[derive(Component, Clone, Copy, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct XyPadRing {
    /// Ring radius in the pad's normalized units (`0.5` touches the edges).
    pub radius: f32,
    /// Width of the annulus around `radius` that accepts a new press or drag;
    /// once a drag is on, it follows the pointer anywhere. `None` (the default)
    /// accepts the whole pad.
    pub hit_width: Option<f32>,
}

impl Default for XyPadRing {
    fn default() -> Self {
        Self {
            radius: 0.5,
            hit_width: None,
        }
    }
}

// Whether the latest press over the pad engaged it; a ring pad's dead zones
// decline the press, and the drag that follows keys off this record rather than
// re-testing its own (already-moved) start position.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct XyPadPressEngaged(bool);

// The stretch child that fills the pad inside its border and carries the pointer
// picks; the reticle is positioned relative to it.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct XyPadInner;

// The draggable reticle.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct XyPadThumb;

// Plain root marker, inserted on both the retained and imm paths. The systems key on
// this — and it carries the pad's requirements — rather than the [`PlumeXyPad`] scene
// component, which only the retained path inserts.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
#[require(XyPadDragging, XyPadPressEngaged)]
struct XyPadFrame;

impl PlumeXyPad {
    fn scene(props: PlumeXyPadProps) -> impl Scene {
        bsn! {
            Node {
                // Small floor so a caller can size the pad down to a thin value
                // bar; the SV plane sizes itself up explicitly.
                min_height: size::em_from_px(16.0),
                min_width: size::em_from_px(16.0),
            }
            XyPadFrame
            XyPadValue
            ThemeBorderToken(tokens::COLOR_SWATCH_BORDER)
            EntityCursor::System(bevy::window::SystemCursorIcon::Crosshair)
            CursorLock
            Children [
                (
                    Node {
                        align_self: AlignSelf::Stretch,
                        flex_grow: 1.0,
                    }
                    XyPadInner
                    Children [
                        (
                            Node {
                                position_type: PositionType::Absolute,
                                left: percent(50),
                                top: percent(50),
                                width: {props.reticle_size.x},
                                height: {props.reticle_size.y},
                                border: RETICLE_BORDER,
                                border_radius: {props.reticle_border_radius},
                                // Half-reticle offsets center the ring on the value position.
                                margin: UiRect {
                                    left: {-props.reticle_size.x / 2.0},
                                    top: {-props.reticle_size.y / 2.0},
                                },
                            }
                            XyPadThumb
                            // A white ring with a dark outline reads on any background.
                            bevy::ui::BorderColor::all(bevy::color::Color::WHITE)
                            bevy::ui::Outline {
                                width: size::HAIRLINE,
                                offset: Val::ZERO,
                                color: bevy::color::Color::BLACK,
                            }
                            // Let picks fall through to the inner pad so the reticle never
                            // eats a drag that starts on top of it.
                            Pickable::IGNORE
                        ),
                    ]
                ),
            ]
        }
    }
}

// Move the reticle to match the value whenever it changes.
fn update_xy_pad_thumb(
    q_pad: Query<(&XyPadValue, &Children), Changed<XyPadValue>>,
    q_children: Query<&Children>,
    mut q_thumb: Query<&mut Node, With<XyPadThumb>>,
) {
    for (value, children) in q_pad.iter() {
        let Some(inner) = children.first() else {
            continue;
        };
        let Ok(inner_children) = q_children.get(*inner) else {
            continue;
        };
        let Some(thumb) = inner_children.first() else {
            continue;
        };
        let Ok(mut node) = q_thumb.get_mut(*thumb) else {
            continue;
        };
        node.left = percent(value.0.x * 100.0);
        node.top = percent(value.0.y * 100.0);
    }
}

// Map the pointer position onto the inner pad's rect, returning the `0..=1` value.
fn value_from_pointer(
    node: &ComputedNode,
    node_target: &ComputedUiRenderTargetInfo,
    transform: &UiGlobalTransform,
    pointer_position: Vec2,
    ui_scale: f32,
) -> Option<Vec2> {
    let pos = node.normalize_point(
        *transform,
        pointer_position * node_target.scale_factor() / ui_scale,
    )?;
    // `normalize_point` is center-origin (-0.5..0.5); shift to 0..1. Unclamped —
    // the caller clamps or ring-projects, keeping an outside drag directional.
    Some(pos + Vec2::splat(0.5))
}

// Constrain `pos` to the pad's shape and store it. A ring pad projects onto its
// circle, a plain pad clamps to the rect so dragging outside pins to the edge;
// a locked axis then holds its fixed value, so a 1D bar only moves along its
// free axis.
fn write_value(
    pad: Entity,
    pos: Vec2,
    q_lock: &Query<&XyPadLock>,
    q_ring: &Query<&XyPadRing>,
    q_value: &mut Query<&mut XyPadValue>,
) {
    let mut value = if let Ok(ring) = q_ring.get(pad) {
        let direction = (pos - Vec2::splat(0.5))
            .try_normalize()
            .unwrap_or(Vec2::NEG_Y);
        Vec2::splat(0.5) + direction * ring.radius
    } else {
        pos.clamp(Vec2::ZERO, Vec2::ONE)
    };
    if let Ok(lock) = q_lock.get(pad) {
        if let Some(x) = lock.x {
            value.x = x;
        }
        if let Some(y) = lock.y {
            value.y = y;
        }
    }
    if let Ok(mut current) = q_value.get_mut(pad)
        && current.0 != value
    {
        current.0 = value;
    }
}

// Whether a press at normalized `pos` engages the pad: a ring pad with a hit
// band only accepts inside its annulus, so the dead zones around the ring let
// the press fall through to whatever is behind.
fn ring_hit(pos: Vec2, ring: Option<&XyPadRing>) -> bool {
    let Some(&XyPadRing {
        radius,
        hit_width: Some(hit_width),
    }) = ring
    else {
        return true;
    };
    let distance = (pos - Vec2::splat(0.5)).length();
    (distance - radius).abs() <= hit_width / 2.0
}

// Write the value for a pointer event whose target is an inner pad.
fn apply_pointer(
    inner: Entity,
    pointer_position: Vec2,
    ui_scale: f32,
    q_inner: &Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &ChildOf,
        ),
        With<XyPadInner>,
    >,
    q_disabled: &Query<Has<InteractionDisabled>, With<XyPadFrame>>,
    q_lock: &Query<&XyPadLock>,
    q_ring: &Query<&XyPadRing>,
    q_value: &mut Query<&mut XyPadValue>,
) {
    let Ok((node, node_target, transform, parent)) = q_inner.get(inner) else {
        return;
    };
    let pad = parent.parent();
    if !matches!(q_disabled.get(pad), Ok(false)) {
        return;
    }
    let Some(pos) = value_from_pointer(node, node_target, transform, pointer_position, ui_scale)
    else {
        return;
    };
    write_value(pad, pos, q_lock, q_ring, q_value);
}

#[allow(clippy::too_many_arguments)]
fn on_pointer_press(
    mut press: On<PointerPress>,
    q_inner: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &ChildOf,
        ),
        With<XyPadInner>,
    >,
    q_disabled: Query<Has<InteractionDisabled>, With<XyPadFrame>>,
    q_lock: Query<&XyPadLock>,
    q_ring: Query<&XyPadRing>,
    mut q_engaged: Query<&mut XyPadPressEngaged>,
    mut q_value: Query<&mut XyPadValue>,
    ui_scale: Res<UiScale>,
) {
    let Ok((node, node_target, transform, parent)) = q_inner.get(press.entity) else {
        return;
    };
    let pad = parent.parent();
    let pos = value_from_pointer(
        node,
        node_target,
        transform,
        press.pointer.position,
        ui_scale.0,
    );
    let engaged = pos.is_some_and(|pos| ring_hit(pos, q_ring.get(pad).ok()));
    if let Ok(mut record) = q_engaged.get_mut(pad) {
        record.0 = engaged;
    }
    if !engaged {
        return;
    }
    press.propagate(false);
    if matches!(q_disabled.get(pad), Ok(false))
        && let Some(pos) = pos
    {
        write_value(pad, pos, &q_lock, &q_ring, &mut q_value);
    }
}

fn on_drag_start(
    mut drag_start: On<PointerDragStart>,
    q_inner: Query<&ChildOf, With<XyPadInner>>,
    q_engaged: Query<&XyPadPressEngaged>,
    mut q_dragging: Query<(&mut XyPadDragging, Has<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if let Ok(parent) = q_inner.get(drag_start.entity)
        && let Ok((mut dragging, disabled)) = q_dragging.get_mut(parent.parent())
    {
        // The initiating press decides engagement — the pointer may already have
        // left a ring's annulus by the time DragStart is delivered.
        if !q_engaged
            .get(parent.parent())
            .is_ok_and(|engaged| engaged.0)
        {
            return;
        }
        drag_start.propagate(false);
        if !disabled {
            dragging.0 = true;
            // `Pressed` on the root pairs with its `CursorLock`, holding the
            // crosshair while the drag pins values from outside the pad.
            commands.entity(parent.parent()).insert(Pressed);
        }
    }
}

fn on_drag(
    mut drag: On<PointerDrag>,
    q_inner: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &ChildOf,
        ),
        With<XyPadInner>,
    >,
    q_disabled: Query<Has<InteractionDisabled>, With<XyPadFrame>>,
    q_dragging: Query<&XyPadDragging>,
    q_lock: Query<&XyPadLock>,
    q_ring: Query<&XyPadRing>,
    mut q_value: Query<&mut XyPadValue>,
    ui_scale: Res<UiScale>,
) {
    if let Ok((.., parent)) = q_inner.get(drag.entity)
        && q_dragging.get(parent.parent()).is_ok_and(|d| d.0)
    {
        drag.propagate(false);
        apply_pointer(
            drag.entity,
            drag.pointer.position,
            ui_scale.0,
            &q_inner,
            &q_disabled,
            &q_lock,
            &q_ring,
            &mut q_value,
        );
    }
}

fn on_drag_end(
    mut drag_end: On<PointerDragEnd>,
    q_inner: Query<&ChildOf, With<XyPadInner>>,
    mut q_dragging: Query<&mut XyPadDragging>,
    mut commands: Commands,
) {
    if let Ok(parent) = q_inner.get(drag_end.entity)
        && let Ok(mut dragging) = q_dragging.get_mut(parent.parent())
    {
        drag_end.propagate(false);
        dragging.0 = false;
        commands.entity(parent.parent()).remove::<Pressed>();
    }
}

fn on_drag_cancel(
    drag_cancel: On<PointerCancel>,
    q_inner: Query<&ChildOf, With<XyPadInner>>,
    mut q_dragging: Query<&mut XyPadDragging>,
    mut commands: Commands,
) {
    if let Ok(parent) = q_inner.get(drag_cancel.entity)
        && let Ok(mut dragging) = q_dragging.get_mut(parent.parent())
    {
        dragging.0 = false;
        commands.entity(parent.parent()).remove::<Pressed>();
    }
}

// Registers the reticle-positioning system and the drag observers.
pub(crate) struct XyPadPlugin;

impl Plugin for XyPadPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(PostUpdate, update_xy_pad_thumb);
        app.add_observer(on_pointer_press)
            .add_observer(on_drag_start)
            .add_observer(on_drag)
            .add_observer(on_drag_end)
            .add_observer(on_drag_cancel);
    }
}
