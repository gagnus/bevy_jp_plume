//! On/off toggle switch control.
use accesskit::Role;
use bevy::a11y::AccessibilityNode;
use bevy::app::{Plugin, PreUpdate};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::Children;
use bevy::ecs::lifecycle::RemovedComponents;
use bevy::ecs::query::{Added, Has, Or, With};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::PickingSystems;
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::ui::{
    BorderRadius, BoxShadow, Checked, InteractionDisabled, Node, PositionType, UiRect, UiTransform,
    Val, percent,
};
use bevy::ui_widgets::{Checkbox, checkbox_self_update};

use crate::constants::size;
use crate::cursor::EntityCursor;
use crate::focus::FocusIndicator;
use crate::font_styles::TextStyleRelay;
use crate::theme::{GradientAmount, ThemeBackgroundToken, ThemeBorderToken, control_box_shadow};
use crate::tokens;
use crate::utils::anim::AnimState;
use crate::utils::hierarchy::descendant_get;

const SLIDE_GRADIENT_AMOUNT: f32 = 0.3;

// Horizontal knob travel between off and on: pill width 32 − knob 16 − a 1px
// inset at each end (at the standard font), so the knob keeps its margins.
const KNOB_TRAVEL: Val = size::em_from_px(14.0);

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
                margin: UiRect::vertical(
                    size::ROW_HEIGHT.try_sub(size::TOGGLE_SIZE.y).unwrap() / 2.0,
                ),
                border_radius: {size::TOGGLE_SIZE.y / 2.0},
            }
            Checkbox
            ToggleSwitchFrame
            // Em-sized chrome needs the chain's `EmSize`.
            TextStyleRelay
            TabIndex(0)
            FocusIndicator
            on(checkbox_self_update)
            ThemeBackgroundToken(tokens::SWITCH_BG)
            GradientAmount::STANDARD
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
                        border: size::HAIRLINE,
                        border_radius: {size::TOGGLE_SIZE.y / 2.0},
                    }
                    ToggleSwitchOutline
                    TextStyleRelay
                    ThemeBorderToken(tokens::SWITCH_BORDER)
                ),
                (
                    // The 2px inset nests the 16px knob (radius 8) concentrically inside
                    // the pill's outer radius (9) minus the ring's border. The on/off
                    // slide is a post-layout `UiTransform`, so it never relayouts.
                    Node {
                        position_type: PositionType::Absolute,
                        left: size::em_from_px(1.0),
                        top: size::em_from_px(1.0),
                        width: size::KNOB_SIZE,
                        height: size::KNOB_SIZE,
                        border_radius: BorderRadius::MAX,
                    }
                    ToggleSwitchSlide
                    TextStyleRelay
                    template_value(AnimState::translate_x(size::em_from_px(0.0), KNOB_TRAVEL))
                    UiTransform::default()
                    ThemeBackgroundToken(tokens::SWITCH_SLIDE_BG)
                    GradientAmount(SLIDE_GRADIENT_AMOUNT)
                    template_value(control_box_shadow())
                ),
            ]
        }
    }
}

// Plain root marker, inserted on both the retained and imm paths. The systems key on
// this, not [`PlumeToggleSwitch`], which only the retained path inserts.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ToggleSwitchFrame;

// Marker for the toggle switch border ring.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ToggleSwitchOutline;

// Marker for the toggle switch slide.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ToggleSwitchSlide;

fn update_switch_styles(
    q_switches: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            &ThemeBackgroundToken,
            &GradientAmount,
        ),
        (
            With<ToggleSwitchFrame>,
            // Added<ToggleSwitchFrame> guarantees the initial style pass on spawn.
            Or<(
                Added<ToggleSwitchFrame>,
                Added<Checked>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    q_outline: Query<(Entity, &ThemeBorderToken), With<ToggleSwitchOutline>>,
    q_slide: Query<
        (
            Entity,
            &ThemeBackgroundToken,
            &GradientAmount,
            Has<BoxShadow>,
        ),
        With<ToggleSwitchSlide>,
    >,
    mut q_slide_anim: Query<&mut AnimState, With<ToggleSwitchSlide>>,
    mut commands: Commands,
) {
    for (switch_ent, disabled, checked, pill_token, pill_amount) in q_switches.iter() {
        apply_switch_styles(
            switch_ent,
            disabled,
            checked,
            (pill_token, pill_amount),
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
            &ThemeBackgroundToken,
            &GradientAmount,
        ),
        With<ToggleSwitchFrame>,
    >,
    q_children: Query<&Children>,
    q_outline: Query<(Entity, &ThemeBorderToken), With<ToggleSwitchOutline>>,
    q_slide: Query<
        (
            Entity,
            &ThemeBackgroundToken,
            &GradientAmount,
            Has<BoxShadow>,
        ),
        With<ToggleSwitchSlide>,
    >,
    mut q_slide_anim: Query<&mut AnimState, With<ToggleSwitchSlide>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_checked: RemovedComponents<Checked>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_checked.read())
        .for_each(|ent| {
            if let Ok((switch_ent, disabled, checked, pill_token, pill_amount)) =
                q_switches.get(ent)
            {
                apply_switch_styles(
                    switch_ent,
                    disabled,
                    checked,
                    (pill_token, pill_amount),
                    &q_children,
                    &q_outline,
                    &q_slide,
                    &mut q_slide_anim,
                    &mut commands,
                );
            }
        });
}

// Resolve the switch's child entities and push the current styles onto them.
fn apply_switch_styles(
    switch_ent: Entity,
    disabled: bool,
    checked: bool,
    pill_now: (&ThemeBackgroundToken, &GradientAmount),
    q_children: &Query<&Children>,
    q_outline: &Query<(Entity, &ThemeBorderToken), With<ToggleSwitchOutline>>,
    q_slide: &Query<
        (
            Entity,
            &ThemeBackgroundToken,
            &GradientAmount,
            Has<BoxShadow>,
        ),
        With<ToggleSwitchSlide>,
    >,
    q_slide_anim: &mut Query<&mut AnimState, With<ToggleSwitchSlide>>,
    commands: &mut Commands,
) {
    let Some((outline_ent, outline_border)) = descendant_get(switch_ent, q_children, q_outline)
    else {
        return;
    };
    let Some((slide_ent, slide_token, slide_amount, has_box_shadow)) =
        descendant_get(switch_ent, q_children, q_slide)
    else {
        return;
    };
    // Drive the slide: the knob eases to the on end while checked.
    if let Ok(mut slide_anim) = q_slide_anim.get_mut(slide_ent) {
        slide_anim.set_target(if checked { 1.0 } else { 0.0 });
    }
    set_switch_styles(
        switch_ent,
        outline_ent,
        slide_ent,
        disabled,
        checked,
        pill_now,
        outline_border,
        (slide_token, slide_amount),
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
    pill_now: (&ThemeBackgroundToken, &GradientAmount),
    outline_border: &ThemeBorderToken,
    slide_now: (&ThemeBackgroundToken, &GradientAmount),
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

    // Disabled reads inert: flat fill, no gradient. A `Flat` switch is flattened
    // by the theme layer, so the marker plays no part here.
    let pill_amount = if disabled {
        GradientAmount(0.0)
    } else {
        GradientAmount::STANDARD
    };
    let slide_amount = if disabled {
        GradientAmount(0.0)
    } else {
        GradientAmount(SLIDE_GRADIENT_AMOUNT)
    };

    let (pill_token_now, pill_amount_now) = pill_now;
    if pill_token_now.0 != pill_bg_token {
        commands
            .entity(switch_ent)
            .insert(ThemeBackgroundToken(pill_bg_token));
    }
    if *pill_amount_now != pill_amount {
        commands.entity(switch_ent).insert(pill_amount);
    }

    if outline_border.0 != outline_border_token {
        commands
            .entity(outline_ent)
            .insert(ThemeBorderToken(outline_border_token));
    }

    let (slide_token_now, slide_amount_now) = slide_now;
    if slide_token_now.0 != slide_bg_token {
        commands
            .entity(slide_ent)
            .insert(ThemeBackgroundToken(slide_bg_token));
    }
    if *slide_amount_now != slide_amount {
        commands.entity(slide_ent).insert(slide_amount);
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

// Plugin which registers the systems for updating the toggle switch styles.
pub(crate) struct ToggleSwitchPlugin;

impl Plugin for ToggleSwitchPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            (update_switch_styles, update_switch_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
