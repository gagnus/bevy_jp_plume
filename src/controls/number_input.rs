//! Numeric text field: type a value, commit on Enter or focus loss.
use crate::constants::size;
use bevy::app::{Plugin, PreUpdate};
use bevy::ecs::{
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{Changed, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, ResMut},
};
use bevy::input::{ButtonState, keyboard::KeyCode, keyboard::KeyboardInput};
use bevy::input_focus::{FocusLost, FocusedInput, InputFocus};
use bevy::picking::PickingSystems;
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::scene::prelude::*;
use bevy::text::{
    EditableText, EditableTextFilter, FontSourceTemplate, Justify, LineBreak, TextLayout,
};
use bevy::ui_widgets::{SliderRange, SliderStep, SliderValue, ValueChange};

use crate::{
    constants::fonts,
    controls::{
        DefaultWidth, TextInputField, set_editable_text, text_input_field, text_input_frame,
        text_input_suffix,
    },
    font_styles::InheritableFont,
};

/// A numeric input on the [`PlumeTextInput`](crate::controls::PlumeTextInput)
/// frame, holding its value in [`SliderValue`] / [`SliderRange`] like a slider.
///
/// Typing commits on Enter or focus loss (clamped and rounded to `precision`);
/// Escape or an unparsable entry reverts; Up/Down step by [`SliderStep`].
#[derive(SceneComponent, Clone, Reflect)]
#[scene(PlumeNumberInputProps)]
#[reflect(Component, Default, Clone)]
pub struct PlumeNumberInput {
    /// Decimal places used to display and commit the value.
    pub precision: usize,
}

impl Default for PlumeNumberInput {
    fn default() -> Self {
        Self { precision: 2 }
    }
}

/// Props used to construct the [`PlumeNumberInput`] scene.
#[derive(Clone)]
pub struct PlumeNumberInputProps {
    /// Initial value
    pub value: f32,
    /// Decimal places used to display and commit the value
    pub precision: usize,
    /// Minimum committable value
    pub min: f32,
    /// Maximum committable value
    pub max: f32,
    /// Up/Down arrow increment
    pub step: f32,
    /// Optional non-editable suffix shown after the number (a unit such as `px`, `%`, or `°`).
    pub suffix: Option<String>,
}

impl Default for PlumeNumberInputProps {
    fn default() -> Self {
        Self {
            value: 0.0,
            precision: 2,
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
            step: 1.0,
            suffix: None,
        }
    }
}

impl PlumeNumberInput {
    fn scene(props: PlumeNumberInputProps) -> impl Scene {
        bsn! {
            text_input_frame()
            // A value does measure, but a content-sized field would resize as
            // digits come and go, so it keeps a fixed fallback.
            DefaultWidth(size::em_from_px(60.0))
            PlumeNumberInput { precision: {props.precision} }
            SliderValue({props.value})
            SliderRange::new(props.min, props.max)
            SliderStep({props.step})
            Children [
                (
                    text_input_field(None, None)
                    EditableTextFilter::new(|c| {
                        c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')
                    })
                    // Right-align digits against the field's trailing edge (caret sits at the
                    // right, value grows leftward as you type).
                    TextLayout {
                        justify: Justify::Right,
                        linebreak: LineBreak::NoWrap,
                    }
                    // Monospace face pinned; size inherits.
                    InheritableFont {
                        font: FontSourceTemplate::Handle(fonts::MONOSPACE),
                    }
                    on(number_input_on_key)
                    on(number_input_on_focus_lost)
                ),
                {props.suffix.map(|suffix| bsn_list!(text_input_suffix(suffix)))}
            ]
        }
    }
}

fn format_value(value: f32, precision: usize) -> String {
    format!("{value:.precision$}")
}

// Clamps + rounds `candidate` and commits it, emitting `ValueChange<f32>`.
// `source` is the [`PlumeNumberInput`] frame, not the editable field.
fn apply_value(
    candidate: f32,
    precision: usize,
    value: f32,
    range: &SliderRange,
    editable_text: &mut EditableText,
    source: Entity,
    commands: &mut Commands,
) {
    let factor = 10f32.powi(precision as i32);
    let committed = (range.clamp(candidate) * factor).round() / factor;
    if committed != value {
        // SliderValue is immutable: written by re-insertion, which is what the
        // Changed-filtered text-sync system picks up.
        commands.entity(source).insert(SliderValue(committed));
        commands.trigger(ValueChange {
            source,
            value: committed,
            is_final: true,
        });
    } else {
        // Unchanged value won't retrigger the text-sync system; normalize here.
        set_editable_text(editable_text, format_value(committed, precision));
    }
}

// Parses the typed text; on success commits it via [`apply_value`], otherwise reverts the text.
fn commit(
    precision: usize,
    value: f32,
    range: &SliderRange,
    editable_text: &mut EditableText,
    source: Entity,
    commands: &mut Commands,
) {
    match parse_typed(editable_text) {
        Some(parsed) => apply_value(
            parsed,
            precision,
            value,
            range,
            editable_text,
            source,
            commands,
        ),
        None => set_editable_text(editable_text, format_value(value, precision)),
    }
}

fn parse_typed(editable_text: &EditableText) -> Option<f32> {
    editable_text
        .value()
        .to_string()
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|parsed| parsed.is_finite())
}

fn number_input_on_key(
    key_input: On<FocusedInput<KeyboardInput>>,
    mut query_fields: Query<(&ChildOf, &mut EditableText), With<TextInputField>>,
    query_frames: Query<(&PlumeNumberInput, &SliderValue, &SliderRange, &SliderStep)>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    if key_input.input.state != ButtonState::Pressed {
        return;
    }
    let field = key_input.event_target();
    let Ok((child_of, mut editable_text)) = query_fields.get_mut(field) else {
        return;
    };
    let frame = child_of.parent();
    let Ok((number_input, value, range, step)) = query_frames.get(frame) else {
        return;
    };
    match key_input.input.key_code {
        KeyCode::Enter | KeyCode::NumpadEnter => {
            commit(
                number_input.precision,
                value.0,
                range,
                &mut editable_text,
                frame,
                &mut commands,
            );
            focus.clear();
        }
        KeyCode::Escape => {
            set_editable_text(
                &mut editable_text,
                format_value(value.0, number_input.precision),
            );
            focus.clear();
        }
        KeyCode::ArrowUp | KeyCode::ArrowDown => {
            let delta = match key_input.input.key_code {
                KeyCode::ArrowUp => step.0,
                _ => -step.0,
            };
            // Step from the typed value when it parses, so an uncommitted entry isn't lost.
            let base = parse_typed(&editable_text).unwrap_or(value.0);
            apply_value(
                base + delta,
                number_input.precision,
                value.0,
                range,
                &mut editable_text,
                frame,
                &mut commands,
            );
        }
        _ => {}
    }
}

fn number_input_on_focus_lost(
    focus_lost: On<FocusLost>,
    mut query_fields: Query<(&ChildOf, &mut EditableText), With<TextInputField>>,
    query_frames: Query<(&PlumeNumberInput, &SliderValue, &SliderRange)>,
    mut commands: Commands,
) {
    let field = focus_lost.event_target();
    let Ok((child_of, mut editable_text)) = query_fields.get_mut(field) else {
        return;
    };
    let frame = child_of.parent();
    if let Ok((number_input, value, range)) = query_frames.get(frame) {
        commit(
            number_input.precision,
            value.0,
            range,
            &mut editable_text,
            frame,
            &mut commands,
        );
    }
}

// Reflect any value write (self-commit or external, e.g. a paired slider) into the field's text.
fn update_number_input_text(
    query_frames: Query<(&PlumeNumberInput, &SliderValue, &Children), Changed<SliderValue>>,
    mut query_fields: Query<&mut EditableText, With<TextInputField>>,
) {
    for (number_input, value, children) in query_frames.iter() {
        for &child in children.iter() {
            if let Ok(mut editable_text) = query_fields.get_mut(child) {
                set_editable_text(
                    &mut editable_text,
                    format_value(value.0, number_input.precision),
                );
                break;
            }
        }
    }
}

// Plugin which keeps the number input's text in sync with its value.
pub(crate) struct NumberInputPlugin;

impl Plugin for NumberInputPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            update_number_input_text.in_set(PickingSystems::Last),
        );
    }
}
