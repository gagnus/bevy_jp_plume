//! Labeled checkbox control.
use bevy::app::{Plugin, PreUpdate};
use bevy::camera::visibility::Visibility;
use bevy::ecs::{
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
    template::FromTemplate,
};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::math::Rot2;
use bevy::picking::{Pickable, PickingSystems};
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::scene::prelude::*;
use bevy::ui::{
    AlignItems, BoxShadow, Checked, Display, FlexDirection, InteractionDisabled, JustifyContent,
    Node, PositionType, UiRect, UiTransform, px,
};
use bevy::ui_widgets::{Checkbox, checkbox_self_update};

use crate::{
    constants::size,
    cursor::EntityCursor,
    focus::FocusIndicator,
    font_styles::TextStyleRelay,
    theme::{
        Flat, GRADIENT_AMOUNT, InheritableThemeTextToken, ThemeBackgroundGradient,
        ThemeBorderToken, control_box_shadow,
    },
    tokens,
    utils::anim::AnimState,
};

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
            caption: Box::new(bsn_list!()),
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
                column_gap: size::GAP,
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
            TextStyleRelay
            Children [
                (
                    Node {
                        width: size::CHECKBOX_SIZE,
                        height: size::CHECKBOX_SIZE,
                        border_radius: size::CORNER_RADIUS_SMALL,
                    }
                    CheckboxBg
                    // Ring hugs the box, not the label row.
                    FocusIndicator
                    ThemeBackgroundGradient(tokens::CHECKBOX_BG, 0.0)
                    Children [
                        (
                            Node {
                                width: size::CHECKBOX_SIZE,
                                height: size::CHECKBOX_SIZE,
                                border: size::CONTROL_BORDER,
                                border_radius: size::CORNER_RADIUS_SMALL,
                            }
                            CheckboxOutline
                            ThemeBorderToken(tokens::CHECKBOX_BORDER)
                        ),
                        (
                            // Cheesy checkmark: rotated node with L-shaped border.
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(6),
                                top: px(2),
                                width: px(6),
                                height: px(11),
                                border: UiRect {
                                    bottom: px(2),
                                    right: px(2),
                                },
                            }
                            UiTransform::from_rotation(Rot2::FRAC_PI_4)
                            CheckboxMark
                            template_value(AnimState::scale(0.0, 1.0).hide_at_zero())
                            Visibility::Hidden
                            ThemeBorderToken(tokens::CHECKBOX_MARK)
                        )
                    ]
                ),
                {props.caption}
            ]
        }
    }
}

/// Marker for the checkbox frame (contains both checkbox and label)
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct CheckboxFrame;

/// Marker for the checkbox bg
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct CheckboxBg;

/// Marker for the checkbox outline
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct CheckboxOutline;

/// Marker for the checkbox check mark
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct CheckboxMark;

fn update_checkbox_styles(
    q_checkboxes: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            Has<Flat>,
            &InheritableThemeTextToken,
        ),
        (
            With<CheckboxFrame>,
            // Added<CheckboxFrame> guarantees the initial style pass on spawn.
            Or<(
                Added<CheckboxFrame>,
                Added<Checked>,
                Added<InteractionDisabled>,
                Added<Flat>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    q_bg: Query<(&ThemeBackgroundGradient, Has<BoxShadow>), With<CheckboxBg>>,
    q_outline: Query<&ThemeBorderToken, With<CheckboxOutline>>,
    q_mark: Query<&ThemeBorderToken, With<CheckboxMark>>,
    mut q_mark_anim: Query<&mut AnimState, With<CheckboxMark>>,
    mut commands: Commands,
) {
    for (checkbox_ent, disabled, checked, flat, font_color) in q_checkboxes.iter() {
        apply_checkbox_styles(
            checkbox_ent,
            disabled,
            checked,
            flat,
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
            Has<Flat>,
            &InheritableThemeTextToken,
        ),
        With<CheckboxFrame>,
    >,
    q_children: Query<&Children>,
    q_bg: Query<(&ThemeBackgroundGradient, Has<BoxShadow>), With<CheckboxBg>>,
    q_outline: Query<&ThemeBorderToken, With<CheckboxOutline>>,
    q_mark: Query<&ThemeBorderToken, With<CheckboxMark>>,
    mut q_mark_anim: Query<&mut AnimState, With<CheckboxMark>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_checked: RemovedComponents<Checked>,
    mut removed_flat: RemovedComponents<Flat>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_checked.read())
        .chain(removed_flat.read())
        .for_each(|ent| {
            if let Ok((checkbox_ent, disabled, checked, flat, font_color)) = q_checkboxes.get(ent) {
                apply_checkbox_styles(
                    checkbox_ent,
                    disabled,
                    checked,
                    flat,
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

/// Resolve the checkbox's child entities and push the current styles onto them.
fn apply_checkbox_styles(
    checkbox_ent: Entity,
    disabled: bool,
    checked: bool,
    flat: bool,
    font_color: &InheritableThemeTextToken,
    q_children: &Query<&Children>,
    q_bg: &Query<(&ThemeBackgroundGradient, Has<BoxShadow>), With<CheckboxBg>>,
    q_outline: &Query<&ThemeBorderToken, With<CheckboxOutline>>,
    q_mark: &Query<&ThemeBorderToken, With<CheckboxMark>>,
    q_mark_anim: &mut Query<&mut AnimState, With<CheckboxMark>>,
    commands: &mut Commands,
) {
    let Some(bg_ent) = q_children
        .iter_descendants(checkbox_ent)
        .find(|en| q_bg.contains(*en))
    else {
        return;
    };
    let Some(outline_ent) = q_children
        .iter_descendants(checkbox_ent)
        .find(|en| q_outline.contains(*en))
    else {
        return;
    };
    let Some(mark_ent) = q_children
        .iter_descendants(checkbox_ent)
        .find(|en| q_mark.contains(*en))
    else {
        return;
    };

    // Drive the pop: the tick eases to full scale (and shows) while checked.
    if let Ok(mut mark_anim) = q_mark_anim.get_mut(mark_ent) {
        mark_anim.set_target(if checked { 1.0 } else { 0.0 });
    }
    let (bg_color, has_box_shadow) = q_bg
        .get(bg_ent)
        .expect("bg entity was just found via q_bg::contains");
    let outline_color = q_outline
        .get(outline_ent)
        .expect("outline entity was just found via q_outline::contains");
    let mark_color = q_mark
        .get(mark_ent)
        .expect("mark entity was just found via q_mark::contains");
    set_checkbox_styles(
        checkbox_ent,
        bg_ent,
        outline_ent,
        mark_ent,
        disabled,
        checked,
        flat,
        bg_color,
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
    flat: bool,
    bg_color: &ThemeBackgroundGradient,
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

    // Gradient only when ticked, flat fill otherwise.
    let bg_gradient_amount = if checked && !flat {
        GRADIENT_AMOUNT
    } else {
        0.0
    };
    if bg_color.0 != bg_token || bg_color.1 != bg_gradient_amount {
        commands
            .entity(bg_ent)
            .insert(ThemeBackgroundGradient(bg_token, bg_gradient_amount));
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

/// Plugin which registers the systems for updating the checkbox styles.
pub(crate) struct CheckboxPlugin;

impl Plugin for CheckboxPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            (update_checkbox_styles, update_checkbox_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
