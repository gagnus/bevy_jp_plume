//! Parent-aware default width for controls with nothing inside to measure.
use bevy_app::{Plugin, PostUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::ChildOf,
    query::{Has, Without},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_ui::{AlignItems, AlignSelf, Display, FlexDirection, Node, UiSystems, Val};

/// Width for a control with nothing inside to measure, applied only where the
/// layout won't size it; a stretching column fills it, and an explicit width wins.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct DefaultWidth(pub Val);

// Marks a width this system wrote, so a reparent can hand sizing back; without it
// a system-applied width is indistinguishable from an app-set one.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct AppliedDefaultWidth;

fn parent_stretches(parent: &Node, node: &Node) -> bool {
    // Only flex parents stretch children this way; grid/block get the fallback.
    if parent.display != Display::Flex {
        return false;
    }
    if !matches!(
        parent.flex_direction,
        FlexDirection::Column | FlexDirection::ColumnReverse
    ) {
        return false;
    }
    match node.align_self {
        // `Default` is flexbox's own default, which is stretch.
        AlignSelf::Auto => matches!(
            parent.align_items,
            AlignItems::Stretch | AlignItems::Default
        ),
        align => align == AlignSelf::Stretch,
    }
}

// Runs every frame, not on change detection: the outcome depends on the parent's
// flex_direction/align_items and the node's align_self, any of which may change.
fn apply_default_width(
    mut q_controls: Query<(
        Entity,
        &mut Node,
        &ChildOf,
        &DefaultWidth,
        Has<AppliedDefaultWidth>,
    )>,
    // Disjoint from the query above so both can borrow `Node`; safe because these
    // controls are leaves, so a parent is never itself a `DefaultWidth` entity.
    q_parents: Query<&Node, Without<DefaultWidth>>,
    mut commands: Commands,
) {
    for (entity, mut node, child_of, default, applied) in q_controls.iter_mut() {
        let Ok(parent) = q_parents.get(child_of.parent()) else {
            continue;
        };
        if parent_stretches(parent, &node) {
            // Only hand back a width this system wrote; an app's own no longer matches.
            if applied && node.width == default.0 {
                node.width = Val::Auto;
                commands.entity(entity).remove::<AppliedDefaultWidth>();
            }
        } else if node.width == Val::Auto {
            node.width = default.0;
            commands.entity(entity).insert(AppliedDefaultWidth);
        }
    }
}

/// Registers the [`DefaultWidth`] resolution system.
pub struct DefaultWidthPlugin;

impl Plugin for DefaultWidthPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // Before layout, so a control is never laid out at the wrong width.
        app.add_systems(PostUpdate, apply_default_width.in_set(UiSystems::Prepare));
    }
}
