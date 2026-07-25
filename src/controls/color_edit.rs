//! Editable colour swatch: a swatch that opens a [`PlumeColorPicker`] in a popup,
//! auto-positioned below/above the swatch and dismissed by clicking outside.
//!
//! The popup borrows bevy's [`Popover`] positioner but *not* the menu focus
//! machinery: a menu popup closes whenever focus leaves its subtree (clicking a
//! non-focusable pad would dismiss it), so this manages open/close itself — a press
//! anywhere outside the control hides the popup, presses inside it don't.
//!
//! Its colour is the public [`ColorPickerValue`] on the root, mirrored to and from
//! the inner picker, so the existing colour capability drives it through the imm
//! layer with no extra work.
use bevy::app::{Plugin, PostUpdate};
use bevy::camera::visibility::Visibility;
use bevy::color::Color;
use bevy::ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{Changed, Has, With, Without},
    reflect::ReflectComponent,
    system::{Commands, Query},
};
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::picking::events::{Drag, Pointer, Press};
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::scene::prelude::*;
use bevy::text::FontWeight;
use bevy::ui::{
    AlignItems, Display, FlexDirection, GlobalZIndex, Node, OverrideClip, PositionType, UiRect,
    UiTransform, Val, Val2, px,
};
use bevy::ui_widgets::popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide};

use crate::constants::{font_awesome, fonts, size};
use crate::containers::{popup_socket, row};
use crate::controls::{ColorPickerValue, ColorSwatchValue, PlumeColorPicker, PlumeColorSwatch};
use crate::cursor::EntityCursor;
use crate::display::{caption_small_caps, fa_icon};
use crate::font_styles::InheritableFont;
use crate::theme::{
    InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor,
    control_box_shadow,
};
use crate::tokens;

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

// The popup's auto-placement: beside the swatch, flipping to whichever side has
// room. Re-applied when the popup reopens so a dragged panel re-anchors.
fn picker_popover() -> Popover {
    let placement = |side| PopoverPlacement {
        side,
        align: PopoverAlign::Center,
        gap: 8.0,
    };
    Popover {
        positions: vec![
            placement(PopoverSide::Right),
            placement(PopoverSide::Left),
            placement(PopoverSide::Top),
            placement(PopoverSide::Bottom),
        ],
        window_margin: 10.0,
    }
}

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
                // The picker popup: absolutely positioned by `Popover` relative to
                // this control, above everything, hidden until the swatch is clicked.
                (
                    popup_socket()
                    Children [(
                        Node {
                            position_type: PositionType::Absolute,
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            border: size::CONTAINER_BORDER,
                            padding: size::GAP_TIGHT,
                            border_radius: size::CORNER_RADIUS,
                        }
                        ColorEditPopup
                        Visibility::Hidden
                        ThemeBackgroundColor(tokens::MENU_BG)
                        ThemeBorderColor(tokens::MENU_BORDER)
                        template_value(control_box_shadow())
                        TabGroup::new(0)
                        GlobalZIndex(100)
                        template_value(picker_popover())
                        // Drag any non-control part of the panel to move it. Inner controls
                        // consume their own drags, so only background drags reach this.
                        on(on_popup_drag)
                        OverrideClip
                        InheritableThemeTextColor(tokens::TEXT_DIM)
                        InheritableFont {
                            font: fonts::REGULAR,
                            font_size: size::MEDIUM_FONT,
                            weight: FontWeight::NORMAL,
                        }
                        Children [
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
                                    @initial_color: {props.initial_color},
                                }
                                ColorEditPicker
                            )
                        ]
                    )]
                )
            ]
        }
    }
}

// A press on the swatch toggles its popup open/closed.
fn on_swatch_click(
    mut click: On<Pointer<Press>>,
    q_childof: Query<&ChildOf>,
    q_is_swatch: Query<(), With<ColorEditSwatch>>,
    q_is_edit: Query<(), With<PlumeColorEdit>>,
    q_children: Query<&Children>,
    q_popup_marker: Query<(), With<ColorEditPopup>>,
    mut q_vis: Query<&mut Visibility, With<ColorEditPopup>>,
) {
    // Only react to presses landing on a swatch (its border-overlay child included).
    if nearest_with(click.entity, &q_childof, &q_is_swatch).is_none() {
        return;
    }
    let Some(root) = nearest_with(click.entity, &q_childof, &q_is_edit) else {
        return;
    };
    let Some(popup) = descendant(root, &q_children, &q_popup_marker) else {
        return;
    };
    if let Ok(mut visibility) = q_vis.get_mut(popup) {
        *visibility = match *visibility {
            Visibility::Visible => Visibility::Hidden,
            _ => Visibility::Visible,
        };
        click.propagate(false);
    }
}

// A press anywhere outside an open control closes its popup; presses on the swatch
// or inside the popup (both under the `PlumeColorEdit` root) leave it open.
fn on_dismiss_press(
    mut click: On<Pointer<Press>>,
    q_childof: Query<&ChildOf>,
    q_is_edit: Query<(), With<PlumeColorEdit>>,
    mut q_popups: Query<(Entity, &mut Visibility), With<ColorEditPopup>>,
) {
    for (popup, mut visibility) in q_popups.iter_mut() {
        if *visibility != Visibility::Visible {
            continue;
        }
        let Some(root) = nearest_with(popup, &q_childof, &q_is_edit) else {
            continue;
        };
        let inside = click.entity == root
            || q_childof
                .iter_ancestors(click.entity)
                .any(|ancestor| ancestor == root);
        if !inside {
            *visibility = Visibility::Hidden;
        }
        click.propagate(false);
    }
}

// Drag a non-control part of the panel to move it. This is an entity observer on
// the popup, so it only fires for drags that bubbled up unconsumed — i.e. not on a
// pad/number/button, which stop their own drags. The first drag drops `Popover` so
// the manual position stops fighting the auto-placement; reopening restores it.
fn on_popup_drag(
    drag: On<Pointer<Drag>>,
    q_childof: Query<&ChildOf>,
    q_is_popup: Query<(), With<ColorEditPopup>>,
    mut q_popup: Query<(&mut UiTransform, Has<Popover>), With<ColorEditPopup>>,
    mut commands: Commands,
) {
    let Some(popup) = nearest_with(drag.entity, &q_childof, &q_is_popup) else {
        return;
    };
    let Ok((mut transform, has_popover)) = q_popup.get_mut(popup) else {
        return;
    };
    if has_popover {
        commands.entity(popup).remove::<Popover>();
    }
    let px_of = |value: Val| match value {
        Val::Px(px) => px,
        _ => 0.0,
    };
    transform.translation = Val2::px(
        px_of(transform.translation.x) + drag.delta.x,
        px_of(transform.translation.y) + drag.delta.y,
    );
}

// When a dragged-away popup reopens, restore `Popover` so it re-anchors to the swatch.
fn reanchor_on_open(
    q_opened: Query<(Entity, &Visibility), (With<ColorEditPopup>, Changed<Visibility>)>,
    q_has_popover: Query<(), With<Popover>>,
    mut commands: Commands,
) {
    for (popup, visibility) in q_opened.iter() {
        if *visibility == Visibility::Visible && !q_has_popover.contains(popup) {
            commands.entity(popup).insert(picker_popover());
        }
    }
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

// `entity` itself if it carries marker `M`, else the nearest such ancestor.
fn nearest_with<M: Component>(
    entity: Entity,
    q_childof: &Query<&ChildOf>,
    q_marker: &Query<(), With<M>>,
) -> Option<Entity> {
    if q_marker.contains(entity) {
        return Some(entity);
    }
    q_childof
        .iter_ancestors(entity)
        .find(|ancestor| q_marker.contains(*ancestor))
}

// Find the first marked descendant of `root`.
fn descendant<M: Component>(
    root: Entity,
    q_children: &Query<&Children>,
    q_marker: &Query<(), With<M>>,
) -> Option<Entity> {
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if entity != root && q_marker.contains(entity) {
            return Some(entity);
        }
        if let Ok(children) = q_children.get(entity) {
            stack.extend(children.iter().copied());
        }
    }
    None
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

/// Registers the colour-edit open/dismiss observers and value-mirror systems.
pub struct ColorEditPlugin;

impl Plugin for ColorEditPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_observer(on_swatch_click)
            .add_observer(on_dismiss_press)
            .add_systems(
                PostUpdate,
                (reanchor_on_open, sync_edit_from_picker, sync_edit_to_picker),
            );
    }
}
