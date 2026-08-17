//! Query-based hierarchy walks shared by controls.
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::query::{QueryData, QueryFilter, ROQueryItem};
use bevy::ecs::system::Query;

// `entity` itself if it matches `query`, else the nearest matching ancestor.
// Callers wanting ancestors only must ensure `entity` can never match.
pub(crate) fn nearest_with<D: QueryData, F: QueryFilter>(
    entity: Entity,
    q_childof: &Query<&ChildOf>,
    query: &Query<D, F>,
) -> Option<Entity> {
    if query.contains(entity) {
        return Some(entity);
    }
    q_childof
        .iter_ancestors(entity)
        .find(|ancestor| query.contains(*ancestor))
}

// First descendant of `root` (`root` itself excluded) matching `query`.
pub(crate) fn descendant_with<D: QueryData, F: QueryFilter>(
    root: Entity,
    q_children: &Query<&Children>,
    query: &Query<D, F>,
) -> Option<Entity> {
    q_children
        .iter_descendants(root)
        .find(|descendant| query.contains(*descendant))
}

// `nearest_with`, but returning the match's query item. Callers that also
// want the entity add `Entity` to the query's data tuple.
pub(crate) fn nearest_get<'q, D: QueryData, F: QueryFilter>(
    entity: Entity,
    q_childof: &Query<&ChildOf>,
    query: &'q Query<D, F>,
) -> Option<ROQueryItem<'q, 'q, D>> {
    if let Ok(item) = query.get(entity) {
        return Some(item);
    }
    q_childof
        .iter_ancestors(entity)
        .find_map(|ancestor| query.get(ancestor).ok())
}

// `nearest_get` with mutable access to the item.
pub(crate) fn nearest_get_mut<'q, 's, D: QueryData, F: QueryFilter>(
    entity: Entity,
    q_childof: &Query<&ChildOf>,
    query: &'q mut Query<'_, 's, D, F>,
) -> Option<D::Item<'q, 's>> {
    let found = nearest_with(entity, q_childof, &*query)?;
    Some(
        query
            .get_mut(found)
            .expect("entity was just found via this same query"),
    )
}

// `descendant_with`, but returning the match's query item. Callers that
// also want the entity add `Entity` to the query's data tuple.
pub(crate) fn descendant_get<'q, D: QueryData, F: QueryFilter>(
    root: Entity,
    q_children: &Query<&Children>,
    query: &'q Query<D, F>,
) -> Option<ROQueryItem<'q, 'q, D>> {
    q_children
        .iter_descendants(root)
        .find_map(|descendant| query.get(descendant).ok())
}

// `descendant_get` with mutable access to the item.
pub(crate) fn descendant_get_mut<'q, 's, D: QueryData, F: QueryFilter>(
    root: Entity,
    q_children: &Query<&Children>,
    query: &'q mut Query<'_, 's, D, F>,
) -> Option<D::Item<'q, 's>> {
    let found = descendant_with(root, q_children, &*query)?;
    Some(
        query
            .get_mut(found)
            .expect("entity was just found via this same query"),
    )
}
