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
use bevy_ui::{Checked, InteractionDisabled, Node, PositionType, UiRect, Val, px};
use bevy_ui_widgets::{Checkbox, checkbox_self_update};

use crate::{
    constants::size,
    cursor::EntityCursor,
    theme::{ThemeBackgroundColor, ThemeBorderColor},
    tokens,
};

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
            // Pill outline; vertical margin pads the outer box up to ROW_HEIGHT.
            Node {
                width: size::TOGGLE_WIDTH,
                height: size::TOGGLE_HEIGHT,
                margin: UiRect::vertical(px(2)),
                border: px(2),
                border_radius: px(10),
            }
            Checkbox
            PlumeToggleSwitch
            on(checkbox_self_update)
            ThemeBackgroundColor(tokens::SWITCH_BG)
            ThemeBorderColor(tokens::SWITCH_BORDER)
            AccessibilityNode(accesskit::Node::new(Role::Switch))
            EntityCursor::System(bevy_window::SystemCursorIcon::Pointer)
            Children [(
                // Circular knob; styles slide it between the left/right insets.
                // Diameter fills the pill's content height and its radius (8) nests
                // concentrically inside the pill's outer radius (10) minus the border.
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0),
                    top: px(0),
                    width: px(16),
                    height: px(16),
                    border: px(2),
                    border_radius: px(8),
                }
                ToggleSwitchSlide
                ThemeBackgroundColor(tokens::SWITCH_SLIDE_BG)
                ThemeBorderColor(tokens::SWITCH_SLIDE_BORDER)
            )]
        }
    }
}

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
            &ThemeBackgroundColor,
            &ThemeBorderColor,
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
    mut q_slide: Query<
        (&mut Node, &ThemeBackgroundColor, &ThemeBorderColor),
        With<ToggleSwitchSlide>,
    >,
    mut commands: Commands,
) {
    for (switch_ent, disabled, checked, outline_bg, outline_border) in q_switches.iter() {
        let Some(slide_ent) = q_children
            .iter_descendants(switch_ent)
            .find(|en| q_slide.contains(*en))
        else {
            continue;
        };
        // Safety: since we just checked the query, should always work.
        let (ref mut slide_style, slide_bg_color, slide_border_color) =
            q_slide.get_mut(slide_ent).unwrap();
        set_switch_styles(
            switch_ent,
            slide_ent,
            disabled,
            checked,
            outline_bg,
            outline_border,
            slide_style,
            slide_bg_color,
            slide_border_color,
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
            &ThemeBackgroundColor,
            &ThemeBorderColor,
        ),
        With<PlumeToggleSwitch>,
    >,
    q_children: Query<&Children>,
    mut q_slide: Query<
        (&mut Node, &ThemeBackgroundColor, &ThemeBorderColor),
        With<ToggleSwitchSlide>,
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_checked: RemovedComponents<Checked>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_checked.read())
        .for_each(|ent| {
            if let Ok((switch_ent, disabled, checked, outline_bg, outline_border)) =
                q_switches.get(ent)
            {
                let Some(slide_ent) = q_children
                    .iter_descendants(switch_ent)
                    .find(|en| q_slide.contains(*en))
                else {
                    return;
                };
                // Safety: since we just checked the query, should always work.
                let (ref mut slide_style, slide_bg_color, slide_border_color) =
                    q_slide.get_mut(slide_ent).unwrap();
                set_switch_styles(
                    switch_ent,
                    slide_ent,
                    disabled,
                    checked,
                    outline_bg,
                    outline_border,
                    slide_style,
                    slide_bg_color,
                    slide_border_color,
                    &mut commands,
                );
            }
        });
}

fn set_switch_styles(
    switch_ent: Entity,
    slide_ent: Entity,
    disabled: bool,
    checked: bool,
    outline_bg: &ThemeBackgroundColor,
    outline_border: &ThemeBorderColor,
    slide_style: &mut Mut<Node>,
    slide_bg_color: &ThemeBackgroundColor,
    slide_border_color: &ThemeBorderColor,
    commands: &mut Commands,
) {
    let (outline_border_token, outline_bg_token, slide_border_token, slide_bg_token) =
        match (checked, disabled) {
            (true, true) => (
                tokens::SWITCH_BORDER_CHECKED_DISABLED,
                tokens::SWITCH_BG_CHECKED_DISABLED,
                tokens::SWITCH_SLIDE_BORDER_CHECKED_DISABLED,
                tokens::SWITCH_SLIDE_BG_CHECKED_DISABLED,
            ),
            (true, false) => (
                tokens::SWITCH_BORDER_CHECKED,
                tokens::SWITCH_BG_CHECKED,
                tokens::SWITCH_SLIDE_BORDER_CHECKED,
                tokens::SWITCH_SLIDE_BG_CHECKED,
            ),
            (false, true) => (
                tokens::SWITCH_BORDER_DISABLED,
                tokens::SWITCH_BG_DISABLED,
                tokens::SWITCH_SLIDE_BORDER_DISABLED,
                tokens::SWITCH_SLIDE_BG_DISABLED,
            ),
            (false, false) => (
                tokens::SWITCH_BORDER,
                tokens::SWITCH_BG,
                tokens::SWITCH_SLIDE_BORDER,
                tokens::SWITCH_SLIDE_BG,
            ),
        };

    let (slide_left, slide_right) = match checked {
        true => (Val::Auto, px(0)),
        false => (px(0), Val::Auto),
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
    };

    // Change outline background
    if outline_bg.0 != outline_bg_token {
        commands
            .entity(switch_ent)
            .insert(ThemeBackgroundColor(outline_bg_token));
    }

    // Change outline border
    if outline_border.0 != outline_border_token {
        commands
            .entity(switch_ent)
            .insert(ThemeBorderColor(outline_border_token));
    }

    // Change slide background color
    if slide_bg_color.0 != slide_bg_token {
        commands
            .entity(slide_ent)
            .insert(ThemeBackgroundColor(slide_bg_token));
    }

    // Change slide border color
    if slide_border_color.0 != slide_border_token {
        commands
            .entity(slide_ent)
            .insert(ThemeBorderColor(slide_border_token));
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
