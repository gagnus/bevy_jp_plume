//! Body spacing shared by every container with a frame/body split: two
//! components a caller sets on the frame, which each widget's relay forwards.
use bevy::ecs::component::Component;
use bevy::ecs::reflect::ReflectComponent;
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::ui::{Node, UiRect, Val};

/// Set on a container to space the items its body stacks: section, tabs, scroll
/// area, dialog, panel, popup. Anything else has no relay and ignores it.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct BodyGap(pub Val);

/// Set on a container to override its body's padding — the same containers
/// [`BodyGap`] lists, and silent on any other in the same way.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct BodyPadding(pub UiRect);

// Write whichever of the two the caller set onto `node`; the other keeps what the
// scene gave it. Both gap axes, since the non-stacking one is inert without wrapping.
pub(crate) fn apply_body_style(
    node: &mut Node,
    gap: Option<&BodyGap>,
    padding: Option<&BodyPadding>,
) {
    if let Some(gap) = gap {
        if node.row_gap != gap.0 {
            node.row_gap = gap.0;
        }
        if node.column_gap != gap.0 {
            node.column_gap = gap.0;
        }
    }
    if let Some(padding) = padding
        && node.padding != padding.0
    {
        node.padding = padding.0;
    }
}
