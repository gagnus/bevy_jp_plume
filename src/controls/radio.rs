//! Radio button control.
use bevy::app::{Plugin, PreUpdate};
use bevy::camera::visibility::Visibility;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::lifecycle::RemovedComponents;
use bevy::ecs::observer::On;
use bevy::ecs::query::{Added, Has, Or, With};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::{Pickable, PickingSystems};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::ui::{
    AlignItems, BorderRadius, BoxShadow, Checked, Display, FlexDirection, InteractionDisabled,
    JustifyContent, Node, PositionType, UiTransform, Val, percent,
};
use bevy::ui_widgets::{RadioButton, RadioGroup, ValueChange};

use crate::constants::size;
use crate::controls::SetValue;
use crate::cursor::EntityCursor;
use crate::focus::FocusIndicator;
use crate::font_styles::TextStyleRelay;
use crate::theme::{
    Flat, GRADIENT_AMOUNT, InheritableThemeTextToken, ThemeBackgroundGradient, ThemeBorderToken,
    control_box_shadow,
};
use crate::tokens;
use crate::utils::anim::AnimState;

/// A radio, spawnable as a scene component with optional [`PlumeRadioProps`].
/// Emits [`bevy::ui_widgets::ValueChange<bool>`] (always true) when checked.
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
            TabIndex(0)
            // The row stretches to its container, but only the disc and label react to
            // clicks (children stay pickable and bubble up); the dead space is inert.
            Pickable::IGNORE
            on(radio_check_self)
            EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)
            InheritableThemeTextToken(tokens::RADIO_TEXT)
            TextStyleRelay
            Children [(
                // Gradient only when checked, since the unchecked fill is transparent.
                Node {
                    display: Display::Flex,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    width: size::RADIO_SIZE,
                    height: size::RADIO_SIZE,
                    border_radius: BorderRadius::MAX,
                }
                RadioBg
                // Em-sized chrome needs the chain's `EmSize`.
                TextStyleRelay
                // Ring hugs the disc, not the label row.
                FocusIndicator
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
                        ThemeBorderToken(tokens::RADIO_BORDER)
                    ),
                    (
                        // Proportioned to the disc so it scales with it.
                        Node {
                            width: {size::em_from_px(12.0)},
                            height: {size::em_from_px(12.0)},
                            border: {size::em_from_px(2.0)},
                            border_radius: BorderRadius::MAX,
                        }
                        RadioMark
                        TextStyleRelay
                        template_value(AnimState::scale(0.0, 1.0).hide_at_zero())
                        UiTransform::default()
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
/// exclusive.
/// # Emitted events
/// * [`ValueChange<usize>`](bevy::ui_widgets::ValueChange) with the picked radio's
///   position in the group.
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
            TextStyleRelay
            on(radio_group_uncheck_others)
        }
    }
}

// Programmatic selection by position, the counterpart of the group's emitted index.
fn radio_group_on_set_value(
    ev: On<SetValue<usize>>,
    q_groups: Query<(), With<RadioGroup>>,
    q_children: Query<&Children>,
    q_radio: Query<(), With<RadioButton>>,
    mut commands: Commands,
) {
    if !q_groups.contains(ev.entity) {
        return;
    }
    for (index, radio) in q_children
        .iter_descendants(ev.entity)
        .filter(|&descendant| q_radio.contains(descendant))
        .enumerate()
    {
        if index == ev.value {
            commands.entity(radio).insert(Checked);
        } else {
            commands.entity(radio).remove::<Checked>();
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
// unchecked radio, so no re-checks are needed here. The group's pick is re-announced
// by position from the radio rather than from the group's own observer, which is the
// arrangement that reaches app observers on the group.
fn radio_check_self(
    ev: On<ValueChange<bool>>,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    q_groups: Query<(), With<RadioGroup>>,
    q_radio: Query<(), With<RadioButton>>,
    mut commands: Commands,
) {
    if !ev.value {
        return;
    }
    commands.entity(ev.source).insert(Checked);

    let Some(group) = q_parents
        .iter_ancestors(ev.source)
        .find(|ancestor| q_groups.contains(*ancestor))
    else {
        return;
    };
    let index = q_children
        .iter_descendants(group)
        .filter(|&descendant| q_radio.contains(descendant))
        .position(|descendant| descendant == ev.source);
    if let Some(index) = index {
        commands.trigger(ValueChange {
            source: group,
            value: index,
            is_final: true,
        });
    }
}

// Marker for the radio filled disc
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct RadioBg;

// Marker for the radio outline
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct RadioOutline;

// Marker for the radio check mark
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
            &InheritableThemeTextToken,
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
    q_outline: Query<&ThemeBorderToken, With<RadioOutline>>,
    q_mark: Query<&ThemeBackgroundGradient, With<RadioMark>>,
    mut q_mark_anim: Query<&mut AnimState, With<RadioMark>>,
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
            &mut q_mark_anim,
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
            &InheritableThemeTextToken,
            Has<BoxShadow>,
        ),
        With<RadioButton>,
    >,
    q_children: Query<&Children>,
    q_bg: Query<&ThemeBackgroundGradient, With<RadioBg>>,
    q_outline: Query<&ThemeBorderToken, With<RadioOutline>>,
    q_mark: Query<&ThemeBackgroundGradient, With<RadioMark>>,
    mut q_mark_anim: Query<&mut AnimState, With<RadioMark>>,
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
                    &mut q_mark_anim,
                    has_box_shadow,
                    &mut commands,
                );
            }
        });
}

// Resolve the radio's child entities and push the current styles onto them.
fn apply_radio_styles(
    radio_ent: Entity,
    disabled: bool,
    checked: bool,
    flat: bool,
    font_color: &InheritableThemeTextToken,
    q_children: &Query<&Children>,
    q_bg: &Query<&ThemeBackgroundGradient, With<RadioBg>>,
    q_outline: &Query<&ThemeBorderToken, With<RadioOutline>>,
    q_mark: &Query<&ThemeBackgroundGradient, With<RadioMark>>,
    q_mark_anim: &mut Query<&mut AnimState, With<RadioMark>>,
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

    // Drive the pop: the disc eases to full scale (and shows) while checked.
    if let Ok(mut mark_anim) = q_mark_anim.get_mut(mark_ent) {
        mark_anim.set_target(if checked { 1.0 } else { 0.0 });
    }

    let bg = q_bg
        .get(bg_ent)
        .expect("bg entity was just found via q_bg::contains");
    let outline_border = q_outline
        .get(outline_ent)
        .expect("outline entity was just found via q_outline::contains");
    let mark_color = q_mark
        .get(mark_ent)
        .expect("mark entity was just found via q_mark::contains");
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
    outline_border: &ThemeBorderToken,
    mark_color: &ThemeBackgroundGradient,
    font_color: &InheritableThemeTextToken,
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
        true => bevy::window::SystemCursorIcon::NotAllowed,
        false => bevy::window::SystemCursorIcon::Pointer,
    };

    if outline_border.0 != outline_border_token {
        commands
            .entity(outline_ent)
            .insert(ThemeBorderToken(outline_border_token));
    }

    // Gradient only when checked, flat fill otherwise.
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

    if mark_color.0 != mark_token || bg.1 != bg_gradient_amount {
        commands
            .entity(mark_ent)
            .insert(ThemeBackgroundGradient(mark_token, bg_gradient_amount));
    }

    if font_color.0 != font_color_token {
        commands
            .entity(radio_ent)
            .insert(InheritableThemeTextToken(font_color_token));
    }

    let should_have_box_shadow = checked && !disabled;
    if should_have_box_shadow && !has_box_shadow {
        commands.entity(bg_ent).insert(control_box_shadow());
    } else if !should_have_box_shadow && has_box_shadow {
        commands.entity(bg_ent).remove::<BoxShadow>();
    }

    commands
        .entity(radio_ent)
        .insert(EntityCursor::System(cursor_shape));
}

// Plugin which registers the systems for updating the radio styles.
pub(crate) struct RadioPlugin;

impl Plugin for RadioPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_observer(radio_group_on_set_value);
        app.add_systems(
            PreUpdate,
            (update_radio_styles, update_radio_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
