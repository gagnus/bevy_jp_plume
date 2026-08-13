//! A framework for inheritable font styles.
use bevy::app::{Inherited, Propagate, PropagateOver};
use bevy::asset::AssetServer;
use bevy::ecs::component::Component;
use bevy::ecs::entity::{Entity, EntityHashMap};
use bevy::ecs::hierarchy::ChildOf;
use bevy::ecs::query::{Changed, Has};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::SystemSet;
use bevy::ecs::system::{Commands, Local, Query, Res};
use bevy::ecs::template::FromTemplate;
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::{Scene, bsn};
use bevy::text::{
    EditableText, EmSize, FontFeatureTag, FontFeatures, FontSize, FontSource, RemSize, TextColor,
    TextFont,
};
use bevy::ui::ComputedUiRenderTargetInfo;
#[cfg(debug_assertions)]
use bevy::ui::CornerRadius;
use bevy::ui::widget::Text;
#[cfg(debug_assertions)]
use {
    bevy::ecs::{entity::EntityHashSet, name::Name, query::Without},
    bevy::log::warn,
    bevy::ui::{BoxShadow, Node, Outline, UiRect, UiTransform, Val, Val2},
};

use crate::constants::fonts;
use crate::theme::ThemedText;

/// Keeps a structural node on the text-style propagation chain without styling the
/// node itself — stamp it on any wrapper authoring `Val::Em` inside a scaled subtree.
// `ThemedText` keeps the wrapper on the propagation chain (the recurse filter
// drops any entity without it), while `PropagateOver` keeps the propagated
// `TextFont`/`TextColor` off the wrapper itself.
#[derive(Component, Default, Clone)]
#[require(ThemedText, PropagateOver::<TextFont>, PropagateOver::<TextColor>)]
pub struct TextStyleRelay;

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

// Bevy resolves `Val::Em` against a node's own `TextFont`, else its `EmSize`,
// else the `RemSize` resource — it does not propagate `EmSize`. This mirrors
// the inherited font onto every chain node so em-authored chrome scales with
// its context; `RemSize` is set to the standard size at plugin init, so nodes
// the mirror hasn't reached yet resolve at 1.0 scale rather than wrong.
pub(crate) fn mirror_em_size(
    changed: Query<
        (
            Entity,
            &Inherited<TextFont>,
            Option<&ComputedUiRenderTargetInfo>,
            Option<&EmSize>,
        ),
        Changed<Inherited<TextFont>>,
    >,
    rem_size: Res<RemSize>,
    mut commands: Commands,
) {
    for (entity, inherited, target, existing) in &changed {
        let logical_size = target.map(ComputedUiRenderTargetInfo::logical_size);
        let em = EmSize::from_font_size(
            inherited.0.font_size,
            logical_size.unwrap_or_default(),
            *rem_size,
        );
        if existing != Some(&em) {
            commands.entity(entity).insert(em);
        }
    }
}

// The regression net for the mirror above. An em value on a node the mirror
// never reaches resolves against `RemSize` — right at the standard font and
// wrong everywhere else, so the bug hides until someone scales a subtree. This
// names the node instead: a node authoring `Val::Em` with neither its own
// `TextFont` (text leaves resolve from that) nor an `EmSize` is off the chain
// and wants a `TextStyleRelay`.
//
// Debug builds only: it scans every node lacking an `EmSize`, which in an app
// with a large non-plume UI is not free, and the answer never differs in
// release.
#[cfg(debug_assertions)]
pub(crate) fn warn_em_without_em_size(
    suspects: Query<
        (
            Entity,
            Option<&Name>,
            &Node,
            Option<&Outline>,
            Option<&BoxShadow>,
            Option<&UiTransform>,
        ),
        (Without<EmSize>, Without<TextFont>),
    >,
    mut suspect_frames: Local<EntityHashMap<u8>>,
    mut warned: Local<EntityHashSet>,
) {
    // A freshly spawned node has no `EmSize` until propagation and the mirror
    // have both run, and a popup socket takes an extra hop; only a node still
    // bare after that is genuinely off the chain.
    const GRACE_FRAMES: u8 = 3;

    let mut current = EntityHashMap::default();
    for (entity, name, node, outline, shadow, transform) in &suspects {
        // Counting stops at the warning: a node that is genuinely off the chain
        // stays a suspect for the rest of the session, and there is nothing left
        // to learn from it.
        if warned.contains(&entity) {
            continue;
        }
        let fields = em_fields(node, outline, shadow, transform);
        if fields.is_empty() {
            continue;
        }
        let frames = suspect_frames.get(&entity).copied().unwrap_or(0) + 1;
        current.insert(entity, frames);
        if frames >= GRACE_FRAMES && warned.insert(entity) {
            warn!(
                entity = ?entity,
                name = name.map_or("<unnamed>", Name::as_str),
                fields = fields.join(", "),
                "Node authors `Val::Em` but has no `EmSize`, so it resolves against the global \
                 `RemSize` and stays standard-sized in a scaled subtree. Plume mirrors `EmSize` \
                 onto text-chain nodes only — give this node a `TextStyleRelay`."
            );
        }
    }
    *suspect_frames = current;
}

// The em-authoring fields of one node, named for the warning. `Em(0.0)` is zero
// at any font size, so it is not a miss.
#[cfg(debug_assertions)]
fn em_fields(
    node: &Node,
    outline: Option<&Outline>,
    shadow: Option<&BoxShadow>,
    transform: Option<&UiTransform>,
) -> Vec<&'static str> {
    fn is_em(val: Val) -> bool {
        matches!(val, Val::Em(v) if v != 0.0)
    }
    fn rect_is_em(rect: UiRect) -> bool {
        is_em(rect.left) || is_em(rect.right) || is_em(rect.top) || is_em(rect.bottom)
    }
    fn val2_is_em(val2: Val2) -> bool {
        is_em(val2.x) || is_em(val2.y)
    }
    fn corner_radius_is_em(corner_radius: CornerRadius) -> bool {
        is_em(corner_radius.x) || is_em(corner_radius.y)
    }

    let mut fields = Vec::new();
    let mut check = |present: bool, field| {
        if present {
            fields.push(field);
        }
    };
    check(
        is_em(node.left) || is_em(node.right) || is_em(node.top) || is_em(node.bottom),
        "offsets",
    );
    check(is_em(node.width) || is_em(node.height), "size");
    check(
        is_em(node.min_width)
            || is_em(node.min_height)
            || is_em(node.max_width)
            || is_em(node.max_height),
        "min/max size",
    );
    check(is_em(node.flex_basis), "flex_basis");
    check(is_em(node.row_gap) || is_em(node.column_gap), "gap");
    check(rect_is_em(node.margin), "margin");
    check(rect_is_em(node.padding), "padding");
    check(rect_is_em(node.border), "border");
    let radius = node.border_radius;
    check(
        corner_radius_is_em(radius.top_left)
            || corner_radius_is_em(radius.top_right)
            || corner_radius_is_em(radius.bottom_right)
            || corner_radius_is_em(radius.bottom_left),
        "border_radius",
    );
    check(
        outline.is_some_and(|outline| is_em(outline.width) || is_em(outline.offset)),
        "outline",
    );
    check(
        shadow.is_some_and(|shadow| {
            shadow.0.iter().any(|style| {
                is_em(style.x_offset)
                    || is_em(style.y_offset)
                    || is_em(style.spread_radius)
                    || is_em(style.blur_radius)
            })
        }),
        "box_shadow",
    );
    check(
        transform.is_some_and(|transform| val2_is_em(transform.translation)),
        "ui_transform",
    );
    fields
}

// Resolves each `InheritableFont` into a `Propagate<TextFont>` source: `None`
// fields fill from the nearest ancestor source, or the standard font at a root.
// A holder that is itself `Text` gets the plain `TextFont` too — `PropagateOver`
// blocks the output write, but a text leaf styles itself.
//
// A holder resolves from the ancestor chain rather than from its parent's
// `Inherited<TextFont>` alone, because propagation runs after this system: on the
// frame a subtree spawns, nothing in it has an inherited value yet. Reading the
// chain (and resolving the holders on it, outermost first) settles a whole nest
// in one frame, so a newly spawned caption never renders a frame at the standard
// size — a jump the layout above it would inherit as a height pop.
pub(crate) fn resolve_inheritable_font(
    holders: Query<(Entity, &InheritableFont, (Has<Text>, Has<EditableText>))>,
    parents: Query<&ChildOf>,
    existing: Query<&Propagate<TextFont>>,
    asset_server: Res<AssetServer>,
    mut resolved: Local<EntityHashMap<TextFont>>,
    mut chain: Local<Vec<Entity>>,
    mut commands: Commands,
) {
    // Holders resolved this run, keyed by entity: an outer holder is the base for
    // every holder under it, and one ancestor walk can settle several at once.
    resolved.clear();

    for (entity, _, (has_text, has_editable)) in &holders {
        // Walk up collecting the holders above this one, stopping at the first
        // already resolved this run. Only holders are consulted: `Inherited` is a
        // copy of some ancestor holder's font, and a fresh node's copy is seeded
        // from its parent at spawn (by the propagation plugin's `ChildOf`
        // observer), so on a spawn frame it still holds the pre-scaling font.
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
            let Ok((_, inheritable, _)) = holders.get(*holder) else {
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

        // A same-value re-insert would still ripple a re-resolve wave through
        // nested holders' `Inherited<TextFont>`.
        if existing.get(entity).is_ok_and(|p| p.0 == font) {
            continue;
        }
        // An `EditableText` field is a text leaf too — its required
        // `PropagateOver` otherwise leaves it on the default `TextFont`.
        if has_text || has_editable {
            commands
                .entity(entity)
                .insert((Propagate(font.clone()), font));
        } else {
            commands.entity(entity).insert(Propagate(font));
        }
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
    use bevy::text::FontSize;
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
            HierarchyPropagatePlugin::<TextFont, bevy::ecs::query::With<ThemedText>>::new(
                PostUpdate,
            ),
        ));
        app.init_asset::<bevy::text::Font>();
        app.insert_resource(RemSize(size::MEDIUM_FONT_PX));
        app.add_systems(
            PostUpdate,
            resolve_inheritable_font.before(bevy::app::PropagateSet::<TextFont>::default()),
        );
        app
    }

    // A scaled container, a plain wrapper, and a caption that only pins a face:
    // the caption's size has to come from the container, in the frame they spawn.
    // The wrapper is what makes this bite — the propagation plugin's `ChildOf`
    // observer seeds its `Inherited<TextFont>` from the unscaled font the
    // container had before this run, so anything trusting that copy resolves the
    // caption a frame behind.
    #[test]
    fn nested_holder_scales_in_its_first_frame() {
        let mut app = font_app();
        // An established tree at the standard font, as a dialog is by the time a
        // widget appears in it: its settled value is what the observer hands down.
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
                Children::spawn_one((
                    ThemedText,
                    Children::spawn_one((
                        Text::new("0.00/1.00s"),
                        ThemedText,
                        InheritableFont {
                            font: Some(FontSource::Handle(Default::default())),
                            ..Default::default()
                        },
                    )),
                )),
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
                    ThemedText,
                    InheritableFont {
                        font_size: Some(PlumeFontSize::Em(1.25)),
                        ..Default::default()
                    },
                    Children::spawn_one((
                        Text::new("nested"),
                        ThemedText,
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
