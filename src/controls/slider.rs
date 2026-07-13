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
use bevy_picking::{PickingSystems, hover::Hovered};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::prelude::*;
use bevy_ui::{
    AlignItems, BackgroundGradient, BorderRadius, ColorStop, Gradient, InteractionDisabled,
    InterpolationColorSpace, LinearGradient, Node, PositionType, Pressed, UiRect, percent, px,
};
use bevy_ui_widgets::{
    Slider, SliderOrientation, SliderRange, SliderValue, TrackClick, slider_self_update,
};

use crate::{
    constants::size,
    cursor::EntityCursor,
    theme::{ThemeBackgroundColor, UiTheme},
    tokens,
};

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
}

impl Default for PlumeSliderProps {
    fn default() -> Self {
        Self { min: 0.0, max: 1.0 }
    }
}

impl PlumeSlider {
    fn scene(props: PlumeSliderProps) -> impl Scene {
        bsn! {
            // Full-height hit area; the visible track is a thin child strip.
            Node {
                height: size::ROW_HEIGHT,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(px(8)),
                flex_grow: 1.0,
            }
            Hovered
            Slider {
                track_click: TrackClick::Snap,
                orientation: SliderOrientation::Horizontal,
            }
            PlumeSlider
            on(slider_self_update)
            SliderValue({props.min})
            SliderRange::new(props.min, props.max)
            EntityCursor::System(bevy_window::SystemCursorIcon::EwResize)
            Children [
                (
                    Node {
                        height: px(4),
                        flex_grow: 1.0,
                        border_radius: {BorderRadius::all(px(2))},
                    }
                    PlumeSliderTrack
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
                        width: px(16),
                        height: px(16),
                        margin: UiRect {
                            left: px(-8),
                            top: px(-8),
                        },
                        border_radius: {BorderRadius::all(px(8))},
                    }
                    PlumeSliderThumb
                    ThemeBackgroundColor(tokens::SLIDER_THUMB)
                )
            ]
        }
    }
}

/// Marker for the track strip
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct PlumeSliderTrack;

/// Marker for the thumb
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct PlumeSliderThumb;

fn update_slider_styles(
    q_sliders: Query<
        (Entity, Has<InteractionDisabled>, Has<Pressed>, &Hovered),
        (
            With<PlumeSlider>,
            Or<(Added<InteractionDisabled>, Changed<Hovered>, Added<Pressed>)>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_tracks: Query<&mut BackgroundGradient, With<PlumeSliderTrack>>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    for (slider_ent, disabled, pressed, hovered) in q_sliders.iter() {
        set_slider_styles(
            slider_ent,
            &theme,
            disabled,
            pressed,
            hovered.0,
            &q_children,
            &mut q_tracks,
            &mut commands,
        );
    }
}

fn update_slider_styles_remove(
    q_sliders: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>, &Hovered), With<PlumeSlider>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut remove_pressed: RemovedComponents<Pressed>,
    q_children: Query<&Children>,
    mut q_tracks: Query<&mut BackgroundGradient, With<PlumeSliderTrack>>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(remove_pressed.read())
        .for_each(|ent| {
            if let Ok((slider_ent, disabled, pressed, hovered)) = q_sliders.get(ent) {
                set_slider_styles(
                    slider_ent,
                    &theme,
                    disabled,
                    pressed,
                    hovered.0,
                    &q_children,
                    &mut q_tracks,
                    &mut commands,
                );
            }
        });
}

/// Re-apply slider styles to every slider when the theme changes.
fn update_slider_styles_theme(
    q_sliders: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>, &Hovered), With<PlumeSlider>>,
    q_children: Query<&Children>,
    mut q_tracks: Query<&mut BackgroundGradient, With<PlumeSliderTrack>>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    if !theme.is_changed() {
        return;
    }
    for (slider_ent, disabled, pressed, hovered) in q_sliders.iter() {
        set_slider_styles(
            slider_ent,
            &theme,
            disabled,
            pressed,
            hovered.0,
            &q_children,
            &mut q_tracks,
            &mut commands,
        );
    }
}

fn set_slider_styles(
    slider_ent: Entity,
    theme: &Res<'_, UiTheme>,
    disabled: bool,
    pressed: bool,
    hovered: bool,
    q_children: &Query<&Children>,
    q_tracks: &mut Query<&mut BackgroundGradient, With<PlumeSliderTrack>>,
    commands: &mut Commands,
) {
    let bar_color = theme.color(&if disabled {
        tokens::SLIDER_BAR_DISABLED
    } else if pressed {
        tokens::SLIDER_BAR_PRESSED
    } else if hovered {
        tokens::SLIDER_BAR_HOVER
    } else {
        tokens::SLIDER_BAR
    });

    let bg_color = theme.color(&if disabled {
        tokens::SLIDER_BG_DISABLED
    } else if pressed {
        tokens::SLIDER_BG_PRESSED
    } else if hovered {
        tokens::SLIDER_BG_HOVER
    } else {
        tokens::SLIDER_BG
    });

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::EwResize,
    };

    q_children.iter_descendants(slider_ent).for_each(|child| {
        if let Ok(mut gradient) = q_tracks.get_mut(child)
            && let [Gradient::Linear(linear_gradient)] = &mut gradient.0[..]
        {
            linear_gradient.stops[0].color = bar_color;
            linear_gradient.stops[1].color = bar_color;
            linear_gradient.stops[2].color = bg_color;
            linear_gradient.stops[3].color = bg_color;
        }
    });

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
    mut q_tracks: Query<&mut BackgroundGradient, With<PlumeSliderTrack>>,
    mut q_thumbs: Query<&mut Node, With<PlumeSliderThumb>>,
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
