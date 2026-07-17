//! Radio button control.
use bevy_app::{Plugin, PreUpdate};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    observer::On,
    query::{Added, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
};
use bevy_picking::PickingSystems;
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::prelude::*;
use bevy_text::FontWeight;
use bevy_ui::{
    AlignItems, BorderRadius, BoxShadow, Checked, Display, FlexDirection, InteractionDisabled, JustifyContent, Node, PositionType, Val, percent, px,
};
use bevy_ui_widgets::{RadioButton, RadioGroup, ValueChange};

use crate::{
    constants::{fonts, size},
    cursor::EntityCursor,
    font_styles::InheritableFont,
    theme::{
        Flat, GRADIENT_AMOUNT, InheritableThemeTextColor, ThemeBackgroundGradient,
        ThemeBorderColor, control_box_shadow,
    },
    tokens,
};

/// A radio widget.
///
/// This is spawnable by inheriting it as a "scene component" with optional [`PlumeRadioProps`].
///
/// Emits [`bevy_ui_widgets::ValueChange<bool>`] (true) when checked, and the radio group emits
/// [`bevy_ui_widgets::ValueChange<Entity>`] with the newly selected radio.
#[derive(SceneComponent, Default, Clone)]
#[scene(PlumeRadioProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
pub struct PlumeRadio;

/// Props used to construct a [`PlumeRadio`] scene.
pub struct PlumeRadioProps {
    /// Label for this radio button. This can contain multiple entities, which will be contained
    /// in a flexbox.
    pub caption: Box<dyn SceneList>,
}

impl Default for PlumeRadioProps {
    fn default() -> Self {
        Self {
            caption: Box::new(bsn_list!()),
        }
    }
}

impl PlumeRadio {
    fn scene(props: PlumeRadioProps) -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Start,
                align_items: AlignItems::Center,
                column_gap: size::GAP,
                min_height: size::ROW_HEIGHT,
            }
            RadioButton
            on(radio_check_self)
            EntityCursor::System(bevy_window::SystemCursorIcon::Pointer)
            InheritableThemeTextColor(tokens::RADIO_TEXT)
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::MEDIUM_FONT,
                weight: FontWeight::NORMAL,
            }
            Children [(
                // Filled disc; the flex centering positions the mark dot.
                // Gradient only when checked (the unchecked fill is transparent),
                // matching the checkbox.
                Node {
                    display: Display::Flex,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    width: size::RADIO_SIZE,
                    height: size::RADIO_SIZE,
                    border_radius: BorderRadius::MAX,
                }
                RadioBg
                ThemeBackgroundGradient(tokens::RADIO_BG, 0.0)
                Children [
                    (
                        // Border ring overlaying the disc; only its color is themed.
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::ZERO,
                            top: Val::ZERO,
                            width: percent(100),
                            height: percent(100),
                            border: size::CONTROL_BORDER,
                            border_radius: BorderRadius::MAX,
                        }
                        RadioOutline
                        ThemeBorderColor(tokens::RADIO_BORDER)
                    ),
                    (
                        Node {
                            width: px(12),
                            height: px(12),
                            border: px(2),
                            border_radius: BorderRadius::MAX,
                        }
                        RadioMark
                        Visibility::Hidden
                        ThemeBackgroundGradient(tokens::RADIO_MARK)
                    )
                ]),
                {props.caption}
            ]
        }
    }
}

/// Groups [`PlumeRadio`] children into a column and keeps their checks mutually
/// exclusive. A radio outside any group still checks itself when clicked; only
/// the unchecking of siblings needs the group.
///
/// This is spawnable by inheriting it as a "scene component".
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct PlumeRadioGroup;

impl PlumeRadioGroup {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                row_gap: size::GAP_TIGHT,
            }
            RadioGroup
            PlumeRadioGroup
            on(radio_group_uncheck_others)
        }
    }
}

// The clicked radio checks itself ([`radio_check_self`]); the group only clears siblings.
fn radio_group_uncheck_others(
    ev: On<ValueChange<Entity>>,
    q_children: Query<&Children>,
    q_radio: Query<(), With<RadioButton>>,
    mut commands: Commands,
) {
    for descendant in q_children.iter_descendants(ev.source) {
        if q_radio.contains(descendant) && descendant != ev.value {
            commands.entity(descendant).remove::<Checked>();
        }
    }
}

// The headless widget only emits `ValueChange<bool>` (always `true`) for an enabled,
// unchecked radio, so no re-checks are needed here.
fn radio_check_self(ev: On<ValueChange<bool>>, mut commands: Commands) {
    if ev.value {
        commands.entity(ev.source).insert(Checked);
    }
}

/// Marker for the radio filled disc
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct RadioBg;

/// Marker for the radio outline
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct RadioOutline;

/// Marker for the radio check mark
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct RadioMark;

fn update_radio_styles(
    q_radios: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            Has<Flat>,
            &InheritableThemeTextColor,
            Has<BoxShadow>,
        ),
        (
            With<RadioButton>,
            // Added<PlumeRadio> guarantees the initial style pass on spawn.
            Or<(
                Added<PlumeRadio>,
                Added<Checked>,
                Added<InteractionDisabled>,
                Added<Flat>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    q_bg: Query<&ThemeBackgroundGradient, With<RadioBg>>,
    q_outline: Query<&ThemeBorderColor, With<RadioOutline>>,
    q_mark: Query<&ThemeBackgroundGradient, With<RadioMark>>,
    mut commands: Commands,
) {
    for (radio_ent, disabled, checked, flat, font_color, has_box_shadow) in q_radios.iter() {
        apply_radio_styles(
            radio_ent,
            disabled,
            checked,
            flat,
            font_color,
            &q_children,
            &q_bg,
            &q_outline,
            &q_mark,
            has_box_shadow,
            &mut commands,
        );
    }
}

fn update_radio_styles_remove(
    q_radios: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            Has<Flat>,
            &InheritableThemeTextColor,
            Has<BoxShadow>,
        ),
        With<RadioButton>,
    >,
    q_children: Query<&Children>,
    q_bg: Query<&ThemeBackgroundGradient, With<RadioBg>>,
    q_outline: Query<&ThemeBorderColor, With<RadioOutline>>,
    q_mark: Query<&ThemeBackgroundGradient, With<RadioMark>>,
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
            if let Ok((radio_ent, disabled, checked, flat, font_color, has_box_shadow)) =
                q_radios.get(ent)
            {
                apply_radio_styles(
                    radio_ent,
                    disabled,
                    checked,
                    flat,
                    font_color,
                    &q_children,
                    &q_bg,
                    &q_outline,
                    &q_mark,
                    has_box_shadow,
                    &mut commands,
                );
            }
        });
}

/// Resolve the radio's child entities and push the current styles onto them.
fn apply_radio_styles(
    radio_ent: Entity,
    disabled: bool,
    checked: bool,
    flat: bool,
    font_color: &InheritableThemeTextColor,
    q_children: &Query<&Children>,
    q_bg: &Query<&ThemeBackgroundGradient, With<RadioBg>>,
    q_outline: &Query<&ThemeBorderColor, With<RadioOutline>>,
    q_mark: &Query<&ThemeBackgroundGradient, With<RadioMark>>,
    has_box_shadow: bool,
    commands: &mut Commands,
) {
    let Some(bg_ent) = q_children
        .iter_descendants(radio_ent)
        .find(|en| q_bg.contains(*en))
    else {
        return;
    };
    let Some(outline_ent) = q_children
        .iter_descendants(radio_ent)
        .find(|en| q_outline.contains(*en))
    else {
        return;
    };
    let Some(mark_ent) = q_children
        .iter_descendants(radio_ent)
        .find(|en| q_mark.contains(*en))
    else {
        return;
    };
    // Safety: all three entities were just confirmed present in their queries.
    let bg = q_bg.get(bg_ent).unwrap();
    let outline_border = q_outline.get(outline_ent).unwrap();
    let mark_color = q_mark.get(mark_ent).unwrap();
    set_radio_styles(
        radio_ent,
        bg_ent,
        outline_ent,
        mark_ent,
        disabled,
        checked,
        flat,
        bg,
        outline_border,
        mark_color,
        font_color,
        has_box_shadow,
        commands,
    );
}

fn set_radio_styles(
    radio_ent: Entity,
    bg_ent: Entity,
    outline_ent: Entity,
    mark_ent: Entity,
    disabled: bool,
    checked: bool,
    flat: bool,
    bg: &ThemeBackgroundGradient,
    outline_border: &ThemeBorderColor,
    mark_color: &ThemeBackgroundGradient,
    font_color: &InheritableThemeTextColor,
    has_box_shadow: bool,
    commands: &mut Commands,
) {
    let outline_border_token = tokens::sets::RADIO_BORDER.pick(checked, disabled);
    let bg_token = tokens::sets::RADIO_BG.pick(checked, disabled);

    let mark_token = match disabled {
        true => tokens::RADIO_MARK_DISABLED,
        false => tokens::RADIO_MARK,
    };

    let font_color_token = match disabled {
        true => tokens::RADIO_TEXT_DISABLED,
        false => tokens::RADIO_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
    };

    // Change outline border
    if outline_border.0 != outline_border_token {
        commands
            .entity(outline_ent)
            .insert(ThemeBorderColor(outline_border_token));
    }

    // Change disc background: gradient only when checked, flat fill otherwise.
    let bg_gradient_amount = if checked && !flat {
        GRADIENT_AMOUNT
    } else {
        0.0
    };
    if bg.0 != bg_token || bg.1 != bg_gradient_amount {
        commands
            .entity(bg_ent)
            .insert(ThemeBackgroundGradient(bg_token, bg_gradient_amount));
    }

    // Change mark color
    if mark_color.0 != mark_token || bg.1 != bg_gradient_amount {
        commands
            .entity(mark_ent)
            .insert(ThemeBackgroundGradient(mark_token, bg_gradient_amount));
    }

    // Change mark visibility
    commands.entity(mark_ent).insert(match checked {
        true => Visibility::Inherited,
        false => Visibility::Hidden,
    });

    // Change font color
    if font_color.0 != font_color_token {
        commands
            .entity(radio_ent)
            .insert(InheritableThemeTextColor(font_color_token));
    }

    // Checked & Selected has a box shadow
    let should_have_box_shadow = checked && !disabled;
    if should_have_box_shadow && !has_box_shadow {
        commands.entity(bg_ent).insert(control_box_shadow());
    } else if !should_have_box_shadow && has_box_shadow {
        commands.entity(bg_ent).remove::<BoxShadow>();
    }

    // Change cursor shape
    commands
        .entity(radio_ent)
        .insert(EntityCursor::System(cursor_shape));
}

/// Plugin which registers the systems for updating the radio styles.
pub struct RadioPlugin;

impl Plugin for RadioPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_radio_styles, update_radio_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
