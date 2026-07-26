//! Editable colour swatch: a swatch that opens a [`PlumeColorPicker`] in a
//! movable [`PlumePopup`], dismissed by pressing outside the control.
//!
//! Its colour is the public [`ColorPickerValue`] on the root, mirrored to and from
//! the inner picker, so the existing colour capability drives it through the imm
//! layer with no extra work.
use bevy::app::{Plugin, PostUpdate};
use bevy::color::Color;
use bevy::ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    lifecycle::Add,
    observer::On,
    query::{Changed, With, Without},
    reflect::ReflectComponent,
    system::{Commands, Query},
};
use bevy::picking::events::{Pointer, Press};
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::scene::prelude::*;
use bevy::ui::{AlignItems, Node, UiRect, Val, px};

use crate::constants::{font_awesome, size};
use crate::containers::{
    CloseRequested, PlumePopup, PopupDismiss, PopupPlacement, PopupSocket, close_popup,
    popup_socket, row,
};
use crate::controls::{ColorPickerValue, ColorSwatchValue, PlumeColorPicker, PlumeColorSwatch};
use crate::cursor::EntityCursor;
use crate::display::{caption_small_caps, fa_icon};
use crate::theme::ThemeTextColor;
use crate::tokens;
use crate::utils::hierarchy::{descendant, nearest_with};

// Two colours this close (per linear channel) are treated as equal, so a mirror
// push that merely echoes the current value doesn't ping-pong across the pair.
const EPS: f32 = 1.0e-6;

#[derive(Default, Clone)]
pub struct PlumeColorEditProps {
    pub initial_color: Color,
}

/// An editable colour swatch: click to open a colour-picker popup. Spawnable as a
/// scene component; reports its colour in [`ColorPickerValue`] on the root.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeColorEditProps)]
#[reflect(Component, Clone, Default)]
#[require(ColorPickerValue)]
pub struct PlumeColorEdit;

/// Marks the swatch shown on the closed control (also the click target that opens it).
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorEditSwatch;

/// Marks the popup whose visibility is toggled open/closed.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorEditPopup;

/// Marks the picker inside the popup.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ColorEditPicker;

impl PlumeColorEdit {
    fn scene(props: PlumeColorEditProps) -> impl Scene {
        bsn! {
            Node {
                align_items: AlignItems::Start,
            }
            PlumeColorEdit
            template_value(ColorPickerValue(props.initial_color))
            Children [
                // The swatch is the click target that toggles the popup.
                (
                    @PlumeColorSwatch
                    template_value(ColorSwatchValue(props.initial_color))
                    ColorEditSwatch
                    EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)
                ),
                // The picker popup spawns into this socket while open.
                (
                    popup_socket()
                )
            ]
        }
    }
}

// A press on the swatch toggles its popup: despawn if open, else spawn a fresh
// picker (movable, dismissed by pressing outside) seeded with the current colour.
#[allow(clippy::too_many_arguments)]
fn on_swatch_click(
    mut click: On<Pointer<Press>>,
    q_childof: Query<&ChildOf>,
    q_is_swatch: Query<(), With<ColorEditSwatch>>,
    q_is_edit: Query<(), With<PlumeColorEdit>>,
    q_children: Query<&Children>,
    q_popup_marker: Query<(), With<ColorEditPopup>>,
    q_socket: Query<(), With<PopupSocket>>,
    q_value: Query<&ColorPickerValue, With<PlumeColorEdit>>,
    mut commands: Commands,
) {
    // Only react to presses landing on a swatch (its border-overlay child included).
    if nearest_with(click.entity, &q_childof, &q_is_swatch).is_none() {
        return;
    }
    let Some(root) = nearest_with(click.entity, &q_childof, &q_is_edit) else {
        return;
    };
    click.propagate(false);
    if let Some(popup) = descendant(root, &q_children, &q_popup_marker) {
        close_popup(&mut commands, popup);
        return;
    }
    let Some(socket) = descendant(root, &q_children, &q_socket) else {
        return;
    };
    let color = q_value.get(root).map(|value| value.0).unwrap_or_default();
    commands
        .spawn_scene(bsn! {
            @PlumePopup {
                @placement: {PopupPlacement::Beside(socket)},
                @dismiss: PopupDismiss::OutsideClick,
                @movable: true,
                @contents: bsn_list!(
                    (
                        row() Node { padding: {UiRect::new(size::PAD, size::PAD, Val::ZERO, size::PAD)} } Children [
                            (
                                fa_icon(font_awesome::solid::PALETTE)
                            ),
                            (
                                caption_small_caps("Color Edit")
                                Node { width: px(100) }
                                ThemeTextColor(tokens::TEXT_MAIN)
                            )
                        ]
                    ),
                    (
                        @PlumeColorPicker {
                            @initial_color: {color},
                        }
                        ColorEditPicker
                    )
                ),
            }
            ColorEditPopup
        })
        .insert(ChildOf(socket));
}

// User edits inside the popup: carry the picker's colour up to the public value on
// the root and onto the closed-control swatch.
fn sync_edit_from_picker(
    q_picker: Query<
        (Entity, &ColorPickerValue),
        (With<ColorEditPicker>, Changed<ColorPickerValue>),
    >,
    q_childof: Query<&ChildOf>,
    q_is_edit: Query<(), With<PlumeColorEdit>>,
    q_children: Query<&Children>,
    q_swatch_marker: Query<(), With<ColorEditSwatch>>,
    mut q_root_value: Query<&mut ColorPickerValue, Without<ColorEditPicker>>,
    mut q_swatch: Query<&mut ColorSwatchValue>,
) {
    for (picker, value) in q_picker.iter() {
        let Some(root) = nearest_with(picker, &q_childof, &q_is_edit) else {
            continue;
        };
        if let Ok(mut root_value) = q_root_value.get_mut(root)
            && !colors_close(root_value.0, value.0)
        {
            root_value.0 = value.0;
        }
        set_swatch(
            root,
            value.0,
            &q_children,
            &q_swatch_marker,
            &mut q_swatch,
            true,
        );
    }
}

// An external or imm-driven write to the public value: push it into the picker and
// onto the swatch.
fn sync_edit_to_picker(
    q_edit: Query<(Entity, &ColorPickerValue), (With<PlumeColorEdit>, Changed<ColorPickerValue>)>,
    q_children: Query<&Children>,
    q_picker_marker: Query<(), With<ColorEditPicker>>,
    q_swatch_marker: Query<(), With<ColorEditSwatch>>,
    mut q_picker_value: Query<
        &mut ColorPickerValue,
        (With<ColorEditPicker>, Without<PlumeColorEdit>),
    >,
    mut q_swatch: Query<&mut ColorSwatchValue>,
) {
    for (root, value) in q_edit.iter() {
        if let Some(picker) = descendant(root, &q_children, &q_picker_marker)
            && let Ok(mut picker_value) = q_picker_value.get_mut(picker)
            && !colors_close(picker_value.0, value.0)
        {
            picker_value.0 = value.0;
        }
        set_swatch(
            root,
            value.0,
            &q_children,
            &q_swatch_marker,
            &mut q_swatch,
            false,
        );
    }
}

// Update the closed-control swatch to `color`, if it moved.
fn set_swatch(
    root: Entity,
    color: Color,
    q_children: &Query<&Children>,
    q_swatch_marker: &Query<(), With<ColorEditSwatch>>,
    q_swatch: &mut Query<&mut ColorSwatchValue>,
    sync_edit_from_picker: bool,
) {
    if let Some(swatch) = descendant(root, q_children, q_swatch_marker)
        && let Ok(mut value) = q_swatch.get_mut(swatch)
        && !colors_close(value.0, color)
    {
        println!(
            "set_swatch {color:?} {}",
            if sync_edit_from_picker {
                "sync_edit_from_picker"
            } else {
                "sync_edit_to_picker"
            }
        );
        value.0 = color;
    }
}

// Equal to within `EPS` per linear channel, so any two `Color` variants of the same
// colour compare equal and mirror pushes don't churn on representation.
fn colors_close(a: Color, b: Color) -> bool {
    let (a, b) = (a.to_linear(), b.to_linear());
    (a.red - b.red).abs() <= EPS
        && (a.green - b.green).abs() <= EPS
        && (a.blue - b.blue).abs() <= EPS
        && (a.alpha - b.alpha).abs() <= EPS
}

// Fulfil an outside-press close request on this control's popup.
fn on_popup_close_requested(
    ev: On<Add, CloseRequested>,
    q_popup: Query<(), With<ColorEditPopup>>,
    mut commands: Commands,
) {
    if q_popup.contains(ev.entity) {
        close_popup(&mut commands, ev.entity);
    }
}

/// Registers the colour-edit open observer and value-mirror systems.
pub struct ColorEditPlugin;

impl Plugin for ColorEditPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_observer(on_swatch_click)
            .add_observer(on_popup_close_requested)
            .add_systems(PostUpdate, (sync_edit_from_picker, sync_edit_to_picker));
    }
}
