//! Color preview swatch: a themed rounded box filled with the current color,
//! over a checkerboard that shows through a translucent fill.
use bevy::app::{Plugin, PostUpdate};
use bevy::asset::{Assets, Handle, RenderAssetUsages};
use bevy::color::{Alpha, Color};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::Children;
use bevy::ecs::query::{Changed, With, Without};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::system::{Commands, Local, Query, ResMut};
use bevy::image::Image;
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::scene::prelude::*;
use bevy::ui::widget::{ImageNode, NodeImageMode};
use bevy::ui::{BackgroundColor, Node, PositionType, Val, VisualBox, percent};

use crate::constants::size;
use crate::font_styles::TextStyleRelay;
use crate::theme::ThemeBorderToken;
use crate::tokens;
use crate::utils::hierarchy::descendant;

/// Scene props for [`PlumeColorSwatch`].
#[derive(Clone)]
pub struct PlumeColorSwatchProps {
    /// Color the swatch shows until [`ColorSwatchValue`] is written.
    pub initial_color: Color,
    /// Show alpha: a checker reads through a translucent fill. `false` paints
    /// the color opaque, leaving its alpha untouched.
    pub alpha: bool,
}

impl Default for PlumeColorSwatchProps {
    fn default() -> Self {
        PlumeColorSwatchProps {
            initial_color: Color::default(),
            alpha: true,
        }
    }
}

/// A color swatch. Spawnable as a scene component; override its size and corners by
/// inserting a [`Node`](bevy::ui::Node) beside it.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
#[scene(PlumeColorSwatchProps)]
pub struct PlumeColorSwatch;

/// Component that contains the value of the color swatch. This is copied to the swatch's
/// fill color; alpha below one lets the checkerboard behind it show through.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct ColorSwatchValue(pub Color);

// A checkerboard layer marking translucency, tiled behind whatever sits over it.
// The texture is attached by `attach_checker`, which owns the shared handle.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub(crate) struct CheckerUnderlay;

// Marks the fill layer the swatch's color is painted onto.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorSwatchFill;

// Whether this swatch shows alpha; `false` paints its color opaque.
#[derive(Component, Clone, Reflect)]
#[reflect(Component, Clone)]
struct SwatchAlpha(bool);

impl PlumeColorSwatch {
    fn scene(props: PlumeColorSwatchProps) -> impl Scene {
        let checker: Vec<Box<dyn Scene>> = props
            .alpha
            .then(|| -> Box<dyn Scene> {
                Box::new(bsn! {
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::ZERO,
                        top: Val::ZERO,
                        width: percent(100),
                        height: percent(100),
                    }
                    CheckerUnderlay
                })
            })
            .into_iter()
            .collect();
        bsn! {
            Node {
                height: size::ROW_HEIGHT,
                width: size::ROW_HEIGHT,
            }
            template_value(ColorSwatchValue(props.initial_color))
            template_value(SwatchAlpha(props.alpha))
            // Em-sized chrome needs the chain's `EmSize`.
            TextStyleRelay
            Children [
                {checker},
                (
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::ZERO,
                        top: Val::ZERO,
                        width: percent(100),
                        height: percent(100),
                    }
                    ColorSwatchFill
                ),
                // Border overlay: sits over the fill so the color reaches every edge.
                (
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::ZERO,
                        top: Val::ZERO,
                        width: percent(100),
                        height: percent(100),
                        border: size::HAIRLINE,
                    }
                    ThemeBorderToken(tokens::COLOR_SWATCH_BORDER)
                ),
            ]
        }
    }
}

fn update_swatch_color(
    q_swatch: Query<(Entity, &ColorSwatchValue, Option<&SwatchAlpha>), Changed<ColorSwatchValue>>,
    q_children: Query<&Children>,
    q_fill: Query<(), With<ColorSwatchFill>>,
    mut commands: Commands,
) {
    for (swatch_ent, value, alpha) in q_swatch.iter() {
        let color = match alpha.is_none_or(|alpha| alpha.0) {
            true => value.0,
            false => value.0.with_alpha(1.0),
        };
        if let Some(fill) = descendant(swatch_ent, &q_children, &q_fill) {
            commands.entity(fill).insert(BackgroundColor(color));
        }
    }
}

// Every underlay gets the one shared checker texture, created on first need so
// nothing depends on plugin build order.
fn attach_checker(
    q_checkers: Query<Entity, (With<CheckerUnderlay>, Without<ImageNode>)>,
    mut images: ResMut<Assets<Image>>,
    mut checker: Local<Option<Handle<Image>>>,
    mut commands: Commands,
) {
    if q_checkers.is_empty() {
        return;
    }
    let handle = checker
        .get_or_insert_with(|| images.add(checker_image()))
        .clone();
    for entity in q_checkers.iter() {
        commands.entity(entity).insert(ImageNode {
            image: handle.clone(),
            image_mode: NodeImageMode::Tiled {
                tile_x: true,
                tile_y: true,
                stretch_value: 1.0,
            },
            visual_box: VisualBox::PaddingBox,
            ..Default::default()
        });
    }
}

// The classic two-grey transparency checker, one 2×2 pattern tile.
fn checker_image() -> Image {
    const QUAD: usize = 4;
    const SIDE: usize = QUAD * 2;
    let mut data = Vec::with_capacity(SIDE * SIDE * 4);
    for y in 0..SIDE {
        for x in 0..SIDE {
            let level = if ((x / QUAD) + (y / QUAD)).is_multiple_of(2) {
                0x66
            } else {
                0x99
            };
            data.extend([level, level, level, 0xFF]);
        }
    }
    Image::new(
        Extent3d {
            width: SIDE as u32,
            height: SIDE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

// Plugin which registers the systems for updating the swatch color.
pub(crate) struct ColorSwatchPlugin;

impl Plugin for ColorSwatchPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(PostUpdate, (update_swatch_color, attach_checker));
    }
}
