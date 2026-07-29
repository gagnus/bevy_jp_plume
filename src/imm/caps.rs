//! Plume-owned immediate-mode capabilities: who-wins value flow, select sync,
//! and dialog close detection.
use bevy::color::Color;
use bevy::ecs::hierarchy::Children;
use bevy::ecs::resource::Resource;
use bevy::input_focus::InputFocus;
use bevy::platform::collections::HashMap;
use bevy::ui::{Checked, Pressed};
use bevy::ui_widgets::SliderValue;
use bevy_immediate::{
    CapSet, ImmCapAccessRequests, ImmCapability, ImmEntity, ImmId, ImplCap, imm_id,
    ui::track_value_change_plugin::{NewValueChange, TrackValueChangePlugin},
};

use crate::utils::numeric::Numeric;
use crate::{
    containers::CloseRequested,
    controls::{
        ColorPickerValue, SelectedIndex, SetSelectedIndex, SetTextInputValue, TextInputValue,
    },
    display::TooltipShowing,
};

/// Synchronises an app [`Numeric`] with a control's [`SliderValue`] (slider,
/// number input).
///
/// Who-wins: a pending user edit (via `ValueChange<f32>`) always lands in the app
/// value; otherwise the app value is pushed to the widget, except while the user
/// is interacting (dragging, or editing the focused field).
pub struct CapabilityPlumeValue;

impl ImmCapability for CapabilityPlumeValue {
    fn build<Cap: CapSet>(app: &mut bevy::app::App, cap_req: &mut ImmCapAccessRequests<Cap>) {
        cap_req.request_component_write::<NewValueChange<f32>>(app.world_mut());
        cap_req.request_component_read::<SliderValue>(app.world_mut());
        cap_req.request_component_read::<Pressed>(app.world_mut());
        cap_req.request_component_read::<Children>(app.world_mut());
        cap_req.request_resource_read::<InputFocus>(app.world_mut());

        if !app.is_plugin_added::<TrackValueChangePlugin<f32>>() {
            app.add_plugins(TrackValueChangePlugin::<f32>::default());
        }
    }
}

/// Widget-side entry point for [`CapabilityPlumeValue`].
pub trait ImmPlumeValue {
    /// Two-way sync between `value` and the entity's [`SliderValue`];
    /// sets `changed` when a user edit landed in `value`.
    fn plume_value<T: Numeric>(self, value: &mut T, changed: &mut bool) -> Self;
}

impl<Cap> ImmPlumeValue for ImmEntity<'_, '_, '_, Cap>
where
    Cap: ImplCap<CapabilityPlumeValue>,
{
    fn plume_value<T: Numeric>(mut self, value: &mut T, changed: &mut bool) -> Self {
        let state = 'read: {
            let Ok(mut entity) = self.cap_get_entity_mut() else {
                break 'read None;
            };
            let pending = {
                let Some(mut mailbox) = entity.get_mut::<NewValueChange<f32>>() else {
                    break 'read None;
                };
                NewValueChange::take(&mut mailbox)
            };
            let Some(widget_value) = entity.get::<SliderValue>().map(|widget| widget.0) else {
                break 'read None;
            };
            Some((pending, widget_value, entity.contains::<Pressed>()))
        };

        let Some((pending, widget_value, pressed)) = state else {
            self.entity_commands().insert((
                NewValueChange::<f32>::default(),
                SliderValue(value.to_f32()),
            ));
            return self;
        };

        let interacting = pressed || focused_within(&self);
        // The edit is compared in `T`, so a sub-integer drag of an integer value
        // isn't reported as a change; the push-back is compared in `f32`, so an
        // integer snaps its widget back on release and an `f64` that has no exact
        // `f32` stops re-pushing every frame.
        if let Some(new_value) = pending
            && T::from_f32(new_value) != *value
        {
            *value = T::from_f32(new_value);
            *changed = true;
        } else if !interacting && value.to_f32() != widget_value {
            self.entity_commands().insert(SliderValue(value.to_f32()));
        }
        self
    }
}

// The focused entity is the editable field child of a text-input frame, so a
// direct-children check covers the number input.
fn focused_within<Cap: CapSet>(entity: &ImmEntity<'_, '_, '_, Cap>) -> bool {
    let Ok(focus) = entity.cap_get_resource::<InputFocus>() else {
        return false;
    };
    let Some(focused) = focus.get() else {
        return false;
    };
    if focused == entity.entity() {
        return true;
    }
    let Ok(entity_ref) = entity.cap_get_entity() else {
        return false;
    };
    entity_ref
        .get::<Children>()
        .is_some_and(|children| children.contains(&focused))
}

/// Synchronises an app `bool` with a self-updating checked control ([`Checked`]).
pub struct CapabilityPlumeChecked;

impl ImmCapability for CapabilityPlumeChecked {
    fn build<Cap: CapSet>(app: &mut bevy::app::App, cap_req: &mut ImmCapAccessRequests<Cap>) {
        cap_req.request_component_write::<NewValueChange<bool>>(app.world_mut());
        cap_req.request_component_read::<Checked>(app.world_mut());

        if !app.is_plugin_added::<TrackValueChangePlugin<bool>>() {
            app.add_plugins(TrackValueChangePlugin::<bool>::default());
        }
    }
}

/// Widget-side entry point for [`CapabilityPlumeChecked`].
pub trait ImmPlumeChecked {
    /// Two-way sync between `value` and the entity's [`Checked`] state;
    /// sets `changed` when a user toggle landed in `value`.
    fn plume_checked(self, value: &mut bool, changed: &mut bool) -> Self;
}

impl<Cap> ImmPlumeChecked for ImmEntity<'_, '_, '_, Cap>
where
    Cap: ImplCap<CapabilityPlumeChecked>,
{
    fn plume_checked(mut self, value: &mut bool, changed: &mut bool) -> Self {
        let state = 'read: {
            let Ok(mut entity) = self.cap_get_entity_mut() else {
                break 'read None;
            };
            let pending = {
                let Some(mut mailbox) = entity.get_mut::<NewValueChange<bool>>() else {
                    break 'read None;
                };
                NewValueChange::take(&mut mailbox)
            };
            Some((pending, entity.contains::<Checked>()))
        };

        let Some((pending, widget_checked)) = state else {
            let mut commands = self.entity_commands();
            commands.insert(NewValueChange::<bool>::default());
            if *value {
                commands.insert(Checked);
            }
            return self;
        };

        if let Some(new_value) = pending
            && new_value != *value
        {
            // The control already self-updated; only the app value needs the news.
            *value = new_value;
            *changed = true;
        } else if *value != widget_checked {
            if *value {
                self.entity_commands().insert(Checked);
            } else {
                self.entity_commands().remove::<Checked>();
            }
        }
        self
    }
}

/// Synchronises an app `usize` index with a select's picked row.
///
/// The capability query only sees imm-managed entities, so all state flows
/// through the select root: [`SelectedIndex`] (maintained by the retained layer)
/// for reads, [`SetSelectedIndex`] for writes.
pub struct CapabilityPlumeSelect;

impl ImmCapability for CapabilityPlumeSelect {
    fn build<Cap: CapSet>(app: &mut bevy::app::App, cap_req: &mut ImmCapAccessRequests<Cap>) {
        cap_req.request_component_read::<SelectedIndex>(app.world_mut());
    }
}

/// Widget-side entry point for [`CapabilityPlumeSelect`].
pub trait ImmPlumeSelect {
    /// Two-way sync between `index` and the select's picked row;
    /// sets `changed` when a user pick landed in `index`.
    fn plume_select(self, index: &mut usize, changed: &mut bool) -> Self;
}

// Hash-memory key for the last widget index the imm layer synced against.
struct SelectSyncKey;

impl<Cap> ImmPlumeSelect for ImmEntity<'_, '_, '_, Cap>
where
    Cap: ImplCap<CapabilityPlumeSelect>,
{
    fn plume_select(mut self, index: &mut usize, changed: &mut bool) -> Self {
        let widget_index = match self.cap_get_component::<SelectedIndex>() {
            Ok(Some(selected)) => selected.0,
            _ => {
                // Not spawned/settled yet; the scene seeds the initial selection.
                self.hash_set_typ::<SelectSyncKey>(imm_id(*index));
                return self;
            }
        };

        if self.hash_get_typ::<SelectSyncKey>() != Some(imm_id(widget_index)) {
            // Widget moved since last sync: the user's pick wins.
            if widget_index != *index {
                *index = widget_index;
                *changed = true;
            }
            self.hash_set_typ::<SelectSyncKey>(imm_id(*index));
        } else if *index != widget_index {
            // The hash is deliberately left at the widget's value: until the push
            // lands, re-push rather than reading the stale widget back.
            let select_entity = self.entity();
            self.commands().trigger(SetSelectedIndex {
                entity: select_entity,
                index: *index,
            });
        }
        self
    }
}

/// Synchronises an app `String` with a text input's buffer, via the
/// [`TextInputValue`] mirror (reads) and [`SetTextInputValue`] (writes).
///
/// The user's typing always lands in the app string; app pushes are held back
/// while the field is focused.
pub struct CapabilityPlumeText;

impl ImmCapability for CapabilityPlumeText {
    fn build<Cap: CapSet>(app: &mut bevy::app::App, cap_req: &mut ImmCapAccessRequests<Cap>) {
        cap_req.request_component_read::<TextInputValue>(app.world_mut());
        cap_req.request_component_read::<Children>(app.world_mut());
        cap_req.request_resource_read::<InputFocus>(app.world_mut());
    }
}

/// Widget-side entry point for [`CapabilityPlumeText`].
pub trait ImmPlumeText {
    /// Two-way sync between `text` and the input's buffer;
    /// sets `changed` when a user edit landed in `text`.
    fn plume_text(self, text: &mut String, changed: &mut bool) -> Self;
}

// Hash-memory key for the last widget text the imm layer synced against.
struct TextSyncKey;

impl<Cap> ImmPlumeText for ImmEntity<'_, '_, '_, Cap>
where
    Cap: ImplCap<CapabilityPlumeText>,
{
    fn plume_text(mut self, text: &mut String, changed: &mut bool) -> Self {
        let widget_text = match self.cap_get_component::<TextInputValue>() {
            Ok(Some(value)) => value.0.clone(),
            _ => {
                // Not spawned/settled yet; the scene seeds the initial text.
                self.hash_set_typ::<TextSyncKey>(imm_id(&*text));
                return self;
            }
        };

        if self.hash_get_typ::<TextSyncKey>() != Some(imm_id(&widget_text)) {
            // Buffer moved since last sync: the user's typing wins.
            if widget_text != *text {
                *text = widget_text;
                *changed = true;
            }
            self.hash_set_typ::<TextSyncKey>(imm_id(&*text));
        } else if *text != widget_text && !focused_within(&self) {
            // Hash deliberately left at the widget's value: re-push until it lands
            // rather than reading the stale widget back. Focused fields wait for blur.
            let input_entity = self.entity();
            let new_text = text.clone();
            self.commands().trigger(SetTextInputValue {
                entity: input_entity,
                text: new_text,
            });
        }
        self
    }
}

/// Synchronises an app [`Color`] with a color picker's [`ColorPickerValue`].
///
/// The picker self-updates its value as the user drags, so this mirrors the
/// xy/select/text pattern: a widget value that moved since the last sync is the
/// user's edit and wins; otherwise the app value is pushed to the widget.
///
/// Everything is keyed on the color's linear-RGBA bits, not `Color` equality —
/// the widget stores its value as `Color::Hsva` while an app may hand in any
/// variant, and cross-variant `PartialEq` would report equal colors as different
/// and fight forever.
pub struct CapabilityPlumeColor;

impl ImmCapability for CapabilityPlumeColor {
    fn build<Cap: CapSet>(app: &mut bevy::app::App, cap_req: &mut ImmCapAccessRequests<Cap>) {
        cap_req.request_component_read::<ColorPickerValue>(app.world_mut());
    }
}

/// Widget-side entry point for [`CapabilityPlumeColor`].
pub trait ImmPlumeColor {
    /// Two-way sync between `value` and the picker's [`ColorPickerValue`];
    /// sets `changed` when a user edit landed in `value`.
    fn plume_color(self, value: &mut Color, changed: &mut bool) -> Self;
}

// Hash-memory key for the last widget color the imm layer synced against.
struct ColorSyncKey;

// `Color` isn't `Hash`, so key it on its linear-RGBA component bit patterns — a
// canonical space so any two variants of the same color compare equal.
fn color_bits(color: Color) -> (u32, u32, u32, u32) {
    let linear = color.to_linear();
    (
        linear.red.to_bits(),
        linear.green.to_bits(),
        linear.blue.to_bits(),
        linear.alpha.to_bits(),
    )
}

impl<Cap> ImmPlumeColor for ImmEntity<'_, '_, '_, Cap>
where
    Cap: ImplCap<CapabilityPlumeColor>,
{
    fn plume_color(mut self, value: &mut Color, changed: &mut bool) -> Self {
        let widget_value = match self.cap_get_component::<ColorPickerValue>() {
            Ok(Some(picker)) => picker.0,
            _ => {
                // Not spawned/settled yet; the scene seeds the initial color.
                self.hash_set_typ::<ColorSyncKey>(imm_id(color_bits(*value)));
                return self;
            }
        };

        if self.hash_get_typ::<ColorSyncKey>() != Some(imm_id(color_bits(widget_value))) {
            // Value moved since last sync: the user's edit wins.
            if color_bits(widget_value) != color_bits(*value) {
                *value = widget_value;
                *changed = true;
            }
            self.hash_set_typ::<ColorSyncKey>(imm_id(color_bits(*value)));
        } else if color_bits(*value) != color_bits(widget_value) {
            // Hash deliberately left at the widget's value: re-push until it lands
            // rather than reading the stale widget back.
            self.entity_commands().insert(ColorPickerValue(*value));
        }
        self
    }
}

/// Per-pass table counting how many times each `(parent, base id)` pair has been
/// requested, so repeated sibling widgets get distinct ids without the caller
/// supplying one. This reimplements, on plume's side, the occurrence
/// disambiguation that would otherwise have to live in `bevy_immediate`'s id
/// resolver — letting plume track the unmodified upstream crate. Cleared at the
/// start of every [`PlumeRoot`](crate::imm::PlumeRoot) build (i.e. per system run).
#[derive(Resource, Default)]
pub(crate) struct PlumeOccurrences(pub(crate) HashMap<ImmId, u32>);

/// Registers [`PlumeOccurrences`] and the write access the imm layer needs to
/// auto-disambiguate repeated sibling ids. Carries no widget-side entry point; it
/// exists purely so the resource is reachable mid-build via `Imm`'s capability
/// resources. See `ch_loc` in [`crate::imm`]'s `widgets` module.
pub struct CapabilityPlumeIds;

impl ImmCapability for CapabilityPlumeIds {
    fn build<Cap: CapSet>(app: &mut bevy::app::App, cap_req: &mut ImmCapAccessRequests<Cap>) {
        app.init_resource::<PlumeOccurrences>();
        cap_req.request_resource_write::<PlumeOccurrences>(app.world_mut());
    }
}

/// Lets the imm layer see a dialog's pending close request.
pub struct CapabilityPlumeDialog;

impl ImmCapability for CapabilityPlumeDialog {
    fn build<Cap: CapSet>(app: &mut bevy::app::App, cap_req: &mut ImmCapAccessRequests<Cap>) {
        cap_req.request_component_read::<CloseRequested>(app.world_mut());
    }
}

/// Widget-side entry point for [`CapabilityPlumeDialog`].
pub trait ImmPlumeDialog {
    /// The dialog's ✕ (or another `RequestClose` source) fired since last frame.
    fn close_requested(&self) -> bool;
}

impl<Cap> ImmPlumeDialog for ImmEntity<'_, '_, '_, Cap>
where
    Cap: ImplCap<CapabilityPlumeDialog>,
{
    fn close_requested(&self) -> bool {
        self.cap_entity_contains::<CloseRequested>()
    }
}

/// Lets `tooltip_ui` ask whether the tooltip state machine is showing for its
/// control.
pub struct CapabilityPlumeTooltip;

impl ImmCapability for CapabilityPlumeTooltip {
    fn build<Cap: CapSet>(app: &mut bevy::app::App, cap_req: &mut ImmCapAccessRequests<Cap>) {
        cap_req.request_component_read::<TooltipShowing>(app.world_mut());
    }
}

/// Widget-side entry point for [`CapabilityPlumeTooltip`].
pub trait ImmPlumeTooltip {
    /// The tooltip state machine is showing for this control.
    fn tooltip_showing(&self) -> bool;
}

impl<Cap> ImmPlumeTooltip for ImmEntity<'_, '_, '_, Cap>
where
    Cap: ImplCap<CapabilityPlumeTooltip>,
{
    fn tooltip_showing(&self) -> bool {
        self.cap_entity_contains::<TooltipShowing>()
    }
}
