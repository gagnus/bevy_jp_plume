//! Plume-owned immediate-mode capabilities: who-wins value flow, select sync,
//! and dialog close detection.
use bevy::color::Color;
use bevy::ecs::hierarchy::Children;
use bevy::ecs::resource::Resource;
use bevy::input_focus::InputFocus;
use bevy::input_focus::tab_navigation::NavAction;
use bevy::platform::collections::HashMap;
use bevy::ui::{Checked, Pressed};
use bevy::ui_widgets::SliderValue;
use bevy_immediate::ui::track_value_change_plugin::{NewValueChange, TrackValueChangePlugin};
use bevy_immediate::{
    CapSet, ImmCapAccessRequests, ImmCapability, ImmEntity, ImmId, ImplCap, imm_id,
};

use crate::containers::{CloseRequested, SplitSize};
use crate::controls::{ColorPickerValue, MenuOpen, SelectedIndex, SetValue, TextInputValue};
use crate::display::TooltipShowing;
use crate::utils::numeric::Numeric;

/// Synchronizes an app [`Numeric`] with a control's [`SliderValue`] (slider,
/// number input).
///
/// Who-wins: a pending user edit always lands in the app value; otherwise the app
/// value is pushed to the widget, except while the user is interacting.
pub struct CapabilityPlumeValue;

impl ImmCapability for CapabilityPlumeValue {
    fn build<Cap: CapSet>(app: &mut bevy::app::App, cap_req: &mut ImmCapAccessRequests<Cap>) {
        cap_req.request_component_write::<NewValueChange<f32>>(app.world_mut());
        cap_req.request_component_write::<NewValueChange<SplitSize>>(app.world_mut());
        cap_req.request_component_read::<SliderValue>(app.world_mut());
        cap_req.request_component_read::<SplitSize>(app.world_mut());
        cap_req.request_component_read::<Pressed>(app.world_mut());
        cap_req.request_component_read::<Children>(app.world_mut());
        cap_req.request_resource_read::<InputFocus>(app.world_mut());

        if !app.is_plugin_added::<TrackValueChangePlugin<f32>>() {
            app.add_plugins(TrackValueChangePlugin::<f32>::default());
        }
        if !app.is_plugin_added::<TrackValueChangePlugin<SplitSize>>() {
            app.add_plugins(TrackValueChangePlugin::<SplitSize>::default());
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
        // The edit compares in `T`, so a sub-integer drag isn't reported as a change;
        // the push-back compares in `f32`, so an integer snaps its widget back on
        // release and an `f64` with no exact `f32` stops re-pushing every frame.
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

/// Widget-side entry point for a splitter's divider position.
pub trait ImmPlumeSplit {
    /// Two-way sync between `split` and the entity's [`SplitSize`]; sets
    /// `changed` when a drag (or a layout snap) landed in `split`.
    fn plume_split(self, split: &mut SplitSize, changed: &mut bool) -> Self;
}

impl<Cap> ImmPlumeSplit for ImmEntity<'_, '_, '_, Cap>
where
    Cap: ImplCap<CapabilityPlumeValue>,
{
    fn plume_split(mut self, split: &mut SplitSize, changed: &mut bool) -> Self {
        let state = 'read: {
            let Ok(mut entity) = self.cap_get_entity_mut() else {
                break 'read None;
            };
            let pending = {
                let Some(mut mailbox) = entity.get_mut::<NewValueChange<SplitSize>>() else {
                    break 'read None;
                };
                NewValueChange::take(&mut mailbox)
            };
            let Some(widget_value) = entity.get::<SplitSize>().copied() else {
                break 'read None;
            };
            Some((pending, widget_value))
        };

        // Not spawned yet; the scene seeds the size, and this is the mailbox
        // the divider's `ValueChange` will arrive in.
        let Some((pending, widget_value)) = state else {
            self.entity_commands()
                .insert(NewValueChange::<SplitSize>::default());
            return self;
        };

        // No interaction guard, unlike `plume_value`: the splitter writes its own
        // `SplitSize` as it drags, so the app value is already in step. The guard
        // would suppress what must get through — a drag clamped by a pane's minimum.
        if let Some(new_value) = pending
            && new_value != *split
        {
            *split = new_value;
            *changed = true;
        } else if *split != widget_value {
            self.entity_commands().insert(*split);
        }
        self
    }
}

/// Widget-side entry point for keyboard-focus state; the accesses ride on
/// [`CapabilityPlumeText`], which already requests them.
pub trait ImmPlumeFocus {
    /// Whether keyboard focus is on this widget or a direct child — where a
    /// text input keeps its editable field.
    fn focused(&self) -> bool;
}

impl<Cap> ImmPlumeFocus for ImmEntity<'_, '_, '_, Cap>
where
    Cap: ImplCap<CapabilityPlumeText>,
{
    fn focused(&self) -> bool {
        focused_within(self)
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

/// Synchronizes an app `bool` with a self-updating checked control ([`Checked`]).
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

/// Synchronizes an app `usize` index with a select's picked row.
///
/// The capability query only sees imm-managed entities, so state flows through the
/// select root: [`SelectedIndex`] for reads, [`SetValue<usize>`] for writes.
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
            self.commands().trigger(SetValue {
                entity: select_entity,
                value: *index,
            });
        }
        self
    }
}

/// Synchronizes an app `String` with a text input's buffer, via the
/// [`TextInputValue`] mirror (reads) and [`SetValue<String>`] (writes).
///
/// The user's typing always wins; app pushes are held back while the field is focused.
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
            self.commands().trigger(SetValue {
                entity: input_entity,
                value: new_text,
            });
        }
        self
    }
}

/// Synchronizes an app [`Color`] with a color picker's [`ColorPickerValue`].
///
/// The picker self-updates as the user drags, so a widget value that moved since
/// the last sync wins; otherwise the app value is pushed to the widget.
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

// `Color` isn't `Hash`, so key on linear-RGBA bits — a canonical space, so the
// widget's `Hsva` and an app's any-variant color don't compare unequal forever.
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

// Per-pass count of how many times each `(parent, base id)` pair has been requested,
// so repeated sibling widgets get distinct ids. See `ch_loc` in [`crate::imm`]'s
// `widgets` module; cleared at the start of every `PlumeRoot` build.
#[derive(Resource, Default)]
pub(crate) struct PlumeOccurrences(pub(crate) HashMap<ImmId, u32>);

/// Registers [`PlumeOccurrences`] and the write access the imm layer needs to
/// auto-disambiguate repeated sibling ids. No widget-side entry point of its own.
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

/// Lets the imm layer read a menu button's open state, which the retained menu
/// systems own (toggle, hover-switch, focus-out close).
pub struct CapabilityPlumeMenu;

impl ImmCapability for CapabilityPlumeMenu {
    fn build<Cap: CapSet>(app: &mut bevy::app::App, cap_req: &mut ImmCapAccessRequests<Cap>) {
        cap_req.request_component_read::<MenuOpen>(app.world_mut());
    }
}

/// Widget-side entry point for [`CapabilityPlumeMenu`].
pub trait ImmPlumeMenu {
    /// `Some` while this menu button's popup is open; the inner value is where
    /// keyboard focus should land in it (`None` leaves focus alone).
    fn menu_open(&self) -> Option<Option<NavAction>>;
}

impl<Cap> ImmPlumeMenu for ImmEntity<'_, '_, '_, Cap>
where
    Cap: ImplCap<CapabilityPlumeMenu>,
{
    fn menu_open(&self) -> Option<Option<NavAction>> {
        match self.cap_get_component::<MenuOpen>() {
            Ok(Some(open)) => Some(open.focus),
            _ => None,
        }
    }
}

/// Lets `tooltip_container` ask whether the tooltip state machine is showing for its
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
