//! Disclosure twisty: a bare chevron that rotates between closed and open.
use core::f32::consts::FRAC_PI_2;

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
use bevy::picking::{Pickable, PickingSystems};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::ui::{AlignItems, Checked, InteractionDisabled, JustifyContent, Node, UiTransform};
use bevy::ui_widgets::{Checkbox, checkbox_self_update};

use crate::constants::{lucide, size};
use crate::cursor::EntityCursor;
use crate::display::icon;
use crate::focus::FocusIndicator;
use crate::theme::InheritableThemeTextToken;
use crate::tokens;
use crate::utils::anim::AnimState;
use crate::utils::hierarchy::descendant_get_mut;

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
            DisclosureFrame
            TabIndex(0)
            FocusIndicator
            on(checkbox_self_update)
            AccessibilityNode(accesskit::Node::new(Role::DisclosureTriangle))
            EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)
            InheritableThemeTextToken(tokens::BUTTON_TEXT)
            Children [
                @icon(lucide::CHEVRON_RIGHT)
                DisclosureChevron
                // The glyph is the pick target's decoration, not a target itself.
                Pickable::IGNORE
                AnimState::rotation(0.0, FRAC_PI_2)
                UiTransform::default()
            ]
        }
    }
}

// Plain root marker, inserted on both the retained and imm paths. The systems key on
// this, not [`PlumeDisclosure`], which only the retained path inserts.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct DisclosureFrame;

// Marker for the rotating chevron glyph inside a [`PlumeDisclosure`].
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct DisclosureChevron;

fn update_disclosure_styles(
    q_disclosures: Query<
        (Entity, Has<InteractionDisabled>, Has<Checked>),
        (
            With<DisclosureFrame>,
            // Added<DisclosureFrame> guarantees the initial style pass on spawn.
            Or<(
                Added<DisclosureFrame>,
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
    q_disclosures: Query<(Entity, Has<InteractionDisabled>, Has<Checked>), With<DisclosureFrame>>,
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

// Point every listed disclosure's chevron at its open/closed angle and re-pick its
// enabled/disabled color.
fn apply(
    disclosures: impl Iterator<Item = (Entity, bool, bool)>,
    q_children: &Query<&Children>,
    q_chevron: &mut Query<&mut AnimState, With<DisclosureChevron>>,
    commands: &mut Commands,
) {
    for (disclosure_ent, disabled, checked) in disclosures {
        if let Some(mut chevron) = descendant_get_mut(disclosure_ent, q_children, q_chevron) {
            chevron.set_target(if checked { 1.0 } else { 0.0 });
        }

        let text_token = if disabled {
            tokens::BUTTON_TEXT_DISABLED
        } else {
            tokens::BUTTON_TEXT
        };
        let cursor_shape = match disabled {
            true => bevy::window::SystemCursorIcon::NotAllowed,
            false => bevy::window::SystemCursorIcon::Pointer,
        };
        commands.entity(disclosure_ent).insert((
            InheritableThemeTextToken(text_token),
            EntityCursor::System(cursor_shape),
        ));
    }
}

// Plugin which registers the systems for updating the disclosure styles.
pub(crate) struct DisclosurePlugin;

impl Plugin for DisclosurePlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            (update_disclosure_styles, update_disclosure_styles_remove)
                .in_set(PickingSystems::Last),
        );
    }
}
