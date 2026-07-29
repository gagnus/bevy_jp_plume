//! A framework for inheritable font styles.
use bevy::app::{Propagate, PropagateOver};
use bevy::asset::Handle;
use bevy::ecs::{
    component::Component,
    lifecycle::Insert,
    observer::On,
    reflect::ReflectComponent,
    system::{Commands, Query},
    template::FromTemplate,
};
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::text::{Font, FontSize, FontWeight, TextColor, TextFont};

use crate::theme::ThemedText;

// Structural node that relays inherited text styles without consuming them:
// `ThemedText` keeps the wrapper on the propagation chain (the recurse filter
// drops any entity without it), while `PropagateOver` keeps the propagated
// `TextFont`/`TextColor` off the wrapper itself.
#[derive(Component, Default, Clone)]
#[require(ThemedText, PropagateOver::<TextFont>, PropagateOver::<TextColor>)]
pub(crate) struct TextStyleRelay;

/// A component which, when inserted on an entity, will load the given font and propagate it
/// downward to any child text entity that has the [`ThemedText`] marker.
#[derive(Component, Default, Clone, Debug, Reflect, FromTemplate)]
#[reflect(Component, Default)]
#[require(ThemedText, PropagateOver::<TextFont>)]
pub struct InheritableFont {
    /// The font handle.
    pub font: Handle<Font>,
    /// The desired font size.
    pub font_size: FontSize,
    /// The desired font weight.
    pub weight: FontWeight,
}

/// An observer which looks for changes to the [`InheritableFont`] component on an entity, and
/// propagates downward the font to all participating text entities.
pub(crate) fn on_changed_font(
    insert: On<Insert, InheritableFont>,
    font_style: Query<&InheritableFont>,
    mut commands: Commands,
) {
    if let Ok(inheritable_font) = font_style.get(insert.entity) {
        commands.entity(insert.entity).insert(Propagate(TextFont {
            font: inheritable_font.font.clone().into(),
            font_size: inheritable_font.font_size,
            weight: inheritable_font.weight,
            ..Default::default()
        }));
    }
}
