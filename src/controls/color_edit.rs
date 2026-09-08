//! Editable color: a select-style button (swatch plus arrow) that opens a
//! [`PlumeColorPicker`] in a movable [`PlumePopup`], dismissed by pressing outside.
//!
//! Its color is the public [`ColorPickerValue`] on the root, mirrored to and from
//! the inner picker, so the existing color capability drives it through the imm
//! layer with no extra work.
use bevy::app::{Plugin, PostUpdate, Update};
use bevy::color::Color;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::lifecycle::Add;
use bevy::ecs::observer::On;
use bevy::ecs::query::{Changed, Has, With, Without};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::system::{Commands, Query};
use bevy::picking::events::PointerPress;
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::ui::{AlignItems, InteractionDisabled, Node, Val};
use bevy::ui_widgets::{Activate, ActivateOnPress, ValueChange};

use crate::constants::{lucide, size};
use crate::containers::{
    CloseRequested, PlumePopup, PopupDismiss, PopupPlacement, PopupSocket, close_popup,
    popup_socket,
};
use crate::controls::{
    ColorPickerValue, ColorSwatchValue, PlumeButton, PlumeColorPicker, PlumeColorSwatch,
};
use crate::display::icon;
use crate::font_styles::{InheritableFont, PlumeFontSize};
use crate::utils::hierarchy::{descendant_get, descendant_with, nearest_get, nearest_with};

// Two colors this close (per linear channel) are treated as equal, so a mirror
// push that merely echoes the current value doesn't ping-pong across the pair.
const EPS: f32 = 1.0e-6;

// Swatch side inside the button: a little under the row height so the button
// chrome stays visible around it.
const SWATCH_SIZE: Val = size::em_from_px(16.0);

/// Scene props for [`PlumeColorEdit`].
#[derive(Clone)]
pub struct PlumeColorEditProps {
    /// Color the swatch shows before the user edits it.
    pub initial_color: Color,
    /// Offer alpha: the swatch reads translucency and the picker popup edits
    /// it. `false` shows and edits RGB only, leaving the alpha as it arrived.
    pub alpha: bool,
}

impl Default for PlumeColorEditProps {
    fn default() -> Self {
        PlumeColorEditProps {
            initial_color: Color::default(),
            alpha: true,
        }
    }
}

/// An editable color: a select-style button opening a color-picker popup. Spawnable as a
/// scene component; reports its color in [`ColorPickerValue`] on the root.
/// # Emitted events
/// * [`ValueChange<Color>`](bevy::ui_widgets::ValueChange) on each user edit.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeColorEditProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeColorEdit;

// Plain root marker, inserted on both the retained and imm paths. The systems key on
// this — and it carries the control's requirements — rather than the
// [`PlumeColorEdit`] scene component, which only the retained path inserts.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
#[require(ColorPickerValue)]
struct ColorEditFrame;

// Marks the swatch inside the button, mirroring the current color.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorEditSwatch;

// Marks the select-style button that toggles the picker popup.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorEditButton;

// Marks the popup whose visibility is toggled open/closed.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorEditPopup;

// Marks a control that edits RGB only, so its popup spawns the picker without alpha.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorEditRgbOnly;

// Marks the picker inside the popup.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorEditPicker;

impl PlumeColorEdit {
    fn scene(props: PlumeColorEditProps) -> impl Scene {
        let PlumeColorEditProps {
            initial_color,
            alpha,
        } = props;
        let rgb_only = (!alpha).then(|| bsn! { ColorEditRgbOnly });
        bsn! {
            Node {
                align_items: AlignItems::Start,
            }
            ColorEditFrame
            ColorPickerValue({initial_color})
            @rgb_only
            Children [
                // The button is the click target that toggles the popup.
                @PlumeButton {
                    @caption: bsn! {
                        @PlumeColorSwatch {
                            @initial_color: initial_color,
                            @alpha: alpha,
                        }
                        ColorEditSwatch
                        Node {
                            width: SWATCH_SIZE,
                            height: SWATCH_SIZE,
                            flex_grow: 1.0,
                        }
                        --
                        @icon(lucide::CHEVRON_DOWN)
                    },
                }
                ActivateOnPress
                ColorEditButton
                Node {
                    flex_grow: 1.0,
                }
                --
                // The picker popup spawns into this socket while open.
                @popup_socket()
            ]
        }
    }
}

// Activating the button toggles its popup. Pointer presses on the button node
// itself arrive here through `ActivateOnPress` (matching the select), keyboard
// activation through the button's key handling.
#[allow(clippy::too_many_arguments)]
fn on_button_activate(
    ev: On<Activate>,
    q_childof: Query<&ChildOf>,
    q_is_button: Query<(), With<ColorEditButton>>,
    q_edit: Query<(Entity, Has<InteractionDisabled>), With<ColorEditFrame>>,
    q_children: Query<&Children>,
    q_popup_marker: Query<(), With<ColorEditPopup>>,
    q_socket: Query<(), With<PopupSocket>>,
    q_value: Query<&ColorPickerValue, With<ColorEditFrame>>,
    q_rgb_only: Query<(), With<ColorEditRgbOnly>>,
    mut commands: Commands,
) {
    if !q_is_button.contains(ev.entity) {
        return;
    }
    let Some((root, disabled)) = nearest_get(ev.entity, &q_childof, &q_edit) else {
        return;
    };
    if disabled {
        return;
    }
    toggle_popup(
        root,
        &q_children,
        &q_popup_marker,
        &q_socket,
        &q_value,
        &q_rgb_only,
        &mut commands,
    );
}

// A press on the button's children, handled at the press's original target: when
// another popup's outside-press dismiss swallows the press there, it never
// bubbles to the `Button`, so the `Activate` path alone would cost a second
// click. A press on the button node itself is left to that path, whose observers
// co-fire at this hop regardless of the swallow.
#[allow(clippy::too_many_arguments)]
fn on_button_press(
    mut press: On<PointerPress>,
    q_childof: Query<&ChildOf>,
    q_is_button: Query<(), With<ColorEditButton>>,
    q_edit: Query<(Entity, Has<InteractionDisabled>), With<ColorEditFrame>>,
    q_children: Query<&Children>,
    q_popup_marker: Query<(), With<ColorEditPopup>>,
    q_socket: Query<(), With<PopupSocket>>,
    q_value: Query<&ColorPickerValue, With<ColorEditFrame>>,
    q_rgb_only: Query<(), With<ColorEditRgbOnly>>,
    mut commands: Commands,
) {
    if press.entity != press.original_event_target() {
        return;
    }
    let Some(button) = nearest_with(press.entity, &q_childof, &q_is_button) else {
        return;
    };
    if button == press.entity {
        return;
    }
    let Some((root, disabled)) = nearest_get(button, &q_childof, &q_edit) else {
        return;
    };
    // This path bypasses the button, so the disabled check has to happen here too.
    if disabled {
        return;
    }
    // Swallowed so the press cannot also reach the button and `Activate`.
    press.propagate(false);
    toggle_popup(
        root,
        &q_children,
        &q_popup_marker,
        &q_socket,
        &q_value,
        &q_rgb_only,
        &mut commands,
    );
}

// Toggle `root`'s picker popup: despawn if open, else spawn a fresh picker
// (movable, dismissed by pressing outside) seeded with the current color.
fn toggle_popup(
    root: Entity,
    q_children: &Query<&Children>,
    q_popup_marker: &Query<(), With<ColorEditPopup>>,
    q_socket: &Query<(), With<PopupSocket>>,
    q_value: &Query<&ColorPickerValue, With<ColorEditFrame>>,
    q_rgb_only: &Query<(), With<ColorEditRgbOnly>>,
    commands: &mut Commands,
) {
    if let Some(popup) = descendant_with(root, q_children, q_popup_marker) {
        close_popup(commands, popup);
        return;
    }
    let Some(socket) = descendant_with(root, q_children, q_socket) else {
        return;
    };
    let color = q_value.get(root).map(|value| value.0).unwrap_or_default();
    let alpha = !q_rgb_only.contains(root);
    commands
        .spawn_scene(bsn! {
            @PlumePopup {
                @placement: PopupPlacement::Below,
                @dismiss: PopupDismiss::OutsideClick,
                @movable: true,
                @contents: bsn! {
                    @PlumeColorPicker {
                        @initial_color: color,
                        @alpha: alpha,
                    }
                    ColorEditPicker
                },
            }
            ColorEditPopup
            Node {
                padding: size::SPACE,
            }
            // Reset font size
            InheritableFont { font_size: {Some(PlumeFontSize::Rem(1.0))} }
        })
        .insert(ChildOf(socket));
}

// User edits inside the popup: carry the picker's color up to the public value on
// the root and onto the closed-control swatch.
fn sync_edit_from_picker(
    q_picker: Query<
        (Entity, &ColorPickerValue),
        (With<ColorEditPicker>, Changed<ColorPickerValue>),
    >,
    q_childof: Query<&ChildOf>,
    q_is_edit: Query<(), With<ColorEditFrame>>,
    q_children: Query<&Children>,
    q_swatch_marker: Query<(), With<ColorEditSwatch>>,
    mut q_root_value: Query<&mut ColorPickerValue, Without<ColorEditPicker>>,
    mut q_swatch: Query<&mut ColorSwatchValue>,
    mut commands: Commands,
) {
    for (picker, value) in q_picker.iter() {
        let Some(root) = nearest_with(picker, &q_childof, &q_is_edit) else {
            continue;
        };
        if let Ok(mut root_value) = q_root_value.get_mut(root)
            && !colors_close(root_value.0, value.0)
        {
            root_value.0 = value.0;
            commands.trigger(ValueChange {
                source: root,
                value: value.0,
                is_final: true,
            });
        }
        set_swatch(root, value.0, &q_children, &q_swatch_marker, &mut q_swatch);
    }
}

// An external or imm-driven write to the public value: push it into the picker and
// onto the swatch.
fn sync_edit_to_picker(
    q_edit: Query<(Entity, &ColorPickerValue), (With<ColorEditFrame>, Changed<ColorPickerValue>)>,
    q_children: Query<&Children>,
    q_picker_marker: Query<(), With<ColorEditPicker>>,
    q_swatch_marker: Query<(), With<ColorEditSwatch>>,
    mut q_picker_value: Query<
        &mut ColorPickerValue,
        (With<ColorEditPicker>, Without<ColorEditFrame>),
    >,
    mut q_swatch: Query<&mut ColorSwatchValue>,
) {
    for (root, value) in q_edit.iter() {
        if let Some(picker) = descendant_with(root, &q_children, &q_picker_marker)
            && let Ok(mut picker_value) = q_picker_value.get_mut(picker)
            && !colors_close(picker_value.0, value.0)
        {
            picker_value.0 = value.0;
        }
        set_swatch(root, value.0, &q_children, &q_swatch_marker, &mut q_swatch);
    }
}

// Update the closed-control swatch to `color`, if it moved.
fn set_swatch(
    root: Entity,
    color: Color,
    q_children: &Query<&Children>,
    q_swatch_marker: &Query<(), With<ColorEditSwatch>>,
    q_swatch: &mut Query<&mut ColorSwatchValue>,
) {
    if let Some(swatch) = descendant_with(root, q_children, q_swatch_marker)
        && let Ok(mut value) = q_swatch.get_mut(swatch)
        && !colors_close(value.0, color)
    {
        value.0 = color;
    }
}

// Equal to within `EPS` per linear channel, so any two `Color` variants of the same
// color compare equal and mirror pushes don't churn on representation.
fn colors_close(a: Color, b: Color) -> bool {
    let (a, b) = (a.to_linear(), b.to_linear());
    (a.red - b.red).abs() <= EPS
        && (a.green - b.green).abs() <= EPS
        && (a.blue - b.blue).abs() <= EPS
        && (a.alpha - b.alpha).abs() <= EPS
}

// The headless button reads `InteractionDisabled` on itself, so the marker on the
// root has to be mirrored onto the inner button — which also grays it. Compared
// each frame rather than driven by `Added`, since the marker can land on the root
// before the scene has spawned the button.
fn sync_disabled(
    q_edits: Query<(Entity, Has<InteractionDisabled>), With<ColorEditFrame>>,
    q_children: Query<&Children>,
    q_button: Query<(Entity, Has<InteractionDisabled>), With<ColorEditButton>>,
    q_popup_marker: Query<(), With<ColorEditPopup>>,
    mut commands: Commands,
) {
    for (root, disabled) in q_edits.iter() {
        let Some((button, button_disabled)) = descendant_get(root, &q_children, &q_button) else {
            continue;
        };
        if disabled != button_disabled {
            if disabled {
                commands.entity(button).insert(InteractionDisabled);
            } else {
                commands.entity(button).remove::<InteractionDisabled>();
            }
        }
        if disabled && let Some(popup) = descendant_with(root, &q_children, &q_popup_marker) {
            close_popup(&mut commands, popup);
        }
    }
}

// Fulfil an outside-press close request on this control's popup.
fn on_popup_close_requested(
    ev: On<Add<CloseRequested>>,
    q_popup: Query<(), With<ColorEditPopup>>,
    mut commands: Commands,
) {
    if q_popup.contains(ev.entity) {
        close_popup(&mut commands, ev.entity);
    }
}

// Registers the color-edit open observer and value-mirror systems.
pub(crate) struct ColorEditPlugin;

impl Plugin for ColorEditPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_observer(on_button_activate)
            .add_observer(on_button_press)
            .add_observer(on_popup_close_requested)
            .add_systems(Update, sync_disabled)
            .add_systems(PostUpdate, (sync_edit_from_picker, sync_edit_to_picker));
    }
}
