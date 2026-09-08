//! Numeric text field: type a value, commit on Enter or focus loss; drag to scrub.
use bevy::app::{Plugin, PreUpdate};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EntityEvent;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::observer::On;
use bevy::ecs::query::{Changed, Has, With};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::input::keyboard::{Key, KeyCode, KeyboardInput};
use bevy::input::{ButtonInput, ButtonState};
use bevy::input_focus::{FocusCause, FocusLost, FocusedInput, InputFocus};
use bevy::picking::events::{
    PointerCancel, PointerDrag, PointerDragEnd, PointerDragStart, PointerPress, PointerRelease,
};
use bevy::picking::pointer::PointerButton;
use bevy::picking::{Pickable, PickingSystems};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::text::{
    EditableText, EditableTextFilter, FontSourceTemplate, Justify, LineBreak, TextLayout,
};
use bevy::ui::{InteractionDisabled, Node, PositionType, Pressed, Val};
use bevy::ui_widgets::{SliderRange, SliderStep, SliderValue, ValueChange};
use bevy::window::SystemCursorIcon;

use crate::constants::{fonts, size};
use crate::controls::{
    TextInputField, set_editable_text, text_input_field, text_input_frame, text_input_outline,
    text_input_suffix,
};
use crate::cursor::{CursorLock, EntityCursor};
use crate::default_width::DefaultWidth;
use crate::font_styles::InheritableFont;

// Pointer travel (px) below which a press-and-release is a click (focus the
// field and edit); beyond it the gesture scrubs the value instead.
const DRAG_THRESHOLD_PX: f32 = 4.0;
// A finite range spans about this much pointer travel edge to edge...
const DRAG_RANGE_PX: f32 = 200.0;
// ...unless that would cross more steps than this, which marks the range as a
// bare clamp (e.g. the imm layer's integer-type limits), not a scrubbable span.
const DRAG_MAX_STEPS: f32 = 2000.0;
// Shift while scrubbing slows the drag for fine adjustment.
const DRAG_FINE_FACTOR: f32 = 0.1;

/// A numeric input on the [`PlumeTextInput`](crate::controls::PlumeTextInput)
/// frame, holding its value in [`SliderValue`] / [`SliderRange`] like a slider.
///
/// A click focuses the field for typing; a horizontal drag scrubs the value
/// without focusing (opt out with [`NoDrag`]). Typing commits on Enter or focus
/// loss (clamped and rounded to `precision`); Escape or an unparsable entry
/// reverts; Up/Down step by [`SliderStep`].
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeNumberInputProps)]
#[reflect(Component, Default, Clone)]
pub struct PlumeNumberInput;

// Plain root marker, inserted on both the retained and imm paths. The systems key on
// this — and it carries the precision — rather than the [`PlumeNumberInput`] scene
// component, which only the retained path inserts.
#[derive(Component, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub(crate) struct NumberInputFrame {
    // Decimal places used to display and commit the value.
    pub(crate) precision: usize,
}

impl Default for NumberInputFrame {
    fn default() -> Self {
        Self { precision: 2 }
    }
}

/// Props used to construct the [`PlumeNumberInput`] scene.
#[derive(Clone)]
pub struct PlumeNumberInputProps {
    /// Initial value.
    pub value: f32,
    /// Decimal places used to display and commit the value.
    pub precision: usize,
    /// Minimum committable value.
    pub min: f32,
    /// Maximum committable value.
    pub max: f32,
    /// Up/Down arrow increment.
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

/// Opt-out marker on a [`PlumeNumberInput`] frame: disables drag-to-scrub;
/// clicking to type still works.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct NoDrag;

// The drag-catching overlay child of the frame, and its gesture state. Pickable while
// idle so it sees the press before the field can take focus; `Pickable::IGNORE` while
// focused, disabled or opted out, letting clicks reach the text underneath.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct NumberInputScrubber {
    // Value when the drag started.
    base: f32,
    // Accumulated drag travel in value units, clamped to the range so a drag
    // past an edge reverses immediately instead of unwinding dead travel.
    offset: f32,
    // Value units per pixel of pointer travel.
    speed: f32,
    // Peak pointer distance from the press point: the click-vs-drag discriminator.
    max_distance: f32,
}

impl PlumeNumberInput {
    fn scene(props: PlumeNumberInputProps) -> impl Scene {
        bsn! {
            @text_input_frame()
            // A value does measure, but a content-sized field would resize as
            // digits come and go, so it keeps a fixed fallback.
            DefaultWidth(size::em_from_px(60.0))
            NumberInputFrame { precision: {props.precision} }
            SliderValue({props.value})
            SliderRange::new(props.min, props.max)
            SliderStep({props.step})
            Children [
                @text_input_outline()
                --
                @text_input_field(None, None)
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
                --
                {props.suffix.map(|suffix| bsn! { @text_input_suffix(suffix) })}
                --
                // Scrub/click catcher covering the whole frame, field and suffix alike.
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::ZERO,
                    right: Val::ZERO,
                    top: Val::ZERO,
                    bottom: Val::ZERO,
                }
                NumberInputScrubber
                Pickable::default()
                EntityCursor::System(SystemCursorIcon::Pointer)
                CursorLock
                on(scrubber_on_press)
                on(scrubber_on_release)
                on(scrubber_on_drag_start)
                on(scrubber_on_drag)
                on(scrubber_on_drag_end)
                on(scrubber_on_cancel)
            ]
        }
    }
}

fn format_value(value: f32, precision: usize) -> String {
    format!("{value:.precision$}")
}

// Clamp to the range and round to `precision` — the canonical committed form.
fn clamp_round(candidate: f32, precision: usize, range: &SliderRange) -> f32 {
    let factor = 10f32.powi(precision as i32);
    (range.clamp(candidate) * factor).round() / factor
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
    let committed = clamp_round(candidate, precision, range);
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
    query_frames: Query<(&NumberInputFrame, &SliderValue, &SliderRange, &SliderStep)>,
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
    query_frames: Query<(&NumberInputFrame, &SliderValue, &SliderRange)>,
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

// Begin a potential scrub: reset the gesture state and stop the press so nothing
// upstream reacts. Focus is deliberately not taken — a clean click grants it on
// release, a drag never does.
fn scrubber_on_press(
    mut press: On<PointerPress>,
    mut q_scrubbers: Query<(&ChildOf, &mut NumberInputScrubber)>,
    q_frames: Query<Has<InteractionDisabled>, With<NumberInputFrame>>,
    mut commands: Commands,
) {
    if press.button != PointerButton::Primary {
        return;
    }
    let Ok((child_of, mut scrub)) = q_scrubbers.get_mut(press.event_target()) else {
        return;
    };
    let frame = child_of.parent();
    let Ok(disabled) = q_frames.get(frame) else {
        return;
    };
    if disabled {
        return;
    }
    press.propagate(false);
    *scrub = NumberInputScrubber::default();
    // `Pressed` marks the frame "interacting" for the imm who-wins value sync,
    // exactly as a grabbed slider does.
    commands.entity(frame).insert(Pressed);
}

fn scrubber_on_drag_start(
    mut drag_start: On<PointerDragStart>,
    mut q_scrubbers: Query<(&ChildOf, &mut NumberInputScrubber)>,
    q_frames: Query<(&SliderValue, &SliderRange, &SliderStep), With<NumberInputFrame>>,
    mut commands: Commands,
) {
    if drag_start.button != PointerButton::Primary {
        return;
    }
    let Ok((child_of, mut scrub)) = q_scrubbers.get_mut(drag_start.event_target()) else {
        return;
    };
    let Ok((value, range, step)) = q_frames.get(child_of.parent()) else {
        return;
    };
    drag_start.propagate(false);
    // Only now is this a scrub rather than a click-to-focus: `Pressed` pairs
    // with the scrubber's `CursorLock` to hold the resize cursor swapped in here.
    commands
        .entity(drag_start.event_target())
        .insert((Pressed, EntityCursor::System(SystemCursorIcon::EwResize)));
    scrub.base = value.0;
    scrub.offset = 0.0;

    // A usable finite range maps to ~DRAG_RANGE_PX of travel; anything else
    // (unbounded, or a range that is only a clamp) moves one step per pixel.
    let width = range.end() - range.start();
    scrub.speed = if width.is_finite() && width / step.0 <= DRAG_MAX_STEPS {
        width / DRAG_RANGE_PX
    } else {
        step.0
    };
}

fn scrubber_on_drag(
    mut drag: On<PointerDrag>,
    mut q_scrubbers: Query<(&ChildOf, &mut NumberInputScrubber)>,
    q_frames: Query<
        (
            &NumberInputFrame,
            &SliderValue,
            &SliderRange,
            Has<InteractionDisabled>,
        ),
        With<NumberInputFrame>,
    >,
    keys: Res<ButtonInput<Key>>,
    mut commands: Commands,
) {
    if drag.button != PointerButton::Primary {
        return;
    }
    let Ok((child_of, mut scrub)) = q_scrubbers.get_mut(drag.event_target()) else {
        return;
    };
    let frame = child_of.parent();
    let Ok((number_input, value, range, disabled)) = q_frames.get(frame) else {
        return;
    };
    drag.propagate(false);
    scrub.max_distance = scrub.max_distance.max(drag.distance.length());
    if disabled || scrub.max_distance <= DRAG_THRESHOLD_PX {
        return;
    }
    let fine = if keys.pressed(Key::Shift) {
        DRAG_FINE_FACTOR
    } else {
        1.0
    };
    scrub.offset += drag.delta.x * scrub.speed * fine;
    scrub.offset = range.clamp(scrub.base + scrub.offset) - scrub.base;
    let committed = clamp_round(scrub.base + scrub.offset, number_input.precision, range);
    if committed != value.0 {
        commands.entity(frame).insert(SliderValue(committed));
        commands.trigger(ValueChange {
            source: frame,
            value: committed,
            is_final: false,
        });
    }
}

fn scrubber_on_drag_end(
    mut drag_end: On<PointerDragEnd>,
    q_scrubbers: Query<(&ChildOf, &NumberInputScrubber)>,
    q_frames: Query<&SliderValue, With<NumberInputFrame>>,
    mut commands: Commands,
) {
    if drag_end.button != PointerButton::Primary {
        return;
    }
    let Ok((child_of, scrub)) = q_scrubbers.get(drag_end.event_target()) else {
        return;
    };
    let frame = child_of.parent();
    let Ok(value) = q_frames.get(frame) else {
        return;
    };
    drag_end.propagate(false);
    // A scrub usually ends with the pointer off the strip, where `Release` (sent
    // to the hovered entity) never arrives; only this event comes back here.
    commands.entity(frame).remove::<Pressed>();
    commands
        .entity(drag_end.event_target())
        .remove::<Pressed>()
        .insert(EntityCursor::System(SystemCursorIcon::Pointer));
    if scrub.max_distance > DRAG_THRESHOLD_PX {
        commands.trigger(ValueChange {
            source: frame,
            value: value.0,
            is_final: true,
        });
    }
}

// End of the press: a release that never travelled past the click threshold
// focuses the field (select-all comes from `SelectAllOnFocus`); a scrub does not.
fn scrubber_on_release(
    mut release: On<PointerRelease>,
    q_scrubbers: Query<(&ChildOf, &NumberInputScrubber)>,
    q_children: Query<&Children>,
    q_fields: Query<(), With<TextInputField>>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    if release.button != PointerButton::Primary {
        return;
    }
    let Ok((child_of, scrub)) = q_scrubbers.get(release.event_target()) else {
        return;
    };
    let frame = child_of.parent();
    release.propagate(false);
    commands.entity(frame).remove::<Pressed>();
    commands
        .entity(release.event_target())
        .remove::<Pressed>()
        .insert(EntityCursor::System(SystemCursorIcon::Pointer));
    if scrub.max_distance <= DRAG_THRESHOLD_PX
        && let Ok(children) = q_children.get(frame)
        && let Some(field) = children
            .iter()
            .copied()
            .find(|&child| q_fields.contains(child))
    {
        // `Navigated`, not `Pressed`: a `Pressed` cause defers the select-all to
        // the next pointer release, which for this click has already happened.
        focus.set(field, FocusCause::Navigated);
    }
}

// Pointer lost mid-gesture: abandon the scrub and restore the pre-drag value.
fn scrubber_on_cancel(
    mut cancel: On<PointerCancel>,
    q_scrubbers: Query<(&ChildOf, &NumberInputScrubber)>,
    q_frames: Query<&SliderValue, With<NumberInputFrame>>,
    mut commands: Commands,
) {
    let Ok((child_of, scrub)) = q_scrubbers.get(cancel.event_target()) else {
        return;
    };
    let frame = child_of.parent();
    cancel.propagate(false);
    commands.entity(frame).remove::<Pressed>();
    commands
        .entity(cancel.event_target())
        .remove::<Pressed>()
        .insert(EntityCursor::System(SystemCursorIcon::Pointer));
    if scrub.max_distance > DRAG_THRESHOLD_PX
        && let Ok(value) = q_frames.get(frame)
        && value.0 != scrub.base
    {
        commands.entity(frame).insert(SliderValue(scrub.base));
        commands.trigger(ValueChange {
            source: frame,
            value: scrub.base,
            is_final: true,
        });
    }
}

// Keep each scrubber's pickability in step with its frame: it must swallow
// presses while idle, and get out of the way while the field is being edited,
// the input is disabled, or the frame opted out via [`NoDrag`].
fn update_scrubber_pickable(
    mut q_scrubbers: Query<(&ChildOf, &mut Pickable), With<NumberInputScrubber>>,
    q_frames: Query<(Has<NoDrag>, Has<InteractionDisabled>, &Children), With<NumberInputFrame>>,
    q_fields: Query<(), With<TextInputField>>,
    focus: Res<InputFocus>,
) {
    for (child_of, mut pickable) in q_scrubbers.iter_mut() {
        let Ok((no_drag, disabled, children)) = q_frames.get(child_of.parent()) else {
            continue;
        };
        let editing = focus
            .get()
            .is_some_and(|focused| children.contains(&focused) && q_fields.contains(focused));
        let wanted = if no_drag || disabled || editing {
            Pickable::IGNORE
        } else {
            Pickable::default()
        };
        if *pickable != wanted {
            *pickable = wanted;
        }
    }
}

// Reflect any value write (self-commit or external, e.g. a paired slider) into the field's text.
fn update_number_input_text(
    query_frames: Query<(&NumberInputFrame, &SliderValue, &Children), Changed<SliderValue>>,
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
            (update_number_input_text, update_scrubber_pickable).in_set(PickingSystems::Last),
        );
    }
}
