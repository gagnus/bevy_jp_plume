//! Horizontal slider: thin track with a round draggable thumb.
use core::f32::consts::PI;

use bevy::app::{Plugin, PreUpdate};
use bevy::ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    observer::On,
    query::{Added, Changed, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
    template::template,
};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::{
    PickingSystems,
    events::{Pointer, Press},
    hover::Hovered,
};
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::scene::prelude::*;
use bevy::ui::{
    AlignItems, BackgroundGradient, BorderRadius, BoxShadow, ColorStop, Gradient,
    InteractionDisabled, InterpolationColorSpace, LinearGradient, Node, PositionType, Pressed,
    UiRect, UiTransform, Val, percent, px,
};
use bevy::ui_widgets::{
    Slider, SliderOrientation, SliderPrecision, SliderRange, SliderStep, SliderThumb, SliderValue,
    TrackClick, slider_self_update,
};

use crate::font_styles::TextStyleRelay;
use crate::{
    constants::size,
    controls::DefaultWidth,
    cursor::EntityCursor,
    focus::FocusIndicator,
    theme::{Flat, GRADIENT_AMOUNT, ThemeBackgroundGradient, UiTheme, control_box_shadow},
    tokens,
    utils::anim::AnimState,
};

/// Thumb scale while grabbed: the knob grows this much on press for grab feedback.
const THUMB_GRABBED_SCALE: f32 = 1.15;

/// A slider, spawnable as a scene component with optional [`PlumeSliderProps`].
/// Emits [`bevy::ui_widgets::ValueChange<f32>`]; shows no value text of its own.
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
            // The half-knob inset lives on the track inside, not as outer margin:
            // margin sits outside an explicit width, so a caller's would overflow.
            Node {
                height: size::ROW_HEIGHT,
                align_items: AlignItems::Center,
                min_width: px(40.0),
            }
            // An empty track measures nothing, so `width: auto` would collapse it.
            DefaultWidth(px(180.0))
            Hovered
            TabIndex(0)
            FocusIndicator
            Slider {
                track_click: TrackClick::Snap,
                orientation: SliderOrientation::Horizontal,
            }
            PlumeSlider
            // Em-sized chrome needs the chain's `EmSize`.
            TextStyleRelay
            on(slider_self_update)
            SliderValue({props.min})
            SliderRange::new(props.min, props.max)
            // Default step = 1% of range: arrow keys (and a11y increments) move a
            // continuous slider usefully instead of by the headless default of 1.0.
            SliderStep({props.step.unwrap_or((props.max - props.min) / 100.0)})
            {props.precision.map(|precision| bsn!(SliderPrecision({precision})))}
            Children [
                (
                    // Inset half a knob each end so the thumb's sweep, not the bare
                    // track, spans the full width; grown so the inset is subtracted.
                    Node {
                        height: size::SLIDER_TRACK_HEIGHT,
                        width: {Val::ZERO},
                        flex_grow: 1.0,
                        margin: {UiRect::horizontal(size::KNOB_SIZE / 2.0)},
                        border_radius: {size::SLIDER_TRACK_HEIGHT / 2.0},
                    }
                    SliderTrack
                    TextStyleRelay
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
                    Children [
                        (
                            // A child of the track, sharing its inset span — which is
                            // the (width - thumb) span bevy's drag math assumes.
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
                            TextStyleRelay
                            template_value(AnimState::scale(1.0, THUMB_GRABBED_SCALE))
                            UiTransform::default()
                            on(grab_thumb_on_press)
                            ThemeBackgroundGradient(tokens::SLIDER_THUMB, GRADIENT_AMOUNT)
                        )
                    ]
                )
            ]
        }
    }
}

/// Marker for the track strip
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct SliderTrack;

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
            // Added<PlumeSlider> guarantees the initial style pass on spawn.
            Or<(
                Added<PlumeSlider>,
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
    mut q_thumb_anim: Query<&mut AnimState, With<SliderThumb>>,
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
            &mut q_thumb_anim,
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
    mut q_thumb_anim: Query<&mut AnimState, With<SliderThumb>>,
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
                    &mut q_thumb_anim,
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
    mut q_thumb_anim: Query<&mut AnimState, With<SliderThumb>>,
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
            &mut q_thumb_anim,
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
    q_thumb_anim: &mut Query<&mut AnimState, With<SliderThumb>>,
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

    // Drive the grow: the thumb eases to full scale while the slider is pressed.
    if let Ok(mut thumb_anim) = q_thumb_anim.get_mut(thumb_ent) {
        thumb_anim.set_target(if pressed { 1.0 } else { 0.0 });
    }

    let mut track_background_gradient = q_tracks
        .get_mut(track_ent)
        .expect("track entity was just found via q_tracks::contains");
    let (thumb_gradient_color, has_box_shadow) = q_thumbs
        .get(thumb_ent)
        .expect("thumb entity was just found via q_thumbs::contains");
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
        true => bevy::window::SystemCursorIcon::NotAllowed,
        false => bevy::window::SystemCursorIcon::Pointer,
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

/// On mouse-down, mark the slider [`Pressed`] so the thumb grows at once. The
/// widget only sets [`Pressed`] on a track click, not a thumb grab; its own
/// release/cancel/drag-end handlers clear it either way.
fn grab_thumb_on_press(
    press: On<Pointer<Press>>,
    q_child_of: Query<&ChildOf>,
    q_slider: Query<Has<InteractionDisabled>, With<PlumeSlider>>,
    mut commands: Commands,
) {
    // Thumb → track → slider, which carries the Pressed state.
    let Ok(track) = q_child_of.get(press.event_target()) else {
        return;
    };
    let Ok(slider) = q_child_of.get(track.parent()) else {
        return;
    };
    let slider_ent = slider.parent();
    if q_slider.get(slider_ent).is_ok_and(|disabled| !disabled) {
        commands.entity(slider_ent).insert(Pressed);
    }
}

/// Plugin which registers the systems for updating the slider styles.
pub(crate) struct SliderPlugin;

impl Plugin for SliderPlugin {
    fn build(&self, app: &mut bevy::app::App) {
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
