//! Labeled checkbox control.
use bevy_app::{Plugin, PreUpdate};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
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
use bevy_math::Rot2;
use bevy_picking::PickingSystems;
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::prelude::*;
use bevy_text::FontWeight;
use bevy_ui::{
    AlignItems, Checked, Display, FlexDirection, InteractionDisabled, JustifyContent, Node,
    PositionType, UiRect, UiTransform, px,
};
use bevy_ui_widgets::{Checkbox, checkbox_self_update};

use crate::{
    constants::{fonts, size},
    cursor::EntityCursor,
    font_styles::InheritableFont,
    theme::{
        GRADIENT_AMOUNT, InheritableThemeTextColor, ThemeBackgroundGradient, ThemeBorderColor,
    },
    tokens,
};

/// A checkbox widget.
///
/// This is spawnable by inheriting it as a "scene component" with optional [`PlumeCheckboxProps`].
///
/// Emits [`bevy_ui_widgets::ValueChange<bool>`] with the new value when the checkbox changes
/// state; disabled by adding [`bevy_ui::InteractionDisabled`].
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
                column_gap: size::GAP_TIGHT,
                min_height: size::ROW_HEIGHT,
            }
            Checkbox
            CheckboxFrame
            on(checkbox_self_update)
            EntityCursor::System(bevy_window::SystemCursorIcon::Pointer)
            InheritableThemeTextColor(tokens::CHECKBOX_TEXT)
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::MEDIUM_FONT,
                weight: FontWeight::NORMAL,
            }
            Children [
                (
                    Node {
                        width: size::CHECKBOX_SIZE,
                        height: size::CHECKBOX_SIZE,
                        border_radius: px(size::CORNER_RADIUS),
                    }
                    CheckboxBg
                    ThemeBackgroundGradient(tokens::CHECKBOX_BG, 0.0)
                    Children [
                        (
                            Node {
                                width: size::CHECKBOX_SIZE,
                                height: size::CHECKBOX_SIZE,
                                border: size::CONTROL_BORDER,
                                border_radius: px(size::CORNER_RADIUS),
                            }
                            CheckboxOutline
                            ThemeBorderColor(tokens::CHECKBOX_BORDER)
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
                            Visibility::Hidden
                            ThemeBorderColor(tokens::CHECKBOX_MARK)
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
            &InheritableThemeTextColor,
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
    mut q_bg: Query<&ThemeBackgroundGradient, With<CheckboxBg>>,
    mut q_outline: Query<&ThemeBorderColor, With<CheckboxOutline>>,
    mut q_mark: Query<&ThemeBorderColor, With<CheckboxMark>>,
    mut commands: Commands,
) {
    for (checkbox_ent, disabled, checked, font_color) in q_checkboxes.iter() {
        let Some(bg_ent) = q_children
            .iter_descendants(checkbox_ent)
            .find(|en| q_bg.contains(*en))
        else {
            continue;
        };
        let Some(outline_ent) = q_children
            .iter_descendants(checkbox_ent)
            .find(|en| q_outline.contains(*en))
        else {
            continue;
        };
        let Some(mark_ent) = q_children
            .iter_descendants(checkbox_ent)
            .find(|en| q_mark.contains(*en))
        else {
            continue;
        };
        let bg_color = q_bg.get_mut(bg_ent).unwrap();
        let outline_color = q_outline.get_mut(outline_ent).unwrap();
        let mark_color = q_mark.get_mut(mark_ent).unwrap();
        set_checkbox_styles(
            checkbox_ent,
            bg_ent,
            outline_ent,
            mark_ent,
            disabled,
            checked,
            bg_color,
            outline_color,
            mark_color,
            font_color,
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
            &InheritableThemeTextColor,
        ),
        With<CheckboxFrame>,
    >,
    q_children: Query<&Children>,
    mut q_bg: Query<&ThemeBackgroundGradient, With<CheckboxBg>>,
    mut q_outline: Query<&ThemeBorderColor, With<CheckboxOutline>>,
    mut q_mark: Query<&ThemeBorderColor, With<CheckboxMark>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_checked: RemovedComponents<Checked>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_checked.read())
        .for_each(|ent| {
            if let Ok((checkbox_ent, disabled, checked, font_color)) = q_checkboxes.get(ent) {
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
                let bg_color = q_bg.get_mut(bg_ent).unwrap();
                let outline_color = q_outline.get_mut(outline_ent).unwrap();
                let mark_color = q_mark.get_mut(mark_ent).unwrap();
                set_checkbox_styles(
                    checkbox_ent,
                    bg_ent,
                    outline_ent,
                    mark_ent,
                    disabled,
                    checked,
                    bg_color,
                    outline_color,
                    mark_color,
                    font_color,
                    &mut commands,
                );
            }
        });
}

fn set_checkbox_styles(
    checkbox_ent: Entity,
    bg_ent: Entity,
    outline_ent: Entity,
    mark_ent: Entity,
    disabled: bool,
    checked: bool,
    bg_color: &ThemeBackgroundGradient,
    outline_color: &ThemeBorderColor,
    mark_color: &ThemeBorderColor,
    font_color: &InheritableThemeTextColor,
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
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
    };

    // Change background: gradient only when ticked, flat fill otherwise.
    let bg_gradient_amount = if checked { GRADIENT_AMOUNT } else { 0.0 };
    if bg_color.0 != bg_token || bg_color.1 != bg_gradient_amount {
        commands
            .entity(bg_ent)
            .insert(ThemeBackgroundGradient(bg_token, bg_gradient_amount));
    }

    // Change outline border
    if outline_color.0 != outline_token {
        commands
            .entity(outline_ent)
            .insert(ThemeBorderColor(outline_token));
    }

    // Change mark color
    if mark_color.0 != mark_token {
        commands
            .entity(mark_ent)
            .insert(ThemeBorderColor(mark_token));
    }

    // Change mark visibility
    commands.entity(mark_ent).insert(match checked {
        true => Visibility::Inherited,
        false => Visibility::Hidden,
    });

    // Change font color
    if font_color.0 != font_color_token {
        commands
            .entity(checkbox_ent)
            .insert(InheritableThemeTextColor(font_color_token));
    }

    // Change cursor shape
    commands
        .entity(checkbox_ent)
        .insert(EntityCursor::System(cursor_shape));
}

/// Plugin which registers the systems for updating the checkbox styles.
pub struct CheckboxPlugin;

impl Plugin for CheckboxPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_checkbox_styles, update_checkbox_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
