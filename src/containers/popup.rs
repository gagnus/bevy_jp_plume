//! Shared popup panel: the floating chrome a control anchors over the UI,
//! plus the socket that mounts it without disturbing ancestor layout.
use bevy::app::{Last, Plugin, Update};
use bevy::camera::visibility::Visibility;
use bevy::ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{Has, With},
    reflect::ReflectComponent,
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
use bevy::text::FontWeight;
use bevy::ui::{
    AlignItems, Display, FlexDirection, GlobalZIndex, JustifyContent, Node, Overflow, OverrideClip,
    PositionType, UiRect, UiTransform, Val, Val2,
};
use bevy::ui_widgets::{
    MenuPopup,
    popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
};

use super::dialog::CloseRequested;
use crate::constants::{fonts, size};
use crate::font_styles::InheritableFont;
use crate::theme::{
    InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor, control_box_shadow,
};
use crate::tokens;
use crate::utils::hierarchy::nearest_with;

// Marker for the popup mount point a control keeps in its scene.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct PopupSocket;

// Mount point for a control's popup: overlays the control exactly, so a popup
// spawned into it `Popover`-anchors to the same rect. Clipped because taffy
// counts absolute children in `content_size` — an unclipped open popup would
// inflate every ancestor scroll range (it escapes visually via `OverrideClip`).
pub(crate) fn popup_socket() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: Val::ZERO,
            right: Val::ZERO,
            top: Val::ZERO,
            bottom: Val::ZERO,
            overflow: Overflow::clip(),
        }
        Pickable::IGNORE
        PopupSocket
    }
}

// Where a popup opens: anchored to an entity, or centered without one.
#[derive(Default, Clone, Copy, PartialEq)]
pub(crate) enum PopupPlacement {
    // Below the entity, start-aligned; flips above when out of room.
    Below(Entity),
    // Beside the entity, center-aligned; tries right, left, above, below.
    Beside(Entity),
    // No anchor: centered in the window.
    #[default]
    Center,
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

// The floating popup panel shared by select and colour edit: themed chrome,
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
    // Where the popup opens relative to its anchor.
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

// Outside-press dismissal scope for an imm popup: presses on `Some(anchor)` (or
// only inside the popup itself, for `None`) don't dismiss. Retained popups omit
// this and scope to the socket's parent — their control root.
#[derive(Component, Clone, Copy)]
pub(crate) struct DismissScope(pub Option<Entity>);

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
            ThemeBackgroundColor(tokens::MENU_BG)
            ThemeBorderColor(tokens::MENU_BORDER)
            template_value(control_box_shadow())
            GlobalZIndex(100)
            template_value(popover_for(placement))
            OverrideClip
            InheritableThemeTextColor(tokens::TEXT_DIM)
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::MEDIUM_FONT,
                weight: FontWeight::NORMAL,
            }
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

// Auto-placement candidates for a [`PopupPlacement`].
fn popover_for(placement: PopupPlacement) -> Popover {
    let popover = |positions, anchor| Popover {
        positions,
        window_margin: 10.0,
        anchor,
    };
    match placement {
        PopupPlacement::Below(anchor) => {
            let below = |side| PopoverPlacement {
                side,
                align: PopoverAlign::Start,
                gap: 2.0,
            };
            popover(
                vec![below(PopoverSide::Bottom), below(PopoverSide::Top)],
                Some(anchor),
            )
        }
        PopupPlacement::Beside(anchor) => {
            let beside = |side| PopoverPlacement {
                side,
                align: PopoverAlign::Center,
                gap: 8.0,
            };
            popover(
                vec![
                    beside(PopoverSide::Right),
                    beside(PopoverSide::Left),
                    beside(PopoverSide::Top),
                    beside(PopoverSide::Bottom),
                ],
                Some(anchor),
            )
        }
        // An empty position list centers the popover in the window.
        PopupPlacement::Center => popover(Vec::new(), None),
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
// (hidden at once; the owner — colour edit's observer or the imm layer — closes
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
            Some(scope) => scope.0,
            None => q_childof
                .get(popup)
                .ok()
                .and_then(|socket| q_childof.get(socket.parent()).ok())
                .map(|control| control.parent()),
        };
        // Inside = on the anchor scope or in the popup itself (an anchored popup
        // is not a descendant of its scope, so the popup needs its own check).
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

// Registers popup dismissal (outside press, Escape) and the end-of-frame despawn.
pub(crate) struct PopupPlugin;

impl Plugin for PopupPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_observer(on_dismiss_outside_press)
            .add_systems(Update, close_popups_on_escape)
            .add_systems(Last, despawn_closing_popups);
    }
}
