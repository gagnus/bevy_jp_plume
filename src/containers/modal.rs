//! Modal dialog: a frame centered over a barrier that blocks the app behind it.
use bevy::app::{App, Plugin};
use bevy::camera::visibility::Visibility;
use bevy::color::Color;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EntityEvent;
use bevy::ecs::hierarchy::Children;
use bevy::ecs::lifecycle::Add;
use bevy::ecs::observer::On;
use bevy::ecs::query::With;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::system::{Commands, Query};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::{Scene, SceneComponent, SceneList, bsn, bsn_list, on};
use bevy::ui::{
    AlignItems, BackgroundColor, Display, FixedNode, GlobalZIndex, JustifyContent, Node,
    OverrideClip, PositionType, UiRect, Val, px,
};
use bevy::ui_widgets::{ModalDialog, ModalDialogBarrier, RequestClose};

use crate::constants::{size, z_order};
use crate::containers::{
    CloseRequested, DialogChrome, DialogHeader, DismissOnOutsideClick, dialog_body, dialog_frame,
};
use crate::display::caption;
use crate::font_styles::InheritableFont;

// Dims whatever is behind rather than painting a surface, so it is a fixed
// wash rather than a theme slot: every palette wants the same darkening. The
// alpha is high because blending is linear - 0.5 here reads as a quarter.
const SCRIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.75);

/// Props used to construct a [`PlumeModal`] scene.
pub struct PlumeModalProps {
    /// Title content shown in the header (e.g. `bsn! { caption("…") }`).
    pub title: Box<dyn SceneList>,
    /// Body content of the modal.
    pub contents: Box<dyn SceneList>,
    /// How wide the modal should be.
    pub width: Val,
    /// `false` omits the ✕ **and** the dismissals that go with it - a barrier
    /// click and Escape stop closing it, leaving the body's buttons as the only
    /// answer. What a destructive confirmation wants.
    pub closable: bool,
}

impl Default for PlumeModalProps {
    fn default() -> Self {
        Self {
            title: Box::new(bsn_list! {}),
            contents: Box::new(bsn_list! {}),
            width: Val::Auto,
            closable: true,
        }
    }
}

/// A modal dialog: the app behind it takes no picks until it is answered.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeModalProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeModal;

impl PlumeModal {
    /// Scene function for the modal.
    pub fn scene(props: PlumeModalProps) -> impl Scene {
        let PlumeModalProps {
            title,
            contents,
            width,
            closable,
        } = props;
        bsn! {
            @modal_barrier()
            // Every dismissal - the ✕, a barrier click, Escape - arrives here,
            // since `RequestClose` propagates up out of the frame. Closing
            // despawns the modal, as it does for [`PlumeDialog`].
            @{closable.then_some(bsn! {
                on(|close: On<RequestClose>, mut commands: Commands| {
                    commands.entity(close.event_target()).despawn();
                })
            })}
            Children [
                @modal_frame(DialogChrome {
                    name: "PlumeModal".into(),
                    body: Box::new(bsn! {
                        @dialog_body()
                        Children [
                            {contents}
                        ]
                    }),
                    header: Some(DialogHeader {
                        title,
                        closable,
                        movable: false,
                    }),
                    width,
                    height: Val::Auto,
                    max_height: Val::Percent(100.0),
                    inset: UiRect::AUTO,
                })
            ]
        }
    }
}

/// A modal's title, styled like a dialog's.
pub fn modal_title(title: impl Into<String>) -> impl Scene {
    let title = title.into();
    bsn! {
        @caption(title)
        InheritableFont { font_size: size::DIALOG_HEADER_TEXT_SIZE }
    }
}

// The scrim: a full-viewport layout root that blocks the app behind it and
// centers the frame on it. `ModalDialogBarrier` is what earns bevy's
// outside-click, Escape and focus handling.
pub(crate) fn modal_barrier() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            right: px(0),
            top: px(0),
            bottom: px(0),
            display: Display::Flex,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            // Keeps a tall modal off the viewport edges.
            padding: UiRect::all(size::SPACE * 4.0),
        }
        ModalDialogBarrier
        FixedNode
        OverrideClip
        GlobalZIndex(z_order::MODAL)
        BackgroundColor(SCRIM)
    }
}

// The dialog frame, marked modal: `ModalDialog` keeps it out of the floating
// stack's z (so the barrier's layer holds) and turns its `TabGroup` modal.
fn modal_frame(chrome: DialogChrome) -> impl Scene {
    bsn! {
        @dialog_frame(chrome)
        ModalDialog
    }
}

/// Closes the popups a modal covers.
pub(crate) struct ModalPlugin;

impl Plugin for ModalPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(close_popups_on_modal);
    }
}

// Popups sit *above* a modal's layer, so one left open would float a live control
// over the scrim, out of reach of the barrier that is meant to have swallowed it.
// That dismissal, not the z order, is what makes a modal a mode.
//
// Only the outside-click kind needs telling: a menu or select closes itself when
// the modal takes the focus, which `ModalDialogPlugin` does on spawn.
fn close_popups_on_modal(
    _add: On<Add<ModalDialog>>,
    q_popups: Query<Entity, With<DismissOnOutsideClick>>,
    mut commands: Commands,
) {
    for popup in q_popups.iter() {
        commands
            .entity(popup)
            .insert((Visibility::Hidden, CloseRequested));
    }
}
