//! Shared popup panel: the floating chrome a control anchors over the UI,
//! plus the socket that mounts it without disturbing ancestor layout.
use bevy::app::{Inherited, Last, Plugin, PostUpdate, Update};
use bevy::camera::visibility::Visibility;
use bevy::ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{Has, With, Without},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
};
use bevy::input::{ButtonInput, keyboard::KeyCode};
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::picking::{
    Pickable,
    events::{Drag, Pointer, Press},
};
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::scene::prelude::*;
use bevy::text::TextFont;
use bevy::ui::{
    AlignItems, ComputedNode, Display, FixedNode, FlexDirection, GlobalZIndex, JustifyContent,
    Node, OverrideClip, PositionType, UiGlobalTransform, UiRect, UiSystems, UiTransform, Val, Val2,
};
use bevy::ui_widgets::{
    MenuPopup,
    popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
};

use super::dialog::CloseRequested;
use crate::constants::size;
use crate::font_styles::{InheritableFont, TextStyleRelay};
use crate::theme::{
    InheritableThemeTextToken, ThemeBackgroundToken, ThemeBorderToken, control_box_shadow,
};
use crate::tokens;
use crate::utils::hierarchy::nearest_with;

// Marker for the popup mount point a control keeps in its scene. The relay
// keeps a retained socket (a child of its control) on the text-style chain.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
#[require(TextStyleRelay)]
pub(crate) struct PopupSocket;

// The rect a socket overlays, when it is not the socket's own parent.
#[derive(Component, Clone, Copy)]
pub(crate) struct PopupAnchor(pub Entity);

// Mount point for a control's popup: overlays the anchor exactly, so a popup
// spawned into it `Popover`-anchors to the same rect. `FixedNode` makes it a
// layout root — it neither inherits ancestor layout and clipping nor, as an
// absolute child would, inflates their `content_size` and scroll range.
pub(crate) fn popup_socket() -> impl Scene {
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

// A parentless (imm) socket sits outside every propagation chain, so the
// ambient text style is bridged the same way the rect is: the anchor's
// `Inherited<TextFont>` is copied onto the socket, and the popup's all-inherit
// `InheritableFont` resolves through it. Nothing else writes `Inherited` on a
// parentless entity, so the copy is authoritative.
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

// Where a popup opens relative to its anchor.
#[derive(Default, Clone, Copy, PartialEq)]
pub(crate) enum PopupPlacement {
    // Below the anchor, start-aligned; flips above when out of room.
    #[default]
    Below,
    // Beside the anchor, center-aligned; tries right, left, above, below.
    Beside,
}

// What requests a popup's close besides code.
#[derive(Clone, Copy, PartialEq, Default)]
pub(crate) enum PopupDismiss {
    // Close when focus leaves the popup (the bevy menu machinery; the anchor
    // control routes the resulting `MenuEvent`s).
    FocusOut,
    // Close when a press lands outside the popup's anchor control.
    #[default]
    OutsideClick,
    // Only the owner closes it.
    Explicit,
}

// The floating popup panel shared by select and color edit: themed chrome,
// `Popover` auto-placement, and the configured dismiss/move behaviour. Spawned
// into a [`popup_socket`] on open and despawned on close — existing is open.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumePopupProps)]
#[reflect(Component, Default, Clone)]
pub(crate) struct PlumePopup;

// Props for a [`PlumePopup`].
pub(crate) struct PlumePopupProps {
    // Body content of the popup.
    pub contents: Box<dyn SceneList>,
    // Where the popup opens relative to its socket.
    pub placement: PopupPlacement,
    // What closes the popup besides code.
    pub dismiss: PopupDismiss,
    // Whether background drags move the popup (a reopen re-anchors it).
    pub movable: bool,
    // Padding inside the chrome.
    pub padding: UiRect,
}

impl Default for PlumePopupProps {
    fn default() -> Self {
        Self {
            contents: Box::new(bsn_list!()),
            placement: Default::default(),
            dismiss: Default::default(),
            movable: false,
            padding: size::GAP_TIGHT.into(),
        }
    }
}

// Outside-press dismissal scope for an imm popup: presses on the anchor don't
// dismiss. Retained popups omit this and scope to their control root instead.
#[derive(Component, Clone, Copy)]
pub(crate) struct DismissScope(pub Entity);

// Marker for popups dismissed by a press outside their anchor control.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct DismissOnOutsideClick;

impl PlumePopup {
    fn scene(props: PlumePopupProps) -> impl Scene {
        let placement = props.placement;
        bsn! {
            Node {
                position_type: PositionType::Absolute,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Stretch,
                align_items: AlignItems::Stretch,
                border: size::CONTAINER_BORDER,
                padding: {props.padding},
                border_radius: size::CORNER_RADIUS,
            }
            PlumePopup
            ThemeBackgroundToken(tokens::MENU_BG)
            ThemeBorderToken(tokens::MENU_BORDER)
            template_value(control_box_shadow())
            GlobalZIndex(100)
            template_value(popover_for(placement))
            OverrideClip
            InheritableThemeTextToken(tokens::TEXT_DIM)
            // Parentless socket: resolves to the standard font. Empty braces
            // stop bsn claiming the next interpolation block as a field list.
            InheritableFont {}
            {(props.dismiss == PopupDismiss::FocusOut).then(|| bsn!(MenuPopup))}
            {(props.dismiss == PopupDismiss::OutsideClick).then(|| bsn!(DismissOnOutsideClick))}
            // A focus-out popup's focus is owned by the menu machinery; the rest
            // scope Tab traversal like a dialog does.
            {(props.dismiss != PopupDismiss::FocusOut).then(|| bsn!(TabGroup::new(0)))}
            {props.movable.then(|| bsn!(on(on_popup_drag)))}
            Children [
                {props.contents}
            ]
        }
    }
}

// Auto-placement candidates for a [`PopupPlacement`], tried in order. `Popover`
// measures them against the popup's parent — the socket, i.e. the anchor rect.
fn popover_for(placement: PopupPlacement) -> Popover {
    let popover = |positions| Popover {
        positions,
        window_margin: 10.0,
    };
    match placement {
        PopupPlacement::Below => {
            let below = |side| PopoverPlacement {
                side,
                align: PopoverAlign::Start,
                gap: 2.0,
            };
            popover(vec![below(PopoverSide::Bottom), below(PopoverSide::Top)])
        }
        PopupPlacement::Beside => {
            let beside = |side| PopoverPlacement {
                side,
                align: PopoverAlign::Center,
                gap: 8.0,
            };
            popover(vec![
                beside(PopoverSide::Right),
                beside(PopoverSide::Left),
                beside(PopoverSide::Top),
                beside(PopoverSide::Bottom),
            ])
        }
    }
}

// Drag a non-control part of the popup to move it. This is an entity observer on
// the popup, so it only fires for drags that bubbled up unconsumed — i.e. not on a
// pad/number/button, which stop their own drags. The first drag drops `Popover` so
// the manual position stops fighting the auto-placement; reopening restores it.
fn on_popup_drag(
    drag: On<Pointer<Drag>>,
    q_childof: Query<&ChildOf>,
    q_is_popup: Query<(), With<PlumePopup>>,
    mut q_popup: Query<(&mut UiTransform, Has<Popover>), With<PlumePopup>>,
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

// Close `popup`: hidden immediately, despawned at end of frame. The deferral
// matters — closing usually happens mid-cascade (a row click, a dismiss press),
// and commands later in the same cascade still target the popup's entities.
pub(crate) fn close_popup(commands: &mut Commands, popup: Entity) {
    commands
        .entity(popup)
        .insert((Visibility::Hidden, ClosingPopup));
}

fn despawn_closing_popups(q_closing: Query<Entity, With<ClosingPopup>>, mut commands: Commands) {
    for popup in q_closing.iter() {
        commands.entity(popup).despawn();
    }
}

// A press anywhere outside an open popup's anchor control requests its close
// (hidden at once; the owner — color edit's observer or the imm layer — closes
// it). The scope is the socket's parent, so presses on the anchor (e.g. the
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
            .add_systems(Last, despawn_closing_popups);
    }
}
