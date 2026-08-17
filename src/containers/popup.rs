//! Shared popup panel: the floating chrome a control anchors over the UI,
//! plus the socket that mounts it without disturbing ancestor layout.
use bevy::app::{Inherited, Last, Plugin, PostUpdate, Update};
use bevy::camera::visibility::Visibility;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::observer::On;
use bevy::ecs::query::{Has, Or, With, Without};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::picking::Pickable;
use bevy::picking::events::{Click, Drag, DragEnd, DragStart, Pointer, Press};
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
use crate::constants::size;
use crate::containers::{BodyGap, BodyPadding, apply_body_style};
use crate::font_styles::{InheritableFont, TextStyleRelay};
use crate::theme::{
    InheritableThemeTextToken, ThemeBackgroundToken, ThemeBorderToken, control_box_shadow,
};
use crate::tokens;
use crate::utils::hierarchy::nearest_with;

/// Marker for the popup mount point a control keeps in its scene. The relay
/// keeps a retained socket (a child of its control) on the text-style chain.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
#[require(TextStyleRelay)]
pub struct PopupSocket;

// The rect a socket overlays, when it is not the socket's own parent.
#[derive(Component, Clone, Copy)]
pub(crate) struct PopupAnchor(pub Entity);

/// Mount point for a control's popup: spawn it as a child of the control the
/// popup should anchor to, then spawn a [`PlumePopup`] into it to open.
// `FixedNode` makes it a layout root — it neither inherits ancestor layout and
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

// A parentless (imm) socket sits outside every propagation chain, so the ambient text
// style is bridged like the rect: the anchor's `Inherited<TextFont>` is copied onto the
// socket. Nothing else writes `Inherited` on a parentless entity, so the copy stands.
fn bridge_socket_text_style(
    q_sockets: Query<(Entity, &PopupAnchor), (With<PopupSocket>, Without<ChildOf>)>,
    q_inherited: Query<&Inherited<TextFont>>,
    mut commands: Commands,
) {
    for (socket, anchor) in &q_sockets {
        let Ok(inherited) = q_inherited.get(anchor.0) else {
            continue;
        };
        if q_inherited.get(socket).is_ok_and(|i| i.0 == inherited.0) {
            continue;
        }
        commands.entity(socket).insert(inherited.clone());
    }
}

/// Where a popup opens relative to its anchor.
#[derive(Default, Clone, Copy, PartialEq)]
pub enum PopupPlacement {
    /// Below the anchor, start-aligned; flips above when out of room.
    #[default]
    Below,
    /// Beside the anchor, center-aligned; tries right, left, above, below.
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
/// configured dismiss behavior. Existing is open — spawn one into a
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
            contents: Box::new(bsn_list![]),
            placement: Default::default(),
            dismiss: Default::default(),
            movable: false,
            place_very_close: false,
        }
    }
}

// Outside-press dismissal scope for an imm popup: presses on the anchor don't
// dismiss. Retained popups omit this and scope to their control root instead.
#[derive(Component, Clone, Copy)]
struct DismissScope(pub Entity);

// The imm layer's popup: the same chrome the retained path spawns, scoped to the
// anchor it opens from. Lives here rather than in `imm` so `DismissScope` stays
// private to this module — as `imm_menu_frame` does for the menu.
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
        template_value(DismissScope(anchor))
    }
}

// Plain root marker, inserted on both the retained and imm paths. The systems key on
// this, not the [`PlumePopup`] scene component, which only the retained path inserts.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct PopupRoot;

// Marker for popups dismissed by a press outside their anchor control.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct DismissOnOutsideClick;

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
            ThemeBackgroundToken(tokens::POPUP_BG)
            ThemeBorderToken(tokens::POPUP_BORDER)
            template_value(control_box_shadow())
            GlobalZIndex(100)
            template_value(popover_for(props.placement, props.place_very_close))
            OverrideClip
            InheritableThemeTextToken(tokens::TEXT_DIM)
            // Parentless socket: resolves to the standard font. Empty braces
            // stop bsn claiming the next interpolation block as a field list.
            InheritableFont {}
            {(props.dismiss == PopupDismiss::FocusOut).then(|| bsn! { MenuPopup })}
            {(props.dismiss == PopupDismiss::OutsideClick).then(|| bsn! { DismissOnOutsideClick })}
            // A focus-out popup's focus is owned by the menu machinery; the rest
            // scope Tab traversal like a dialog does.
            {(props.dismiss != PopupDismiss::FocusOut).then(|| bsn! { TabGroup::new(0) })}
            {props.movable.then(|| bsn! { on(on_popup_drag) })}
            // A popup floats over whatever its anchor sits in, so pointer activity
            // inside it must not bubble on to that surface: a retained socket is a
            // hierarchy child, and its anchor's ancestors would read the popup's
            // presses and drags as their own (a canvas node drag, a window raise).
            on(stop_pointer::<Press>)
            on(stop_pointer::<Click>)
            on(stop_pointer::<DragStart>)
            on(stop_pointer::<Drag>)
            on(stop_pointer::<DragEnd>)
            Children [
                {props.contents},
            ]
        }
    }
}

// Auto-placement candidates for a [`PopupPlacement`], tried in order. `Popover`
// measures them against the popup's parent — the socket, i.e. the anchor rect.
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

    let side_to_position = |side| PopoverPlacement {
        side,
        align: PopoverAlign::Center,
        gap: if place_very_close { 2.0 } else { 8.0 },
    };

    Popover {
        positions: sides.map(side_to_position).to_vec(),
        window_margin: 10.0,
    }
}

// Ends a pointer event's ancestor walk at the popup root. Entity observers below
// and on the root have already run; only the world outside the popup loses it.
fn stop_pointer<E: core::fmt::Debug + Clone + Reflect>(mut event: On<Pointer<E>>) {
    event.propagate(false);
}

// Drag a non-control part of the popup to move it: an entity observer, so it fires
// only for drags that bubbled up unconsumed. The first drag drops `Popover` so the
// manual position stops fighting auto-placement; reopening restores it.
fn on_popup_drag(
    drag: On<Pointer<Drag>>,
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
// The deferral matters — closing usually happens mid-cascade (a row click, a
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
    mut click: On<Pointer<Press>>,
    q_childof: Query<&ChildOf>,
    q_popups: Query<(Entity, Option<&DismissScope>), With<DismissOnOutsideClick>>,
    mut commands: Commands,
) {
    // Bubbling re-triggers this observer at every ancestor hop; evaluate the
    // press once, against its original target — a press inside the popup must
    // not read as "outside" when its bubble climbs above the popup.
    if click.entity != click.original_event_target() {
        return;
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
        let inside = scope.into_iter().chain([popup]).any(|root| {
            click.entity == root
                || q_childof
                    .iter_ancestors(click.entity)
                    .any(|ancestor| ancestor == root)
        });
        if !inside {
            commands
                .entity(popup)
                .insert((Visibility::Hidden, CloseRequested));
            // Only the dismissing press is swallowed — a press inside the popup
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
            // Before the resolver, so the popup re-resolves the same frame.
            .add_systems(
                PostUpdate,
                bridge_socket_text_style.before(crate::font_styles::resolve_inheritable_font),
            )
            .add_systems(PostUpdate, relay_popup_style.before(UiSystems::Layout))
            .add_systems(Last, despawn_closing_popups);
    }
}
