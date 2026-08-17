//! The write half of a control's value: [`SetValue`], the counterpart of
//! [`ValueChange`](bevy::ui_widgets::ValueChange).
use bevy::app::{App, Plugin};
use bevy::color::Color;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EntityEvent;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::observer::On;
use bevy::ecs::query::{Or, With};
use bevy::ecs::system::{Commands, Query};
use bevy::ui::Checked;
use bevy::ui_widgets::{Checkbox, RadioButton, RadioGroup, SliderRange, SliderValue};

use crate::controls::ColorPickerValue;
use crate::utils::hierarchy::nearest_with;

/// Programmatically set a control's value, whatever kind of control it is. The
/// control's state and its mirror follow; no `ValueChange` is emitted, so an app
/// pushing a value back never hears its own write.
///
/// One event per value kind: `bool` checks a checkbox, toggle, disclosure or radio;
/// `f32` moves a slider or number input; `usize` picks a select option, a tab, or a
/// radio within its group; `String` replaces a text input's buffer; [`Color`]
/// retargets a color edit or picker.
#[derive(EntityEvent)]
pub struct SetValue<T> {
    /// The control root.
    pub entity: Entity,
    /// The value to set.
    pub value: T,
}

// Installs the [`SetValue`] handlers shared by every control that stores its value
// in a common component; the rest live with the control that owns them.
pub(crate) struct SetValuePlugin;

impl Plugin for SetValuePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_set_bool)
            .add_observer(on_set_f32)
            .add_observer(on_set_color);
    }
}

// Checkbox, toggle switch, disclosure and radio all hold their state in `Checked`.
fn on_set_bool(
    ev: On<SetValue<bool>>,
    q_checkable: Query<(), Or<(With<Checkbox>, With<RadioButton>)>>,
    q_radio: Query<(), With<RadioButton>>,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    q_groups: Query<(), With<RadioGroup>>,
    mut commands: Commands,
) {
    if !q_checkable.contains(ev.entity) {
        return;
    }
    if !ev.value {
        commands.entity(ev.entity).remove::<Checked>();
        return;
    }
    commands.entity(ev.entity).insert(Checked);

    // Checking a grouped radio has to clear its siblings, exactly as a click does.
    if q_radio.contains(ev.entity)
        && let Some(group) = nearest_with(ev.entity, &q_parents, &q_groups)
    {
        for descendant in q_children.iter_descendants(group) {
            if descendant != ev.entity && q_radio.contains(descendant) {
                commands.entity(descendant).remove::<Checked>();
            }
        }
    }
}

// Slider and number input both hold their value in `SliderValue`.
fn on_set_f32(
    ev: On<SetValue<f32>>,
    q_values: Query<Option<&SliderRange>, With<SliderValue>>,
    mut commands: Commands,
) {
    let Ok(range) = q_values.get(ev.entity) else {
        return;
    };
    let value = range.map_or(ev.value, |range| range.clamp(ev.value));
    commands.entity(ev.entity).insert(SliderValue(value));
}

// Color edit and color picker both report through `ColorPickerValue`.
fn on_set_color(
    ev: On<SetValue<Color>>,
    q_values: Query<(), With<ColorPickerValue>>,
    mut commands: Commands,
) {
    if q_values.contains(ev.entity) {
        commands
            .entity(ev.entity)
            .insert(ColorPickerValue(ev.value));
    }
}
