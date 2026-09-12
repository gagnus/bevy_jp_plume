//! Shared popup panel: the floating chrome a control anchors over the UI,
//! plus the socket that mounts it without disturbing ancestor layout.
use bevy::app::{Inherited, Last, Plugin, PostUpdate, Propagate, Update};
use bevy::camera::visibility::Visibility;
use bevy::ecs::change_detection::DetectChangesMut;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::{EntityEvent, PropagateEntityTrigger};
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::observer::On;
use bevy::ecs::query::{Has, Or, With, Without};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::ecs::template::FromTemplate;
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::picking::Pickable;
use bevy::picking::events::{
    PointerClick, PointerDrag, PointerDragEnd, PointerDragStart, PointerEvent, PointerPress,
    PointerScroll, PointerTraversal,
};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::text::TextFont;
use bevy::ui::{
    AlignItems, ComputedNode, Display, FixedNode, FlexDirection, GlobalZIndex, JustifyContent,
    Node, OverrideClip, PositionType, UiGlobalTransform, UiSystems, UiTransform, Val, Val2,
};
use bevy::ui_widgets::MenuPopup;
use bevy::ui_widgets::popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide};

use super::dialog::CloseRequested;
use crate::body::{BodyGap, BodyPadding, apply_body_style};
use crate::constants::{size, z_order};
use crate::controls::MenuAnchorLink;
use crate::font_styles::InheritableFont;
use crate::theme::{
    InheritableThemeTextToken, ThemeBackgroundToken, ThemeBorderToken, ThemeId, control_box_shadow,
};
use crate::tokens;
use crate::utils::hierarchy::nearest_with;

/// Marker for the popup mount point a control keeps in its scene.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct PopupSocket;

// The rect a socket overlays, when it is not the socket's own parent.
#[derive(Component, Clone, Copy)]
pub(crate) struct PopupAnchor(pub Entity);

// A socket whose anchor only places it. A panel hung at a corner of something
// is still the app's, so it keeps the root's theme and text style rather than
// taking its anchor's, as a popup or tooltip does.
#[derive(Component, Clone, Copy, Default)]
pub(crate) struct PlacementAnchor;

/// Mount point for a control's popup: spawn it as a child of the control the
/// popup should anchor to, then spawn a [`PlumePopup`] into it to open.
// `FixedNode` makes it a layout root - it neither inherits ancestor layout and
// clipping nor, as an absolute child would, inflate their scroll range.
pub fn popup_socket() -> impl Scene {
    bsn! {
        Node { position_type: PositionType::Absolute }
        FixedNode
        Pickable::IGNORE
        PopupSocket
    }
}

// Holds every socket over its anchor. A `FixedNode`'s inset is viewport-relative,
// so the anchor's global rect goes straight in. It trails the anchor by a frame:
// the rect is the one the last layout produced.
fn track_popup_anchors(
    mut q_sockets: Query<(&mut Node, Option<&PopupAnchor>, Option<&ChildOf>), With<PopupSocket>>,
    q_rects: Query<(&ComputedNode, &UiGlobalTransform)>,
) {
    for (mut node, anchor, socket_parent) in q_sockets.iter_mut() {
        // An imm socket is parentless and always carries its anchor; a retained
        // one sits inside the control it anchors to.
        let anchor = anchor
            .map(|anchor| anchor.0)
            .or_else(|| socket_parent.map(ChildOf::parent));
        let Some((rect, transform)) = anchor.and_then(|anchor| q_rects.get(anchor).ok()) else {
            continue;
        };
        let size = rect.size() * rect.inverse_scale_factor;
        let top_left = transform.translation * rect.inverse_scale_factor - 0.5 * size;
        let (left, top) = (Val::Px(top_left.x), Val::Px(top_left.y));
        let (width, height) = (Val::Px(size.x), Val::Px(size.y));
        // Read through the immutable deref first: writing unconditionally would
        // dirty layout every frame.
        if (node.left, node.top, node.width, node.height) != (left, top, width, height) {
            (node.left, node.top, node.width, node.height) = (left, top, width, height);
        }
    }
}

// A parentless floating root - an imm popup socket, or either kind of tooltip
// box - sits outside every propagation chain, so the ambient text style and
// theme are bridged like the rect: the anchor's `Inherited` values are copied
// onto it. Nothing else writes `Inherited` on a parentless entity, so the
// copies stand, and the root's own propagation carries them into its panel.
// A root given a theme of its own (`Propagate<ThemeId>`) keeps it: the bridge
// would otherwise overwrite it every frame. A `PlacementAnchor` bridges nothing.
fn bridge_floating_anchor_style(
    q_floating: Query<(Entity, &PopupAnchor), (Without<ChildOf>, Without<PlacementAnchor>)>,
    q_inherited: Query<&Inherited<TextFont>>,
    q_inherited_theme: Query<&Inherited<ThemeId>>,
    q_own_theme: Query<(), With<Propagate<ThemeId>>>,
    mut commands: Commands,
) {
    for (floating, anchor) in &q_floating {
        if let Ok(inherited) = q_inherited.get(anchor.0)
            && !q_inherited.get(floating).is_ok_and(|i| i.0 == inherited.0)
        {
            commands.entity(floating).insert(inherited.clone());
        }
        if q_own_theme.contains(floating) {
            continue;
        }
        match q_inherited_theme.get(anchor.0) {
            Ok(inherited)
                if !q_inherited_theme
                    .get(floating)
                    .is_ok_and(|i| i.0 == inherited.0) =>
            {
                commands.entity(floating).insert(inherited.clone());
            }
            // An anchor back on the default theme takes the root with it.
            Err(_) if q_inherited_theme.contains(floating) => {
                commands.entity(floating).remove::<Inherited<ThemeId>>();
            }
            _ => {}
        }
    }
}

/// Where a popup opens relative to its anchor.
#[derive(Default, Clone, Copy, PartialEq)]
pub enum PopupPlacement {
    /// Below the anchor, centred; slides along it, then flips above, then beside.
    #[default]
    Below,
    /// Beside the anchor, centred; slides along it, tries right, left, then
    /// below, above.
    Beside,
}

/// What requests a popup's close besides code.
#[derive(Clone, Copy, PartialEq, Default)]
pub enum PopupDismiss {
    /// Close when focus leaves the popup (the bevy menu machinery; the anchor
    /// control routes the resulting `MenuEvent`s).
    FocusOut,
    /// Close when a press lands outside the popup's anchor control.
    #[default]
    OutsideClick,
    /// Only the owner closes it.
    Explicit,
}

/// A floating popup panel: themed chrome, `Popover` auto-placement, and the
/// configured dismiss behavior. Existing is open - spawn one into a
/// [`popup_socket`] to open it, [`close_popup`] to close it.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumePopupProps)]
#[reflect(Component, Default, Clone)]
pub struct PlumePopup;

/// Props for a [`PlumePopup`].
pub struct PlumePopupProps {
    /// Body content of the popup.
    pub contents: Box<dyn SceneList>,
    /// Where the popup opens relative to its socket.
    pub placement: PopupPlacement,
    /// If true popup right next to the control, otherwise a little away (default).
    pub place_very_close: bool,
    /// What closes the popup besides code.
    pub dismiss: PopupDismiss,
    /// Whether background drags move the popup (a reopen re-anchors it).
    pub movable: bool,
}

impl Default for PlumePopupProps {
    fn default() -> Self {
        Self {
            contents: Box::new(()),
            placement: Default::default(),
            dismiss: Default::default(),
            movable: false,
            place_very_close: false,
        }
    }
}

// Outside-press dismissal scope for an imm popup: presses on the anchor don't
// dismiss. Retained popups omit this and scope to their control root instead.
#[derive(Component, FromTemplate, Clone, Copy)]
struct DismissScope(pub Entity);

// The imm layer's popup: the same chrome the retained path spawns, scoped to the
// anchor it opens from. Lives here rather than in `imm` so `DismissScope` stays
// private to this module - as `imm_menu_frame` does for the menu.
pub(crate) fn imm_popup_scene(
    anchor: Entity,
    placement: PopupPlacement,
    dismiss: PopupDismiss,
    movable: bool,
) -> impl Scene {
    bsn! {
        @PlumePopup {
            @placement: placement,
            @dismiss: dismiss,
            @movable: movable,
        }
        DismissScope(anchor)
    }
}

// Plain root marker, inserted on both the retained and imm paths. The systems key on
// this, not the [`PlumePopup`] scene component, which only the retained path inserts.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct PopupRoot;

/// Every floating surface in the popup band, popup frames and menu frames alike:
/// what [`stack_popups`] counts to give each one its layer.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct PopupSurface;

// Marker for popups dismissed by a press outside their anchor control.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct DismissOnOutsideClick;

impl PlumePopup {
    fn scene(props: PlumePopupProps) -> impl Scene {
        bsn! {
            Node {
                position_type: PositionType::Absolute,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Stretch,
                align_items: AlignItems::Stretch,
                border: size::HAIRLINE,
                padding: size::SPACE_TIGHT,
                border_radius: size::CORNER_RADIUS,
                row_gap: size::SPACE,
            }
            PopupRoot
            PopupSurface
            ThemeBackgroundToken(tokens::POPUP_BG)
            ThemeBorderToken(tokens::POPUP_BORDER)
            control_box_shadow()
            GlobalZIndex(z_order::POPUP)
            popover_for(props.placement, props.place_very_close)
            OverrideClip
            InheritableThemeTextToken(tokens::TEXT_DIM)
            // Parentless socket: resolves to the standard font. Empty braces
            // stop bsn claiming the next interpolation block as a field list.
            InheritableFont {}
            @{(props.dismiss == PopupDismiss::FocusOut).then(|| bsn! { MenuPopup })}
            @{(props.dismiss == PopupDismiss::OutsideClick).then(|| bsn! { DismissOnOutsideClick })}
            // A focus-out popup's focus is owned by the menu machinery; the rest
            // scope Tab traversal like a dialog does.
            @{(props.dismiss != PopupDismiss::FocusOut).then(|| bsn! { TabGroup::new(0) })}
            @{props.movable.then(|| bsn! { on(on_popup_drag) })}
            // A popup floats over whatever its anchor sits in, so pointer activity
            // inside it must not bubble on to that surface: a retained socket is a
            // hierarchy child, and its anchor's ancestors would read the popup's
            // presses, drags and wheel as their own (a canvas node drag, a window
            // raise, a canvas zoom under a popup's own scroll list).
            on(stop_pointer::<PointerPress>)
            on(stop_pointer::<PointerClick>)
            on(stop_pointer::<PointerDragStart>)
            on(stop_pointer::<PointerDrag>)
            on(stop_pointer::<PointerDragEnd>)
            on(stop_pointer::<PointerScroll>)
            Children [
                {props.contents}
            ]
        }
    }
}

// Auto-placement candidates for a [`PopupPlacement`], tried in order. `Popover`
// measures them against the popup's parent - the socket, i.e. the anchor rect -
// and takes the first that fits, never sliding one that does not: so each side
// is offered centred, then aligned to either end, before the next side.
fn popover_for(placement: PopupPlacement, place_very_close: bool) -> Popover {
    let sides = match placement {
        PopupPlacement::Below => &[
            PopoverSide::Bottom,
            PopoverSide::Top,
            PopoverSide::Right,
            PopoverSide::Left,
        ],
        PopupPlacement::Beside => &[
            PopoverSide::Right,
            PopoverSide::Left,
            PopoverSide::Bottom,
            PopoverSide::Top,
        ],
    };

    let gap = if place_very_close { 2.0 } else { 8.0 };
    let positions = sides
        .iter()
        .flat_map(|&side| {
            [PopoverAlign::Center, PopoverAlign::Start, PopoverAlign::End]
                .map(|align| PopoverPlacement { side, align, gap })
        })
        .collect();

    Popover {
        positions,
        window_margin: 10.0,
    }
}

// Ends a pointer event's ancestor walk at the popup root. Entity observers below
// and on the root have already run; only the world outside the popup loses it.
fn stop_pointer<E>(mut event: On<E>)
where
    E: PointerEvent
        + for<'t> EntityEvent<Trigger<'t> = PropagateEntityTrigger<true, E, PointerTraversal>>,
{
    event.propagate(false);
}

// Drag a non-control part of the popup to move it: an entity observer, so it fires
// only for drags that bubbled up unconsumed. The first drag drops `Popover` so the
// manual position stops fighting auto-placement; reopening restores it.
fn on_popup_drag(
    drag: On<PointerDrag>,
    q_childof: Query<&ChildOf>,
    q_is_popup: Query<(), With<PopupRoot>>,
    mut q_popup: Query<(&mut UiTransform, Has<Popover>), With<PopupRoot>>,
    mut commands: Commands,
) {
    let Some(popup) = nearest_with(drag.entity, &q_childof, &q_is_popup) else {
        return;
    };
    let Ok((mut transform, has_popover)) = q_popup.get_mut(popup) else {
        return;
    };
    if has_popover {
        commands.entity(popup).remove::<Popover>();
    }
    let px_of = |value: Val| match value {
        Val::Px(px) => px,
        _ => 0.0,
    };
    transform.translation = Val2::px(
        px_of(transform.translation.x) + drag.delta.x,
        px_of(transform.translation.y) + drag.delta.y,
    );
}

// Marks a closed popup awaiting its end-of-frame despawn.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct ClosingPopup;

/// Close `popup`: hidden immediately, despawned at end of frame.
// The deferral matters - closing usually happens mid-cascade (a row click, a
// dismiss press), and later commands in that cascade still target its entities.
pub fn close_popup(commands: &mut Commands, popup: Entity) {
    commands
        .entity(popup)
        .insert((Visibility::Hidden, ClosingPopup));
}

fn despawn_closing_popups(q_closing: Query<Entity, With<ClosingPopup>>, mut commands: Commands) {
    for popup in q_closing.iter() {
        commands.entity(popup).despawn();
    }
}

// A press outside an open popup's anchor control requests its close; the owner does
// the closing. The scope is the socket's parent, so presses on the anchor (e.g. the
// swatch) stay toggle-only.
fn on_dismiss_outside_press(
    mut click: On<PointerPress>,
    q_childof: Query<&ChildOf>,
    q_links: Query<&MenuAnchorLink>,
    q_anchors: Query<&PopupAnchor>,
    q_popups: Query<(Entity, Option<&DismissScope>), With<DismissOnOutsideClick>>,
    mut commands: Commands,
) {
    // Bubbling re-triggers this observer at every ancestor hop; evaluate the
    // press once, against its original target - a press inside the popup must
    // not read as "outside" when its bubble climbs above the popup.
    if click.entity != click.original_event_target() {
        return;
    }
    // The press target and everything it sits inside, openers included: a menu
    // opened from a row of a popup is unrooted, but a press on its items must
    // not dismiss the popup it came from - that would despawn the row, the menu
    // and the activation together.
    let mut lineage = vec![click.entity];
    while lineage.len() < MAX_LINEAGE
        && let Some(&last) = lineage.last()
        && let Some(next) = step_to_opener(last, &q_childof, &q_links, &q_anchors)
    {
        lineage.push(next);
    }
    for (popup, dismiss_scope) in q_popups.iter() {
        let scope = match dismiss_scope {
            Some(scope) => Some(scope.0),
            None => q_childof
                .get(popup)
                .ok()
                .and_then(|socket| q_childof.get(socket.parent()).ok())
                .map(|control| control.parent()),
        };
        // Inside = on the anchor scope or in the popup itself (an imm popup is
        // not a descendant of its anchor, so it needs its own check).
        let inside = scope
            .into_iter()
            .chain([popup])
            .any(|root| lineage.contains(&root));
        if !inside {
            commands
                .entity(popup)
                .insert((Visibility::Hidden, CloseRequested));
            // Only the dismissing press is swallowed - a press inside the popup
            // must keep bubbling (a button's caption bubbles up to the button).
            click.propagate(false);
        }
    }
}

// Escape requests close of every open dismissable popup (a select's popup gets
// its Escape handling from the menu focus machinery instead).
fn close_popups_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    q_popups: Query<Entity, With<DismissOnOutsideClick>>,
    mut commands: Commands,
) {
    if keys.just_pressed(KeyCode::Escape) {
        for popup in q_popups.iter() {
            commands
                .entity(popup)
                .insert((Visibility::Hidden, CloseRequested));
        }
    }
}

// A popup is its own body, so the pair every other container relays to a child lands
// here on the entity that already carries it. Still a relay, to keep the vocabulary.
fn relay_popup_style(
    mut q_popups: Query<
        (Option<&BodyGap>, Option<&BodyPadding>, &mut Node),
        (With<PopupRoot>, Or<(With<BodyGap>, With<BodyPadding>)>),
    >,
) {
    for (gap, padding, mut node) in q_popups.iter_mut() {
        apply_body_style(&mut node, gap, padding);
    }
}

// How far a link chain may be followed before it is treated as a cycle. Deeper
// than any real menu, shallow enough that a malformed one cannot hang a frame.
const MAX_POPUP_NESTING: usize = 32;
// The same bound for a full walk to the root, which also climbs `ChildOf`.
const MAX_LINEAGE: usize = 256;

// One step from a popup's content towards what opened it. An imm menu frame is
// unrooted and jumps through its `MenuAnchorLink`; a parentless socket through
// its `PopupAnchor`; anything else climbs to its parent. Following this from a
// press is what makes a popup opened from inside another count as inside it.
fn step_to_opener(
    entity: Entity,
    q_parents: &Query<&ChildOf>,
    q_links: &Query<&MenuAnchorLink>,
    q_anchors: &Query<&PopupAnchor>,
) -> Option<Entity> {
    if let Ok(&MenuAnchorLink(anchor)) = q_links.get(entity) {
        return Some(anchor);
    }
    if let Ok(child_of) = q_parents.get(entity) {
        return Some(child_of.parent());
    }
    q_anchors.get(entity).ok().map(|anchor| anchor.0)
}

// Popups stack by nesting: one opened from inside another has to draw over it.
// A flat z leaves that to bevy's tie-break, which sorts equal layers by ECS
// storage order - so whichever kind of popup a session opened first won every
// time, for the life of the process.
fn stack_popups(
    q_surfaces: Query<Entity, With<PopupSurface>>,
    q_is_surface: Query<(), With<PopupSurface>>,
    q_parents: Query<&ChildOf>,
    q_links: Query<&MenuAnchorLink>,
    q_anchors: Query<&PopupAnchor>,
    mut q_layers: Query<&mut GlobalZIndex>,
) {
    for surface in q_surfaces.iter() {
        let mut depth = 0;
        let mut cursor = surface;
        for _ in 0..MAX_POPUP_NESTING {
            cursor = match step_to_opener(cursor, &q_parents, &q_links, &q_anchors) {
                Some(opener) => opener,
                None => break,
            };
            if q_is_surface.contains(cursor) {
                depth += 1;
            }
        }
        let layer = (z_order::POPUP + depth).min(z_order::POPUP_MAX);
        if let Ok(mut z) = q_layers.get_mut(surface) {
            z.set_if_neq(GlobalZIndex(layer));
        }
    }
}

// Registers socket anchor tracking, popup dismissal (outside press, Escape) and
// the end-of-frame despawn.
pub(crate) struct PopupPlugin;

impl Plugin for PopupPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_observer(on_dismiss_outside_press)
            .add_systems(Update, close_popups_on_escape)
            // Ahead of layout, so `Popover` places the popup against a socket
            // this frame's layout has already moved onto the anchor.
            .add_systems(PostUpdate, track_popup_anchors.in_set(UiSystems::Prepare))
            // Before the resolver and the id propagation, so the popup re-resolves
            // the same frame.
            .add_systems(
                PostUpdate,
                bridge_floating_anchor_style
                    .before(crate::font_styles::resolve_inheritable_font)
                    .before(bevy::app::PropagateSet::<crate::theme::ThemeId>::default()),
            )
            .add_systems(PostUpdate, relay_popup_style.before(UiSystems::Layout))
            // Ahead of the stack it feeds; the hierarchy it walks is whatever
            // this frame's commands left behind.
            .add_systems(PostUpdate, stack_popups.before(UiSystems::Stack))
            .add_systems(Last, despawn_closing_popups);
    }
}
