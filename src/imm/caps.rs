//! Plume-owned immediate-mode capabilities: who-wins value flow, select sync,
//! and dialog close detection.
use bevy_ecs::hierarchy::Children;
use bevy_immediate::{
    CapSet, ImmCapAccessRequests, ImmCapability, ImmEntity, ImplCap, imm_id,
    ui::track_value_change_plugin::{NewValueChange, TrackValueChangePlugin},
};
use bevy_input_focus::InputFocus;
use bevy_ui::{Checked, Pressed};
use bevy_ui_widgets::SliderValue;

use crate::{
    containers::DialogCloseRequested,
    controls::{SelectedIndex, SetSelectedIndex, SetTextInputValue, TextInputValue},
};

/// Synchronises an app `f32` with a control's [`SliderValue`] (slider, number input).
///
/// Who-wins: a pending user edit (via `ValueChange<f32>`) always lands in the app
/// value; otherwise the app value is pushed to the widget, except while the user
/// is interacting (dragging, or editing the focused field).
pub struct CapabilityPlumeValue;

impl ImmCapability for CapabilityPlumeValue {
    fn build<Cap: CapSet>(app: &mut bevy_app::App, cap_req: &mut ImmCapAccessRequests<Cap>) {
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
    fn plume_value(self, value: &mut f32, changed: &mut bool) -> Self;
}

impl<Cap> ImmPlumeValue for ImmEntity<'_, '_, '_, Cap>
where
    Cap: ImplCap<CapabilityPlumeValue>,
{
    fn plume_value(mut self, value: &mut f32, changed: &mut bool) -> Self {
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
            self.entity_commands()
                .insert((NewValueChange::<f32>::default(), SliderValue(*value)));
            return self;
        };

        let interacting = pressed || focused_within(&self);
        if let Some(new_value) = pending
            && new_value != *value
        {
            *value = new_value;
            *changed = true;
        } else if !interacting && *value != widget_value {
            self.entity_commands().insert(SliderValue(*value));
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
    fn build<Cap: CapSet>(app: &mut bevy_app::App, cap_req: &mut ImmCapAccessRequests<Cap>) {
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
    fn build<Cap: CapSet>(app: &mut bevy_app::App, cap_req: &mut ImmCapAccessRequests<Cap>) {
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
    fn build<Cap: CapSet>(app: &mut bevy_app::App, cap_req: &mut ImmCapAccessRequests<Cap>) {
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

/// Lets the imm layer see a dialog's pending close request.
pub struct CapabilityPlumeDialog;

impl ImmCapability for CapabilityPlumeDialog {
    fn build<Cap: CapSet>(app: &mut bevy_app::App, cap_req: &mut ImmCapAccessRequests<Cap>) {
        cap_req.request_component_read::<DialogCloseRequested>(app.world_mut());
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
        self.cap_entity_contains::<DialogCloseRequested>()
    }
}
