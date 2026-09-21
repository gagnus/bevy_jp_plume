//! Labeled checkbox control.
use bevy::app::{Plugin, PreUpdate};
use bevy::camera::visibility::Visibility;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::Children;
use bevy::ecs::lifecycle::RemovedComponents;
use bevy::ecs::query::{Added, Has, Or, With};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query};
use bevy::ecs::template::FromTemplate;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::math::Rot2;
use bevy::picking::{Pickable, PickingSystems};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::ui::{
    AlignItems, BoxShadow, Checked, Display, FlexDirection, InteractionDisabled, JustifyContent,
    Node, PositionType, UiRect, UiTransform,
};
use bevy::ui_widgets::{Checkbox, checkbox_self_update};

use crate::constants::size;
use crate::cursor::EntityCursor;
use crate::focus::FocusIndicator;
use crate::theme::{
    GradientAmount, InheritableThemeTextToken, ThemeBackgroundToken, ThemeBorderToken,
    control_box_shadow,
};
use crate::tokens;
use crate::utils::anim::AnimState;
use crate::utils::hierarchy::descendant_get;

/// A checkbox, spawnable as a scene component with optional [`PlumeCheckboxProps`].
/// Emits [`bevy::ui_widgets::ValueChange<bool>`] with the new state.
#[derive(SceneComponent, FromTemplate)]
#[scene(PlumeCheckboxProps)]
#[derive(Reflect)]
#[reflect(Component)]
pub struct PlumeCheckbox;

/// Props used to construct a [`PlumeCheckbox`] scene.
pub struct PlumeCheckboxProps {
    /// Label for this checkbox. This can contain multiple entities, which will be contained
    /// in a flexbox.
    pub caption: Box<dyn SceneList>,
}

impl Default for PlumeCheckboxProps {
    fn default() -> Self {
        Self {
            caption: Box::new(bsn_list! {}),
        }
    }
}

impl PlumeCheckbox {
    fn scene(props: PlumeCheckboxProps) -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Start,
                align_items: AlignItems::Center,
                column_gap: size::SPACE,
                min_height: size::ROW_HEIGHT,
            }
            Checkbox
            CheckboxFrame
            TabIndex(0)
            // The row stretches to its container, but only the box and label react to
            // clicks (children stay pickable and bubble up); the dead space is inert.
            Pickable::IGNORE
            on(checkbox_self_update)
            EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)
            InheritableThemeTextToken(tokens::CHECKBOX_TEXT)
            Children [
                Node {
                    width: size::CHECKBOX_SIZE,
                    height: size::CHECKBOX_SIZE,
                    border_radius: size::CORNER_RADIUS_SMALL,
                }
                CheckboxBg
                // Ring hugs the box, not the label row.
                FocusIndicator
                ThemeBackgroundToken(tokens::CHECKBOX_BG)
                GradientAmount(0.0)
                Children [
                    Node {
                        width: size::CHECKBOX_SIZE,
                        height: size::CHECKBOX_SIZE,
                        border: size::HAIRLINE,
                        border_radius: size::CORNER_RADIUS_SMALL,
                    }
                    CheckboxOutline
                    ThemeBorderToken(tokens::CHECKBOX_BORDER)
                    --
                    // Cheesy checkmark: rotated node with L-shaped border,
                    // proportioned to the box so it scales with it.
                    Node {
                        position_type: PositionType::Absolute,
                        left: size::em_from_px(6.0),
                        top: size::em_from_px(2.0),
                        width: size::em_from_px(6.0),
                        height: size::em_from_px(11.0),
                        border: {UiRect {
                            bottom: size::em_from_px(2.0),
                            right: size::em_from_px(2.0),
                            ..UiRect::ZERO
                        }},
                    }
                    UiTransform::from_rotation(Rot2::FRAC_PI_4)
                    CheckboxMark
                    AnimState::scale(0.0, 1.0).hide_at_zero()
                    Visibility::Hidden
                    ThemeBorderToken(tokens::CHECKBOX_MARK)
                ]
                --
                {props.caption}
            ]
        }
    }
}

// Marker for the checkbox frame (contains both checkbox and label).
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct CheckboxFrame;

// Marker for the checkbox bg.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct CheckboxBg;

// Marker for the checkbox outline.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct CheckboxOutline;

// Marker for the checkbox check mark.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct CheckboxMark;

fn update_checkbox_styles(
    q_checkboxes: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            &InheritableThemeTextToken,
        ),
        (
            With<CheckboxFrame>,
            // Added<CheckboxFrame> guarantees the initial style pass on spawn.
            Or<(
                Added<CheckboxFrame>,
                Added<Checked>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    q_bg: Query<
        (
            Entity,
            &ThemeBackgroundToken,
            &GradientAmount,
            Has<BoxShadow>,
        ),
        With<CheckboxBg>,
    >,
    q_outline: Query<(Entity, &ThemeBorderToken), With<CheckboxOutline>>,
    q_mark: Query<(Entity, &ThemeBorderToken), With<CheckboxMark>>,
    mut q_mark_anim: Query<&mut AnimState, With<CheckboxMark>>,
    mut commands: Commands,
) {
    for (checkbox_ent, disabled, checked, font_color) in q_checkboxes.iter() {
        apply_checkbox_styles(
            checkbox_ent,
            disabled,
            checked,
            font_color,
            &q_children,
            &q_bg,
            &q_outline,
            &q_mark,
            &mut q_mark_anim,
            &mut commands,
        );
    }
}

fn update_checkbox_styles_remove(
    q_checkboxes: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            &InheritableThemeTextToken,
        ),
        With<CheckboxFrame>,
    >,
    q_children: Query<&Children>,
    q_bg: Query<
        (
            Entity,
            &ThemeBackgroundToken,
            &GradientAmount,
            Has<BoxShadow>,
        ),
        With<CheckboxBg>,
    >,
    q_outline: Query<(Entity, &ThemeBorderToken), With<CheckboxOutline>>,
    q_mark: Query<(Entity, &ThemeBorderToken), With<CheckboxMark>>,
    mut q_mark_anim: Query<&mut AnimState, With<CheckboxMark>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_checked: RemovedComponents<Checked>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_checked.read())
        .for_each(|ent| {
            if let Ok((checkbox_ent, disabled, checked, font_color)) = q_checkboxes.get(ent) {
                apply_checkbox_styles(
                    checkbox_ent,
                    disabled,
                    checked,
                    font_color,
                    &q_children,
                    &q_bg,
                    &q_outline,
                    &q_mark,
                    &mut q_mark_anim,
                    &mut commands,
                );
            }
        });
}

// Resolve the checkbox's child entities and push the current styles onto them.
fn apply_checkbox_styles(
    checkbox_ent: Entity,
    disabled: bool,
    checked: bool,
    font_color: &InheritableThemeTextToken,
    q_children: &Query<&Children>,
    q_bg: &Query<
        (
            Entity,
            &ThemeBackgroundToken,
            &GradientAmount,
            Has<BoxShadow>,
        ),
        With<CheckboxBg>,
    >,
    q_outline: &Query<(Entity, &ThemeBorderToken), With<CheckboxOutline>>,
    q_mark: &Query<(Entity, &ThemeBorderToken), With<CheckboxMark>>,
    q_mark_anim: &mut Query<&mut AnimState, With<CheckboxMark>>,
    commands: &mut Commands,
) {
    let Some((bg_ent, bg_token, bg_amount, has_box_shadow)) =
        descendant_get(checkbox_ent, q_children, q_bg)
    else {
        return;
    };
    let Some((outline_ent, outline_color)) = descendant_get(checkbox_ent, q_children, q_outline)
    else {
        return;
    };
    let Some((mark_ent, mark_color)) = descendant_get(checkbox_ent, q_children, q_mark) else {
        return;
    };

    // Drive the pop: the tick eases to full scale (and shows) while checked.
    if let Ok(mut mark_anim) = q_mark_anim.get_mut(mark_ent) {
        mark_anim.set_target(if checked { 1.0 } else { 0.0 });
    }
    set_checkbox_styles(
        checkbox_ent,
        bg_ent,
        outline_ent,
        mark_ent,
        disabled,
        checked,
        bg_token,
        bg_amount,
        outline_color,
        mark_color,
        font_color,
        has_box_shadow,
        commands,
    );
}

fn set_checkbox_styles(
    checkbox_ent: Entity,
    bg_ent: Entity,
    outline_ent: Entity,
    mark_ent: Entity,
    disabled: bool,
    checked: bool,
    bg_token_now: &ThemeBackgroundToken,
    bg_amount_now: &GradientAmount,
    outline_color: &ThemeBorderToken,
    mark_color: &ThemeBorderToken,
    font_color: &InheritableThemeTextToken,
    has_box_shadow: bool,
    commands: &mut Commands,
) {
    let outline_token = tokens::sets::CHECKBOX_BORDER.pick(checked, disabled);
    let bg_token = tokens::sets::CHECKBOX_BG.pick(checked, disabled);

    let mark_token = match disabled {
        true => tokens::CHECKBOX_MARK_DISABLED,
        false => tokens::CHECKBOX_MARK,
    };

    let font_color_token = match disabled {
        true => tokens::CHECKBOX_TEXT_DISABLED,
        false => tokens::CHECKBOX_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy::window::SystemCursorIcon::NotAllowed,
        false => bevy::window::SystemCursorIcon::Pointer,
    };

    // Gradient only when ticked; the unticked fill is transparent.
    let bg_amount = if checked {
        GradientAmount::STANDARD
    } else {
        GradientAmount(0.0)
    };
    if bg_token_now.0 != bg_token {
        commands
            .entity(bg_ent)
            .insert(ThemeBackgroundToken(bg_token));
    }
    if *bg_amount_now != bg_amount {
        commands.entity(bg_ent).insert(bg_amount);
    }

    if outline_color.0 != outline_token {
        commands
            .entity(outline_ent)
            .insert(ThemeBorderToken(outline_token));
    }

    if mark_color.0 != mark_token {
        commands
            .entity(mark_ent)
            .insert(ThemeBorderToken(mark_token));
    }

    if font_color.0 != font_color_token {
        commands
            .entity(checkbox_ent)
            .insert(InheritableThemeTextToken(font_color_token));
    }

    let should_have_box_shadow = checked && !disabled;
    if should_have_box_shadow && !has_box_shadow {
        commands.entity(bg_ent).insert(control_box_shadow());
    } else if !should_have_box_shadow && has_box_shadow {
        commands.entity(bg_ent).remove::<BoxShadow>();
    }

    commands
        .entity(checkbox_ent)
        .insert(EntityCursor::System(cursor_shape));
}

// Plugin which registers the systems for updating the checkbox styles.
pub(crate) struct CheckboxPlugin;

impl Plugin for CheckboxPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            (update_checkbox_styles, update_checkbox_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
