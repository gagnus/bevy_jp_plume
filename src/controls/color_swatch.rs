//! Color preview swatch over an alpha checkerboard.
use bevy_app::{Plugin, PostUpdate};
use bevy_color::{Alpha, Color};
use bevy_ecs::{
    component::Component,
    hierarchy::Children,
    query::Changed,
    reflect::ReflectComponent,
    system::{Commands, Query},
};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::prelude::*;
use bevy_ui::{BackgroundColor, BorderRadius, Node, PositionType, ZIndex, percent, px};
use bevy_ui_render::ui_material::MaterialNode;

use crate::{
    alpha_pattern::{AlphaPattern, AlphaPatternMaterial},
    constants::size,
};

/// A color swatch widget.
///
/// This is spawnable by inheriting it as a "scene component" with optional
/// [`PlumeColorSwatchProps`].
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
#[scene(PlumeColorSwatchProps)]
pub struct PlumeColorSwatch;

/// Props used to construct a [`PlumeColorSwatch`] scene.
#[derive(Default)]
pub struct PlumeColorSwatchProps {
    /// Set a percentage of the swatch to display the opaque version of the
    /// current color.
    pub opaque_color_percentage: f32,
}

/// Component that contains the value of the color swatch. This is copied to the child element
/// background.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct ColorSwatchValue(pub Color);

/// Marker identifying the color swatch foreground, the piece that actually displays the color
/// in front of the alpha pattern. This exists so that users can reach in and change the color
/// dynamically.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct ColorSwatchFg;

impl PlumeColorSwatch {
    fn scene(props: PlumeColorSwatchProps) -> impl Scene {
        let non_alpha_fg = (props.opaque_color_percentage > 0.0).then(|| {
            bsn! {
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(100.0 - props.opaque_color_percentage.clamp(0.0, 100.0)),
                    top: px(0),
                    bottom: px(0),
                    right: px(0),
                    border_radius: BorderRadius::right(px(size::CORNER_RADIUS)),
                }
                ColorSwatchFg
                BackgroundColor(Color::srgb(1.0, 0.0, 1.0))
                ZIndex(1)
            }
        });

        bsn! {
            Node {
                height: size::ROW_HEIGHT,
                min_width: size::ROW_HEIGHT,
                border_radius: px(size::CORNER_RADIUS),
            }
            PlumeColorSwatch
            ColorSwatchValue
            AlphaPattern
            MaterialNode::<AlphaPatternMaterial>
            Children [
                (
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        bottom: px(0),
                        right: px(0),
                        border_radius: px(size::CORNER_RADIUS),
                    }
                    ColorSwatchFg
                    BackgroundColor({Color::srgb(1.0, 0.0, 1.0).with_alpha(0.5)})
                ),
                non_alpha_fg
            ]
        }
    }
}

fn update_swatch_color(
    q_swatch: Query<(&ColorSwatchValue, &Children), Changed<ColorSwatchValue>>,
    mut commands: Commands,
) {
    for (value, children) in q_swatch.iter() {
        if let Some(first_child) = children.first() {
            commands
                .entity(*first_child)
                .insert(BackgroundColor(value.0));
        }

        if let Some(second_child) = children.get(1) {
            commands
                .entity(*second_child)
                .insert(BackgroundColor(value.0.with_alpha(1.0)));
        }
    }
}

/// Plugin which registers the observers for updating the swatch color.
pub struct ColorSwatchPlugin;

impl Plugin for ColorSwatchPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(PostUpdate, update_swatch_color);
    }
}
