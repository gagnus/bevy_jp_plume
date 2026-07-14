//! Numeric text field: type a value, commit on Enter or focus loss.
use bevy_app::{Plugin, PreUpdate};
use bevy_ecs::{
    entity::Entity,
    event::EntityEvent,
    observer::On,
    query::Changed,
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, ResMut},
};
use bevy_input::{ButtonState, keyboard::KeyCode, keyboard::KeyboardInput};
use bevy_input_focus::{FocusLost, FocusedInput, InputFocus};
use bevy_picking::PickingSystems;
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::prelude::*;
use bevy_text::{EditableText, EditableTextFilter, TextEdit};
use bevy_ui::{Node, px};
use bevy_ui_widgets::{SliderRange, SliderValue, ValueChange};

use crate::controls::PlumeTextInput;

/// A numeric input built on [`PlumeTextInput`]. Holds its value in [`SliderValue`] /
/// [`SliderRange`] (interchangeable with a slider) and emits [`ValueChange<f32>`] on commit.
///
/// This is spawnable by inheriting it as a "scene component" with optional
/// [`PlumeNumberInputProps`]. Typing commits on Enter or focus loss (clamped to the
/// range, rounded to `precision`); Escape or an unparsable entry reverts the text.
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
}

impl Default for PlumeNumberInputProps {
    fn default() -> Self {
        Self {
            value: 0.0,
            precision: 2,
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
        }
    }
}

impl PlumeNumberInput {
    fn scene(props: PlumeNumberInputProps) -> impl Scene {
        bsn! {
            @PlumeTextInput
            Node {
                width: px(64.0),
            }
            PlumeNumberInput { precision: {props.precision} }
            SliderValue({props.value})
            SliderRange::new(props.min, props.max)
            EditableTextFilter::new(|c| {
                c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')
            })
            on(number_input_on_key)
            on(number_input_on_focus_lost)
        }
    }
}

fn format_value(value: f32, precision: usize) -> String {
    format!("{value:.precision$}")
}

// Rewrites the visible text from the committed value (also the parse-failure revert path).
fn set_text(editable_text: &mut EditableText, formatted: String) {
    if editable_text.value() != &formatted {
        editable_text.queue_edit(TextEdit::SelectAll);
        editable_text.queue_edit(TextEdit::Insert(formatted.into()));
    }
}

// Parses the typed text; on success clamps + rounds and writes the value (emitting
// `ValueChange<f32>`), otherwise reverts the text. Idempotent for repeated calls.
fn commit(
    precision: usize,
    value: f32,
    range: &SliderRange,
    editable_text: &mut EditableText,
    source: Entity,
    commands: &mut Commands,
) {
    let typed = editable_text.value().to_string();
    match typed.trim().parse::<f32>() {
        Ok(parsed) if parsed.is_finite() => {
            fn round_to_precision(value: f32, precision: usize) -> f32 {
                let factor = 10f32.powi(precision as i32);
                (value * factor).round() / factor
            }
            let committed = round_to_precision(range.clamp(parsed), precision);
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
                set_text(editable_text, format_value(committed, precision));
            }
        }
        _ => set_text(editable_text, format_value(value, precision)),
    }
}

fn number_input_on_key(
    key_input: On<FocusedInput<KeyboardInput>>,
    mut query_inputs: Query<(
        &PlumeNumberInput,
        &SliderValue,
        &SliderRange,
        &mut EditableText,
    )>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    if key_input.input.state != ButtonState::Pressed {
        return;
    }
    let input_entity = key_input.event_target();
    let Ok((number_input, value, range, mut editable_text)) = query_inputs.get_mut(input_entity)
    else {
        return;
    };
    match key_input.input.key_code {
        KeyCode::Enter | KeyCode::NumpadEnter => {
            commit(
                number_input.precision,
                value.0,
                range,
                &mut editable_text,
                input_entity,
                &mut commands,
            );
            focus.clear();
        }
        KeyCode::Escape => {
            set_text(
                &mut editable_text,
                format_value(value.0, number_input.precision),
            );
            focus.clear();
        }
        _ => {}
    }
}

fn number_input_on_focus_lost(
    focus_lost: On<FocusLost>,
    mut query_inputs: Query<(
        &PlumeNumberInput,
        &SliderValue,
        &SliderRange,
        &mut EditableText,
    )>,
    mut commands: Commands,
) {
    let input_entity = focus_lost.event_target();
    if let Ok((number_input, value, range, mut editable_text)) = query_inputs.get_mut(input_entity)
    {
        commit(
            number_input.precision,
            value.0,
            range,
            &mut editable_text,
            input_entity,
            &mut commands,
        );
    }
}

/// Reflect any value write (self-commit or external, e.g. a paired slider) into the text.
fn update_number_input_text(
    mut query_inputs: Query<
        (&PlumeNumberInput, &SliderValue, &mut EditableText),
        Changed<SliderValue>,
    >,
) {
    for (number_input, value, mut editable_text) in query_inputs.iter_mut() {
        set_text(
            &mut editable_text,
            format_value(value.0, number_input.precision),
        );
    }
}

/// Plugin which keeps the number input's text in sync with its value.
pub struct NumberInputPlugin;

impl Plugin for NumberInputPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            update_number_input_text.in_set(PickingSystems::Last),
        );
    }
}
