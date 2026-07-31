//! HSV color picker: a saturation/value plane, a hue bar and a preview swatch,
//! composed from [`PlumeXyPad`] and [`PlumeColorSwatch`] and coordinated as one
//! retained control. Reports its color in [`ColorPickerValue`] and self-updates
//! it as the user drags, so it works dropped straight into a scene.
use core::f32::consts::PI;

use bevy::app::{Plugin, PostUpdate};
use bevy::color::{Color, Hsva, Srgba};
use bevy::ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    query::{Changed, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
};
use bevy::math::Vec2;
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::scene::prelude::*;
use bevy::ui::{
    AlignItems, AlignSelf, BackgroundGradient, ColorStop, Display, FlexDirection, Gradient,
    InterpolationColorSpace, LinearGradient, Node, Val, Val2, percent,
};
use bevy::ui_widgets::{SliderValue, ValueChange};

use crate::constants::size;
use crate::containers::space;
use crate::controls::{
    ColorSwatchValue, PlumeColorSwatch, PlumeNumberInput, PlumeXyPad, XyPadLock, XyPadValue,
};
use crate::display::caption;
use crate::font_styles::TextStyleRelay;

// The SV plane's side and the hue bar's dimensions, em-sized so the picker
// tracks the effective font. The bar shares the plane's height so the two line up.
const PLANE_SIZE: Val = size::em_from_px(220.0);
const HUE_BAR_WIDTH: Val = size::em_from_px(20.0);
// The hue bar's reticle spans the bar, so it is wider than it is tall.
const HUE_RETICLE_SIZE: Val2 = Val2 {
    x: size::em_from_px(24.0),
    y: size::em_from_px(12.0),
};

/// Scene props for [`PlumeColorPicker`].
#[derive(Default, Clone)]
pub struct PlumeColorPickerProps {
    /// Color the picker starts on before the user drags it.
    pub initial_color: Color,
}

/// A composed HSV color picker. Spawnable as a scene component; it lays out its
/// own SV plane, hue bar and swatch.
///
/// Reports [`ColorPickerValue`] and self-updates it as the user drags.
/// # Emitted events
/// * [`ValueChange<Color>`](bevy::ui_widgets::ValueChange) on each user edit.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
#[scene(PlumeColorPickerProps)]
#[require(ColorPickerValue)]
pub struct PlumeColorPicker;

/// The picker's current color — the public read/write surface. Setting it (from
/// an app or the imm layer) retargets the picker; the change is folded into the
/// working HSV truth with the hue preserved.
#[derive(Component, Clone, Copy, Reflect, Default)]
#[reflect(Component, Clone, Default)]
pub struct ColorPickerValue(pub Color);

/// Marks the saturation/value plane child.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorPickerSv;

/// Marks the hue bar child.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorPickerHue;

/// Marks the preview swatch child.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorPickerSwatch;

/// Which color component a numeric field edits.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Reflect)]
enum Channel {
    /// sRGB red.
    #[default]
    R,
    /// sRGB green.
    G,
    /// sRGB blue.
    B,
    /// Hue, degrees.
    H,
    /// Saturation.
    S,
    /// Value / brightness.
    V,
}

/// Marks a numeric field and names the color component it edits.
#[derive(Component, Default, Clone, Copy, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorPickerChannel(Channel);

// One labelled numeric field: a fixed-width caption and a number input carrying
// the channel marker. The value is seeded by the sync system on spawn.
fn channel_row(label: &'static str, channel: Channel, precision: usize, max: f32) -> impl Scene {
    let suffix = matches!(channel, Channel::H).then(|| "\u{b0}".to_string());
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::ZERO,
        }
        TextStyleRelay
        Children [
            (
                space(size::em_from_px(4.0))
            )
            (
                caption(label)
                Node { width: {size::TEXT_HEIGHT}, flex_shrink: 0.0 }
            ),
            (
                @PlumeNumberInput {
                    @precision: {precision},
                    @min: {0.0_f32},
                    @max: {max},
                    @suffix: {suffix},
                }
                Node { flex_grow: 1.0 }
                ColorPickerChannel({channel})
            ),
        ]
    }
}

impl PlumeColorPicker {
    fn scene(props: PlumeColorPickerProps) -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: size::em_from_px(8.0),
                align_items: AlignItems::Start,
            }
            PlumeColorPicker
            TextStyleRelay
            template_value(ColorPickerValue(props.initial_color))
            Children [
                // Saturation (x) / value (y). The plane's hue-tinted gradient is
                // driven by the sync system, since it tracks the hue bar.
                (
                    @PlumeXyPad
                    Node { width: PLANE_SIZE, height: PLANE_SIZE }
                    ColorPickerSv
                ),
                // Hue bar: a pad locked to x, so it only moves along the hue axis.
                (
                    @PlumeXyPad {
                        @reticle_size: HUE_RETICLE_SIZE
                    }
                    Node { width: HUE_BAR_WIDTH, height: PLANE_SIZE }
                    XyPadLock { x: {Some(0.5)}, y: {None} }
                    BackgroundGradient({hue_gradient()})
                    ColorPickerHue
                ),
                // Preview swatch over the numeric fields; the swatch grows to fill
                // the spare height so the R/G/B/H/S/V rows sit at the bottom.
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_self: AlignSelf::Stretch,
                        row_gap: size::em_from_px(2.0),
                        flex_grow: 1.0,
                    }
                    TextStyleRelay
                    Children [
                        (
                            @PlumeColorSwatch
                            Node { width: percent(100), flex_grow: 1.0 }
                            ColorPickerSwatch
                        ),
                        // sRGB channels, then a gap, then the HSV channels.
                        (channel_row("R", Channel::R, 3, 1.0)),
                        (channel_row("G", Channel::G, 3, 1.0)),
                        (channel_row("B", Channel::B, 3, 1.0)),
                        (space(size::GAP_TIGHT / 2.0)),
                        (channel_row("H", Channel::H, 0, 360.0)),
                        (channel_row("S", Channel::S, 3, 1.0)),
                        (channel_row("V", Channel::V, 3, 1.0)),
                    ]
                ),
            ]
        }
    }
}

// The marked view children of one picker, gathered in a single descendant walk.
struct PickerViews {
    sv: Option<Entity>,
    hue: Option<Entity>,
    swatch: Option<Entity>,
    channels: Vec<(Entity, Channel)>,
}

// Below this the two representations are treated as agreeing: a field push whose
// value is this close to what the HSV implies is our own echo, not a user edit.
const CHANNEL_EPS: f32 = 1.0e-6;

// Push the working HSV out to every view (plane, hue bar, swatch, numeric fields)
// and the public `Color` mirror, whenever the working color changes.
fn sync_color_to_views(
    q_picker: Query<(Entity, &ColorPickerValue), Changed<ColorPickerValue>>,
    q_children: Query<&Children>,
    q_sv: Query<(), With<ColorPickerSv>>,
    q_hue: Query<(), With<ColorPickerHue>>,
    q_swatch: Query<(), With<ColorPickerSwatch>>,
    q_channel: Query<&ColorPickerChannel>,
    mut q_xy: Query<&mut XyPadValue>,
    mut q_swatch_val: Query<&mut ColorSwatchValue>,
    q_slider: Query<&SliderValue>,
    mut commands: Commands,
) {
    for (root, color) in q_picker.iter() {
        // we push out in the color space the color is in
        let (hsva, srgba): (Hsva, Srgba) = (color.0.into(), color.0.into());
        let views = collect_views(root, &q_children, &q_sv, &q_hue, &q_swatch, &q_channel);

        // hsv zone
        if let Some(sv) = views.sv {
            set_xy(&mut q_xy, sv, Vec2::new(hsva.saturation, 1.0 - hsva.value));
            // The plane's white→hue tint follows the hue bar.
            commands
                .entity(sv)
                .insert(BackgroundGradient(sv_gradient(hsva.hue)));
        }
        if let Some(hue) = views.hue {
            set_xy(&mut q_xy, hue, Vec2::new(0.5, hsva.hue / 360.0));
        }

        // swatch
        if let Some(swatch) = views.swatch
            && let Ok(mut swatch_val) = q_swatch_val.get_mut(swatch)
            && swatch_val.0 != color.0
        {
            swatch_val.0 = color.0;
        }

        // channels
        for (field, channel) in views.channels {
            let target = channel_value(hsva, srgba, channel);
            // `SliderValue` is immutable, so a change is a re-insert; only push when
            // the field actually disagrees, so unedited fields stay quiet.
            let current = q_slider.get(field).map_or(f32::NAN, |slider| slider.0);
            if (current - target).abs() > CHANNEL_EPS {
                commands.entity(field).insert(SliderValue(target));
            }
        }
    }
}

// Only write the pad value when it actually moved, so a programmatic push doesn't
// re-trigger the view→model fold with a value it already agrees with.
fn set_xy(q_xy: &mut Query<&mut XyPadValue>, pad: Entity, target: Vec2) {
    if let Ok(mut value) = q_xy.get_mut(pad)
        && value.0 != target
    {
        value.0 = target;
    }
}

// Walk a picker's descendants for its marked view children in one pass.
fn collect_views(
    root: Entity,
    q_children: &Query<&Children>,
    q_sv: &Query<(), With<ColorPickerSv>>,
    q_hue: &Query<(), With<ColorPickerHue>>,
    q_swatch: &Query<(), With<ColorPickerSwatch>>,
    q_channel: &Query<&ColorPickerChannel>,
) -> PickerViews {
    let mut views = PickerViews {
        sv: None,
        hue: None,
        swatch: None,
        channels: Vec::new(),
    };
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if entity != root {
            if q_sv.contains(entity) {
                views.sv = Some(entity);
            } else if q_hue.contains(entity) {
                views.hue = Some(entity);
            } else if q_swatch.contains(entity) {
                views.swatch = Some(entity);
            } else if let Ok(channel) = q_channel.get(entity) {
                views.channels.push((entity, channel.0));
            }
        }
        if let Ok(children) = q_children.get(entity) {
            stack.extend(children.iter().copied());
        }
    }
    views
}

// The value a given channel's field should show for the current color.
fn channel_value(hsva: Hsva, srgb: Srgba, channel: Channel) -> f32 {
    match channel {
        Channel::R => srgb.red,
        Channel::G => srgb.green,
        Channel::B => srgb.blue,
        Channel::H => hsva.hue,
        Channel::S => hsva.saturation,
        Channel::V => hsva.value,
    }
}

// Fold a plane or hue-bar drag back into the working HSV. Both pads are direct
// children of the picker root, so `ChildOf` points straight at it.
fn fold_pad_edits(
    q_sv: Query<(&XyPadValue, &ChildOf), (Changed<XyPadValue>, With<ColorPickerSv>)>,
    q_hue: Query<(&XyPadValue, &ChildOf), (Changed<XyPadValue>, With<ColorPickerHue>)>,
    mut q_color: Query<&mut ColorPickerValue>,
    mut commands: Commands,
) {
    for (pad_value, parent) in q_sv.iter() {
        if let Ok(mut color) = q_color.get_mut(parent.parent()) {
            let mut hsva: Hsva = color.0.into();
            let (saturation, value) = (pad_value.0.x, 1.0 - pad_value.0.y);
            if hsva.saturation != saturation || hsva.value != value {
                hsva.saturation = saturation;
                hsva.value = value;
                color.0 = hsva.into();
                emit_value_change(parent.parent(), color.0, &mut commands);
            }
        }
    }
    for (value, parent) in q_hue.iter() {
        if let Ok(mut color) = q_color.get_mut(parent.parent()) {
            let mut hsva: Hsva = color.0.into();
            let hue = (value.0.y * 360.0).clamp(0.0, 360.0);
            if hsva.hue != hue {
                hsva.hue = hue;
                color.0 = hsva.into();
                emit_value_change(parent.parent(), color.0, &mut commands);
            }
        }
    }
}

// Announce a user-driven color change on the picker root. Programmatic pushes
// into `ColorPickerValue` deliberately stay silent.
fn emit_value_change(root: Entity, color: Color, commands: &mut Commands) {
    commands.trigger(ValueChange {
        source: root,
        value: color,
        is_final: true,
    });
}

// Fold a committed numeric-field edit back into the working HSV. Our own pushes
// from `sync_hsv_to_views` also mark `SliderValue` changed, so a field whose value
// still agrees with the HSV is skipped as an echo; only a genuine user edit folds.
fn fold_channel_edits(
    q_changed: Query<(Entity, &SliderValue, &ColorPickerChannel), Changed<SliderValue>>,
    q_childof: Query<&ChildOf>,
    q_is_picker: Query<(), With<PlumeColorPicker>>,
    mut q_color: Query<&mut ColorPickerValue>,
    mut commands: Commands,
) {
    for (field, slider, channel) in q_changed.iter() {
        let Some(root) = find_picker_root(field, &q_childof, &q_is_picker) else {
            continue;
        };
        let Ok(mut color) = q_color.get_mut(root) else {
            continue;
        };

        let (mut hsva, mut srgba) = (color.0.into(), color.0.into());

        if (slider.0 - channel_value(hsva, srgba, channel.0)).abs() <= CHANNEL_EPS {
            continue;
        }

        // we push out the color in the color space
        // that was changed
        match channel.0 {
            Channel::R => {
                srgba.red = slider.0;
                color.0 = srgba.into();
            }
            Channel::G => {
                srgba.green = slider.0;
                color.0 = srgba.into();
            }
            Channel::B => {
                srgba.blue = slider.0;
                color.0 = srgba.into();
            }
            Channel::H => {
                hsva.hue = slider.0;
                color.0 = hsva.into();
            }
            Channel::S => {
                hsva.saturation = slider.0;
                color.0 = hsva.into();
            }
            Channel::V => {
                hsva.value = slider.0;
                color.0 = hsva.into();
            }
        }
        emit_value_change(root, color.0, &mut commands);
    }
}

// Walk up from a numeric field to the picker root it belongs to.
fn find_picker_root(
    mut entity: Entity,
    q_childof: &Query<&ChildOf>,
    q_is_picker: &Query<(), With<PlumeColorPicker>>,
) -> Option<Entity> {
    loop {
        if q_is_picker.contains(entity) {
            return Some(entity);
        }
        entity = q_childof.get(entity).ok()?.parent();
    }
}

// The saturation/value plane for a fixed `hue`: white→hue across x, with a
// transparent→black layer down y composited over it. Pure gradients, no shader.
fn sv_gradient(hue: f32) -> Vec<Gradient> {
    let pure_hue = Color::from(Hsva::new(hue, 1.0, 1.0, 1.0));
    vec![
        // Horizontal: white (left) → pure hue (right). 90° = left→right.
        Gradient::Linear(LinearGradient {
            angle: PI * 0.5,
            stops: vec![
                ColorStop::new(Color::WHITE, percent(0.0)),
                ColorStop::new(pure_hue, percent(100.0)),
            ],
            color_space: InterpolationColorSpace::Srgba,
        }),
        // Vertical: transparent (top) → black (bottom). 180° = top→bottom.
        Gradient::Linear(LinearGradient {
            angle: PI,
            stops: vec![
                ColorStop::new(Color::NONE, percent(0.0)),
                ColorStop::new(Color::BLACK, percent(100.0)),
            ],
            color_space: InterpolationColorSpace::Srgba,
        }),
    ]
}

// The vertical hue strip: the spectral wheel from top (hue 0) to bottom (hue 360).
fn hue_gradient() -> Vec<Gradient> {
    let stops = (0..=6)
        .map(|i| {
            let hue = i as f32 * 60.0;
            ColorStop::new(Color::hsl(hue, 1.0, 0.5), percent(i as f32 / 6.0 * 100.0))
        })
        .collect();
    vec![Gradient::Linear(LinearGradient {
        angle: PI, // top→bottom
        stops,
        color_space: InterpolationColorSpace::Srgba,
    })]
}

/// Registers the color-picker sync systems.
pub(crate) struct ColorPickerPlugin;

impl Plugin for ColorPickerPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PostUpdate,
            (sync_color_to_views, fold_pad_edits, fold_channel_edits).chain(),
        );
    }
}
