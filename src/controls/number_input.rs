//! Numeric text field: type a value, commit on Enter or focus loss.
use bevy_app::{Plugin, PreUpdate};
use bevy_ecs::{
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{Changed, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, ResMut},
};
use bevy_input::{ButtonState, keyboard::KeyCode, keyboard::KeyboardInput};
use bevy_input_focus::{FocusLost, FocusedInput, InputFocus};
use bevy_picking::PickingSystems;
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::prelude::*;
use bevy_text::{
    EditableText, EditableTextFilter, FontSourceTemplate, Justify, LineBreak, TextEdit, TextFont,
    TextLayout,
};
use bevy_ui::{Node, px};
use bevy_ui_widgets::{SliderRange, SliderValue, ValueChange};

use crate::{
    constants::fonts,
    controls::{TextInputField, text_input_field, text_input_frame, text_input_suffix},
};

/// A numeric input built on the [`PlumeTextInput`](crate::controls::PlumeTextInput) frame. Holds
/// its value in [`SliderValue`] / [`SliderRange`] (interchangeable with a slider) and emits
/// [`ValueChange<f32>`] on commit.
///
/// This is spawnable by inheriting it as a "scene component" with optional
/// [`PlumeNumberInputProps`]. Typing commits on Enter or focus loss (clamped to the
/// range, rounded to `precision`); Escape or an unparsable entry reverts the text. The value and
/// range live on this frame entity; the editable `TextInputField` child does the typing.
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
            suffix: None,
        }
    }
}

impl PlumeNumberInput {
    fn scene(props: PlumeNumberInputProps) -> impl Scene {
        bsn! {
            text_input_frame()
            Node {
                width: px(64.0),
            }
            PlumeNumberInput { precision: {props.precision} }
            SliderValue({props.value})
            SliderRange::new(props.min, props.max)
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
                    TextFont {
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

// Rewrites the visible text from the committed value (also the parse-failure revert path).
fn set_text(editable_text: &mut EditableText, formatted: String) {
    if editable_text.value() != &formatted {
        editable_text.queue_edit(TextEdit::SelectAll);
        editable_text.queue_edit(TextEdit::Insert(formatted.into()));
    }
}

// Parses the typed text; on success clamps + rounds and writes the value (emitting
// `ValueChange<f32>`), otherwise reverts the text. Idempotent for repeated calls. `source` is the
// [`PlumeNumberInput`] frame that owns the value, not the editable field.
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
    mut query_fields: Query<(&ChildOf, &mut EditableText), With<TextInputField>>,
    query_frames: Query<(&PlumeNumberInput, &SliderValue, &SliderRange)>,
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
    let Ok((number_input, value, range)) = query_frames.get(frame) else {
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

/// Reflect any value write (self-commit or external, e.g. a paired slider) into the field's text.
fn update_number_input_text(
    query_frames: Query<(&PlumeNumberInput, &SliderValue, &Children), Changed<SliderValue>>,
    mut query_fields: Query<&mut EditableText, With<TextInputField>>,
) {
    for (number_input, value, children) in query_frames.iter() {
        for &child in children.iter() {
            if let Ok(mut editable_text) = query_fields.get_mut(child) {
                set_text(
                    &mut editable_text,
                    format_value(value.0, number_input.precision),
                );
                break;
            }
        }
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
