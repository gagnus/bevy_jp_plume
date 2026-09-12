//! App-registered data components: imm code stores per-widget state on the
//! widget's own entity - `get` reads it, `insert` writes it back (deferred).
use alloc::sync::Arc;

use bevy::app::App;
use bevy::ecs::component::Component;
use bevy::ecs::world::FilteredEntityRef;
use bevy::log::warn_once;
use bevy_immediate::ImmCapAccessRequestsResource;

use super::{ImmResponse, PlumeCaps, Ui};

/// Grants the imm layer read access to app components, for [`Ui::get`] and
/// [`ImmResponse::get`]. Writes need no registration - they go through commands.
pub trait ImmDataAppExt {
    /// Register `T` as imm-readable data. Must run after [`crate::PlumePlugins`]
    /// and before the app runs: the access set freezes once imm systems start.
    fn register_imm_data<T: Component>(&mut self) -> &mut Self;
}

impl ImmDataAppExt for App {
    fn register_imm_data<T: Component>(&mut self) -> &mut Self {
        let world = self.world_mut();
        assert!(
            world.contains_resource::<ImmCapAccessRequestsResource<PlumeCaps>>(),
            "register_imm_data::<{}> before PlumePlugins was added",
            core::any::type_name::<T>()
        );
        world.resource_scope(
            |world, mut requests: bevy::ecs::world::Mut<ImmCapAccessRequestsResource<PlumeCaps>>| {
                Arc::get_mut(&mut requests.capabilities)
                    .expect(
                        "register_imm_data after an imm system initialized: the access set is \
                         frozen once the app runs",
                    )
                    .request_component_read::<T>(world);
            },
        );
        self
    }
}

// Read `T` off a capability entity, telling "absent" apart from "present but
// never registered" - which would otherwise read as an eternal, silent `None`.
fn registered_get<'e, T: Component>(entity: &'e FilteredEntityRef) -> Option<&'e T> {
    if let Some(value) = entity.get::<T>() {
        return Some(value);
    }
    assert!(
        !entity.contains::<T>(),
        "`{ty}` is on the entity but unreadable from imm code: call \
         `app.register_imm_data::<{ty}>()`",
        ty = core::any::type_name::<T>()
    );
    None
}

/// Entity data on the current container: state private to a stretch of imm code
/// lives here rather than in an app resource, and despawns with the widget.
impl Ui<'_, '_> {
    /// The data component `T` on the container this scope is filling. `None` until
    /// a frame after [`insert`](Self::insert), matching [`hovered`](Self::hovered).
    pub fn get<T: Component + Clone>(&self) -> Option<T> {
        let entity = self.0.current_entity()?;
        let entity = self.0.ctx().cap_entities.get(entity).ok()?;
        registered_get::<T>(&entity).cloned()
    }

    /// Store `value` on the container this scope is filling, readable next frame.
    /// Skipped while an equal value is already there, so `Changed<T>` stays honest.
    pub fn insert<T: Component + PartialEq>(&mut self, value: T) {
        let Some(entity) = self.0.current_entity() else {
            warn_once!("Ui::insert at root scope: there is no entity to hold the value");
            return;
        };
        if let Ok(current) = self.0.ctx().cap_entities.get(entity)
            && registered_get::<T>(&current) == Some(&value)
        {
            return;
        }
        self.0.ctx_mut().commands.entity(entity).insert(value);
    }
}

/// Entity data, available on every kind.
impl<K> ImmResponse<'_, '_, '_, K> {
    /// The data component `T` on this widget's entity. `None` until a frame after
    /// [`insert`](Self::insert) - reads trail writes by one frame.
    pub fn get<T: Component + Clone>(&self) -> Option<T> {
        let entity = self.e.cap_get_entity().ok()?;
        registered_get::<T>(&entity).cloned()
    }

    /// Store `value` on this widget's entity, readable next frame. Skipped while
    /// an equal value is already there, so `Changed<T>` stays honest.
    pub fn insert<T: Component + PartialEq>(mut self, value: T) -> Self {
        let unchanged = self
            .e
            .cap_get_entity()
            .is_ok_and(|entity| registered_get::<T>(&entity) == Some(&value));
        if !unchanged {
            self.e.entity_commands().insert(value);
        }
        self
    }
}
