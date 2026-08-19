//! A framework for inheritable font styles.
use bevy::app::Propagate;
use bevy::asset::AssetServer;
use bevy::ecs::component::Component;
use bevy::ecs::entity::{Entity, EntityHashMap};
use bevy::ecs::hierarchy::ChildOf;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::SystemSet;
use bevy::ecs::system::{Commands, Local, Query, Res};
use bevy::ecs::template::FromTemplate;
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::{Scene, bsn};
use bevy::text::{FontFeatureTag, FontFeatures, FontSize, FontSource, TextFont};

use crate::constants::fonts;

/// The `PostUpdate` pass resolving `InheritableFont`s into propagated fonts; a
/// system writing font sizes (a zoom, a UI scale) runs `.before` it to land the same frame.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontStyleSystems;

/// A font size for an [`InheritableFont`]: logical pixels, or a multiple of
/// the inherited size.
#[derive(Copy, Clone, Debug, PartialEq, Reflect)]
pub enum PlumeFontSize {
    /// This size in logical pixels, replacing the inherited one.
    Px(f32),
    /// A multiple of the inherited size — CSS's `em`.
    Em(f32),
    /// A multiple of the `RemSize` global resource.
    Rem(f32),
}

impl Default for PlumeFontSize {
    fn default() -> Self {
        Self::Rem(1.0)
    }
}

impl From<f32> for PlumeFontSize {
    fn from(px: f32) -> Self {
        Self::Px(px)
    }
}

/// Establishes the font for descendant text; `None` fields inherit from the
/// nearest ancestor source (the standard font at a root).
// Deliberately no `PropagateOver<TextFont>`: the holder receives its own
// resolved font, which is what drives `EmSize` for its em-sized layout.
#[derive(Component, Default, Clone, Debug, Reflect, FromTemplate)]
#[reflect(Component, Default)]
pub struct InheritableFont {
    /// The font face; `None` inherits the ancestor's face.
    #[template(built_in)]
    pub font: Option<FontSource>,
    /// The font size; `None` inherits the ancestor's size.
    #[template(built_in)]
    pub font_size: Option<PlumeFontSize>,
    /// Font features (small caps &c.); `None` inherits the ancestor's.
    #[template(built_in)]
    pub font_features: Option<FontFeatures>,
}

/// Renders descendant text in small caps, whatever the input casing. Compose it
/// onto any scene: on a text entity it restyles that text, on a container it
/// restyles everything under it.
// Patches the same [`InheritableFont`] a caller may set fields on directly; bsn
// merges the two patches, so `InheritableFont { font_size }` survives beside it.
pub fn small_caps() -> impl Scene {
    bsn! {
        InheritableFont {
            font_features: FontFeatures::from([
                FontFeatureTag::SMALL_CAPS,
                FontFeatureTag::CAPS_TO_SMALL_CAPS,
            ]),
        }
    }
}

// Turns each `InheritableFont` into a `Propagate<TextFont>` source: `None`
// fields fill in from the nearest ancestor holder, or the standard font at a root.
//
// Ancestor values come from the holders themselves, not `Inherited<TextFont>`,
// which is stale on the frame a subtree spawns — new text would render one
// frame at the wrong size, and the layout around it would visibly pop.
pub(crate) fn resolve_inheritable_font(
    holders: Query<(Entity, &InheritableFont)>,
    parents: Query<&ChildOf>,
    existing: Query<&Propagate<TextFont>>,
    asset_server: Res<AssetServer>,
    mut resolved: Local<EntityHashMap<TextFont>>,
    mut chain: Local<Vec<Entity>>,
    mut commands: Commands,
) {
    // Holders already resolved this run; an outer holder is the base for every
    // holder below it, so one ancestor walk can settle several.
    resolved.clear();

    for (entity, _) in &holders {
        // Walk up, collecting unresolved holders until one resolved this run
        // (or the root) provides the base.
        chain.clear();
        let mut base = None;
        let mut cursor = Some(entity);
        while let Some(current) = cursor {
            if let Some(done) = resolved.get(&current) {
                base = Some(done.clone());
                break;
            }
            if holders.contains(current) {
                chain.push(current);
            }
            cursor = parents.get(current).ok().map(ChildOf::parent);
        }

        // Outermost first, each holder overriding the fields it declares.
        let mut font = base.unwrap_or_else(|| TextFont {
            font: asset_server.load(fonts::REGULAR).into(),
            font_size: FontSize::Rem(1.0),
            ..Default::default()
        });
        for holder in chain.iter().rev() {
            let Ok((_, inheritable)) = holders.get(*holder) else {
                continue;
            };
            if let Some(face) = &inheritable.font {
                font.font = face.clone();
            }
            match inheritable.font_size {
                Some(PlumeFontSize::Px(px)) => font.font_size = FontSize::Px(px),
                Some(PlumeFontSize::Em(factor)) => font.font_size = font.font_size * factor,
                Some(PlumeFontSize::Rem(factor)) => font.font_size = FontSize::Rem(factor),
                None => {}
            }
            if let Some(features) = &inheritable.font_features {
                font.font_features = features.clone();
            }
            resolved.insert(*holder, font.clone());
        }

        // Skip same-value re-inserts: they would still mark `Inherited<TextFont>`
        // changed all the way down the subtree.
        if existing.get(entity).is_ok_and(|p| p.0 == font) {
            continue;
        }
        commands.entity(entity).insert(Propagate(font));
    }
}

#[cfg(test)]
mod tests {
    use bevy::MinimalPlugins;
    use bevy::app::{App, HierarchyPropagatePlugin, PostUpdate};
    use bevy::asset::{AssetApp, AssetPlugin};
    use bevy::ecs::hierarchy::Children;
    use bevy::ecs::schedule::IntoScheduleConfigs;
    use bevy::ecs::spawn::SpawnRelated;
    use bevy::text::{FontSize, RemSize};
    use bevy::ui::widget::Text;

    use super::*;
    use crate::style::size;

    // The font pipeline as `PlumeCorePlugin` wires it, minus everything that
    // needs a window: resolve, then propagate.
    fn font_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            HierarchyPropagatePlugin::<TextFont>::new(PostUpdate),
        ));
        app.init_asset::<bevy::text::Font>();
        app.insert_resource(RemSize(size::MEDIUM_FONT_PX));
        app.add_systems(
            PostUpdate,
            resolve_inheritable_font.before(bevy::app::PropagateSet::<TextFont>::default()),
        );
        app
    }

    // The wrapper between container and caption is what makes this bite: its
    // `Inherited<TextFont>` copy is stale on the spawn frame, so resolving
    // through it would size the caption a frame late.
    #[test]
    fn nested_holder_scales_in_its_first_frame() {
        let mut app = font_app();
        // A settled tree at the standard font, as a dialog is by the time a
        // widget appears in it.
        let root = app.world_mut().spawn(InheritableFont::default()).id();
        app.update();
        app.update();

        let container = app
            .world_mut()
            .spawn((
                ChildOf(root),
                InheritableFont {
                    font_size: Some(PlumeFontSize::Px(12.0)),
                    ..Default::default()
                },
                Children::spawn_one((Children::spawn_one((
                    Text::new("0.00/1.00s"),
                    InheritableFont {
                        font: Some(FontSource::Handle(Default::default())),
                        ..Default::default()
                    },
                )),)),
            ))
            .id();

        app.update();

        let wrapper = app.world().get::<Children>(container).unwrap()[0];
        let caption = app.world().get::<Children>(wrapper).unwrap()[0];

        assert_eq!(
            app.world().get::<TextFont>(caption).map(|f| f.font_size),
            Some(FontSize::Px(12.0)),
            "caption fell back to the standard font for a frame",
        );
    }

    // A relative size multiplies whatever the chain resolves above it, and
    // nested relatives compound — in the frame they spawn, like the test above.
    #[test]
    fn relative_size_multiplies_inherited() {
        let mut app = font_app();
        let root = app.world_mut().spawn(InheritableFont::default()).id();
        app.update();
        app.update();

        let container = app
            .world_mut()
            .spawn((
                ChildOf(root),
                InheritableFont {
                    font_size: Some(PlumeFontSize::Px(20.0)),
                    ..Default::default()
                },
                Children::spawn_one((
                    Text::new("header"),
                    InheritableFont {
                        font_size: Some(PlumeFontSize::Em(1.25)),
                        ..Default::default()
                    },
                    Children::spawn_one((
                        Text::new("nested"),
                        InheritableFont {
                            font_size: Some(PlumeFontSize::Em(2.0)),
                            ..Default::default()
                        },
                    )),
                )),
            ))
            .id();

        app.update();

        let header = app.world().get::<Children>(container).unwrap()[0];
        let nested = app.world().get::<Children>(header).unwrap()[0];

        assert_eq!(
            app.world().get::<TextFont>(header).map(|f| f.font_size),
            Some(FontSize::Px(25.0)),
        );
        assert_eq!(
            app.world().get::<TextFont>(nested).map(|f| f.font_size),
            Some(FontSize::Px(50.0)),
        );
    }
}
