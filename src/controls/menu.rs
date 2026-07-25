//! Internal menu container, button, and popup used by the select control.
use bevy::camera::visibility::Visibility;
use bevy::ecs::{
    hierarchy::Children,
    observer::On,
    query::With,
    reflect::ReflectComponent,
    system::{Commands, Query, ResMut},
};
use bevy::log::warn;
use bevy::reflect::Reflect;
use bevy::reflect::std_traits::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::ui::{
    AlignItems, Display, FlexDirection, GlobalZIndex, JustifyContent, Node, OverrideClip,
    PositionType, UiRect, Val,
};
use bevy::ui_widgets::{
    ActivateOnPress, MenuAction, MenuButton, MenuEvent, MenuFocusState, MenuPopup,
    popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
};

use crate::theme::control_box_shadow;
use crate::{
    constants::{font_awesome, size},
    controls::{ButtonVariant, PlumeButton},
    display::fa_icon,
    rounded_corners::RoundedCorners,
    theme::{ThemeBackgroundColor, ThemeBorderColor},
    tokens,
};
use bevy::input_focus::{FocusCause, InputFocus, tab_navigation::NavAction};

// TODO:SELECT - do we need this, PlumeMenuButton could just be a button???
/// Top-level menu container. This wraps the menu button and provides an anchor for the popover.
///
/// This is spawnable by inheriting it as a "scene component".
#[derive(SceneComponent, Clone, Default, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct PlumeMenu;

impl PlumeMenu {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                height: size::ROW_HEIGHT,
                justify_content: JustifyContent::Stretch,
                align_items: AlignItems::Stretch,
            }
            PlumeMenu
            on(on_menu_event)
        }
    }
}

fn on_menu_event(
    mut ev: On<MenuEvent>,
    q_menu_children: Query<&Children>,
    q_popovers: Query<&mut Visibility, With<PlumeSelectPopup>>,
    q_buttons: Query<(), With<PlumeSelectButton>>,
    mut commands: Commands,
    mut focus: ResMut<InputFocus>,
) {
    // The popup sits under a `popup_socket` wrapper, so these scan descendants.
    match ev.event().action {
        MenuAction::Open(nav) => {
            ev.propagate(false);
            for descendant in q_menu_children.iter_descendants(ev.source) {
                if q_popovers.contains(descendant) {
                    commands
                        .entity(descendant)
                        .try_insert((Visibility::Visible, MenuFocusState::Opening(nav)));
                    return;
                }
            }
            warn!("Menu popup not found");
        }
        MenuAction::Toggle => {
            for descendant in q_menu_children.iter_descendants(ev.source) {
                if let Ok(visibility) = q_popovers.get(descendant) {
                    ev.propagate(false);
                    if visibility == Visibility::Visible {
                        commands.entity(descendant).try_insert(Visibility::Hidden);
                    } else {
                        commands.entity(descendant).try_insert((
                            Visibility::Visible,
                            MenuFocusState::Opening(NavAction::First),
                        ));
                    }
                    return;
                }
            }
            warn!("Menu popup not found");
        }
        MenuAction::CloseAll => {
            for descendant in q_menu_children.iter_descendants(ev.source) {
                if q_popovers.contains(descendant) {
                    ev.propagate(false);
                    commands.entity(descendant).try_insert(Visibility::Hidden);
                }
            }
        }
        MenuAction::FocusRoot => {
            for descendant in q_menu_children.iter_descendants(ev.source) {
                if q_buttons.contains(descendant) {
                    ev.propagate(false);
                    focus.set(descendant, FocusCause::Navigated);
                    break;
                }
            }
        }
    }
}

// TODO:SELECT this can move in to select but to be honest might not need to be a class, ie if its
// just a button we could inline the bsn inside PlumeSelect, the props are only used one way
/// A menu button widget. This produces a button that has a dropdown arrow.
///
/// This is spawnable by inheriting it as a "scene component" with optional [`PlumeMenuButtonProps`].
#[derive(SceneComponent, Default, Clone)]
#[scene(PlumeSelectButtonProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
pub struct PlumeSelectButton;

/// Props used to construct a [`PlumeMenuButton`] scene.
pub struct PlumeSelectButtonProps {
    /// Label for this menu button
    pub caption: Box<dyn SceneList>,
    /// Rounded corners options
    pub corners: RoundedCorners,
    /// Include the standard downward-pointing chevron (default true).
    pub arrow: bool,
}

impl Default for PlumeSelectButtonProps {
    fn default() -> Self {
        Self {
            caption: Box::new(bsn_list!()),
            corners: Default::default(),
            arrow: true,
        }
    }
}
impl PlumeSelectButton {
    fn scene(props: PlumeSelectButtonProps) -> impl Scene {
        bsn! {
            @PlumeButton {
                @caption: {props.caption},
                @variant: ButtonVariant::Normal,
                @corners: {props.corners},
            }
            ActivateOnPress
            MenuButton
            PlumeSelectButton
            Children [
                {
                    props.arrow.then(|| bsn_list!(
                        Node {
                            flex_grow: 1.0,
                        },
                        fa_icon(font_awesome::solid::ANGLE_DOWN),
                    ))
                }
            ]
        }
    }
}

// TODO:SELECT this can move in to select or possibly be inlined in PlumeSelect, could
// we combine this in to a single new popup with the one from the color edit? Although
// one issue with that is that one has different close criteria?
/// A menu popup widget.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct PlumeSelectPopup;

impl PlumeSelectPopup {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                position_type: PositionType::Absolute,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Stretch,
                align_items: AlignItems::Stretch,
                border: size::CONTAINER_BORDER,
                padding: UiRect::axes(Val::ZERO, size::GAP_TIGHT),
                border_radius: size::CORNER_RADIUS,
            }
            PlumeSelectPopup
            MenuPopup
            Visibility::Hidden
            ThemeBackgroundColor(tokens::MENU_BG)
            ThemeBorderColor(tokens::MENU_BORDER)
            template_value(control_box_shadow())
            GlobalZIndex(100)
            Popover {
                positions: vec![
                    PopoverPlacement {
                        side: PopoverSide::Bottom,
                        align: PopoverAlign::Start,
                        gap: 2.0,
                    },
                    PopoverPlacement {
                        side: PopoverSide::Top,
                        align: PopoverAlign::Start,
                        gap: 2.0,
                    },
                ],
                window_margin: 10.0,
            }
            OverrideClip
        }
    }
}
