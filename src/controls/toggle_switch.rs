//! On/off toggle switch control.
use accesskit::Role;
use bevy::a11y::AccessibilityNode;
use bevy::app::{Plugin, PreUpdate};
use bevy::ecs::{
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::PickingSystems;
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::scene::prelude::*;
use bevy::ui::{
    BorderRadius, BoxShadow, Checked, InteractionDisabled, Node, PositionType, UiRect, UiTransform,
    Val, percent, px,
};
use bevy::ui_widgets::{Checkbox, checkbox_self_update};

use crate::{
    constants::size,
    cursor::EntityCursor,
    focus::FocusIndicator,
    theme::{Flat, GRADIENT_AMOUNT, ThemeBackgroundGradient, ThemeBorderColor, control_box_shadow},
    tokens,
    utils::anim::AnimState,
};

const SLIDE_GRADIENT_AMOUNT: f32 = 0.3;

/// Horizontal knob travel between off and on, in px: pill width 32 − knob 16 −
/// a 1px inset at each end, so the knob keeps a 1px margin on both sides.
const KNOB_TRAVEL: f32 = 14.0;

/// A toggle switch, spawnable as a scene component. Emits
/// [`bevy::ui_widgets::ValueChange<bool>`] with the new state.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct PlumeToggleSwitch;

impl PlumeToggleSwitch {
    fn scene() -> impl Scene {
        bsn! {
            // The border lives on a separate SwitchOutline overlay so the fill
            // gradient and the ring antialias cleanly (matches the checkbox split).
            Node {
                width: {size::TOGGLE_SIZE.x},
                height: {size::TOGGLE_SIZE.y},
                margin: {UiRect::vertical(
                    size::ROW_HEIGHT.try_sub(size::TOGGLE_SIZE.y).unwrap() / 2.0,
                )},
                border_radius: {size::TOGGLE_SIZE.y / 2.0},
            }
            Checkbox
            PlumeToggleSwitch
            TabIndex(0)
            FocusIndicator
            on(checkbox_self_update)
            ThemeBackgroundGradient(tokens::SWITCH_BG, GRADIENT_AMOUNT)
            template_value(control_box_shadow())
            AccessibilityNode(accesskit::Node::new(Role::Switch))
            EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)
            Children [
                (
                    // Border ring overlaying the pill; only its color is themed.
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::ZERO,
                        top: Val::ZERO,
                        width: percent(100),
                        height: percent(100),
                        border: size::CONTROL_BORDER,
                        border_radius: {size::TOGGLE_SIZE.y / 2.0},
                    }
                    ToggleSwitchOutline
                    ThemeBorderColor(tokens::SWITCH_BORDER)
                ),
                (
                    // The 2px inset nests the 16px knob (radius 8) concentrically inside
                    // the pill's outer radius (9) minus the ring's border. The knob keeps
                    // this off-position in layout; the on/off slide is a post-layout
                    // `UiTransform` translation so it never triggers a relayout.
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(1),
                        top: px(1),
                        width: size::KNOB_SIZE,
                        height: size::KNOB_SIZE,
                        border_radius: BorderRadius::MAX,
                    }
                    ToggleSwitchSlide
                    template_value(AnimState::translate_x(0.0, KNOB_TRAVEL))
                    UiTransform::default()
                    ThemeBackgroundGradient(tokens::SWITCH_SLIDE_BG, SLIDE_GRADIENT_AMOUNT)
                    template_value(control_box_shadow())
                )
            ]
        }
    }
}

/// Marker for the toggle switch border ring
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ToggleSwitchOutline;

/// Marker for the toggle switch slide
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ToggleSwitchSlide;

fn update_switch_styles(
    q_switches: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            Has<Flat>,
            &ThemeBackgroundGradient,
        ),
        (
            With<PlumeToggleSwitch>,
            // Added<PlumeToggleSwitch> guarantees the initial style pass on spawn.
            Or<(
                Added<PlumeToggleSwitch>,
                Added<Checked>,
                Added<InteractionDisabled>,
                Added<Flat>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    q_outline: Query<&ThemeBorderColor, With<ToggleSwitchOutline>>,
    q_slide: Query<(&ThemeBackgroundGradient, Has<BoxShadow>), With<ToggleSwitchSlide>>,
    mut q_slide_anim: Query<&mut AnimState, With<ToggleSwitchSlide>>,
    mut commands: Commands,
) {
    for (switch_ent, disabled, checked, flat, pill_bg) in q_switches.iter() {
        apply_switch_styles(
            switch_ent,
            disabled,
            checked,
            flat,
            pill_bg,
            &q_children,
            &q_outline,
            &q_slide,
            &mut q_slide_anim,
            &mut commands,
        );
    }
}

fn update_switch_styles_remove(
    q_switches: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            Has<Flat>,
            &ThemeBackgroundGradient,
        ),
        With<PlumeToggleSwitch>,
    >,
    q_children: Query<&Children>,
    q_outline: Query<&ThemeBorderColor, With<ToggleSwitchOutline>>,
    q_slide: Query<(&ThemeBackgroundGradient, Has<BoxShadow>), With<ToggleSwitchSlide>>,
    mut q_slide_anim: Query<&mut AnimState, With<ToggleSwitchSlide>>,
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
            if let Ok((switch_ent, disabled, checked, flat, pill_bg)) = q_switches.get(ent) {
                apply_switch_styles(
                    switch_ent,
                    disabled,
                    checked,
                    flat,
                    pill_bg,
                    &q_children,
                    &q_outline,
                    &q_slide,
                    &mut q_slide_anim,
                    &mut commands,
                );
            }
        });
}

/// Resolve the switch's child entities and push the current styles onto them.
fn apply_switch_styles(
    switch_ent: Entity,
    disabled: bool,
    checked: bool,
    flat: bool,
    pill_bg: &ThemeBackgroundGradient,
    q_children: &Query<&Children>,
    q_outline: &Query<&ThemeBorderColor, With<ToggleSwitchOutline>>,
    q_slide: &Query<(&ThemeBackgroundGradient, Has<BoxShadow>), With<ToggleSwitchSlide>>,
    q_slide_anim: &mut Query<&mut AnimState, With<ToggleSwitchSlide>>,
    commands: &mut Commands,
) {
    let Some(outline_ent) = q_children
        .iter_descendants(switch_ent)
        .find(|en| q_outline.contains(*en))
    else {
        return;
    };
    let Some(slide_ent) = q_children
        .iter_descendants(switch_ent)
        .find(|en| q_slide.contains(*en))
    else {
        return;
    };
    // Drive the slide: the knob eases to the on end while checked.
    if let Ok(mut slide_anim) = q_slide_anim.get_mut(slide_ent) {
        slide_anim.set_target(if checked { 1.0 } else { 0.0 });
    }

    let outline_border = q_outline
        .get(outline_ent)
        .expect("outline entity was just found via q_outline::contains");
    let (slide_bg, has_box_shadow) = q_slide
        .get(slide_ent)
        .expect("slide entity was just found via q_slide::contains");
    set_switch_styles(
        switch_ent,
        outline_ent,
        slide_ent,
        disabled,
        checked,
        flat,
        pill_bg,
        outline_border,
        slide_bg,
        has_box_shadow,
        commands,
    );
}

fn set_switch_styles(
    switch_ent: Entity,
    outline_ent: Entity,
    slide_ent: Entity,
    disabled: bool,
    checked: bool,
    flat: bool,
    pill_bg: &ThemeBackgroundGradient,
    outline_border: &ThemeBorderColor,
    slide_bg: &ThemeBackgroundGradient,
    has_box_shadow: bool,
    commands: &mut Commands,
) {
    let outline_border_token = tokens::sets::SWITCH_BORDER.pick(checked, disabled);
    let pill_bg_token = tokens::sets::SWITCH_BG.pick(checked, disabled);
    let slide_bg_token = tokens::sets::SWITCH_SLIDE_BG.pick(checked, disabled);

    let cursor_shape = match disabled {
        true => bevy::window::SystemCursorIcon::NotAllowed,
        false => bevy::window::SystemCursorIcon::Pointer,
    };

    // Disabled reads inert: flat fill, no gradient.
    let gradient_amount = if disabled || flat {
        0.0
    } else {
        GRADIENT_AMOUNT
    };

    if pill_bg.0 != pill_bg_token || pill_bg.1 != gradient_amount {
        commands
            .entity(switch_ent)
            .insert(ThemeBackgroundGradient(pill_bg_token, gradient_amount));
    }

    if outline_border.0 != outline_border_token {
        commands
            .entity(outline_ent)
            .insert(ThemeBorderColor(outline_border_token));
    }

    let slide_gradient_amount = if disabled || flat {
        0.0
    } else {
        SLIDE_GRADIENT_AMOUNT
    };

    if slide_bg.0 != slide_bg_token || slide_bg.1 != slide_gradient_amount {
        commands.entity(slide_ent).insert(ThemeBackgroundGradient(
            slide_bg_token,
            slide_gradient_amount,
        ));
    }

    let should_have_box_shadow = !disabled;
    if should_have_box_shadow && !has_box_shadow {
        commands.entity(slide_ent).insert(control_box_shadow());
        commands.entity(switch_ent).insert(control_box_shadow());
    } else if !should_have_box_shadow && has_box_shadow {
        commands.entity(slide_ent).remove::<BoxShadow>();
        commands.entity(switch_ent).remove::<BoxShadow>();
    }

    commands
        .entity(switch_ent)
        .insert(EntityCursor::System(cursor_shape));
}

/// Plugin which registers the systems for updating the toggle switch styles.
pub(crate) struct ToggleSwitchPlugin;

impl Plugin for ToggleSwitchPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            (update_switch_styles, update_switch_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
