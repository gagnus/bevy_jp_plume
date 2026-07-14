//! Styled button controls.
use bevy_app::{Plugin, PreUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
};
use bevy_picking::{PickingSystems, hover::Hovered};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::prelude::*;
use bevy_text::FontWeight;
use bevy_ui::{AlignItems, InteractionDisabled, JustifyContent, Node, Pressed, UiRect};
use bevy_ui_widgets::Button;

use crate::{
    constants::{fonts, size},
    cursor::EntityCursor,
    font_styles::InheritableFont,
    rounded_corners::RoundedCorners,
    theme::{GRADIENT_AMOUNT, InheritableThemeTextColor, ThemeBackgroundGradient},
    tokens,
};

/// Color variants for buttons. This also functions as a component used by the dynamic styling
/// system to identify which entities are buttons.
#[derive(Component, Default, Clone, Reflect, Debug, PartialEq, Eq)]
#[reflect(Component, Clone, Default)]
pub enum ButtonVariant {
    /// The standard button appearance
    #[default]
    Normal,
    /// A button with a more prominent color, this is used for "call to action" buttons,
    /// default buttons for dialog boxes, and so on.
    Primary,
    /// Don't display the button background unless hovering or pressed.
    Plain,
}

/// A button widget.
///
/// This is spawnable by inheriting it as a "scene component" with optional [`PlumeButtonProps`].
///
/// Emits [`bevy_ui_widgets::Activate`] when the pointer is released while hovering over the
/// button; disabled by adding [`bevy_ui::InteractionDisabled`].
#[derive(SceneComponent, Default, Clone)]
#[scene(PlumeButtonProps)]
#[derive(Reflect)]
#[reflect(Component, Clone, Default)]
pub struct PlumeButton;

/// Props used to construct a [`PlumeButton`] scene.
pub struct PlumeButtonProps {
    /// Label for this button. This can contain multiple entities, which will be contained
    /// in a horizontal flexbox.
    pub caption: Box<dyn SceneList>,
    /// Color variant for the button.
    pub variant: ButtonVariant,
    /// Rounded corners options
    pub corners: RoundedCorners,
}

impl Default for PlumeButtonProps {
    fn default() -> Self {
        Self {
            caption: Box::new(bsn_list!()),
            variant: ButtonVariant::default(),
            corners: Default::default(),
        }
    }
}

impl PlumeButton {
    fn scene(props: PlumeButtonProps) -> impl Scene {
        bsn! {
            Node {
                height: size::ROW_HEIGHT,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(size::GAP),
                border_radius: {props.corners.to_border_radius(size::CORNER_RADIUS)},
            }
            Button
            template_value(props.variant)
            Hovered
            EntityCursor::System(bevy_window::SystemCursorIcon::Pointer)
            ThemeBackgroundGradient(tokens::BUTTON_BG, GRADIENT_AMOUNT)
            InheritableThemeTextColor(tokens::BUTTON_TEXT)
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::MEDIUM_FONT,
                weight: FontWeight::NORMAL,
            }
            Children [
                {props.caption}
            ]
        }
    }
}

/// Tool button scene function: a smaller button for embedding in panel headers.
///
/// This is spawnable by inheriting it as a "scene component" with optional [`PlumeButtonProps`].
///
/// Emits [`bevy_ui_widgets::Activate`] when the pointer is released while hovering over the
/// button; disabled by adding [`bevy_ui::InteractionDisabled`].
#[derive(SceneComponent, Default, Clone)]
#[scene(PlumeButtonProps)]
#[derive(Reflect)]
#[reflect(Component, Clone, Default)]
pub struct PlumeToolButton;

impl PlumeToolButton {
    fn scene(props: PlumeButtonProps) -> impl Scene {
        bsn! {
            @PlumeButton {
                @caption: {props.caption},
                @variant: {props.variant},
                @corners: {props.corners}
            }
            Node {
                padding: UiRect::horizontal(size::GAP_TIGHT),
                min_width: size::ROW_HEIGHT,
            }
        }
    }
}

fn update_button_styles(
    q_buttons: Query<
        (
            Entity,
            &ButtonVariant,
            Has<InteractionDisabled>,
            Has<Pressed>,
            &Hovered,
            &ThemeBackgroundGradient,
            &InheritableThemeTextColor,
        ),
        Or<(
            Changed<Hovered>,
            Changed<ButtonVariant>,
            Added<Pressed>,
            Added<InteractionDisabled>,
        )>,
    >,
    mut commands: Commands,
) {
    for (button_ent, variant, disabled, pressed, hovered, bg_color, font_color) in q_buttons.iter()
    {
        set_button_styles(
            button_ent,
            variant,
            disabled,
            pressed,
            hovered.0,
            bg_color,
            font_color,
            &mut commands,
        );
    }
}

fn update_button_styles_remove(
    q_buttons: Query<(
        Entity,
        &ButtonVariant,
        Has<InteractionDisabled>,
        Has<Pressed>,
        &Hovered,
        &ThemeBackgroundGradient,
        &InheritableThemeTextColor,
    )>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_pressed.read())
        .for_each(|ent| {
            if let Ok((button_ent, variant, disabled, pressed, hovered, bg_color, font_color)) =
                q_buttons.get(ent)
            {
                set_button_styles(
                    button_ent,
                    variant,
                    disabled,
                    pressed,
                    hovered.0,
                    bg_color,
                    font_color,
                    &mut commands,
                );
            }
        });
}

fn set_button_styles(
    button_ent: Entity,
    variant: &ButtonVariant,
    disabled: bool,
    pressed: bool,
    hovered: bool,
    bg_color: &ThemeBackgroundGradient,
    font_color: &InheritableThemeTextColor,
    commands: &mut Commands,
) {
    let bg_set = match variant {
        ButtonVariant::Normal => tokens::sets::BUTTON_BG,
        ButtonVariant::Primary => tokens::sets::BUTTON_PRIMARY_BG,
        ButtonVariant::Plain => tokens::sets::BUTTON_PLAIN_BG,
    };
    let bg_token = bg_set.pick(disabled, pressed, hovered);
    // Disabled buttons read as inert: flat fill, no gradient.
    let bg_gradient_amount = if disabled { 0.0 } else { GRADIENT_AMOUNT };

    let font_color_token = match (variant, disabled) {
        (ButtonVariant::Primary, true) => tokens::BUTTON_PRIMARY_TEXT_DISABLED,
        (ButtonVariant::Primary, false) => tokens::BUTTON_PRIMARY_TEXT,
        (ButtonVariant::Normal | ButtonVariant::Plain, true) => tokens::BUTTON_TEXT_DISABLED,
        (ButtonVariant::Normal | ButtonVariant::Plain, false) => tokens::BUTTON_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
    };

    // Change background gradient
    if bg_color.0 != bg_token {
        commands
            .entity(button_ent)
            .insert(ThemeBackgroundGradient(bg_token, bg_gradient_amount));
    }

    // Change font color
    if font_color.0 != font_color_token {
        commands
            .entity(button_ent)
            .insert(InheritableThemeTextColor(font_color_token));
    }

    // Change cursor shape
    commands
        .entity(button_ent)
        .insert(EntityCursor::System(cursor_shape));
}

/// Plugin which registers the systems for updating the button styles.
pub struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_button_styles, update_button_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
