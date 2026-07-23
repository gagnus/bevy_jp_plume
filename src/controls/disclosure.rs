//! Disclosure twisty: a bare chevron that rotates between closed and open.
use core::f32::consts::FRAC_PI_2;

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
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::{Pickable, PickingSystems};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::prelude::*;
use bevy_ui::{AlignItems, Checked, InteractionDisabled, JustifyContent, Node, UiTransform};
use bevy_ui_widgets::{Checkbox, checkbox_self_update};

use crate::{
    constants::{font_awesome, size},
    cursor::EntityCursor,
    display::fa_icon,
    focus::FocusIndicator,
    theme::InheritableThemeTextColor,
    tokens,
    utils::anim::AnimState,
};

/// A disclosure twisty: a chevron that points right when closed and eases through
/// a quarter turn to point down when open. No fill or border.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct PlumeDisclosure;

impl PlumeDisclosure {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                width: size::ROW_HEIGHT,
                height: size::ROW_HEIGHT,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
            }
            Checkbox
            PlumeDisclosure
            TabIndex(0)
            FocusIndicator
            on(checkbox_self_update)
            AccessibilityNode(accesskit::Node::new(Role::DisclosureTriangle))
            EntityCursor::System(bevy_window::SystemCursorIcon::Pointer)
            InheritableThemeTextColor(tokens::BUTTON_TEXT)
            Children [
                (
                    fa_icon(font_awesome::solid::ANGLE_RIGHT)
                    DisclosureChevron
                    // The glyph is the pick target's decoration, not a target itself.
                    Pickable::IGNORE
                    template_value(AnimState::rotation(0.0, FRAC_PI_2))
                    UiTransform::default()
                )
            ]
        }
    }
}

/// Marker for the rotating chevron glyph inside a [`PlumeDisclosure`].
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct DisclosureChevron;

fn update_disclosure_styles(
    q_disclosures: Query<
        (Entity, Has<InteractionDisabled>, Has<Checked>),
        (
            With<PlumeDisclosure>,
            // Added<PlumeDisclosure> guarantees the initial style pass on spawn.
            Or<(
                Added<PlumeDisclosure>,
                Added<Checked>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_chevron: Query<&mut AnimState, With<DisclosureChevron>>,
    mut commands: Commands,
) {
    apply(
        q_disclosures.iter(),
        &q_children,
        &mut q_chevron,
        &mut commands,
    );
}

fn update_disclosure_styles_remove(
    q_disclosures: Query<(Entity, Has<InteractionDisabled>, Has<Checked>), With<PlumeDisclosure>>,
    q_children: Query<&Children>,
    mut q_chevron: Query<&mut AnimState, With<DisclosureChevron>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_checked: RemovedComponents<Checked>,
    mut commands: Commands,
) {
    let touched: Vec<_> = removed_disabled
        .read()
        .chain(removed_checked.read())
        .filter_map(|ent| q_disclosures.get(ent).ok())
        .collect();
    apply(
        touched.into_iter(),
        &q_children,
        &mut q_chevron,
        &mut commands,
    );
}

/// Point every listed disclosure's chevron at its open/closed angle and re-pick its
/// enabled/disabled color.
fn apply(
    disclosures: impl Iterator<Item = (Entity, bool, bool)>,
    q_children: &Query<&Children>,
    q_chevron: &mut Query<&mut AnimState, With<DisclosureChevron>>,
    commands: &mut Commands,
) {
    for (disclosure_ent, disabled, checked) in disclosures {
        for descendant in q_children.iter_descendants(disclosure_ent) {
            if let Ok(mut chevron) = q_chevron.get_mut(descendant) {
                chevron.set_target(if checked { 1.0 } else { 0.0 });
            }
        }

        let text_token = if disabled {
            tokens::BUTTON_TEXT_DISABLED
        } else {
            tokens::BUTTON_TEXT
        };
        let cursor_shape = match disabled {
            true => bevy_window::SystemCursorIcon::NotAllowed,
            false => bevy_window::SystemCursorIcon::Pointer,
        };
        commands.entity(disclosure_ent).insert((
            InheritableThemeTextColor(text_token),
            EntityCursor::System(cursor_shape),
        ));
    }
}

/// Plugin which registers the systems for updating the disclosure styles.
pub struct DisclosurePlugin;

impl Plugin for DisclosurePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_disclosure_styles, update_disclosure_styles_remove)
                .in_set(PickingSystems::Last),
        );
    }
}
