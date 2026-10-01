//! Horizontal slider: thin track with a round draggable thumb.
use core::f32::consts::PI;

use bevy::app::{Plugin, PreUpdate};
use bevy::ecs::change_detection::DetectChanges;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EntityEvent;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::lifecycle::RemovedComponents;
use bevy::ecs::observer::On;
use bevy::ecs::query::{Added, Changed, Has, Or, With};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::ecs::template::template;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::PickingSystems;
use bevy::picking::events::PointerPress;
use bevy::picking::hover::Hovered;
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::ui::{
    AlignItems, BackgroundGradient, BorderRadius, BoxShadow, ColorStop, Gradient,
    InteractionDisabled, InterpolationColorSpace, LinearGradient, Node, PositionType, Pressed,
    UiRect, UiTransform, Val, percent,
};
use bevy::ui_widgets::{
    Slider, SliderOrientation, SliderPrecision, SliderRange, SliderStep, SliderThumb, SliderValue,
    TrackClick, slider_self_update,
};

use crate::constants::size;
use crate::cursor::EntityCursor;
use crate::default_width::DefaultWidth;
use crate::focus::FocusIndicator;
use crate::theme::{GradientAmount, ThemeBackgroundToken, ThemeId, UiTheme, control_box_shadow};
use crate::tokens;
use crate::utils::anim::AnimState;
use crate::utils::hierarchy::{descendant_get, descendant_get_mut};

// Thumb scale while grabbed: the knob grows this much on press for grab feedback.
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
    /// Slider minimum value.
    pub min: f32,
    /// Slider maximum value.
    pub max: f32,
    /// Increment used by arrow keys and relative value changes ([`SliderStep`]);
    /// `None` = 1% of the range.
    pub step: Option<f32>,
    /// Decimal places drag values are rounded to ([`SliderPrecision`]); `None` = unrounded.
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
                min_width: size::em_from_px(40.0),
            }
            // An empty track measures nothing, so `width: auto` would collapse it.
            DefaultWidth(size::em_from_px(180.0))
            Hovered
            TabIndex(0)
            Slider {
                track_click: TrackClick::Snap,
                orientation: SliderOrientation::Horizontal,
            }
            SliderFrame
            EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)
            on(slider_self_update)
            SliderValue({props.min})
            SliderRange::new(props.min, props.max)
            // Default step = 1% of range: arrow keys (and a11y increments) move a
            // continuous slider usefully instead of by the headless default of 1.0.
            SliderStep({props.step.unwrap_or((props.max - props.min) / 100.0)})
            @{props.precision.map(|precision| bsn! { SliderPrecision(precision) })}
            Children [
                // Inset half a knob each end so the thumb's sweep, not the bare
                // track, spans the full width; grown so the inset is subtracted.
                Node {
                    height: size::SLIDER_TRACK_HEIGHT,
                    width: Val::ZERO,
                    flex_grow: 1.0,
                    margin: UiRect::horizontal(size::KNOB_SIZE / 2.0),
                    border_radius: {size::SLIDER_TRACK_HEIGHT / 2.0},
                }
                SliderTrack
                // Bar/track drawn as a gradient, seeded from the default theme so
                // the slider is styled on its first frame regardless of
                // scene-application order; a subtree `ThemeId` repaints it the
                // same frame via `update_slider_styles_theme`.
                template(|ctx| {
                    let theme = ctx.resource::<UiTheme>();
                    let bar = theme.token_color(None, &tokens::SLIDER_BAR);
                    let bg = theme.token_color(None, &tokens::SLIDER_BG);
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
                    // A child of the track, sharing its inset span - which is
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
                    control_box_shadow()
                    SliderThumb
                    // Focus rings follow the node's rounding, so the ring
                    // belongs on the round thumb, not the square frame.
                    FocusIndicator
                    AnimState::scale(1.0, THUMB_GRABBED_SCALE)
                    UiTransform::default()
                    on(grab_thumb_on_press)
                    ThemeBackgroundToken(tokens::SLIDER_THUMB)
                    GradientAmount::STANDARD
                ]
            ]
        }
    }
}

// Marker for the track strip.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct SliderTrack;

// Plain root marker, inserted on both the retained and imm paths. The systems key on
// this, not the [`PlumeSlider`] scene component, which only the retained path inserts.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct SliderFrame;

fn update_slider_styles(
    q_sliders: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Pressed>,
            &Hovered,
            Option<&ThemeId>,
        ),
        (
            With<SliderFrame>,
            // Added<SliderFrame> guarantees the initial style pass on spawn.
            Or<(
                Added<SliderFrame>,
                Added<InteractionDisabled>,
                Changed<Hovered>,
                Added<Pressed>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_tracks: Query<&mut BackgroundGradient, With<SliderTrack>>,
    q_thumbs: Query<
        (
            Entity,
            &ThemeBackgroundToken,
            &GradientAmount,
            Has<BoxShadow>,
        ),
        With<SliderThumb>,
    >,
    mut q_thumb_anim: Query<&mut AnimState, With<SliderThumb>>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    for (slider_ent, disabled, pressed, hovered, theme_id) in q_sliders.iter() {
        apply_slider_styles(
            slider_ent,
            disabled,
            pressed,
            hovered.0,
            theme_id,
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
            Option<&ThemeId>,
        ),
        With<SliderFrame>,
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut remove_pressed: RemovedComponents<Pressed>,
    q_children: Query<&Children>,
    mut q_tracks: Query<&mut BackgroundGradient, With<SliderTrack>>,
    q_thumbs: Query<
        (
            Entity,
            &ThemeBackgroundToken,
            &GradientAmount,
            Has<BoxShadow>,
        ),
        With<SliderThumb>,
    >,
    mut q_thumb_anim: Query<&mut AnimState, With<SliderThumb>>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(remove_pressed.read())
        .for_each(|ent| {
            if let Ok((slider_ent, disabled, pressed, hovered, theme_id)) = q_sliders.get(ent) {
                apply_slider_styles(
                    slider_ent,
                    disabled,
                    pressed,
                    hovered.0,
                    theme_id,
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

// Re-apply slider styles to every slider when the theme changes, and to a
// slider whose `ThemeId` landed or changed this frame.
fn update_slider_styles_theme(
    q_sliders: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Pressed>,
            &Hovered,
            Option<&ThemeId>,
        ),
        With<SliderFrame>,
    >,
    q_id_changed: Query<(), Changed<ThemeId>>,
    q_children: Query<&Children>,
    mut q_tracks: Query<&mut BackgroundGradient, With<SliderTrack>>,
    q_thumbs: Query<
        (
            Entity,
            &ThemeBackgroundToken,
            &GradientAmount,
            Has<BoxShadow>,
        ),
        With<SliderThumb>,
    >,
    mut q_thumb_anim: Query<&mut AnimState, With<SliderThumb>>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    let all = theme.is_changed();
    if !all && q_id_changed.is_empty() {
        return;
    }
    for (slider_ent, disabled, pressed, hovered, theme_id) in q_sliders.iter() {
        if !all && !q_id_changed.contains(slider_ent) {
            continue;
        }
        apply_slider_styles(
            slider_ent,
            disabled,
            pressed,
            hovered.0,
            theme_id,
            &q_children,
            &mut q_tracks,
            &q_thumbs,
            &mut q_thumb_anim,
            &theme,
            &mut commands,
        );
    }
}

// Resolve the slider's child entities and push the current styles onto them.
fn apply_slider_styles(
    slider_ent: Entity,
    disabled: bool,
    pressed: bool,
    hovered: bool,
    theme_id: Option<&ThemeId>,
    q_children: &Query<&Children>,
    q_tracks: &mut Query<&mut BackgroundGradient, With<SliderTrack>>,
    q_thumbs: &Query<
        (
            Entity,
            &ThemeBackgroundToken,
            &GradientAmount,
            Has<BoxShadow>,
        ),
        With<SliderThumb>,
    >,
    q_thumb_anim: &mut Query<&mut AnimState, With<SliderThumb>>,
    theme: &UiTheme,
    commands: &mut Commands,
) {
    let Some(mut track_background_gradient) = descendant_get_mut(slider_ent, q_children, q_tracks)
    else {
        return;
    };
    let Some((thumb_ent, thumb_token, thumb_amount, has_box_shadow)) =
        descendant_get(slider_ent, q_children, q_thumbs)
    else {
        return;
    };

    // Drive the grow: the thumb eases to full scale while the slider is pressed.
    if let Ok(mut thumb_anim) = q_thumb_anim.get_mut(thumb_ent) {
        thumb_anim.set_target(if pressed { 1.0 } else { 0.0 });
    }
    set_slider_styles(
        slider_ent,
        thumb_ent,
        disabled,
        pressed,
        hovered,
        theme_id,
        &mut track_background_gradient,
        (thumb_token, thumb_amount),
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
    theme_id: Option<&ThemeId>,
    track_background_gradient: &mut BackgroundGradient,
    thumb_now: (&ThemeBackgroundToken, &GradientAmount),
    has_box_shadow: bool,
    theme: &UiTheme,
    commands: &mut Commands,
) {
    let bar_color = theme.token_color(
        theme_id,
        &tokens::sets::SLIDER_BAR.pick(disabled, pressed, hovered),
    );
    let bg_color = theme.token_color(
        theme_id,
        &tokens::sets::SLIDER_BG.pick(disabled, pressed, hovered),
    );
    let thumb_token = tokens::sets::SLIDER_THUMB.pick(disabled, pressed, hovered);

    // Disabled thumb reads inert: flat fill, no gradient. A `Flat` slider is
    // flattened by the theme layer, so the marker plays no part here.
    let thumb_amount = if disabled {
        GradientAmount(0.0)
    } else {
        GradientAmount::STANDARD
    };

    // Resize arrows only for the grab itself, held by `Slider`'s own pointer capture;
    // at rest the slider reads as clickable like every other control.
    let cursor_shape = match (disabled, pressed) {
        (true, _) => bevy::window::SystemCursorIcon::NotAllowed,
        (_, true) => bevy::window::SystemCursorIcon::EwResize,
        _ => bevy::window::SystemCursorIcon::Pointer,
    };

    if let [Gradient::Linear(linear_gradient)] = &mut track_background_gradient.0[..] {
        linear_gradient.stops[0].color = bar_color;
        linear_gradient.stops[1].color = bar_color;
        linear_gradient.stops[2].color = bg_color;
        linear_gradient.stops[3].color = bg_color;
    }

    let (thumb_token_now, thumb_amount_now) = thumb_now;
    if thumb_token_now.0 != thumb_token {
        commands
            .entity(thumb_ent)
            .insert(ThemeBackgroundToken(thumb_token));
    }
    if *thumb_amount_now != thumb_amount {
        commands.entity(thumb_ent).insert(thumb_amount);
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
            With<SliderFrame>,
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
        if let Some(mut gradient) = descendant_get_mut(slider_ent, &q_children, &mut q_tracks)
            && let [Gradient::Linear(linear_gradient)] = &mut gradient.0[..]
        {
            linear_gradient.stops[1].point = percent(percent_value);
            linear_gradient.stops[2].point = percent(percent_value);
        }
        if let Some(mut thumb) = descendant_get_mut(slider_ent, &q_children, &mut q_thumbs) {
            thumb.left = percent(percent_value);
        }
    }
}

// On mouse-down, mark the slider [`Pressed`] so the thumb grows at once. The
// widget only sets [`Pressed`] on a track click, not a thumb grab; its own
// release/cancel/drag-end handlers clear it either way.
fn grab_thumb_on_press(
    press: On<PointerPress>,
    q_child_of: Query<&ChildOf>,
    q_slider: Query<Has<InteractionDisabled>, With<SliderFrame>>,
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

// Plugin which registers the systems for updating the slider styles.
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
