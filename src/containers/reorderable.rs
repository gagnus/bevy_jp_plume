//! Vertical list whose items the user drags into a new order by the grip at each
//! item's left. Order is the root's `Children` order; each step reports a move.
use bevy::app::{Plugin, PreUpdate};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EntityEvent;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::lifecycle::RemovedComponents;
use bevy::ecs::observer::On;
use bevy::ecs::query::{Added, Changed, Has, Or, With};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::picking::PickingSystems;
use bevy::picking::events::{
    PointerCancel, PointerDrag, PointerDragEnd, PointerDragStart, PointerPress, PointerRelease,
};
use bevy::picking::hover::Hovered;
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::{Scene, SceneComponent, SceneList, bsn, bsn_list, on};
use bevy::text::{EmSize, RemSize};
use bevy::ui::{
    AlignItems, ComputedNode, ComputedUiRenderTargetInfo, Display, FlexDirection,
    InteractionDisabled, Node, Pressed, UiGlobalTransform, UiTransform, Val, Val2, ZIndex,
};
use bevy::ui_widgets::ValueChange;
use bevy::window::SystemCursorIcon;

use crate::constants::{lucide, size};
use crate::cursor::{CursorLock, EntityCursor};
use crate::display::icon;
use crate::theme::InheritableThemeTextToken;
use crate::tokens;

// How far past the list's own edge a drag may carry the item: a little give, so
// a drag that cannot go anywhere still answers the hand, without reaching into
// whatever sits beside the list.
const OVERSHOOT: Val = size::SPACE;

/// A vertical list the user drags into a new order by the grip at each item's
/// left. Order is the root's `Children` order: read a drag's steps as
/// [`ValueChange<ReorderMove>`](ValueChange) on the root, and push a model-side
/// change with `replace_children` in model order.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeReorderableProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeReorderable;

/// Props used to construct a [`PlumeReorderable`] scene.
pub struct PlumeReorderableProps {
    /// The items, top to bottom; each a [`PlumeReorderableItem`].
    pub contents: Box<dyn SceneList>,
}

impl Default for PlumeReorderableProps {
    fn default() -> Self {
        Self {
            contents: Box::new(bsn_list! {}),
        }
    }
}

impl PlumeReorderable {
    /// Scene function for a reorderable list.
    pub fn scene(props: PlumeReorderableProps) -> impl Scene {
        let contents = props.contents;
        bsn! {
            @reorderable_frame()
            Children [
                {contents}
            ]
        }
    }
}

/// One item of a [`PlumeReorderable`]: a row led by the grip, with `contents`
/// filling the rest of it.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeReorderableItemProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeReorderableItem;

/// Props used to construct a [`PlumeReorderableItem`] scene.
pub struct PlumeReorderableItemProps {
    /// Row content after the grip (e.g. `bsn! { @caption("…") }`).
    pub contents: Box<dyn SceneList>,
}

impl Default for PlumeReorderableItemProps {
    fn default() -> Self {
        Self {
            contents: Box::new(bsn_list! {}),
        }
    }
}

impl PlumeReorderableItem {
    /// Scene function for a reorderable item.
    pub fn scene(props: PlumeReorderableItemProps) -> impl Scene {
        let contents = props.contents;
        bsn! {
            @reorderable_item()
            Children [
                @reorderable_grip()
                --
                {contents}
            ]
        }
    }
}

// Plain root marker, inserted by [`reorderable_frame`] on both paths.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct ReorderableRoot;

// Marks one item row: the grip, then the app's content.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct ReorderableItem;

// The drag target at an item's left.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ReorderGrip;

/// One move of a reorderable's item from index `from` to index `to`, in item
/// order. Reported as a [`ValueChange`] on the list's root at every step of a
/// drag; apply it as `items[from..=to].rotate_left(1)` (or `rotate_right` when
/// `to < from`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub struct ReorderMove {
    /// Where the item was.
    pub from: usize,
    /// Where it is now.
    pub to: usize,
}

impl ReorderMove {
    /// Apply the move to `items`; a move the list has outgrown is ignored.
    pub fn apply<T>(self, items: &mut [T]) -> bool {
        let Self { from, to } = self;
        if from == to || from >= items.len() || to >= items.len() {
            return false;
        }
        if from < to {
            items[from..=to].rotate_left(1);
        } else {
            items[to..=from].rotate_right(1);
        }
        true
    }

    // A later step of the same gesture keeps where the item started this pass
    // and moves where it is now.
    fn then(self, to: usize) -> Self {
        Self {
            from: self.from,
            to,
        }
    }
}

// Steps the app has not consumed yet, on the root. The imm layer takes it each
// pass; a retained app listens for `ValueChange<ReorderMove>` instead.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct ReorderMailbox(pub(crate) Option<ReorderMove>);

// The gesture in flight, on the root. `slot` is where the layout puts the item
// when it is not lifted, kept arithmetically as steps move it; `lift_px` is how
// far the item sits off that slot. All physical px, like `ComputedNode`.
#[derive(Component, Clone, Copy)]
struct ReorderDrag {
    item: Entity,
    index: usize,
    lift_px: f32,
    slot: Span,
}

// A vertical extent in physical px.
#[derive(Clone, Copy)]
struct Span {
    top: f32,
    bottom: f32,
}

impl Span {
    fn of(node: &ComputedNode, transform: &UiGlobalTransform) -> Self {
        let half = node.size().y / 2.0;
        let center = transform.translation.y;
        Self {
            top: center - half,
            bottom: center + half,
        }
    }

    fn shifted(self, by: f32) -> Self {
        Self {
            top: self.top + by,
            bottom: self.bottom + by,
        }
    }
}

// The list's root: a tight column. Order is its `Children` order.
pub(crate) fn reorderable_frame() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            row_gap: size::SPACE_TIGHT,
            min_height: Val::ZERO,
            min_width: Val::ZERO,
        }
        ReorderableRoot
        ReorderMailbox
    }
}

// One item: a row the grip leads and the app's content fills. The transform is
// the lift.
pub(crate) fn reorderable_item() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: size::SPACE,
            min_height: Val::ZERO,
            min_width: Val::ZERO,
        }
        ReorderableItem
        UiTransform::default()
    }
}

// The grip: a dim glyph that takes the drag. Its cursor is held by `CursorLock`
// while pressed, so it survives the pointer outrunning the glyph mid-drag.
pub(crate) fn reorderable_grip() -> impl Scene {
    bsn! {
        @icon(lucide::GRIP_VERTICAL)
        Node { width: size::ICON_WIDTH }
        ReorderGrip
        Hovered
        InheritableThemeTextToken(tokens::TEXT_DIM)
        EntityCursor::System(SystemCursorIcon::Grab)
        CursorLock
        on(press_grip)
        on(release_grip)
        on(drag_start_grip)
        on(drag_grip)
        on(drag_end_grip)
        on(cancel_grip)
    }
}

fn press_grip(press: On<PointerPress>, mut commands: Commands) {
    commands.entity(press.event_target()).insert(Pressed);
}

fn release_grip(release: On<PointerRelease>, mut commands: Commands) {
    commands.entity(release.event_target()).remove::<Pressed>();
}

// The grip's item and the list root above it.
fn grip_context(
    grip: Entity,
    q_child_of: &Query<&ChildOf>,
    q_roots: &Query<(&Children, Has<InteractionDisabled>), With<ReorderableRoot>>,
) -> Option<(Entity, Entity)> {
    let item = q_child_of.get(grip).ok()?.parent();
    let root = q_child_of.get(item).ok()?.parent();
    q_roots.contains(root).then_some((item, root))
}

// The root's item children, in order, each with where the layout has it.
fn item_spans(
    children: &Children,
    q_items: &Query<(&ComputedNode, &UiGlobalTransform), With<ReorderableItem>>,
) -> Vec<(Entity, Span)> {
    children
        .iter()
        .filter_map(|child| {
            q_items
                .get(*child)
                .ok()
                .map(|(node, transform)| (*child, Span::of(node, transform)))
        })
        .collect()
}

fn drag_start_grip(
    mut drag_start: On<PointerDragStart>,
    q_child_of: Query<&ChildOf>,
    q_roots: Query<(&Children, Has<InteractionDisabled>), With<ReorderableRoot>>,
    q_items: Query<(&ComputedNode, &UiGlobalTransform), With<ReorderableItem>>,
    q_drags: Query<(), With<ReorderDrag>>,
    mut commands: Commands,
) {
    let Some((item, root)) = grip_context(drag_start.event_target(), &q_child_of, &q_roots) else {
        return;
    };
    let Ok((children, disabled)) = q_roots.get(root) else {
        return;
    };
    // A second pointer must not start a gesture over a live one.
    if disabled || q_drags.contains(root) {
        return;
    }
    let spans = item_spans(children, &q_items);
    let Some(index) = spans.iter().position(|(entity, _)| *entity == item) else {
        return;
    };
    drag_start.propagate(false);
    commands.entity(root).insert(ReorderDrag {
        item,
        index,
        lift_px: 0.0,
        slot: spans[index].1,
    });
    // Lifted out of document order, or the neighbor it is dragged over paints on
    // top of it.
    commands.entity(item).insert(ZIndex(1));
}

// The gesture: pointer travel grows the lift; once the lift passes half the
// distance a step would move the item, the item steps past that neighbor and
// the lift is rebased by the step, so the item stays under the pointer. Steps
// work on a snapshot of the neighbors' rects, moved arithmetically, since the
// layout has not seen the swap yet.
fn drag_grip(
    mut drag: On<PointerDrag>,
    q_child_of: Query<&ChildOf>,
    q_roots: Query<(&Children, Has<InteractionDisabled>), With<ReorderableRoot>>,
    q_items: Query<(&ComputedNode, &UiGlobalTransform), With<ReorderableItem>>,
    mut q_drags: Query<(
        &mut ReorderDrag,
        &mut ReorderMailbox,
        &ComputedNode,
        &UiGlobalTransform,
        &ComputedUiRenderTargetInfo,
        Option<&EmSize>,
    )>,
    mut q_transforms: Query<&mut UiTransform, With<ReorderableItem>>,
    rem_size: Res<RemSize>,
    mut commands: Commands,
) {
    let Some((item, root)) = grip_context(drag.event_target(), &q_child_of, &q_roots) else {
        return;
    };
    let Ok((mut pending, mut mailbox, root_node, root_transform, target, em_size)) =
        q_drags.get_mut(root)
    else {
        return;
    };
    if pending.item != item {
        return;
    }
    // The list owns this gesture; whatever it sits in must not also move.
    drag.propagate(false);
    let Ok((children, _)) = q_roots.get(root) else {
        return;
    };
    let Ok((item_node, _)) = q_items.get(item) else {
        return;
    };
    // The drag arrives logical; the nodes measure physical.
    let scale = 1.0 / item_node.inverse_scale_factor();
    let mut lift_px = pending.lift_px + drag.delta.y * scale;

    let mut spans = item_spans(children, &q_items);
    let mut index = pending.index;
    if spans.get(index).is_none_or(|(entity, _)| *entity != item) {
        return;
    }
    let mut slot = pending.slot;
    let mut moved: Option<ReorderMove> = None;
    loop {
        if let Some(&(below, below_span)) = spans.get(index + 1) {
            // Moving down: the item lands where the neighbor ends.
            let shift = below_span.bottom - slot.bottom;
            if shift > 0.0 && lift_px > shift / 2.0 {
                lift_px -= shift;
                let height = below_span.bottom - below_span.top;
                spans[index + 1] = (
                    below,
                    Span {
                        top: slot.top,
                        bottom: slot.top + height,
                    },
                );
                slot = slot.shifted(shift);
                spans.swap(index, index + 1);
                index += 1;
                moved = Some(moved.map_or(
                    ReorderMove {
                        from: index - 1,
                        to: index,
                    },
                    |m| m.then(index),
                ));
                continue;
            }
        }
        if index > 0 {
            let (above, above_span) = spans[index - 1];
            // Moving up: the item lands where the neighbor starts.
            let shift = slot.top - above_span.top;
            if shift > 0.0 && lift_px < -shift / 2.0 {
                lift_px += shift;
                let height = above_span.bottom - above_span.top;
                spans[index - 1] = (
                    above,
                    Span {
                        top: slot.bottom - height,
                        bottom: slot.bottom,
                    },
                );
                slot = slot.shifted(-shift);
                spans.swap(index, index - 1);
                index -= 1;
                moved = Some(moved.map_or(
                    ReorderMove {
                        from: index + 1,
                        to: index,
                    },
                    |m| m.then(index),
                ));
                continue;
            }
        }
        break;
    }

    // Past either end there is no neighbor to step past, so the lift would only
    // carry the item out over chrome; it gets the list's padding plus
    // `OVERSHOOT`. Clamping the stored lift, not the transform, means the way
    // back answers at once.
    let root_span = Span::of(root_node, root_transform);
    let give = OVERSHOOT
        .resolve(
            target.scale_factor(),
            root_span.bottom - root_span.top,
            target.physical_size().as_vec2(),
            em_size.copied().unwrap_or_default(),
            *rem_size,
        )
        .unwrap_or(0.0);
    lift_px = lift_px.clamp(
        (root_span.top - give - slot.top).min(0.0),
        (root_span.bottom + give - slot.bottom).max(0.0),
    );

    if let Some(step) = moved {
        // Move the row itself, so the retained path is complete on its own; the
        // imm path re-declares the same order, and the layout-order sort agrees.
        commands.entity(root).insert_child(index, item);
        mailbox.0 = Some(mailbox.0.map_or(step, |earlier| earlier.then(step.to)));
        commands.trigger(ValueChange {
            source: root,
            value: step,
            is_final: false,
        });
    }
    pending.index = index;
    pending.slot = slot;
    pending.lift_px = lift_px;
    if let Ok(mut transform) = q_transforms.get_mut(item) {
        transform.translation = Val2::new(Val::ZERO, Val::Px(lift_px / scale));
    }
}

fn drag_end_grip(
    mut drag_end: On<PointerDragEnd>,
    q_child_of: Query<&ChildOf>,
    q_roots: Query<(&Children, Has<InteractionDisabled>), With<ReorderableRoot>>,
    q_drags: Query<&ReorderDrag>,
    mut q_transforms: Query<&mut UiTransform, With<ReorderableItem>>,
    mut commands: Commands,
) {
    commands.entity(drag_end.event_target()).remove::<Pressed>();
    if settle(
        drag_end.event_target(),
        &q_child_of,
        &q_roots,
        &q_drags,
        &mut q_transforms,
        &mut commands,
    ) {
        drag_end.propagate(false);
    }
}

// A canceled pointer drops the item wherever the order currently has it.
fn cancel_grip(
    cancel: On<PointerCancel>,
    q_child_of: Query<&ChildOf>,
    q_roots: Query<(&Children, Has<InteractionDisabled>), With<ReorderableRoot>>,
    q_drags: Query<&ReorderDrag>,
    mut q_transforms: Query<&mut UiTransform, With<ReorderableItem>>,
    mut commands: Commands,
) {
    commands.entity(cancel.event_target()).remove::<Pressed>();
    settle(
        cancel.event_target(),
        &q_child_of,
        &q_roots,
        &q_drags,
        &mut q_transforms,
        &mut commands,
    );
}

// Drops the lift and ends the gesture, reporting whether there was one to end.
// The mailbox is left alone: an unconsumed step is still a real move.
fn settle(
    grip: Entity,
    q_child_of: &Query<&ChildOf>,
    q_roots: &Query<(&Children, Has<InteractionDisabled>), With<ReorderableRoot>>,
    q_drags: &Query<&ReorderDrag>,
    q_transforms: &mut Query<&mut UiTransform, With<ReorderableItem>>,
    commands: &mut Commands,
) -> bool {
    let Some((item, root)) = grip_context(grip, q_child_of, q_roots) else {
        return false;
    };
    if !q_drags.get(root).is_ok_and(|drag| drag.item == item) {
        return false;
    }
    if let Ok(mut transform) = q_transforms.get_mut(item) {
        transform.translation = Val2::ZERO;
    }
    commands.entity(item).remove::<ZIndex>();
    commands.entity(root).remove::<ReorderDrag>();
    true
}

// The grip's glyph and cursor for its state: disabled lists take the surface's
// disabled text and no cursor; a live grip is dim until the pointer lights it.
fn style_grip(grip: Entity, disabled: bool, pressed: bool, hovered: bool, commands: &mut Commands) {
    let token = if disabled {
        tokens::TEXT_DISABLED
    } else if pressed || hovered {
        tokens::TEXT_MAIN
    } else {
        tokens::TEXT_DIM
    };
    let mut entity = commands.entity(grip);
    entity.insert(InheritableThemeTextToken(token));
    if disabled {
        entity.remove::<EntityCursor>();
    } else {
        let cursor = if pressed {
            SystemCursorIcon::Grabbing
        } else {
            SystemCursorIcon::Grab
        };
        entity.insert(EntityCursor::System(cursor));
    }
}

// Whether the list a grip belongs to is disabled, and the list itself.
fn grip_root(
    grip: Entity,
    q_child_of: &Query<&ChildOf>,
    q_roots: &Query<Has<InteractionDisabled>, With<ReorderableRoot>>,
) -> Option<(Entity, bool)> {
    let item = q_child_of.get(grip).ok()?.parent();
    let root = q_child_of.get(item).ok()?.parent();
    q_roots.get(root).ok().map(|disabled| (root, disabled))
}

// A grip's own state changed, or it is new.
fn update_grip_styles(
    q_grips: Query<
        (Entity, Has<Pressed>, &Hovered),
        (
            With<ReorderGrip>,
            Or<(Added<ReorderGrip>, Added<Pressed>, Changed<Hovered>)>,
        ),
    >,
    q_child_of: Query<&ChildOf>,
    q_roots: Query<Has<InteractionDisabled>, With<ReorderableRoot>>,
    mut commands: Commands,
) {
    for (grip, pressed, hovered) in q_grips.iter() {
        let Some((_, disabled)) = grip_root(grip, &q_child_of, &q_roots) else {
            continue;
        };
        style_grip(grip, disabled, pressed, hovered.get(), &mut commands);
    }
}

fn update_grip_styles_remove(
    q_grips: Query<(Has<Pressed>, &Hovered), With<ReorderGrip>>,
    q_child_of: Query<&ChildOf>,
    q_roots: Query<Has<InteractionDisabled>, With<ReorderableRoot>>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut commands: Commands,
) {
    for grip in removed_pressed.read() {
        let Ok((pressed, hovered)) = q_grips.get(grip) else {
            continue;
        };
        let Some((_, disabled)) = grip_root(grip, &q_child_of, &q_roots) else {
            continue;
        };
        style_grip(grip, disabled, pressed, hovered.get(), &mut commands);
    }
}

// The list's disabled state changed: its text goes to the disabled color - the
// app's captions inherit it, controls keep their own - and every grip follows.
fn update_reorderable_disabled(
    q_added: Query<
        (Entity, Has<InteractionDisabled>),
        (
            With<ReorderableRoot>,
            Or<(Added<ReorderableRoot>, Added<InteractionDisabled>)>,
        ),
    >,
    q_roots: Query<Has<InteractionDisabled>, With<ReorderableRoot>>,
    q_children: Query<&Children>,
    q_grips: Query<(Entity, Has<Pressed>, &Hovered), With<ReorderGrip>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    fn restyle(
        root: Entity,
        disabled: bool,
        q_children: &Query<&Children>,
        q_grips: &Query<(Entity, Has<Pressed>, &Hovered), With<ReorderGrip>>,
        commands: &mut Commands,
    ) {
        if disabled {
            commands
                .entity(root)
                .insert(InheritableThemeTextToken(tokens::TEXT_DISABLED));
        } else {
            commands.entity(root).remove::<InheritableThemeTextToken>();
        }
        let grips = q_children
            .iter_descendants(root)
            .filter_map(|descendant| q_grips.get(descendant).ok());
        for (grip, pressed, hovered) in grips {
            style_grip(grip, disabled, pressed, hovered.get(), commands);
        }
    }

    for (root, disabled) in q_added.iter() {
        // A fresh list stays with whatever text color it inherited.
        if disabled {
            restyle(root, true, &q_children, &q_grips, &mut commands);
        }
    }
    for root in removed_disabled.read() {
        if q_roots.get(root).is_ok_and(|disabled| !disabled) {
            restyle(root, false, &q_children, &q_grips, &mut commands);
        }
    }
}

// Registers the grip and disabled-state styling.
pub(crate) struct ReorderablePlugin;

impl Plugin for ReorderablePlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            (
                update_grip_styles,
                update_grip_styles_remove,
                update_reorderable_disabled,
            )
                .in_set(PickingSystems::Last),
        );
    }
}
