//! Focus outlines and Tab-order upkeep for focusable controls.
use bevy::app::{Plugin, PostUpdate, PreUpdate};
use bevy::ecs::change_detection::DetectChanges;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::lifecycle::RemovedComponents;
use bevy::ecs::query::{Added, With};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{InputFocus, InputFocusVisible};
use bevy::picking::PickingSystems;
use bevy::platform::collections::HashSet;
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::ui::{InteractionDisabled, Outline, UiSystems};

use crate::constants::size;
use crate::theme::UiTheme;
use crate::tokens;

/// Marker: show a focus outline on this entity when it or an ancestor is focused.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FocusIndicator;

/// Marker: show a focus outline on this entity when it or a descendant is focused.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FocusWithinIndicator;

fn manage_focus_indicators(
    mut commands: Commands,
    input_focus: Res<InputFocus>,
    input_focus_visible: Res<InputFocusVisible>,
    q_indicators: Query<Entity, With<FocusIndicator>>,
    q_within_indicators: Query<Entity, With<FocusWithinIndicator>>,
    q_children: Query<&Children>,
    q_parents: Query<&ChildOf>,
    theme: Res<UiTheme>,
) {
    if !input_focus.is_changed() && !input_focus_visible.is_changed() && !theme.is_changed() {
        return;
    }

    let ring = |theme: &UiTheme| Outline {
        color: theme.color(&tokens::FOCUS_RING),
        width: size::FOCUS_RING_WIDTH,
        offset: size::FOCUS_RING_OFFSET,
    };

    let mut visited = HashSet::<Entity>::with_capacity(q_indicators.count());
    if let Some(focus) = input_focus.get()
        && input_focus_visible.0
    {
        for entity in q_children
            .iter_descendants(focus)
            .chain(core::iter::once(focus))
        {
            if q_indicators.contains(entity) {
                commands.entity(entity).insert(ring(&theme));
                visited.insert(entity);
            }
        }

        for entity in q_parents
            .iter_ancestors(focus)
            .chain(core::iter::once(focus))
        {
            if q_within_indicators.contains(entity) {
                commands.entity(entity).insert(ring(&theme));
                visited.insert(entity);
            }
        }
    }

    for entity in q_indicators.iter().chain(q_within_indicators.iter()) {
        if !visited.contains(&entity) {
            commands.entity(entity).remove::<Outline>();
        }
    }
}

// Disabled controls leave the Tab order; negative keeps the click-to-focus marker in place.
// (The text input manages its own field marker: disabled state and `TabIndex` live on
// different entities there.)
fn sync_disabled_tab_index(
    q_added: Query<(Entity, &TabIndex), Added<InteractionDisabled>>,
    q_index: Query<&TabIndex>,
    mut removed: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    for (ent, index) in q_added.iter() {
        if index.0 >= 0 {
            commands.entity(ent).insert(TabIndex(-1));
        }
    }
    removed.read().for_each(|ent| {
        if q_index.get(ent).is_ok_and(|index| index.0 < 0) {
            commands.entity(ent).insert(TabIndex(0));
        }
    });
}

// Plugin which registers the focus outline and Tab-order systems.
pub(crate) struct FocusPlugin;

impl Plugin for FocusPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PostUpdate,
            manage_focus_indicators.in_set(UiSystems::Content),
        );
        app.add_systems(
            PreUpdate,
            sync_disabled_tab_index.in_set(PickingSystems::Last),
        );
    }
}
