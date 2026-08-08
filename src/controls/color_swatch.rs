//! Color preview swatch: a themed rounded box filled with the current color.
use bevy::app::{Plugin, PostUpdate};
use bevy::color::Color;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::Children;
use bevy::ecs::query::Changed;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::system::{Commands, Query};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::ui::{BackgroundColor, Node, PositionType, Val};

use crate::constants::size;
use crate::font_styles::TextStyleRelay;
use crate::theme::ThemeBorderToken;
use crate::tokens;

/// A color swatch widget.
///
/// This is spawnable by inheriting it as a "scene component"; size and corners can be
/// overridden by inserting a [`Node`](bevy::ui::Node) beside it.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct PlumeColorSwatch;

/// Component that contains the value of the color swatch. This is copied to the swatch's
/// background color.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct ColorSwatchValue(pub Color);

impl PlumeColorSwatch {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                height: size::ROW_HEIGHT,
                width: size::ROW_HEIGHT,
                border_radius: size::CORNER_RADIUS_SMALL,
            }
            PlumeColorSwatch
            ColorSwatchValue
            // Em-sized chrome needs the chain's `EmSize`.
            TextStyleRelay
            Children [
                // Border overlay: sits over the fill so the color reaches every edge.
                (
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::ZERO,
                        top: Val::ZERO,
                        right: Val::ZERO,
                        bottom: Val::ZERO,
                        border: size::CONTROL_BORDER,
                        border_radius: size::CORNER_RADIUS_SMALL,
                    }
                    TextStyleRelay
                    ThemeBorderToken(tokens::COLOR_SWATCH_BORDER)
                ),
            ]
        }
    }
}

fn update_swatch_color(
    q_swatch: Query<(Entity, &ColorSwatchValue), Changed<ColorSwatchValue>>,
    mut commands: Commands,
) {
    for (swatch_ent, value) in q_swatch.iter() {
        commands.entity(swatch_ent).insert(BackgroundColor(value.0));
    }
}

// Plugin which registers the systems for updating the swatch color.
pub(crate) struct ColorSwatchPlugin;

impl Plugin for ColorSwatchPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(PostUpdate, update_swatch_color);
    }
}
