//! Provides a way to automatically set the mouse cursor based on hovered entity.
use bevy::app::{App, Plugin, PreUpdate};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::ChildOf;
use bevy::ecs::query::{With, Without};
use bevy::ecs::reflect::{ReflectComponent, ReflectResource};
use bevy::ecs::resource::Resource;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::ecs::template::FromTemplate;
use bevy::picking::PickingSystems;
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::PointerId;
use bevy::prelude::Deref;
use bevy::reflect::Reflect;
use bevy::reflect::std_traits::ReflectDefault;
use bevy::ui::Pressed;
#[cfg(feature = "custom_cursor")]
use bevy::window::CustomCursor;
use bevy::window::{CursorIcon, SystemCursorIcon, Window};

use crate::utils::hierarchy::nearest_get;

/// A resource that specifies the cursor icon to be used when the mouse is not hovering over
/// any other entity. This is used to set the default cursor icon for the window.
#[derive(Deref, Resource, Debug, Clone, Default, Reflect)]
#[reflect(Resource, Debug, Default)]
pub struct DefaultCursor(pub EntityCursor);

/// A component that specifies the cursor shape to be used when the pointer hovers over an entity.
/// This is copied to the windows's [`CursorIcon`] component.
///
/// This is effectively the same type as `bevy::window::CustomCursor` but with
/// different methods, and used in different places.
#[derive(Component, Debug, Clone, Reflect, PartialEq, Eq, FromTemplate)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
pub enum EntityCursor {
    #[cfg(feature = "custom_cursor")]
    /// Custom cursor image.
    Custom(CustomCursor),
    #[default]
    /// System provided cursor icon.
    System(SystemCursorIcon),
}

/// A resource used to override any [`EntityCursor`] cursor changes.
///
/// This is meant for cases like loading where you don't want the cursor to imply you
/// can interact with something.
#[derive(Deref, Resource, Debug, Clone, Default, Reflect)]
#[reflect(Resource, Default, Clone, Debug)]
pub struct OverrideCursor(pub Option<EntityCursor>);

/// Holds this entity's [`EntityCursor`] while it is `Pressed`.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct CursorLock;

impl EntityCursor {
    /// Convert the [`EntityCursor`] to a [`CursorIcon`] so that it can be inserted into a
    /// window.
    pub fn to_cursor_icon(&self) -> CursorIcon {
        match self {
            #[cfg(feature = "custom_cursor")]
            EntityCursor::Custom(custom_cursor) => CursorIcon::Custom(custom_cursor.clone()),
            EntityCursor::System(icon) => CursorIcon::from(*icon),
        }
    }

    /// Compare the [`EntityCursor`] to a [`CursorIcon`] so that we can see whether or not
    /// the window cursor needs to be changed.
    pub fn eq_cursor_icon(&self, cursor_icon: &CursorIcon) -> bool {
        // `as_system` lets bevy::window check its own `custom_cursor` feature; matching
        // on it directly can't stay exhaustive-and-unreachable-free across every combination.
        match (self, cursor_icon, cursor_icon.as_system()) {
            #[cfg(feature = "custom_cursor")]
            (EntityCursor::Custom(custom), CursorIcon::Custom(other), _) => custom == other,
            (EntityCursor::System(system), _, Some(cursor_icon)) => *system == *cursor_icon,
            _ => false,
        }
    }
}

impl Default for EntityCursor {
    fn default() -> Self {
        EntityCursor::System(Default::default())
    }
}

// System which updates the window cursor icon whenever the mouse hovers over an entity with
// a [`CursorIcon`] component. If no entity is hovered, the cursor icon is set to
// the cursor in the [`DefaultCursor`] resource.
pub(crate) fn update_cursor(
    mut commands: Commands,
    hover_map: Option<Res<HoverMap>>,
    parent_query: Query<&ChildOf>,
    cursor_query: Query<&EntityCursor, Without<Window>>,
    locked_query: Query<&EntityCursor, (With<CursorLock>, With<Pressed>, Without<Window>)>,
    q_windows: Query<(Entity, Option<&CursorIcon>), With<Window>>,
    r_default_cursor: Res<DefaultCursor>,
    r_override_cursor: Res<OverrideCursor>,
) {
    let cursor = r_override_cursor
        .0
        .as_ref()
        .or_else(|| locked_query.iter().next())
        .unwrap_or_else(|| {
            hover_map
                .and_then(|hover_map| match hover_map.get(&PointerId::Mouse) {
                    Some(hover_set) => hover_set
                        .keys()
                        .find_map(|entity| nearest_get(*entity, &parent_query, &cursor_query)),
                    None => None,
                })
                .unwrap_or(&r_default_cursor)
        });

    for (entity, prev_cursor) in q_windows.iter() {
        if let Some(prev_cursor) = prev_cursor
            && cursor.eq_cursor_icon(prev_cursor)
        {
            continue;
        }
        commands.entity(entity).insert(cursor.to_cursor_icon());
    }
}

// Plugin that supports automatically changing the cursor based on the hovered entity.
pub(crate) struct CursorIconPlugin;

impl Plugin for CursorIconPlugin {
    fn build(&self, app: &mut App) {
        if app.world().get_resource::<DefaultCursor>().is_none() {
            app.init_resource::<DefaultCursor>();
            app.init_resource::<OverrideCursor>();
        }
        app.add_systems(PreUpdate, update_cursor.in_set(PickingSystems::Last));
    }
}
