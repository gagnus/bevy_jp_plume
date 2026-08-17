//! Marker-based hierarchy walks shared by controls.
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::query::With;
use bevy::ecs::system::Query;

// `entity` itself if it carries marker `M`, else the nearest such ancestor.
pub(crate) fn nearest_with<M: Component>(
    entity: Entity,
    q_childof: &Query<&ChildOf>,
    q_marker: &Query<(), With<M>>,
) -> Option<Entity> {
    if q_marker.contains(entity) {
        return Some(entity);
    }
    q_childof
        .iter_ancestors(entity)
        .find(|ancestor| q_marker.contains(*ancestor))
}

// Find the first marked descendant of `root`.
pub(crate) fn descendant_with<M: Component>(
    root: Entity,
    q_children: &Query<&Children>,
    q_marker: &Query<(), With<M>>,
) -> Option<Entity> {
    q_children
        .iter_descendants(root)
        .find(|descendant| q_marker.contains(*descendant))
}
