//! Horizontal slider: thin track with a round draggable thumb.
use core::f32::consts::PI;

use bevy_app::{Plugin, PreUpdate};
use bevy_color::Color;
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, SystemParam},
    template::template,
};
use bevy_picking::{PickingSystems, hover::Hovered};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::prelude::*;
use bevy_ui::{
    AlignItems, BackgroundGradient, BorderRadius, ColorStop, Gradient, InteractionDisabled, InterpolationColorSpace, LinearGradient, Node, PositionType, Pressed, UiRect, percent, px,
};
use bevy_ui_widgets::{
    Slider, SliderOrientation, SliderRange, SliderValue, TrackClick, slider_self_update,
};

use crate::{
    constants::size, cursor::EntityCursor, theme::{GRADIENT_AMOUNT, ThemeBackgroundGradient, ThemeBorderColor, UiTheme}, tokens,
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
            // No padding: the thumb's percent-left resolves against the padding box,
            // so track and thumb must share the same width basis.
            Node {
                height: size::ROW_HEIGHT,
                align_items: AlignItems::Center,
                // Horizontal margin reserves the half-knob overhang at the track ends.
                margin: UiRect::horizontal(px(8.0)),
                width: size::CONTROL_WIDTH
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
            Children [
                (
                    Node {
                        height: px(4),
                        width: percent(100.),
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
                        width: size::KNOB_SIZE,
                        height: size::KNOB_SIZE,
                        margin: UiRect {
                            left: px(-8),
                            top: px(-8),
                        },
                        border_radius: {BorderRadius::all(px(8))},
                    }
                    PlumeSliderThumb
                    ThemeBackgroundGradient(tokens::SLIDER_THUMB, GRADIENT_AMOUNT)
                    Children [
                        (
                            Node {
                                width: percent(100),
                                height: percent(100),
                                border: size::CONTROL_BORDER,
                                border_radius: BorderRadius::all(px(8)),
                            }
                            PlumeSliderThumbBorder
                            ThemeBorderColor(tokens::SLIDER_THUMB_BORDER)
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
struct PlumeSliderTrack;

/// Marker for the thumb
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct PlumeSliderThumb;

/// Marker for the thumb border
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct PlumeSliderThumbBorder;

fn update_slider_styles(
    q_sliders: Query<
        (Entity, Has<InteractionDisabled>, Has<Pressed>, &Hovered),
        (
            With<PlumeSlider>,
            Or<(Added<InteractionDisabled>, Changed<Hovered>, Added<Pressed>)>,
        ),
    >,
    q_children: Query<&Children>,
    q_tracks: Query<&BackgroundGradient, With<PlumeSliderTrack>>,
    q_thumbs: Query<&ThemeBackgroundGradient, With<PlumeSliderThumb>>,
    q_thumb_borders: Query<&ThemeBorderColor, With<PlumeSliderThumbBorder>>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    for (slider_ent, disabled, pressed, hovered) in q_sliders.iter() {
        let Some(track_ent) = q_children
            .iter_descendants(slider_ent)
            .find(|en| q_tracks.contains(*en))
        else {
            continue;
        };
        let Some(thumb_ent) = q_children
            .iter_descendants(slider_ent)
            .find(|en| q_thumbs.contains(*en))
        else {
            continue;
        };
        let Some(thumb_border_ent) = q_children
            .iter_descendants(slider_ent)
            .find(|en| q_thumb_borders.contains(*en))
        else {
            continue;
        };
        
        // set_slider_styles(slider_ent, disabled, pressed, hovered.0, &mut ctx);
    }
}

/*
fn update_slider_styles_remove(
    q_sliders: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>, &Hovered), With<PlumeSlider>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut remove_pressed: RemovedComponents<Pressed>,
    mut ctx: SliderStyleCtx,
) {
    removed_disabled
        .read()
        .chain(remove_pressed.read())
        .for_each(|ent| {
            if let Ok((slider_ent, disabled, pressed, hovered)) = q_sliders.get(ent) {
                set_slider_styles(slider_ent, disabled, pressed, hovered.0, &mut ctx);
            }
        });
}

/// Re-apply slider styles to every slider when the theme changes.
fn update_slider_styles_theme(
    q_sliders: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>, &Hovered), With<PlumeSlider>>,
    mut ctx: SliderStyleCtx,
) {
    if !ctx.theme.is_changed() {
        return;
    }
    for (slider_ent, disabled, pressed, hovered) in q_sliders.iter() {
        set_slider_styles(slider_ent, disabled, pressed, hovered.0, &mut ctx);
    }
}

fn set_slider_styles(
    slider_ent: Entity,
    disabled: bool,
    pressed: bool,
    hovered: bool,
    ctx: &mut SliderStyleCtx,
) {
    let SliderStyleCtx {
        q_children,
        q_tracks,
        q_thumbs,
        theme,
        commands,
    } = ctx;
    let bar_color = theme.color(&tokens::sets::SLIDER_BAR.pick(disabled, pressed, hovered));
    let bg_color = theme.color(&tokens::sets::SLIDER_BG.pick(disabled, pressed, hovered));
    let thumb_token = tokens::sets::SLIDER_THUMB.pick(disabled, pressed, hovered);
    // Disabled thumb reads inert: flat fill, no gradient.
    let thumb_gradient_amount = if disabled { 0.0 } else { GRADIENT_AMOUNT };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
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
        if let Ok(thumb_bg) = q_thumbs.get(child)
            && (thumb_bg.0 != thumb_token || thumb_bg.1 != thumb_gradient_amount)
        {
            commands.entity(child).insert(ThemeBackgroundGradient(
                thumb_token.clone(),
                thumb_gradient_amount,
            ));
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
*/

/// Plugin which registers the systems for updating the slider styles.
pub struct SliderPlugin;

impl Plugin for SliderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (
                update_slider_styles,
                //update_slider_styles_remove,
                //update_slider_styles_theme,
                //update_slider_pos,
            )
                .in_set(PickingSystems::Last),
        );
    }
}
