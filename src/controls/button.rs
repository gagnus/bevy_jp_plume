//! Styled button controls.
use bevy::app::{Plugin, PreUpdate};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::Children;
use bevy::ecs::lifecycle::RemovedComponents;
use bevy::ecs::observer::On;
use bevy::ecs::query::{Added, Changed, Has, Or, With};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::picking::{Pickable, PickingSystems};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::ui::{
    AlignItems, BoxShadow, Checkable, Checked, InteractionDisabled, JustifyContent, Node,
    PositionType, Pressed, UiRect, Val,
};
use bevy::ui_widgets::{Activate, Button, ValueChange};

use crate::constants::size;
use crate::controls::SetValue;
use crate::cursor::EntityCursor;
use crate::focus::FocusIndicator;
use crate::font_styles::TextStyleRelay;
use crate::rounded_corners::RoundedCorners;
use crate::theme::{
    Flat, GradientAmount, Inert, InheritableThemeTextToken, ThemeBackgroundToken, ThemeBorderToken,
    control_box_shadow,
};
use crate::tokens;

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
    /// As prominent as [`Primary`](Self::Primary) but in the theme's fixed red: the
    /// confirm button for a destructive action (delete, discard, reset).
    Danger,
    /// Don't display the button background unless hovering or pressed.
    Plain,
    /// A bordered button with no fill at rest: a secondary action that stays legible on any
    /// surface, where [`Normal`](Self::Normal) would read as gray-on-gray.
    Outline,
}

/// Rest-state chrome for a checkable button. The loud variants of [`ButtonVariant`] are
/// absent by construction: they spend their emphasis at rest, leaving checked nothing to
/// say. Checked accents the surface the variant leads with — fill for
/// [`Normal`](Self::Normal), ink for the unfilled two.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum ButtonCheckableVariant {
    /// Filled at rest; checked swaps the fill to the accent.
    #[default]
    Normal,
    /// Unfilled at rest; checked accents the text.
    Plain,
    /// Bordered at rest; checked accents the border and text.
    Outline,
}

impl From<ButtonCheckableVariant> for ButtonVariant {
    fn from(variant: ButtonCheckableVariant) -> Self {
        match variant {
            ButtonCheckableVariant::Normal => ButtonVariant::Normal,
            ButtonCheckableVariant::Plain => ButtonVariant::Plain,
            ButtonCheckableVariant::Outline => ButtonVariant::Outline,
        }
    }
}

/// A button, spawnable as a scene component with optional [`PlumeButtonProps`].
/// Emits [`bevy::ui_widgets::Activate`] on release while hovered.
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
    /// If true does not respond with color change hover and pressed
    pub checkable: bool,
}

impl Default for PlumeButtonProps {
    fn default() -> Self {
        Self {
            caption: Box::new(bsn_list![]),
            variant: ButtonVariant::default(),
            corners: Default::default(),
            checkable: false,
        }
    }
}

impl ButtonVariant {
    // Whether this variant paints a solid background at rest. The unfilled variants swap the
    // resting drop shadow for a hover/press-only one.
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
                column_gap: size::SPACE,
                padding: UiRect::horizontal(size::SPACE),
                border_radius: {corners.to_border_radius(size::CORNER_RADIUS)},
            }
            Button
            template_value(props.variant)
            {box_shadow}
            {props.checkable.then(|| bsn! { Checkable })}
            Hovered
            TabIndex(0)
            FocusIndicator
            EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)
            ThemeBackgroundToken(tokens::BUTTON_BG)
            GradientAmount::STANDARD
            InheritableThemeTextToken(tokens::BUTTON_TEXT)
            TextStyleRelay
            Children [
                (
                    // The border lives on an overlay child rather than on the button node: drawn
                    // over the fill, it leaves no seam between body and border the way a node's
                    // own inset border does.
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::ZERO,
                        right: Val::ZERO,
                        top: Val::ZERO,
                        bottom: Val::ZERO,
                        border: size::HAIRLINE,
                        border_radius: {corners.to_border_radius(size::CORNER_RADIUS)},
                    }
                    ButtonOutline
                    // Em-sized chrome needs the chain's `EmSize`.
                    TextStyleRelay
                    Pickable::IGNORE
                    ThemeBorderToken(tokens::BUTTON_BORDER_NONE)
                ),
                {props.caption},
            ]
        }
    }
}

// Marker for a button's border overlay. Every button carries one; only
// [`ButtonVariant::Outline`] paints it, so a runtime variant swap needs no respawn.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct ButtonOutline;

/// A smaller button for embedding in panel headers, spawnable as a scene
/// component with optional [`PlumeButtonProps`]. Emits [`bevy::ui_widgets::Activate`].
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
                @corners: {props.corners},
            }
            Node {
                padding: UiRect::horizontal(size::SPACE_TIGHT),
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
            Has<Checked>,
            Has<Checkable>,
            Has<Flat>,
            Has<Inert>,
            &ThemeBackgroundToken,
            &GradientAmount,
            &InheritableThemeTextToken,
            Has<BoxShadow>,
        ),
        Or<(
            Changed<Hovered>,
            Changed<ButtonVariant>,
            Added<Pressed>,
            Added<InteractionDisabled>,
            Added<Flat>,
            Added<Inert>,
            Added<Checked>,
        )>,
    >,
    q_children: Query<&Children>,
    q_outline: Query<&ThemeBorderToken, With<ButtonOutline>>,
    mut commands: Commands,
) {
    for (
        button_ent,
        variant,
        disabled,
        pressed,
        hovered,
        checked,
        checkable,
        flat,
        inert,
        bg_token,
        bg_amount,
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
            checked,
            checkable,
            flat,
            inert,
            bg_token,
            bg_amount,
            font_color,
            has_box_shadow,
            outline_child(button_ent, &q_children, &q_outline),
            &mut commands,
        );
    }
}

// The button's border overlay child, if it has spawned yet.
fn outline_child<'a>(
    button_ent: Entity,
    q_children: &Query<&Children>,
    q_outline: &'a Query<&ThemeBorderToken, With<ButtonOutline>>,
) -> Option<(Entity, &'a ThemeBorderToken)> {
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
        Has<Checked>,
        Has<Checkable>,
        Has<Flat>,
        Has<Inert>,
        &ThemeBackgroundToken,
        &GradientAmount,
        &InheritableThemeTextToken,
        Has<BoxShadow>,
    )>,
    q_children: Query<&Children>,
    q_outline: Query<&ThemeBorderToken, With<ButtonOutline>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut removed_checked: RemovedComponents<Checked>,
    mut removed_checkable: RemovedComponents<Checkable>,
    mut removed_flat: RemovedComponents<Flat>,
    mut removed_inert: RemovedComponents<Inert>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_pressed.read())
        .chain(removed_flat.read())
        .chain(removed_checked.read())
        .chain(removed_checkable.read())
        .chain(removed_inert.read())
        .for_each(|ent| {
            if let Ok((
                button_ent,
                variant,
                disabled,
                pressed,
                hovered,
                checked,
                checkable,
                flat,
                inert,
                bg_token,
                bg_amount,
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
                    checked,
                    checkable,
                    flat,
                    inert,
                    bg_token,
                    bg_amount,
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
    checked: bool,
    checkable: bool,
    flat: bool,
    inert: bool,
    bg_token_now: &ThemeBackgroundToken,
    bg_amount_now: &GradientAmount,
    font_color: &InheritableThemeTextToken,
    has_box_shadow: bool,
    outline: Option<(Entity, &ThemeBorderToken)>,
    commands: &mut Commands,
) {
    // Checked accents the surface the variant already leads with: the fill for the
    // filled variants, the ink for the unfilled ones, which stay unfilled so that
    // hover and press keep the fill to themselves.
    let on = checkable && checked;

    let bg_set = match variant {
        ButtonVariant::Normal if on => tokens::sets::BUTTON_CHECKED_BG,
        ButtonVariant::Normal => tokens::sets::BUTTON_BG,
        ButtonVariant::Primary => tokens::sets::BUTTON_PRIMARY_BG,
        ButtonVariant::Danger => tokens::sets::BUTTON_DANGER_BG,
        ButtonVariant::Plain => tokens::sets::BUTTON_PLAIN_BG,
        ButtonVariant::Outline => tokens::sets::BUTTON_OUTLINE_BG,
    };
    let bg_token = bg_set.pick(disabled, pressed && hovered && !inert, hovered && !inert);

    let border_token = match variant {
        ButtonVariant::Outline if on => tokens::sets::BUTTON_OUTLINE_BORDER_CHECKED.pick(
            disabled,
            pressed && hovered && !inert,
            hovered && !inert,
        ),
        ButtonVariant::Outline => tokens::sets::BUTTON_OUTLINE_BORDER.pick(
            disabled,
            pressed && hovered && !inert,
            hovered && !inert,
        ),
        _ => tokens::BUTTON_BORDER_NONE,
    };

    let text_token = match variant {
        // The unfilled variants carry the checked state in their ink, so a checkable
        // one sits at dim text when off for the accent to read as on against.
        ButtonVariant::Plain if checkable => tokens::sets::BUTTON_PLAIN_TEXT.pick(on, disabled),
        ButtonVariant::Outline if checkable => tokens::sets::BUTTON_OUTLINE_TEXT.pick(on, disabled),
        ButtonVariant::Normal if on => match disabled {
            true => tokens::BUTTON_CHECKED_TEXT_DISABLED,
            false => tokens::BUTTON_CHECKED_TEXT,
        },
        ButtonVariant::Primary => match disabled {
            true => tokens::BUTTON_PRIMARY_TEXT_DISABLED,
            false => tokens::BUTTON_PRIMARY_TEXT,
        },
        ButtonVariant::Danger => match disabled {
            true => tokens::BUTTON_DANGER_TEXT_DISABLED,
            false => tokens::BUTTON_DANGER_TEXT,
        },
        _ => match disabled {
            true => tokens::BUTTON_TEXT_DISABLED,
            false => tokens::BUTTON_TEXT,
        },
    };

    // Disabled buttons read as dead: flat fill, no gradient. A `Flat` button is
    // flattened by the theme layer, so the marker plays no part here.
    let bg_amount = if disabled {
        GradientAmount(0.0)
    } else {
        GradientAmount::STANDARD
    };

    let cursor_shape = match disabled {
        true => bevy::window::SystemCursorIcon::NotAllowed,
        false => bevy::window::SystemCursorIcon::Pointer,
    };

    if bg_token_now.0 != bg_token {
        commands
            .entity(button_ent)
            .insert(ThemeBackgroundToken(bg_token));
    }

    if *bg_amount_now != bg_amount {
        commands.entity(button_ent).insert(bg_amount);
    }

    if font_color.0 != text_token {
        commands
            .entity(button_ent)
            .insert(InheritableThemeTextToken(text_token));
    }

    if let Some((outline_ent, outline_color)) = outline
        && outline_color.0 != border_token
    {
        commands
            .entity(outline_ent)
            .insert(ThemeBorderToken(border_token));
    }

    // An inert button's unfilled variants stay shadowless: the lift is the same
    // hover/press promise the fill was suppressed for.
    let lifted = !inert && (hovered || pressed);
    let should_have_box_shadow = (variant.filled() || lifted) && !disabled && !flat;
    if should_have_box_shadow && !has_box_shadow {
        commands.entity(button_ent).insert(control_box_shadow());
    } else if !should_have_box_shadow && has_box_shadow {
        commands.entity(button_ent).remove::<BoxShadow>();
    }

    commands
        .entity(button_ent)
        .insert(EntityCursor::System(cursor_shape));
}

// Picking a checkable button toggles its check besides the headless `Activate`.
fn button_toggle_check(
    ev: On<Activate>,
    q_buttons: Query<
        (Has<Checked>, Has<InteractionDisabled>),
        (With<ButtonVariant>, With<Checkable>),
    >,
    mut commands: Commands,
) {
    let Ok((checked, disabled)) = q_buttons.get(ev.entity) else {
        return;
    };
    if disabled {
        return;
    }
    if checked {
        commands.entity(ev.entity).remove::<Checked>();
    } else {
        commands.entity(ev.entity).insert(Checked);
    }
    commands.trigger(ValueChange {
        source: ev.entity,
        value: !checked,
        is_final: true,
    });
}

// Programmatic checked state, the counterpart of the emitted `ValueChange<bool>`.
fn button_on_set_checked(
    ev: On<SetValue<bool>>,
    q_buttons: Query<Has<Checked>, (With<ButtonVariant>, With<Checkable>)>,
    mut commands: Commands,
) {
    let Ok(checked) = q_buttons.get(ev.entity) else {
        return;
    };
    if ev.value == checked {
        return;
    }
    if ev.value {
        commands.entity(ev.entity).insert(Checked);
    } else {
        commands.entity(ev.entity).remove::<Checked>();
    }
}

// Plugin which registers the systems for updating the button styles.
pub(crate) struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            (update_button_styles, update_button_styles_remove).in_set(PickingSystems::Last),
        )
        .add_observer(button_toggle_check)
        .add_observer(button_on_set_checked);
    }
}
