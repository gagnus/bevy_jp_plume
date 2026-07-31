//! A framework for inheritable font styles.
use bevy::app::{Inherited, Propagate, PropagateOver};
use bevy::asset::AssetServer;
use bevy::ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    hierarchy::ChildOf,
    query::{Changed, Has},
    reflect::ReflectComponent,
    system::{Commands, Query, Res},
    template::FromTemplate,
    world::Ref,
};
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::text::{
    EditableText, EmSize, FontFeatures, FontSize, FontSource, RemSize, TextColor, TextFont,
};
use bevy::ui::ComputedUiRenderTargetInfo;
use bevy::ui::widget::Text;
#[cfg(debug_assertions)]
use {
    bevy::ecs::{
        entity::{EntityHashMap, EntityHashSet},
        name::Name,
        query::Without,
        system::Local,
    },
    bevy::log::warn,
    bevy::ui::{BoxShadow, Node, Outline, UiRect, UiTransform, Val, Val2},
};

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
    /// Font features (small caps &c.); `None` inherits the ancestor's.
    #[template(built_in)]
    pub font_features: Option<FontFeatures>,
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
        val2_is_em(radius.top_left)
            || val2_is_em(radius.top_right)
            || val2_is_em(radius.bottom_right)
            || val2_is_em(radius.bottom_left),
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
// fields fill from the parent's `Inherited<TextFont>`, or the standard font at
// a root. Partial holders re-resolve on parent changes, settling one override
// level per frame. A holder that is itself `Text` gets the plain `TextFont`
// too — `PropagateOver` blocks the output write, but a text leaf styles itself.
pub(crate) fn resolve_inheritable_font(
    holders: Query<(
        Entity,
        Ref<InheritableFont>,
        Option<&ChildOf>,
        (Has<Text>, Has<EditableText>),
    )>,
    inherited: Query<Ref<Inherited<TextFont>>>,
    existing: Query<&Propagate<TextFont>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for (entity, inheritable, child_of, (has_text, has_editable)) in &holders {
        // An `EditableText` field is a text leaf too — its required
        // `PropagateOver` otherwise leaves it on the default `TextFont`.
        let is_text = has_text || has_editable;
        let parent_inherited = child_of.and_then(|c| inherited.get(c.parent()).ok());
        let partial = inheritable.font.is_none()
            || inheritable.font_size.is_none()
            || inheritable.font_features.is_none();
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
        if let Some(features) = &inheritable.font_features {
            font.font_features = features.clone();
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
