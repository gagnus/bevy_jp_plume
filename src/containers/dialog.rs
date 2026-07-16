//! Movable floating dialog with a draggable title bar and close button.
use bevy_color::{Alpha, Srgba};
use bevy_ecs::{
    event::EntityEvent, hierarchy::Children, observer::On, reflect::ReflectComponent,
    system::Commands,
};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::{Scene, SceneComponent, SceneList, bsn, bsn_list, on};
use bevy_text::FontWeight;
use bevy_ui::{
    AlignItems, BorderRadius, BoxShadow, Display, FlexDirection, JustifyContent, Node,
    PositionType, UiRect, Val, px,
};
use bevy_ui_widgets::{Activate, Dialog, DialogDragHandle, RequestClose};

use crate::{
    constants::{font_awesome, fonts, size},
    controls::{ButtonVariant, PlumeToolButton},
    display::fa_icon,
    font_styles::InheritableFont,
    rounded_corners::RoundedCorners,
    theme::{InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor},
    tokens,
};

/// Props used to construct a [`PlumeDialog`] scene.
pub struct PlumeDialogProps {
    /// Title content shown in the window's drag bar (e.g. `bsn! { caption("…") }`).
    pub title: Box<dyn SceneList>,
    /// Body content of the window.
    pub contents: Box<dyn SceneList>,
    /// How wide the window should be.
    pub width: Val,
    /// Initial left offset (the window is absolutely positioned).
    pub left: Val,
    /// Initial top offset.
    pub top: Val,
}

impl Default for PlumeDialogProps {
    fn default() -> Self {
        Self {
            title: Box::new(bsn_list!()),
            contents: Box::new(bsn_list!()),
            width: Val::Auto,
            left: px(120),
            top: px(120),
        }
    }
}

/// A movable floating dialog with a draggable title bar and a close button.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeDialogProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeDialog;

impl PlumeDialog {
    /// Scene function for a floating dialog window.
    pub fn scene(props: PlumeDialogProps) -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                position_type: PositionType::Absolute,
                left: {props.left},
                top: {props.top},
                border_radius: BorderRadius::all(px(size::DIALOG_RADIUS)),
                border: UiRect::all(size::CONTAINER_BORDER),
                width: {props.width},
            }
            Dialog
            ThemeBackgroundColor(tokens::DIALOG_BG)
            ThemeBorderColor(tokens::DIALOG_BORDER)
            InheritableThemeTextColor(tokens::TEXT_MAIN)
            BoxShadow::new(
                Srgba::BLACK.with_alpha(0.7).into(),
                px(4),
                px(8),
                px(4),
                px(16),
            )
            // Closing despawns the window.
            on(|close: On<RequestClose>, mut commands: Commands| {
                commands.entity(close.event_target()).despawn();
            })
            Children [
                // Title bar; dragging it moves the window.
                (
                    // Same chrome as the subpane header; the dialog is distinguished
                    // by its drop shadow, not a different header.
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        padding: UiRect::horizontal(size::HEADER_PAD_X),
                        min_height: size::DIALOG_HEADER_HEIGHT,
                        column_gap: size::GAP_TIGHT,
                        border: UiRect::bottom(size::CONTAINER_BORDER),
                        border_radius: {RoundedCorners::Top.to_border_radius(size::DIALOG_RADIUS)}
                    }
                    DialogDragHandle
                    InheritableThemeTextColor(tokens::DIALOG_HEADER_TEXT)
                    ThemeBackgroundColor(tokens::DIALOG_HEADER_BG)
                    ThemeBorderColor(tokens::DIALOG_BORDER)
                    InheritableFont {
                        font: fonts::REGULAR,
                        font_size: size::MEDIUM_FONT,
                        weight: FontWeight::NORMAL,
                    }
                    Children [
                        {props.title},
                        @PlumeDialogClose
                    ]
                ),
                (
                    @PlumeDialogBody
                    Children [
                        {props.contents}
                    ]
                )
            ]
        }
    }
}

/// Close button for dialog header
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct PlumeDialogClose;

impl PlumeDialogClose {
    /// Scene function for dialog close button.
    pub fn scene() -> impl Scene {
        bsn! {
        @PlumeToolButton {
            @variant: ButtonVariant::Plain,
            @caption: bsn! { fa_icon(font_awesome::solid::XMARK) }
        }
        on(|activate: On<Activate>, mut commands: Commands| {
            commands.trigger(RequestClose { source: activate.event_target() });
        })
        }
    }
}

/// Central body section for a dialog
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct PlumeDialogBody;

impl PlumeDialogBody {
    /// Scene function for dialog body.
    pub fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: size::GAP,
                padding: UiRect::all(size::PAD),
            }
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::MEDIUM_FONT,
                weight: FontWeight::NORMAL,
            }
        }
    }
}
