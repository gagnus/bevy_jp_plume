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
use bevy::picking::events::{Cancel, Drag, DragEnd, DragStart, Pointer, Press};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::ui::{
    AlignSelf, ComputedNode, ComputedUiRenderTargetInfo, InteractionDisabled, Node, PositionType,
    Pressed, UiGlobalTransform, UiRect, UiScale, Val, Val2, percent,
};

use crate::constants::size;
use crate::cursor::{CursorLock, EntityCursor};
use crate::font_styles::TextStyleRelay;
use crate::theme::ThemeBorderToken;
use crate::tokens;

// Ring thickness, proportioned to the reticle so it keeps its weight at any font
// size — deliberately not a [`size::HAIRLINE`], which is its own rule.
const RETICLE_BORDER: Val = size::em_from_px(2.0);

/// Props used to construct a [`PlumeXyPad`] scene.
pub struct PlumeXyPadProps {
    /// reticle size
    pub reticle_size: Val2,
}

impl Default for PlumeXyPadProps {
    fn default() -> Self {
        Self {
            reticle_size: size::em_from_px(12.0).into(),
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
#[require(XyPadDragging)]
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

/// True while the user is dragging the reticle; the imm layer reads it to hold
/// back app-driven value pushes mid-drag.
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

// The stretch child that fills the pad inside its border and carries the pointer
// picks; the reticle is positioned relative to it.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct XyPadInner;

// The draggable reticle.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct XyPadThumb;

impl PlumeXyPad {
    fn scene(props: PlumeXyPadProps) -> impl Scene {
        bsn! {
            Node {
                // Small floor so a caller can size the pad down to a thin value
                // bar; the SV plane sizes itself up explicitly.
                min_height: size::em_from_px(16.0),
                min_width: size::em_from_px(16.0),
                border: size::HAIRLINE,
                border_radius: size::CORNER_RADIUS_SMALL,
                padding: UiRect::all(size::HAIRLINE),
            }
            PlumeXyPad
            XyPadValue
            // Em-sized chrome needs the chain's `EmSize`.
            TextStyleRelay
            ThemeBorderToken(tokens::COLOR_SWATCH_BORDER)
            EntityCursor::System(bevy::window::SystemCursorIcon::Crosshair)
            CursorLock
            Children [
                (
                    Node {
                        align_self: AlignSelf::Stretch,
                        flex_grow: 1.0,
                        border_radius: size::CORNER_RADIUS_SMALL,
                    }
                    XyPadInner
                    TextStyleRelay
                    Children [
                        (
                            Node {
                                position_type: PositionType::Absolute,
                                left: percent(50),
                                top: percent(50),
                                width: {props.reticle_size.x},
                                height: {props.reticle_size.y},
                                border: RETICLE_BORDER,
                                border_radius: size::CORNER_RADIUS,
                                // Half-reticle offsets center the ring on the value position.
                                margin: UiRect {
                                    left: {-props.reticle_size.x / 2.0},
                                    top: {-props.reticle_size.y / 2.0},
                                },
                            }
                            XyPadThumb
                            TextStyleRelay
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
    // `normalize_point` is center-origin (-0.5..0.5); shift to 0..1 and clamp so
    // dragging outside the pad pins to the edge rather than overshooting.
    Some((pos + Vec2::splat(0.5)).clamp(Vec2::ZERO, Vec2::ONE))
}

// Write the value for a pointer event whose target is an inner pad; returns the
// pad root so the caller can flag it dragging.
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
    q_disabled: &Query<Has<InteractionDisabled>, With<PlumeXyPad>>,
    q_lock: &Query<&XyPadLock>,
    q_value: &mut Query<&mut XyPadValue>,
) -> Option<Entity> {
    let (node, node_target, transform, parent) = q_inner.get(inner).ok()?;
    let pad = parent.parent();
    if q_disabled.get(pad).ok()? {
        return None;
    }
    let mut value = value_from_pointer(node, node_target, transform, pointer_position, ui_scale)?;
    // A locked axis ignores the pointer and holds its fixed value, so a 1D bar
    // only moves along its free axis.
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
    Some(pad)
}

fn on_pointer_press(
    mut press: On<Pointer<Press>>,
    q_inner: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &ChildOf,
        ),
        With<XyPadInner>,
    >,
    q_disabled: Query<Has<InteractionDisabled>, With<PlumeXyPad>>,
    q_lock: Query<&XyPadLock>,
    mut q_value: Query<&mut XyPadValue>,
    ui_scale: Res<UiScale>,
) {
    if q_inner.contains(press.entity) {
        press.propagate(false);
        apply_pointer(
            press.entity,
            press.pointer_location.position,
            ui_scale.0,
            &q_inner,
            &q_disabled,
            &q_lock,
            &mut q_value,
        );
    }
}

fn on_drag_start(
    mut drag_start: On<Pointer<DragStart>>,
    q_inner: Query<&ChildOf, With<XyPadInner>>,
    mut q_dragging: Query<(&mut XyPadDragging, Has<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if let Ok(parent) = q_inner.get(drag_start.entity)
        && let Ok((mut dragging, disabled)) = q_dragging.get_mut(parent.parent())
    {
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
    mut drag: On<Pointer<Drag>>,
    q_inner: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &ChildOf,
        ),
        With<XyPadInner>,
    >,
    q_disabled: Query<Has<InteractionDisabled>, With<PlumeXyPad>>,
    q_dragging: Query<&XyPadDragging>,
    q_lock: Query<&XyPadLock>,
    mut q_value: Query<&mut XyPadValue>,
    ui_scale: Res<UiScale>,
) {
    if let Ok((.., parent)) = q_inner.get(drag.entity)
        && q_dragging.get(parent.parent()).is_ok_and(|d| d.0)
    {
        drag.propagate(false);
        apply_pointer(
            drag.entity,
            drag.pointer_location.position,
            ui_scale.0,
            &q_inner,
            &q_disabled,
            &q_lock,
            &mut q_value,
        );
    }
}

fn on_drag_end(
    mut drag_end: On<Pointer<DragEnd>>,
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
    drag_cancel: On<Pointer<Cancel>>,
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
