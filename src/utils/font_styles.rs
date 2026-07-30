//! A framework for inheritable font styles.
use bevy::app::{Inherited, Propagate, PropagateOver};
use bevy::asset::AssetServer;
use bevy::ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    hierarchy::ChildOf,
    query::Has,
    reflect::ReflectComponent,
    system::{Commands, Query, Res},
    template::FromTemplate,
    world::Ref,
};
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::text::{FontSize, FontSource, TextColor, TextFont};
use bevy::ui::widget::Text;

use crate::constants::{fonts, size};
use crate::theme::ThemedText;

// Structural node that relays inherited text styles without consuming them:
// `ThemedText` keeps the wrapper on the propagation chain (the recurse filter
// drops any entity without it), while `PropagateOver` keeps the propagated
// `TextFont`/`TextColor` off the wrapper itself.
#[derive(Component, Default, Clone)]
#[require(ThemedText, PropagateOver::<TextFont>, PropagateOver::<TextColor>)]
pub(crate) struct TextStyleRelay;

/// Establishes the font for descendant [`ThemedText`] entities; `None` fields
/// inherit from the nearest ancestor source (the standard font at a root).
#[derive(Component, Default, Clone, Debug, Reflect, FromTemplate)]
#[reflect(Component, Default)]
#[require(ThemedText, PropagateOver::<TextFont>)]
pub struct InheritableFont {
    /// The font face; `None` inherits the ancestor's face.
    #[template(built_in)]
    pub font: Option<FontSource>,
    /// The font size; `None` inherits the ancestor's size.
    #[template(built_in)]
    pub font_size: Option<FontSize>,
}

// Resolves each `InheritableFont` into a `Propagate<TextFont>` source: `None`
// fields fill from the parent's `Inherited<TextFont>`, or the standard font at
// a root. Partial holders re-resolve on parent changes, settling one override
// level per frame. A holder that is itself `Text` gets the plain `TextFont`
// too — `PropagateOver` blocks the output write, but a text leaf styles itself.
pub(crate) fn resolve_inheritable_font(
    holders: Query<(Entity, Ref<InheritableFont>, Option<&ChildOf>, Has<Text>)>,
    inherited: Query<Ref<Inherited<TextFont>>>,
    existing: Query<&Propagate<TextFont>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for (entity, inheritable, child_of, is_text) in &holders {
        let parent_inherited = child_of.and_then(|c| inherited.get(c.parent()).ok());
        let partial = inheritable.font.is_none() || inheritable.font_size.is_none();
        let needs_resolve = inheritable.is_changed()
            || (partial && parent_inherited.as_ref().is_some_and(|i| i.is_changed()));
        if !needs_resolve {
            continue;
        }

        let mut font = parent_inherited
            .map(|i| i.0.clone())
            .unwrap_or_else(|| TextFont {
                font: asset_server.load(fonts::REGULAR).into(),
                font_size: size::MEDIUM_FONT,
                ..Default::default()
            });
        if let Some(face) = &inheritable.font {
            font.font = face.clone();
        }
        if let Some(font_size) = inheritable.font_size {
            font.font_size = font_size;
        }

        // A same-value re-insert would still ripple a re-resolve wave through
        // nested holders' `Inherited<TextFont>`.
        if existing.get(entity).is_ok_and(|p| p.0 == font) {
            continue;
        }
        if is_text {
            commands
                .entity(entity)
                .insert((Propagate(font.clone()), font));
        } else {
            commands.entity(entity).insert(Propagate(font));
        }
    }
}
