//! Marker-based hierarchy walks shared by controls.
use bevy::ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    query::With,
    system::Query,
};

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
pub(crate) fn descendant<M: Component>(
    root: Entity,
    q_children: &Query<&Children>,
    q_marker: &Query<(), With<M>>,
) -> Option<Entity> {
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if entity != root && q_marker.contains(entity) {
            return Some(entity);
        }
        if let Ok(children) = q_children.get(entity) {
            stack.extend(children.iter().copied());
        }
    }
    None
}
