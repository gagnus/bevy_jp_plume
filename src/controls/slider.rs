//! Horizontal slider: thin track with a round draggable thumb.
use core::f32::consts::PI;

use bevy_app::{Plugin, PreUpdate};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
    template::template,
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::{PickingSystems, hover::Hovered};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::prelude::*;
use bevy_ui::{
    AlignItems, BackgroundGradient, BorderRadius, BoxShadow, ColorStop, Gradient,
    InteractionDisabled, InterpolationColorSpace, LinearGradient, Node, PositionType, Pressed,
    UiRect, Val, percent,
};
use bevy_ui_widgets::{
    Slider, SliderOrientation, SliderPrecision, SliderRange, SliderStep, SliderValue, TrackClick,
    slider_self_update,
};

use crate::{
    constants::size,
    cursor::EntityCursor,
    focus::FocusIndicator,
    theme::{Flat, GRADIENT_AMOUNT, ThemeBackgroundGradient, UiTheme, control_box_shadow},
    tokens,
};

/// Visible track strip thickness (the full-height node around it is the hit area).
const TRACK_HEIGHT: Val = Val::Px(4.0);

/// A slider widget.
///
/// This is spawnable by inheriting it as a "scene component" with optional [`PlumeSliderProps`].
///
/// Emits [`bevy_ui_widgets::ValueChange<f32>`] when the slider value is changed; disabled by
/// adding [`bevy_ui::InteractionDisabled`]. Shows no value text — pair it with a separate
/// display element when the number matters.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeSliderProps)]
#[require(Slider)]
#[reflect(Component, Clone, Default)]
pub struct PlumeSlider;

/// Props used to construct the [`PlumeSlider`] scene.
pub struct PlumeSliderProps {
    /// Slider minimum value
    pub min: f32,
    /// Slider maximum value
    pub max: f32,
    /// Increment used by arrow keys and relative value changes ([`SliderStep`]);
    /// `None` = 1% of the range
    pub step: Option<f32>,
    /// Decimal places drag values are rounded to ([`SliderPrecision`]); `None` = unrounded
    pub precision: Option<i32>,
}

impl Default for PlumeSliderProps {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 1.0,
            step: None,
            precision: None,
        }
    }
}

impl PlumeSlider {
    fn scene(props: PlumeSliderProps) -> impl Scene {
        bsn! {
            // Full-height hit area; the visible track is a thin child strip.
            // No padding: the thumb's percent-left resolves against the padding box,
            // so track and thumb must share the same width basis.
            Node {
                height: size::ROW_HEIGHT,
                align_items: AlignItems::Center,
                // Horizontal margin reserves the half-knob overhang at the track ends.
                margin: {UiRect::horizontal(size::KNOB_SIZE / 2.0)},
                width: size::CONTROL_WIDTH
            }
            Hovered
            TabIndex(0)
            FocusIndicator
            Slider {
                track_click: TrackClick::Snap,
                orientation: SliderOrientation::Horizontal,
            }
            PlumeSlider
            on(slider_self_update)
            SliderValue({props.min})
            SliderRange::new(props.min, props.max)
            // Default step = 1% of range: arrow keys (and a11y increments) move a
            // continuous slider usefully instead of by the headless default of 1.0.
            SliderStep({props.step.unwrap_or((props.max - props.min) / 100.0)})
            {props.precision.map(|precision| bsn!(SliderPrecision({precision})))}
            Children [
                (
                    Node {
                        height: {TRACK_HEIGHT},
                        width: percent(100.),
                        border_radius: {TRACK_HEIGHT / 2.0},
                    }
                    SliderTrack
                    // Bar/track drawn as a gradient, seeded from the theme so the
                    // slider is styled on its first frame regardless of scene-application order.
                    template(|ctx| {
                        let theme = ctx.resource::<UiTheme>();
                        let bar = theme.color(&tokens::SLIDER_BAR);
                        let bg = theme.color(&tokens::SLIDER_BG);
                        Ok(BackgroundGradient(vec![Gradient::Linear(LinearGradient {
                            angle: PI * 0.5,
                            stops: vec![
                                ColorStop::new(bar, percent(0)),
                                ColorStop::new(bar, percent(50)),
                                ColorStop::new(bg, percent(50)),
                                ColorStop::new(bg, percent(100)),
                            ],
                            color_space: InterpolationColorSpace::LinearRgba,
                        })]))
                    })
                ),
                (
                    // Thumb; update_slider_pos moves it to the value position.
                    Node {
                        position_type: PositionType::Absolute,
                        left: percent(0),
                        top: percent(50),
                        width: size::KNOB_SIZE,
                        height: size::KNOB_SIZE,
                        // Half-knob offsets center the thumb on the value position.
                        margin: UiRect {
                            left: {-(size::KNOB_SIZE / 2.0)},
                            top: {-(size::KNOB_SIZE / 2.0)},
                        },
                        border_radius: BorderRadius::MAX,
                    }
                    template_value(control_box_shadow())
                    SliderThumb
                    ThemeBackgroundGradient(tokens::SLIDER_THUMB, GRADIENT_AMOUNT)
                )
            ]
        }
    }
}

/// Marker for the track strip
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct SliderTrack;

/// Marker for the thumb
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct SliderThumb;

fn update_slider_styles(
    q_sliders: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Pressed>,
            &Hovered,
            Has<Flat>,
        ),
        (
            With<PlumeSlider>,
            Or<(
                Added<InteractionDisabled>,
                Changed<Hovered>,
                Added<Pressed>,
                Added<Flat>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_tracks: Query<&mut BackgroundGradient, With<SliderTrack>>,
    q_thumbs: Query<(&ThemeBackgroundGradient, Has<BoxShadow>), With<SliderThumb>>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    for (slider_ent, disabled, pressed, hovered, flat) in q_sliders.iter() {
        apply_slider_styles(
            slider_ent,
            disabled,
            pressed,
            hovered.0,
            flat,
            &q_children,
            &mut q_tracks,
            &q_thumbs,
            &theme,
            &mut commands,
        );
    }
}

fn update_slider_styles_remove(
    q_sliders: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Pressed>,
            &Hovered,
            Has<Flat>,
        ),
        With<PlumeSlider>,
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut remove_pressed: RemovedComponents<Pressed>,
    mut removed_flat: RemovedComponents<Flat>,
    q_children: Query<&Children>,
    mut q_tracks: Query<&mut BackgroundGradient, With<SliderTrack>>,
    q_thumbs: Query<(&ThemeBackgroundGradient, Has<BoxShadow>), With<SliderThumb>>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(remove_pressed.read())
        .chain(removed_flat.read())
        .for_each(|ent| {
            if let Ok((slider_ent, disabled, pressed, hovered, flat)) = q_sliders.get(ent) {
                apply_slider_styles(
                    slider_ent,
                    disabled,
                    pressed,
                    hovered.0,
                    flat,
                    &q_children,
                    &mut q_tracks,
                    &q_thumbs,
                    &theme,
                    &mut commands,
                );
            }
        });
}

/// Re-apply slider styles to every slider when the theme changes.
fn update_slider_styles_theme(
    q_sliders: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Pressed>,
            &Hovered,
            Has<Flat>,
        ),
        With<PlumeSlider>,
    >,
    q_children: Query<&Children>,
    mut q_tracks: Query<&mut BackgroundGradient, With<SliderTrack>>,
    q_thumbs: Query<(&ThemeBackgroundGradient, Has<BoxShadow>), With<SliderThumb>>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    if !theme.is_changed() {
        return;
    }
    for (slider_ent, disabled, pressed, hovered, flat) in q_sliders.iter() {
        apply_slider_styles(
            slider_ent,
            disabled,
            pressed,
            hovered.0,
            flat,
            &q_children,
            &mut q_tracks,
            &q_thumbs,
            &theme,
            &mut commands,
        );
    }
}

/// Resolve the slider's child entities and push the current styles onto them.
fn apply_slider_styles(
    slider_ent: Entity,
    disabled: bool,
    pressed: bool,
    hovered: bool,
    flat: bool,
    q_children: &Query<&Children>,
    q_tracks: &mut Query<&mut BackgroundGradient, With<SliderTrack>>,
    q_thumbs: &Query<(&ThemeBackgroundGradient, Has<BoxShadow>), With<SliderThumb>>,
    theme: &UiTheme,
    commands: &mut Commands,
) {
    let Some(track_ent) = q_children
        .iter_descendants(slider_ent)
        .find(|en| q_tracks.contains(*en))
    else {
        return;
    };
    let Some(thumb_ent) = q_children
        .iter_descendants(slider_ent)
        .find(|en| q_thumbs.contains(*en))
    else {
        return;
    };

    // Safety: all three entities were just confirmed present in their queries.
    let mut track_background_gradient = q_tracks.get_mut(track_ent).unwrap();
    let (thumb_gradient_color, has_box_shadow) = q_thumbs.get(thumb_ent).unwrap();
    set_slider_styles(
        slider_ent,
        thumb_ent,
        disabled,
        pressed,
        hovered,
        flat,
        &mut track_background_gradient,
        thumb_gradient_color,
        has_box_shadow,
        theme,
        commands,
    );
}

fn set_slider_styles(
    slider_ent: Entity,
    thumb_ent: Entity,
    disabled: bool,
    pressed: bool,
    hovered: bool,
    flat: bool,
    track_background_gradient: &mut BackgroundGradient,
    thumb_gradient_color: &ThemeBackgroundGradient,
    has_box_shadow: bool,
    theme: &UiTheme,
    commands: &mut Commands,
) {
    let bar_color = theme.color(&tokens::sets::SLIDER_BAR.pick(disabled, pressed, hovered));
    let bg_color = theme.color(&tokens::sets::SLIDER_BG.pick(disabled, pressed, hovered));
    let thumb_token = tokens::sets::SLIDER_THUMB.pick(disabled, pressed, hovered);

    // Disabled thumb reads inert: flat fill, no gradient.
    let thumb_gradient_amount = if disabled || flat {
        0.0
    } else {
        GRADIENT_AMOUNT
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
    };

    if let [Gradient::Linear(linear_gradient)] = &mut track_background_gradient.0[..] {
        linear_gradient.stops[0].color = bar_color;
        linear_gradient.stops[1].color = bar_color;
        linear_gradient.stops[2].color = bg_color;
        linear_gradient.stops[3].color = bg_color;
    }

    if thumb_gradient_color.0 != thumb_token || thumb_gradient_color.1 != thumb_gradient_amount {
        commands
            .entity(thumb_ent)
            .insert(ThemeBackgroundGradient(thumb_token, thumb_gradient_amount));
    }

    let should_have_box_shadow = !disabled;
    if should_have_box_shadow && !has_box_shadow {
        commands.entity(thumb_ent).insert(control_box_shadow());
    } else if !should_have_box_shadow && has_box_shadow {
        commands.entity(thumb_ent).remove::<BoxShadow>();
    }

    // Change cursor shape
    commands
        .entity(slider_ent)
        .insert(EntityCursor::System(cursor_shape));
}

fn update_slider_pos(
    q_sliders: Query<
        (Entity, &SliderValue, &SliderRange),
        (
            With<PlumeSlider>,
            Or<(
                Changed<SliderValue>,
                Changed<SliderRange>,
                Changed<Children>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_tracks: Query<&mut BackgroundGradient, With<SliderTrack>>,
    mut q_thumbs: Query<&mut Node, With<SliderThumb>>,
) {
    for (slider_ent, value, range) in q_sliders.iter() {
        let percent_value = (range.thumb_position(value.0) * 100.0).clamp(0.0, 100.0);
        q_children.iter_descendants(slider_ent).for_each(|child| {
            if let Ok(mut gradient) = q_tracks.get_mut(child)
                && let [Gradient::Linear(linear_gradient)] = &mut gradient.0[..]
            {
                linear_gradient.stops[1].point = percent(percent_value);
                linear_gradient.stops[2].point = percent(percent_value);
            }
            if let Ok(mut thumb) = q_thumbs.get_mut(child) {
                thumb.left = percent(percent_value);
            }
        });
    }
}

/// Plugin which registers the systems for updating the slider styles.
pub struct SliderPlugin;

impl Plugin for SliderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (
                update_slider_styles,
                update_slider_styles_remove,
                update_slider_styles_theme,
                update_slider_pos,
            )
                .in_set(PickingSystems::Last),
        );
    }
}
