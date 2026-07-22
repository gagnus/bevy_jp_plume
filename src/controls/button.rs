//! Styled button controls.
use bevy_app::{Plugin, PreUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::{Pickable, PickingSystems, hover::Hovered};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::prelude::*;
use bevy_text::FontWeight;
use bevy_ui::{
    AlignItems, BoxShadow, InteractionDisabled, JustifyContent, Node, PositionType, Pressed,
    UiRect, px,
};
use bevy_ui_widgets::Button;

use crate::{
    constants::{fonts, size},
    cursor::EntityCursor,
    focus::FocusIndicator,
    font_styles::InheritableFont,
    rounded_corners::RoundedCorners,
    theme::{
        Flat, GRADIENT_AMOUNT, InheritableThemeTextColor, ThemeBackgroundGradient,
        ThemeBorderColor, control_box_shadow,
    },
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
    /// A bordered button with no fill at rest: a secondary action that stays legible on any
    /// surface, where [`Normal`](Self::Normal) would read as grey-on-grey.
    Outline,
}

/// A button, spawnable as a scene component with optional [`PlumeButtonProps`].
/// Emits [`bevy_ui_widgets::Activate`] on release while hovered.
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

impl ButtonVariant {
    /// Whether this variant paints a solid background at rest. The unfilled variants swap the
    /// resting drop shadow for a hover/press-only one.
    fn filled(&self) -> bool {
        !matches!(self, ButtonVariant::Plain | ButtonVariant::Outline)
    }
}

impl PlumeButton {
    fn scene(props: PlumeButtonProps) -> impl Scene {
        let box_shadow = props
            .variant
            .filled()
            .then(|| bsn! { template_value(control_box_shadow()) });
        let corners = props.corners;
        bsn! {
            Node {
                height: size::ROW_HEIGHT,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: size::GAP,
                padding: UiRect::horizontal(size::GAP),
                border_radius: {corners.to_border_radius(size::CORNER_RADIUS)},
            }
            Button
            template_value(props.variant)
            {box_shadow}
            Hovered
            TabIndex(0)
            FocusIndicator
            EntityCursor::System(bevy_window::SystemCursorIcon::Pointer)
            ThemeBackgroundGradient(tokens::BUTTON_BG, GRADIENT_AMOUNT)
            InheritableThemeTextColor(tokens::BUTTON_TEXT)
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::MEDIUM_FONT,
                weight: FontWeight::NORMAL,
            }
            Children [
                (
                    // The border lives on an overlay child rather than on the button node: drawn
                    // over the fill, it leaves no seam between body and border the way a node's
                    // own inset border does.
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        right: px(0),
                        top: px(0),
                        bottom: px(0),
                        border: size::CONTROL_BORDER,
                        border_radius: {corners.to_border_radius(size::CORNER_RADIUS)},
                    }
                    ButtonOutline
                    Pickable::IGNORE
                    ThemeBorderColor(tokens::BUTTON_BORDER_NONE)
                ),
                {props.caption}
            ]
        }
    }
}

/// Marker for a button's border overlay. Every button carries one; only
/// [`ButtonVariant::Outline`] paints it, so a runtime variant swap needs no respawn.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ButtonOutline;

/// A smaller button for embedding in panel headers, spawnable as a scene
/// component with optional [`PlumeButtonProps`]. Emits [`bevy_ui_widgets::Activate`].
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
            Has<Flat>,
            &ThemeBackgroundGradient,
            &InheritableThemeTextColor,
            Has<BoxShadow>,
        ),
        Or<(
            Changed<Hovered>,
            Changed<ButtonVariant>,
            Added<Pressed>,
            Added<InteractionDisabled>,
            Added<Flat>,
        )>,
    >,
    q_children: Query<&Children>,
    q_outline: Query<&ThemeBorderColor, With<ButtonOutline>>,
    mut commands: Commands,
) {
    for (
        button_ent,
        variant,
        disabled,
        pressed,
        hovered,
        flat,
        bg_color,
        font_color,
        has_box_shadow,
    ) in q_buttons.iter()
    {
        set_button_styles(
            button_ent,
            variant,
            disabled,
            pressed,
            hovered.0,
            flat,
            bg_color,
            font_color,
            has_box_shadow,
            outline_child(button_ent, &q_children, &q_outline),
            &mut commands,
        );
    }
}

/// The button's border overlay child, if it has spawned yet.
fn outline_child<'a>(
    button_ent: Entity,
    q_children: &Query<&Children>,
    q_outline: &'a Query<&ThemeBorderColor, With<ButtonOutline>>,
) -> Option<(Entity, &'a ThemeBorderColor)> {
    q_children
        .get(button_ent)
        .ok()?
        .iter()
        .find_map(|&child| Some((child, q_outline.get(child).ok()?)))
}

fn update_button_styles_remove(
    q_buttons: Query<(
        Entity,
        &ButtonVariant,
        Has<InteractionDisabled>,
        Has<Pressed>,
        &Hovered,
        Has<Flat>,
        &ThemeBackgroundGradient,
        &InheritableThemeTextColor,
        Has<BoxShadow>,
    )>,
    q_children: Query<&Children>,
    q_outline: Query<&ThemeBorderColor, With<ButtonOutline>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut removed_flat: RemovedComponents<Flat>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_pressed.read())
        .chain(removed_flat.read())
        .for_each(|ent| {
            if let Ok((
                button_ent,
                variant,
                disabled,
                pressed,
                hovered,
                flat,
                bg_color,
                font_color,
                has_box_shadow,
            )) = q_buttons.get(ent)
            {
                set_button_styles(
                    button_ent,
                    variant,
                    disabled,
                    pressed,
                    hovered.0,
                    flat,
                    bg_color,
                    font_color,
                    has_box_shadow,
                    outline_child(button_ent, &q_children, &q_outline),
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
    flat: bool,
    bg_color: &ThemeBackgroundGradient,
    font_color: &InheritableThemeTextColor,
    has_box_shadow: bool,
    outline: Option<(Entity, &ThemeBorderColor)>,
    commands: &mut Commands,
) {
    let bg_set = match variant {
        ButtonVariant::Normal => tokens::sets::BUTTON_BG,
        ButtonVariant::Primary => tokens::sets::BUTTON_PRIMARY_BG,
        ButtonVariant::Plain => tokens::sets::BUTTON_PLAIN_BG,
        ButtonVariant::Outline => tokens::sets::BUTTON_OUTLINE_BG,
    };
    let bg_token = bg_set.pick(disabled, pressed, hovered);
    // Disabled buttons read as inert: flat fill, no gradient.
    let bg_gradient_amount = if disabled || flat {
        0.0
    } else {
        GRADIENT_AMOUNT
    };

    let border_token = match variant {
        ButtonVariant::Outline => {
            tokens::sets::BUTTON_OUTLINE_BORDER.pick(disabled, pressed, hovered)
        }
        _ => tokens::BUTTON_BORDER_NONE,
    };

    let font_color_token = match (variant, disabled) {
        (ButtonVariant::Primary, true) => tokens::BUTTON_PRIMARY_TEXT_DISABLED,
        (ButtonVariant::Primary, false) => tokens::BUTTON_PRIMARY_TEXT,
        (_, true) => tokens::BUTTON_TEXT_DISABLED,
        (_, false) => tokens::BUTTON_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
    };

    if bg_color.0 != bg_token || bg_color.1 != bg_gradient_amount {
        commands
            .entity(button_ent)
            .insert(ThemeBackgroundGradient(bg_token, bg_gradient_amount));
    }

    if font_color.0 != font_color_token {
        commands
            .entity(button_ent)
            .insert(InheritableThemeTextColor(font_color_token));
    }

    if let Some((outline_ent, outline_color)) = outline
        && outline_color.0 != border_token
    {
        commands
            .entity(outline_ent)
            .insert(ThemeBorderColor(border_token));
    }

    let should_have_box_shadow = (variant.filled() || hovered || pressed) && !disabled && !flat;
    if should_have_box_shadow && !has_box_shadow {
        commands.entity(button_ent).insert(control_box_shadow());
    } else if !should_have_box_shadow && has_box_shadow {
        commands.entity(button_ent).remove::<BoxShadow>();
    }

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
