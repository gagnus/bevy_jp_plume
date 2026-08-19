//! HSV color picker: a hue wheel around a saturation/value plane, an alpha bar
//! and a preview swatch, composed from [`PlumeXyPad`] and [`PlumeColorSwatch`] and coordinated
//! as one retained control. Reports its color in [`ColorPickerValue`] and self-updates
//! it as the user drags, so it works dropped straight into a scene.
use core::f32::consts::{PI, TAU};

use bevy::app::{Plugin, PostUpdate};
use bevy::color::{Alpha, Color, Hsva, Srgba};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::query::{Changed, With};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query};
use bevy::math::Vec2;
use bevy::picking::Pickable;
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::ui::{
    AlignItems, AlignSelf, AngularColorStop, BackgroundGradient, BorderGradient, BorderRadius,
    ColorStop, ConicGradient, Display, FlexDirection, Gradient, InterpolationColorSpace,
    JustifyContent, LinearGradient, Node, PositionType, UiPosition, UiRect, Val, Val2, em, percent,
};
use bevy::ui_widgets::{SliderValue, ValueChange};

use super::color_swatch::CheckerUnderlay;
use crate::constants::size;
use crate::containers::space;
use crate::controls::{
    ColorSwatchValue, PlumeColorSwatch, PlumeNumberInput, PlumeXyPad, XyPadLock, XyPadRing,
    XyPadValue,
};
use crate::display::caption;
use crate::utils::hierarchy::nearest_with;

// Wheel geometry in px-at-standard-font; the em constants and the ring's
// centerline derive from these, so a tweak here moves everything together.
const WHEEL_SIZE_PX: f32 = 250.0;
const RING_THICKNESS_PX: f32 = 20.0;
const WHEEL_SIZE: Val = size::em_from_px(WHEEL_SIZE_PX);
const RING_THICKNESS: Val = size::em_from_px(RING_THICKNESS_PX);

// The SV plane's side, sized to sit inside the ring's inner circle.
const PLANE_SIZE: Val = size::em_from_px(136.0);

// The ring's centerline in the wheel pad's normalized units.
const RING_RADIUS: f32 = (WHEEL_SIZE_PX - RING_THICKNESS_PX) / 2.0 / WHEEL_SIZE_PX;
// Presses engage the wheel only on the painted band, plus 4px of slop each side.
const RING_HIT_WIDTH: f32 = (RING_THICKNESS_PX + 8.0) / WHEEL_SIZE_PX;
const ALPHA_BAR_HEIGHT: Val = size::em_from_px(20.0);

const ALPHA_RETICLE_SIZE: Val2 = Val2 {
    x: Val::Em(1.0),
    y: size::em_from_px(24.0),
};

/// Scene props for [`PlumeColorPicker`].
#[derive(Clone)]
pub struct PlumeColorPickerProps {
    /// Color the picker starts on before the user drags it.
    pub initial_color: Color,
    /// Offer alpha: the alpha bar and A field. `false` hides both and leaves
    /// the color's alpha as it arrived.
    pub alpha: bool,
}

impl Default for PlumeColorPickerProps {
    fn default() -> Self {
        PlumeColorPickerProps {
            initial_color: Color::default(),
            alpha: true,
        }
    }
}

/// A composed HSV color picker. Spawnable as a scene component; it lays out its
/// own hue wheel, inscribed SV plane, alpha bar and swatch.
///
/// Reports [`ColorPickerValue`] and self-updates it as the user drags.
/// # Emitted events
/// * [`ValueChange<Color>`](bevy::ui_widgets::ValueChange) on each user edit.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
#[scene(PlumeColorPickerProps)]
pub struct PlumeColorPicker;

// Plain root marker, inserted on both the retained and imm paths. The systems key on
// this — and it carries the picker's requirements — rather than the
// [`PlumeColorPicker`] scene component, which only the retained path inserts.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
#[require(ColorPickerValue)]
struct ColorPickerFrame;

/// The picker's current color — the public read/write surface. Setting it (from
/// an app or the imm layer) retargets the picker; the change is folded into the
/// working HSV truth with the hue preserved.
#[derive(Component, Clone, Copy, Reflect, Default)]
#[reflect(Component, Clone, Default)]
pub struct ColorPickerValue(pub Color);

// Marks the saturation/value plane child.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorPickerSv;

// Marks the hue bar child.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorPickerHue;

// Marks the alpha bar's pad.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorPickerAlpha;

// Marks the alpha bar's gradient layer, repainted as the color moves.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorPickerAlphaRamp;

// Marks the preview swatch child.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorPickerSwatch;

// Which color component a numeric field edits.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Reflect)]
enum Channel {
    /// sRGB red.
    #[default]
    R,
    /// sRGB green.
    G,
    /// sRGB blue.
    B,
    /// Alpha.
    A,
    /// Hue, degrees.
    H,
    /// Saturation.
    S,
    /// Value / brightness.
    V,
}

// Marks a numeric field and names the color component it edits.
#[derive(Component, Default, Clone, Copy, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorPickerChannel(Channel);

// One labeled numeric field: a fixed-width caption and a number input carrying
// the channel marker. The value is seeded by the sync system on spawn.
fn channel_row(label: &'static str, channel: Channel, precision: usize, max: f32) -> impl Scene {
    let suffix = matches!(channel, Channel::H).then(|| "\u{b0}".to_string());
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::ZERO,
        }
        Children [
            space(em(0.25)),
            (
                caption(label)
                Node { width: em(1), flex_shrink: 0.0 }
            ),
            (
                @PlumeNumberInput {
                    @precision: precision,
                    @min: 0.0_f32,
                    @max: max,
                    @suffix: suffix,
                }
                Node { width: em(6) }
                ColorPickerChannel(channel)
            ),
        ]
    }
}

impl PlumeColorPicker {
    fn scene(props: PlumeColorPickerProps) -> impl Scene {
        // Alpha bar: checker under a color→transparent ramp, the pad on top of
        // both so the layers never shade its input.
        let alpha_bar: Vec<Box<dyn Scene>> = props
            .alpha
            .then(|| -> Box<dyn Scene> {
                Box::new(bsn! {
                    Node { width: percent(100), height: ALPHA_BAR_HEIGHT }
                    Children [
                        (
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::ZERO,
                                top: Val::ZERO,
                                right: Val::ZERO,
                                bottom: Val::ZERO,
                            }
                            CheckerUnderlay
                        ),
                        (
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::ZERO,
                                top: Val::ZERO,
                                right: Val::ZERO,
                                bottom: Val::ZERO,
                            }
                            ColorPickerAlphaRamp
                        ),
                        (
                            @PlumeXyPad {
                                @reticle_size: ALPHA_RETICLE_SIZE,
                            }
                            Node {
                                width: percent(100),
                                height: percent(100),
                                border: size::HAIRLINE,
                            }
                            XyPadLock { x: None, y: {Some(0.5)} }
                            ColorPickerAlpha
                        ),
                    ]
                })
            })
            .into_iter()
            .collect();
        let alpha_row: Vec<Box<dyn Scene>> = props
            .alpha
            .then(|| -> Box<dyn Scene> { Box::new(channel_row("A", Channel::A, 3, 1.0)) })
            .into_iter()
            .collect();
        let alpha = props.alpha;
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: size::em_from_px(8.0),
                align_items: AlignItems::Start,
            }
            ColorPickerFrame
            template_value(ColorPickerValue(props.initial_color))
            Children [
                // The wheel over the alpha bar, which spans the wheel's width.
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: size::em_from_px(8.0),
                    }
                    Children [
                        // The hue ring (a border-only circle), the wheel pad over
                        // it, and the SV pad on top keeping the picks over its square.
                        (
                            Node {
                                width: WHEEL_SIZE,
                                height: WHEEL_SIZE,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                            }
                            Children [
                                (
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::ZERO,
                                        top: Val::ZERO,
                                        right: Val::ZERO,
                                        bottom: Val::ZERO,
                                        border: RING_THICKNESS,
                                        border_radius: BorderRadius::MAX,
                                    }
                                    BorderGradient(hue_wheel_gradient())
                                    Pickable::IGNORE
                                ),
                                (
                                    @PlumeXyPad {
                                        @reticle_border_radius: BorderRadius::MAX_ELLIPTICAL,
                                    }
                                    Node {
                                        position_type: PositionType::Absolute,
                                        left: Val::ZERO,
                                        top: Val::ZERO,
                                        right: Val::ZERO,
                                        bottom: Val::ZERO,
                                    }
                                    XyPadRing {
                                        radius: RING_RADIUS,
                                        hit_width: {Some(RING_HIT_WIDTH)},
                                    }
                                    ColorPickerHue
                                ),
                                // Saturation (x) / value (y). The plane's hue-tinted
                                // gradient is driven by the sync system, since it
                                // tracks the wheel.
                                (
                                    @PlumeXyPad {
                                        @reticle_border_radius: BorderRadius::MAX_ELLIPTICAL,
                                    }
                                    Node { width: PLANE_SIZE, height: PLANE_SIZE }
                                    ColorPickerSv
                                ),
                            ]
                        ),
                        {alpha_bar},
                    ]
                ),
                // Preview swatch over the numeric fields; the swatch grows to fill
                // the spare height so the R/G/B/H/S/V rows sit at the bottom.
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_self: AlignSelf::Stretch,
                        row_gap: size::SPACE_TIGHT,
                        flex_grow: 1.0,
                    }
                    Children [
                        (
                            @PlumeColorSwatch { @alpha: alpha }
                            Node {
                                width: em(6),
                                flex_grow: 1.0,
                                margin: UiRect::left(em(1.25)),
                            }
                            ColorPickerSwatch
                        ),
                        space(size::SPACE_TIGHT),
                        // sRGB channels and alpha, then a gap, then the HSV channels.
                        channel_row("R", Channel::R, 3, 1.0),
                        channel_row("G", Channel::G, 3, 1.0),
                        channel_row("B", Channel::B, 3, 1.0),
                        {alpha_row},
                        space(size::SPACE_TIGHT),
                        channel_row("H", Channel::H, 0, 360.0),
                        channel_row("S", Channel::S, 3, 1.0),
                        channel_row("V", Channel::V, 3, 1.0),
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
    alpha: Option<Entity>,
    ramp: Option<Entity>,
    swatch: Option<Entity>,
    channels: Vec<(Entity, Channel)>,
}

// Below this the two representations are treated as agreeing: a field push whose
// value is this close to what the HSV implies is our own echo, not a user edit.
const CHANNEL_EPS: f32 = 1.0e-6;

// Push the working HSV out to every view (plane, hue bar, swatch, numeric fields)
// and the public `Color` mirror, whenever the working color changes.
#[allow(clippy::too_many_arguments)]
fn sync_color_to_views(
    q_picker: Query<(Entity, &ColorPickerValue), Changed<ColorPickerValue>>,
    q_children: Query<&Children>,
    q_sv: Query<(), With<ColorPickerSv>>,
    q_hue: Query<(), With<ColorPickerHue>>,
    q_alpha: Query<(), With<ColorPickerAlpha>>,
    q_ramp: Query<(), With<ColorPickerAlphaRamp>>,
    q_swatch: Query<(), With<ColorPickerSwatch>>,
    q_channel: Query<&ColorPickerChannel>,
    mut q_xy: Query<&mut XyPadValue>,
    mut q_swatch_val: Query<&mut ColorSwatchValue>,
    q_slider: Query<&SliderValue>,
    mut commands: Commands,
) {
    for (root, color) in q_picker.iter() {
        // Both spaces up front: each view is fed in the one it works in.
        let (hsva, srgba): (Hsva, Srgba) = (color.0.into(), color.0.into());
        let views = collect_views(
            root,
            &q_children,
            &q_sv,
            &q_hue,
            &q_alpha,
            &q_ramp,
            &q_swatch,
            &q_channel,
        );

        if let Some(sv) = views.sv {
            set_xy(&mut q_xy, sv, Vec2::new(hsva.saturation, 1.0 - hsva.value));
            // The plane's white→hue tint follows the hue bar.
            commands
                .entity(sv)
                .insert(BackgroundGradient(sv_gradient(hsva.hue)));
        }
        // The ring position roundtrips through trig, so an exact compare would
        // see its own echo as a move; only write when the implied hue disagrees.
        if let Some(hue) = views.hue
            && let Ok(mut value) = q_xy.get_mut(hue)
            && hue_delta(ring_pos_to_hue(value.0), hsva.hue) > HUE_EPS
        {
            value.0 = hue_to_ring_pos(hsva.hue);
        }
        if let Some(alpha) = views.alpha {
            set_xy(&mut q_xy, alpha, Vec2::new(srgba.alpha, 0.5));
        }
        if let Some(ramp) = views.ramp {
            // The ramp shows what each alpha would make of the current color.
            commands
                .entity(ramp)
                .insert(BackgroundGradient(alpha_gradient(srgba)));
        }

        if let Some(swatch) = views.swatch
            && let Ok(mut swatch_val) = q_swatch_val.get_mut(swatch)
            && swatch_val.0 != color.0
        {
            swatch_val.0 = color.0;
        }

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
#[allow(clippy::too_many_arguments)]
fn collect_views(
    root: Entity,
    q_children: &Query<&Children>,
    q_sv: &Query<(), With<ColorPickerSv>>,
    q_hue: &Query<(), With<ColorPickerHue>>,
    q_alpha: &Query<(), With<ColorPickerAlpha>>,
    q_ramp: &Query<(), With<ColorPickerAlphaRamp>>,
    q_swatch: &Query<(), With<ColorPickerSwatch>>,
    q_channel: &Query<&ColorPickerChannel>,
) -> PickerViews {
    let mut views = PickerViews {
        sv: None,
        hue: None,
        alpha: None,
        ramp: None,
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
            } else if q_alpha.contains(entity) {
                views.alpha = Some(entity);
            } else if q_ramp.contains(entity) {
                views.ramp = Some(entity);
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
        Channel::A => srgb.alpha,
        Channel::H => hsva.hue,
        Channel::S => hsva.saturation,
        Channel::V => hsva.value,
    }
}

// A wheel drag's hue roundtrips through sin/cos and atan2, so our own position
// pushes read back a hair off the hue that produced them; within this they are
// echoes, not edits. A thousandth of a degree is far below anything visible.
const HUE_EPS: f32 = 1.0e-3;

// Fold a plane, wheel or alpha-bar drag back into the working color. Every pad
// sits inside a layered wrapper, so each walks up to its picker root.
fn fold_pad_edits(
    q_sv: Query<(Entity, &XyPadValue), (Changed<XyPadValue>, With<ColorPickerSv>)>,
    q_hue: Query<(Entity, &XyPadValue), (Changed<XyPadValue>, With<ColorPickerHue>)>,
    q_alpha: Query<(Entity, &XyPadValue), (Changed<XyPadValue>, With<ColorPickerAlpha>)>,
    q_childof: Query<&ChildOf>,
    q_is_picker: Query<(), With<ColorPickerFrame>>,
    mut q_color: Query<&mut ColorPickerValue>,
    mut commands: Commands,
) {
    for (pad, value) in q_alpha.iter() {
        let Some(root) = nearest_with(pad, &q_childof, &q_is_picker) else {
            continue;
        };
        if let Ok(mut color) = q_color.get_mut(root) {
            // Alpha edits stay in whatever space the color is in — a conversion
            // here would lose the hue whenever the RGB form is degenerate.
            let alpha = value.0.x.clamp(0.0, 1.0);
            if color.0.alpha() != alpha {
                color.0.set_alpha(alpha);
                emit_value_change(root, color.0, &mut commands);
            }
        }
    }
    for (pad, pad_value) in q_sv.iter() {
        let Some(root) = nearest_with(pad, &q_childof, &q_is_picker) else {
            continue;
        };
        if let Ok(mut color) = q_color.get_mut(root) {
            let mut hsva: Hsva = color.0.into();
            let (saturation, value) = (pad_value.0.x, 1.0 - pad_value.0.y);
            // Epsilons, not equality: `value` roundtrips through 1.0-(1.0-v),
            // whose ulp drift would echo a programmatic push back as an edit.
            if (hsva.saturation - saturation).abs() > CHANNEL_EPS
                || (hsva.value - value).abs() > CHANNEL_EPS
            {
                hsva.saturation = saturation;
                hsva.value = value;
                color.0 = hsva.into();
                emit_value_change(root, color.0, &mut commands);
            }
        }
    }
    for (pad, value) in q_hue.iter() {
        let Some(root) = nearest_with(pad, &q_childof, &q_is_picker) else {
            continue;
        };
        if let Ok(mut color) = q_color.get_mut(root) {
            let mut hsva: Hsva = color.0.into();
            let hue = ring_pos_to_hue(value.0);
            if hue_delta(hsva.hue, hue) > HUE_EPS {
                hsva.hue = hue;
                color.0 = hsva.into();
                emit_value_change(root, color.0, &mut commands);
            }
        }
    }
}

// Angular distance between two hues in degrees, the short way around the
// circle, so 360 and 0 read as the same hue rather than a full turn apart.
fn hue_delta(a: f32, b: f32) -> f32 {
    let delta = (a - b).rem_euclid(360.0);
    delta.min(360.0 - delta)
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
// from `sync_color_to_views` also mark `SliderValue` changed, so a field whose value
// still agrees with the HSV is skipped as an echo; only a genuine user edit folds.
fn fold_channel_edits(
    q_changed: Query<(Entity, &SliderValue, &ColorPickerChannel), Changed<SliderValue>>,
    q_childof: Query<&ChildOf>,
    q_is_picker: Query<(), With<ColorPickerFrame>>,
    mut q_color: Query<&mut ColorPickerValue>,
    mut commands: Commands,
) {
    for (field, slider, channel) in q_changed.iter() {
        let Some(root) = nearest_with(field, &q_childof, &q_is_picker) else {
            continue;
        };
        let Ok(mut color) = q_color.get_mut(root) else {
            continue;
        };

        let (mut hsva, mut srgba) = (color.0.into(), color.0.into());

        if (slider.0 - channel_value(hsva, srgba, channel.0)).abs() <= CHANNEL_EPS {
            continue;
        }

        // Fold the edit back in whichever space the edited channel belongs to.
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
            Channel::A => {
                // Stays in the color's own space, preserving hue through black.
                color.0.set_alpha(slider.0);
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

// The horizontal alpha strip for the current color: gone at the left, opaque at
// the right, over the checker that makes the transparent end readable.
fn alpha_gradient(srgba: Srgba) -> Vec<Gradient> {
    let opaque = Color::from(Srgba {
        alpha: 1.0,
        ..srgba
    });
    let clear = Color::from(Srgba {
        alpha: 0.0,
        ..srgba
    });
    vec![Gradient::Linear(LinearGradient {
        angle: PI * 0.5, // left→right
        stops: vec![
            ColorStop::new(clear, percent(0.0)),
            ColorStop::new(opaque, percent(100.0)),
        ],
        color_space: InterpolationColorSpace::Srgba,
    })]
}

// The spectral ring for the wheel's border: a full conic sweep, hue 0 at the
// top increasing clockwise, with stops every 30° so sRGB stays near-spectral.
fn hue_wheel_gradient() -> Vec<Gradient> {
    let stops = (0..=12)
        .map(|i| {
            let hue = i as f32 * 30.0;
            AngularColorStop::new(Color::hsl(hue, 1.0, 0.5), i as f32 / 12.0 * TAU)
        })
        .collect();
    vec![Gradient::Conic(ConicGradient {
        start: 0.0,
        position: UiPosition::CENTER,
        stops,
        color_space: InterpolationColorSpace::Srgba,
    })]
}

// The wheel's angle convention, shared by both directions: hue 0 at the top of
// the ring, increasing clockwise, matching `hue_wheel_gradient`.
fn hue_to_ring_pos(hue: f32) -> Vec2 {
    let (sin, cos) = hue.to_radians().sin_cos();
    Vec2::new(0.5 + sin * RING_RADIUS, 0.5 - cos * RING_RADIUS)
}

fn ring_pos_to_hue(pos: Vec2) -> f32 {
    let offset = pos - Vec2::splat(0.5);
    offset.x.atan2(-offset.y).to_degrees().rem_euclid(360.0)
}

// Registers the color-picker sync systems.
pub(crate) struct ColorPickerPlugin;

impl Plugin for ColorPickerPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PostUpdate,
            (sync_color_to_views, fold_pad_edits, fold_channel_edits).chain(),
        );
    }
}
