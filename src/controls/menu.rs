//! Internal menu container, button, and popup used by the select control.
use bevy_camera::visibility::Visibility;
use bevy_color::{Alpha, Srgba};
use bevy_ecs::{
    hierarchy::Children,
    observer::On,
    query::With,
    reflect::ReflectComponent,
    system::{Commands, Query, ResMut},
};
use bevy_log::warn;
use bevy_reflect::Reflect;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_scene::prelude::*;
use bevy_ui::{
    AlignItems, BoxShadow, Display, FlexDirection, GlobalZIndex, JustifyContent, Node,
    OverrideClip, PositionType, UiRect, px,
};
use bevy_ui_widgets::{
    ActivateOnPress, MenuAction, MenuButton, MenuEvent, MenuFocusState, MenuPopup,
    popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
};

use crate::{
    constants::{font_awesome, size}, controls::{ButtonVariant, PlumeButton}, display::fa_icon_solid, rounded_corners::RoundedCorners, theme::{ThemeBackgroundColor, ThemeBorderColor}, tokens,
};
use bevy_input_focus::{FocusCause, InputFocus, tab_navigation::NavAction};

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
    q_popovers: Query<&mut Visibility, With<PlumeMenuPopup>>,
    q_buttons: Query<(), With<PlumeMenuButton>>,
    mut commands: Commands,
    mut focus: ResMut<InputFocus>,
) {
    match ev.event().action {
        MenuAction::Open(nav) => {
            let Ok(children) = q_menu_children.get(ev.source) else {
                return;
            };
            ev.propagate(false);
            for child in children.iter() {
                if q_popovers.contains(*child) {
                    commands
                        .entity(*child)
                        .try_insert((Visibility::Visible, MenuFocusState::Opening(nav)));
                    return;
                }
            }
            warn!("Menu popup not found");
        }
        MenuAction::Toggle => {
            let Ok(children) = q_menu_children.get(ev.source) else {
                return;
            };
            for child in children.iter() {
                if let Ok(visibility) = q_popovers.get(*child) {
                    ev.propagate(false);
                    if visibility == Visibility::Visible {
                        commands.entity(*child).try_insert(Visibility::Hidden);
                    } else {
                        commands.entity(*child).try_insert((
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
            let Ok(children) = q_menu_children.get(ev.source) else {
                return;
            };
            for child in children.iter() {
                if q_popovers.contains(*child) {
                    ev.propagate(false);
                    commands.entity(*child).try_insert(Visibility::Hidden);
                }
            }
        }
        MenuAction::FocusRoot => {
            let Ok(children) = q_menu_children.get(ev.source) else {
                return;
            };
            for child in children.iter() {
                if q_buttons.contains(*child) {
                    ev.propagate(false);
                    focus.set(*child, FocusCause::Navigated);
                    break;
                }
            }
        }
    }
}

/// A menu button widget. This produces a button that has a dropdown arrow.
///
/// This is spawnable by inheriting it as a "scene component" with optional [`PlumeMenuButtonProps`].
#[derive(SceneComponent, Default, Clone)]
#[scene(PlumeMenuButtonProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
pub struct PlumeMenuButton;

/// Props used to construct a [`PlumeMenuButton`] scene.
pub struct PlumeMenuButtonProps {
    /// Label for this menu button
    pub caption: Box<dyn SceneList>,
    /// Rounded corners options
    pub corners: RoundedCorners,
    /// Include the standard downward-pointing chevron (default true).
    pub arrow: bool,
}

impl Default for PlumeMenuButtonProps {
    fn default() -> Self {
        Self {
            caption: Box::new(bsn_list!()),
            corners: Default::default(),
            arrow: true,
        }
    }
}
impl PlumeMenuButton {
    fn scene(props: PlumeMenuButtonProps) -> impl Scene {
        bsn! {
            @PlumeButton {
                @caption: {props.caption},
                @variant: ButtonVariant::Normal,
                @corners: {props.corners},
            }
            ActivateOnPress
            MenuButton
            PlumeMenuButton
            // Additional children for menu chevron
            Children [
                {
                    props.arrow.then(|| bsn_list!(
                        Node {
                            flex_grow: 1.0,
                        },
                        fa_icon_solid(font_awesome::FA_ANGLE_DOWN),
                    ))
                }
            ]
        }
    }
}

/// A menu popup widget.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct PlumeMenuPopup;

impl PlumeMenuPopup {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                position_type: PositionType::Absolute,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Stretch,
                align_items: AlignItems::Stretch,
                border: size::CONTAINER_BORDER,
                padding: UiRect::axes(px(0), size::GAP_TIGHT),
                border_radius: {RoundedCorners::All.to_border_radius(size::CORNER_RADIUS)},
            }
            PlumeMenuPopup
            MenuPopup
            Visibility::Hidden
            ThemeBackgroundColor(tokens::MENU_BG)
            ThemeBorderColor(tokens::MENU_BORDER)
            BoxShadow::new(
                Srgba::BLACK.with_alpha(0.9).into(),
                px(0),
                px(0),
                px(1),
                px(4),
            )
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
