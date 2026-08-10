//! BSN scene function for a hairline rule.
use bevy::app::{Plugin, PostUpdate};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::ChildOf;
use bevy::ecs::query::With;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::Query;
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::{Scene, bsn};
use bevy::ui::{AlignSelf, FlexDirection, Node, UiRect, UiSystems, Val};

use crate::constants::size;
use crate::font_styles::TextStyleRelay;
use crate::theme::ThemeBackgroundToken;
use crate::tokens;

/// A hairline rule taking its orientation from the container it sits in:
/// horizontal in a [`column`](crate::containers::column), vertical in a
/// [`row`](crate::containers::row). Spans the container's content box; add
/// [`SeparatorBleed`] to run edge to edge through the padding.
pub fn separator() -> impl Scene {
    bsn! {
        Node {
            flex_basis: size::CONTAINER_BORDER,
            flex_grow: 0.0,
            flex_shrink: 0.0,
            align_self: AlignSelf::Stretch,
        }
        ThemeBackgroundToken(tokens::SEPARATOR)
    }
}

/// Put this on a [`separator`] to run it edge to edge: negative margins mirror
/// the parent container's padding, tracking it if it changes.
// `TextStyleRelay` keeps the entity on the font chain so an em-padded parent's
// mirrored margins resolve at the same em.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
#[require(TextStyleRelay)]
pub struct SeparatorBleed;

fn apply_separator_bleed(
    separators: Query<(Entity, &ChildOf), With<SeparatorBleed>>,
    mut nodes: Query<&mut Node>,
) {
    for (entity, child_of) in &separators {
        let Ok(parent) = nodes.get(child_of.parent()) else {
            continue;
        };
        let bleed = |padding: Val| match padding {
            Val::Auto => Val::ZERO,
            padding => -padding,
        };
        let margin = match parent.flex_direction {
            FlexDirection::Column | FlexDirection::ColumnReverse => UiRect {
                left: bleed(parent.padding.left),
                right: bleed(parent.padding.right),
                ..UiRect::ZERO
            },
            FlexDirection::Row | FlexDirection::RowReverse => UiRect {
                top: bleed(parent.padding.top),
                bottom: bleed(parent.padding.bottom),
                ..UiRect::ZERO
            },
        };
        let Ok(mut node) = nodes.get_mut(entity) else {
            continue;
        };
        if node.margin != margin {
            node.margin = margin;
        }
    }
}

pub(crate) struct SeparatorPlugin;

impl Plugin for SeparatorPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(PostUpdate, apply_separator_bleed.before(UiSystems::Layout));
    }
}
