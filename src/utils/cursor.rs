//! Sets the mouse cursor from the hovered entity, using bevy_picking's cursor support.
//!
//! bevy's `CursorIconPlugin` copies the hovered entity's [`EntityCursor`] to the window;
//! plume adds [`CapturePointer`] so a dragged control stays hovered until release.
use bevy::app::{App, Plugin, PreUpdate};
use bevy::ecs::component::Component;
use bevy::ecs::hierarchy::ChildOf;
use bevy::ecs::message::MessageReader;
use bevy::ecs::query::{With, Without};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Query, ResMut};
use bevy::picking::PickingSystems;
use bevy::picking::cursor::CursorIconPlugin;
pub use bevy::picking::cursor::{DefaultCursor, EntityCursor, OverrideCursor};
use bevy::picking::events::PointerDragStart;
use bevy::picking::hover::PointerCaptureMap;
use bevy::picking::pointer::PointerButton;
use bevy::reflect::Reflect;
use bevy::reflect::std_traits::ReflectDefault;
use bevy::ui::InteractionDisabled;

use crate::utils::hierarchy::nearest_with;

/// Captures the pointer when a primary-button drag starts on this entity or a descendant.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct CapturePointer;

// Drag start on an entity with `CapturePointer` (and no `InteractionDisabled`)
// captures the pointer.
fn capture_on_drag_start(
    mut drag_starts: MessageReader<PointerDragStart>,
    q_childof: Query<&ChildOf>,
    q_capture: Query<(), (With<CapturePointer>, Without<InteractionDisabled>)>,
    mut capture_map: ResMut<PointerCaptureMap>,
) {
    for drag_start in drag_starts.read() {
        if drag_start.button != PointerButton::Primary {
            continue;
        }
        if let Some(entity) = nearest_with(drag_start.entity, &q_childof, &q_capture) {
            capture_map.capture(drag_start.pointer.id, entity, drag_start.hit.clone());
        }
    }
}

// bevy's cursor plugin plus pointer capture for `CapturePointer` controls.
pub(crate) struct PlumeCursorPlugin;

impl Plugin for PlumeCursorPlugin {
    fn build(&self, app: &mut App) {
        // Feathers adds the same plugin, and adding a plugin twice panics.
        if !app.is_plugin_added::<CursorIconPlugin>() {
            app.add_plugins(CursorIconPlugin);
        }
        app.add_systems(
            PreUpdate,
            capture_on_drag_start.in_set(PickingSystems::Last),
        );
    }
}
