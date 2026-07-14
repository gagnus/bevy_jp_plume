//! On/off toggle switch control.
use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{Plugin, PreUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
    world::Mut,
};
use bevy_picking::PickingSystems;
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::prelude::*;
use bevy_ui::{Checked, InteractionDisabled, Node, PositionType, UiRect, Val, percent, px};
use bevy_ui_widgets::{Checkbox, checkbox_self_update};

use crate::{
    constants::size,
    cursor::EntityCursor,
    theme::{GRADIENT_AMOUNT, ThemeBackgroundGradient, ThemeBorderColor},
    tokens,
};

const SLIDE_GRADIENT_AMOUNT: f32 = 0.3;

/// A toggle switch widget.
///
/// This is spawnable by inheriting it as a "scene component".
///
/// Emits [`bevy_ui_widgets::ValueChange<bool>`] with the new value when the switch changes
/// state; disabled by adding [`bevy_ui::InteractionDisabled`].
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct PlumeToggleSwitch;

impl PlumeToggleSwitch {
    fn scene() -> impl Scene {
        bsn! {
            // Pill fill; vertical margin pads the outer box up to ROW_HEIGHT.
            // The border lives on a separate SwitchOutline overlay so the fill
            // gradient and the ring antialias cleanly (matches the checkbox split).
            Node {
                width: size::TOGGLE_WIDTH,
                height: size::TOGGLE_HEIGHT,
                margin: UiRect::vertical(px(4)),
                border_radius: px(9),
            }
            Checkbox
            PlumeToggleSwitch
            on(checkbox_self_update)
            ThemeBackgroundGradient(tokens::SWITCH_BG, GRADIENT_AMOUNT)
            AccessibilityNode(accesskit::Node::new(Role::Switch))
            EntityCursor::System(bevy_window::SystemCursorIcon::Pointer)
            Children [
                (
                    // Border ring overlaying the pill; only its color is themed.
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        width: percent(100),
                        height: percent(100),
                        border: size::CONTROL_BORDER,
                        border_radius: px(9),
                    }
                    SwitchOutline
                    ThemeBorderColor(tokens::SWITCH_BORDER)
                ),
                (
                    // Circular knob; styles slide it between the left/right insets.
                    // A 2px inset nests the 16px knob (radius 8) concentrically inside
                    // the pill's outer radius (9) minus the ring's border.
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(2),
                        top: px(1),
                        width: size::KNOB_SIZE,
                        height: size::KNOB_SIZE,
                        border_radius: px(8),
                    }
                    ToggleSwitchSlide
                    ThemeBackgroundGradient(tokens::SWITCH_SLIDE_BG, SLIDE_GRADIENT_AMOUNT)
                )
            ]
        }
    }
}

/// Marker for the toggle switch border ring
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct SwitchOutline;

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
            &ThemeBackgroundGradient,
        ),
        (
            With<PlumeToggleSwitch>,
            // Added<PlumeToggleSwitch> guarantees the initial style pass on spawn.
            Or<(
                Added<PlumeToggleSwitch>,
                Added<Checked>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    q_outline: Query<&ThemeBorderColor, With<SwitchOutline>>,
    mut q_slide: Query<(&mut Node, &ThemeBackgroundGradient), With<ToggleSwitchSlide>>,
    mut commands: Commands,
) {
    for (switch_ent, disabled, checked, pill_bg) in q_switches.iter() {
        apply_switch_styles(
            switch_ent,
            disabled,
            checked,
            pill_bg,
            &q_children,
            &q_outline,
            &mut q_slide,
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
            &ThemeBackgroundGradient,
        ),
        With<PlumeToggleSwitch>,
    >,
    q_children: Query<&Children>,
    q_outline: Query<&ThemeBorderColor, With<SwitchOutline>>,
    mut q_slide: Query<(&mut Node, &ThemeBackgroundGradient), With<ToggleSwitchSlide>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_checked: RemovedComponents<Checked>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_checked.read())
        .for_each(|ent| {
            if let Ok((switch_ent, disabled, checked, pill_bg)) = q_switches.get(ent) {
                apply_switch_styles(
                    switch_ent,
                    disabled,
                    checked,
                    pill_bg,
                    &q_children,
                    &q_outline,
                    &mut q_slide,
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
    pill_bg: &ThemeBackgroundGradient,
    q_children: &Query<&Children>,
    q_outline: &Query<&ThemeBorderColor, With<SwitchOutline>>,
    q_slide: &mut Query<(&mut Node, &ThemeBackgroundGradient), With<ToggleSwitchSlide>>,
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
    // Safety: both entities were just confirmed present in their queries.
    let outline_border = q_outline.get(outline_ent).unwrap();
    let (ref mut slide_style, slide_bg) = q_slide.get_mut(slide_ent).unwrap();
    set_switch_styles(
        switch_ent,
        outline_ent,
        slide_ent,
        disabled,
        checked,
        pill_bg,
        outline_border,
        slide_style,
        slide_bg,
        commands,
    );
}

fn set_switch_styles(
    switch_ent: Entity,
    outline_ent: Entity,
    slide_ent: Entity,
    disabled: bool,
    checked: bool,
    pill_bg: &ThemeBackgroundGradient,
    outline_border: &ThemeBorderColor,
    slide_style: &mut Mut<Node>,
    slide_bg: &ThemeBackgroundGradient,
    commands: &mut Commands,
) {
    let outline_border_token = tokens::sets::SWITCH_BORDER.pick(checked, disabled);
    let pill_bg_token = tokens::sets::SWITCH_BG.pick(checked, disabled);
    let slide_bg_token = tokens::sets::SWITCH_SLIDE_BG.pick(checked, disabled);

    let (slide_left, slide_right) = match checked {
        true => (Val::Auto, px(2)),
        false => (px(2), Val::Auto),
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
    };

    // Disabled reads inert: flat fill, no gradient.
    let gradient_amount = if disabled { 0.0 } else { GRADIENT_AMOUNT };

    // Change pill background gradient
    if pill_bg.0 != pill_bg_token || pill_bg.1 != gradient_amount {
        commands
            .entity(switch_ent)
            .insert(ThemeBackgroundGradient(pill_bg_token, gradient_amount));
    }

    // Change outline border
    if outline_border.0 != outline_border_token {
        commands
            .entity(outline_ent)
            .insert(ThemeBorderColor(outline_border_token));
    }

    // more gradient for slide
    let slide_gradient_amount = if disabled { 0.0 } else { SLIDE_GRADIENT_AMOUNT };

    // Change slide background gradient
    if slide_bg.0 != slide_bg_token || slide_bg.1 != slide_gradient_amount {
        commands.entity(slide_ent).insert(ThemeBackgroundGradient(
            slide_bg_token,
            slide_gradient_amount,
        ));
    }

    // Change slide position
    if slide_style.left != slide_left {
        slide_style.left = slide_left;
    }
    if slide_style.right != slide_right {
        slide_style.right = slide_right;
    }

    // Change cursor shape
    commands
        .entity(switch_ent)
        .insert(EntityCursor::System(cursor_shape));
}

/// Plugin which registers the systems for updating the toggle switch styles.
pub struct ToggleSwitchPlugin;

impl Plugin for ToggleSwitchPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_switch_styles, update_switch_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
