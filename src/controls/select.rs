//! Dropdown select control built on an internal menu popup and list view.
use bevy_app::{Plugin, Update};
use bevy_camera::visibility::Visibility;
use bevy_ecs::lifecycle::RemovedComponents;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{Added, Changed, With, Without},
    reflect::ReflectComponent,
    system::{Commands, Query, ResMut},
};
use bevy_input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::prelude::*;
use bevy_ui::{ComputedNode, InteractionDisabled, Node, Selected, Val, px, widget::Text};
use bevy_ui_widgets::{ListBox, SetSelected, ValueChange, listbox_update_selection};

use super::listview::{ListRowCheck, PlumeListRow, PlumeListView};
use super::menu::{PlumeMenu, PlumeMenuButton, PlumeMenuPopup};
use crate::display::caption;
use crate::rounded_corners::RoundedCorners;

const SELECT_ROW_PX: f32 = 24.0;

/// Select control which spawns a menu popup with a list of string options
/// # Emitted events
/// * [`ValueChange<Entity>`](bevy_ui_widgets::ValueChange) when the selected option is changed.
#[derive(SceneComponent, Default, Clone)]
#[scene(PlumeSelectProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
pub struct PlumeSelect;

/// Marker for the caption which changes with selected item
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct SelectCaption;

/// Props for the control
pub struct PlumeSelectProps {
    /// String options
    pub options: Box<dyn SceneList>,
    /// Corner roundedness
    pub corners: RoundedCorners,
    /// Maximum visible options before it scrolls
    pub max_visible: usize,
}

impl Default for PlumeSelectProps {
    fn default() -> Self {
        Self {
            options: Box::new(bsn_list!()),
            corners: Default::default(),
            max_visible: 8,
        }
    }
}

// Implements as a [`PlumeMenu`] under the hood with a row per option
impl PlumeSelect {
    fn scene(props: PlumeSelectProps) -> impl Scene {
        let max_visible = props.max_visible.max(1);
        let max_height = px(max_visible as f32 * SELECT_ROW_PX);

        bsn! {
            @PlumeMenu
            PlumeSelect
            Children [
                (
                    @PlumeMenuButton {
                        @caption: bsn! { caption("") SelectCaption },
                        @corners: {props.corners},
                    }
                    Node {
                        flex_grow: 1.0,
                    }
                ),
                (
                    @PlumeMenuPopup
                    Children [
                        (
                            @PlumeListView {
                                @rows: {props.options}
                            }
                            on(listbox_update_selection)
                            on(re_emit_listbox_value)
                            Node {
                                max_height: {max_height},
                            }
                        )
                    ]
                )
            ]
        }
    }
}

fn re_emit_listbox_value(
    ev: On<ValueChange<Entity>>,
    q_select: Query<(), With<PlumeSelect>>,
    q_parents: Query<&ChildOf>,
    q_popup: Query<(), With<PlumeMenuPopup>>,
    mut commands: Commands,
) {
    let mut select_ent = None;
    let mut popup_ent = None;
    for ancestor in q_parents.iter_ancestors(ev.event_target()) {
        if q_select.contains(ancestor) {
            select_ent = Some(ancestor);
            break;
        }
        if q_popup.contains(ancestor) {
            popup_ent = Some(ancestor);
        }
    }

    if let Some(select_ent) = select_ent {
        commands.trigger(ValueChange {
            source: select_ent,
            value: ev.value,
            is_final: true,
        });
    };

    if let Some(popup_ent) = popup_ent {
        commands.entity(popup_ent).insert(Visibility::Hidden);
    }
}

fn select_on_set_selected(
    ev: On<SetSelected>,
    q_select: Query<(), With<PlumeSelect>>,
    q_listbox: Query<(), With<ListBox>>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    if !q_select.contains(ev.entity) {
        return;
    }
    if let Some(listbox) = q_children
        .iter_descendants(ev.entity)
        .find(|descendant| q_listbox.contains(*descendant))
    {
        commands.trigger(SetSelected {
            entity: listbox,
            row: ev.row,
        });
    }
}

fn sync_caption(
    q_newly_selected: Query<Entity, (Added<Selected>, With<PlumeListRow>)>,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    q_text: Query<&Text, (Without<SelectCaption>, Without<ListRowCheck>)>,
    q_select: Query<(), With<PlumeSelect>>,
    mut q_caption: Query<&mut Text, With<SelectCaption>>,
) {
    for row in q_newly_selected.iter() {
        let Some(text) = q_children
            .iter_descendants(row)
            .find_map(|descendant| q_text.get(descendant).ok())
            .map(|text| text.0.clone())
        else {
            continue;
        };

        let Some(select_ent) = q_parents
            .iter_ancestors(row)
            .find(|&ancestor| q_select.contains(ancestor))
        else {
            continue;
        };

        for descendant in q_children.iter_descendants(select_ent) {
            if let Ok(mut caption) = q_caption.get_mut(descendant) {
                if caption.0 != text {
                    caption.0 = text.clone();
                }
                break;
            }
        }
    }
}

fn focus_select_popup(
    q_popups: Query<(Entity, &Visibility), (With<PlumeMenuPopup>, Changed<Visibility>)>,
    q_select: Query<(), With<PlumeSelect>>,
    q_listbox: Query<(), With<ListBox>>,
    q_button: Query<(), With<PlumeMenuButton>>,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    mut focus: ResMut<InputFocus>,
    mut focus_visible: ResMut<InputFocusVisible>,
) {
    for (popup, visibility) in q_popups.iter() {
        let mut select_ent = None;
        for ancestor in q_parents.iter_ancestors(popup) {
            if q_select.contains(ancestor) {
                select_ent = Some(ancestor);
                break;
            }
        }
        let Some(select_ent) = select_ent else {
            continue;
        };

        if *visibility == Visibility::Visible {
            for descendant in q_children.iter_descendants(popup) {
                if q_listbox.contains(descendant) {
                    focus.set(descendant, FocusCause::Navigated);
                    focus_visible.0 = true;
                    break;
                }
            }
        } else {
            let focus_in_select = focus.get().is_some_and(|focused| {
                focused == select_ent || q_parents.iter_ancestors(focused).any(|a| a == select_ent)
            });
            if focus_in_select {
                for descendant in q_children.iter_descendants(select_ent) {
                    if q_button.contains(descendant) {
                        focus.set(descendant, FocusCause::Navigated);
                        break;
                    }
                }
            }
        }
    }
}

// The headless `MenuButton` checks `InteractionDisabled` on itself, so the marker on the
// select root must be mirrored onto the internal menu button (which also restyles it).
fn sync_select_disabled(
    q_newly_disabled: Query<Entity, (With<PlumeSelect>, Added<InteractionDisabled>)>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    q_select: Query<(), With<PlumeSelect>>,
    q_children: Query<&Children>,
    q_button: Query<(), With<PlumeMenuButton>>,
    q_popup: Query<(), With<PlumeMenuPopup>>,
    mut commands: Commands,
) {
    for select_ent in q_newly_disabled.iter() {
        for descendant in q_children.iter_descendants(select_ent) {
            if q_button.contains(descendant) {
                commands.entity(descendant).insert(InteractionDisabled);
            } else if q_popup.contains(descendant) {
                commands.entity(descendant).insert(Visibility::Hidden);
            }
        }
    }
    removed_disabled.read().for_each(|ent| {
        if q_select.contains(ent) {
            for descendant in q_children.iter_descendants(ent) {
                if q_button.contains(descendant) {
                    commands.entity(descendant).remove::<InteractionDisabled>();
                }
            }
        }
    });
}

fn sync_select_width(
    q_selects: Query<(Entity, &ComputedNode), With<PlumeSelect>>,
    q_children: Query<&Children>,
    q_popup: Query<(), With<PlumeMenuPopup>>,
    mut q_node: Query<&mut Node>,
) {
    for (select_ent, computed) in q_selects.iter() {
        let width = (computed.size().x * computed.inverse_scale_factor()).round();
        if width <= 0.0 {
            continue;
        }
        for descendant in q_children.iter_descendants(select_ent) {
            if q_popup.contains(descendant) {
                if let Ok(mut node) = q_node.get_mut(descendant) {
                    let target = px(width);
                    if node.min_width != target {
                        node.min_width = target;
                    }
                }
                break;
            }
        }
    }
}

// Match the menu button's width to the popup (widest row + popup chrome) and pin the caption
// to the widest option, so picking a different option never resizes the control. Rows are
// stretched to the popup, so their natural width is re-summed from their children.
fn sync_select_button_width(
    q_selects: Query<Entity, With<PlumeSelect>>,
    q_children: Query<&Children>,
    q_rows: Query<(&Node, &Children), With<PlumeListRow>>,
    q_buttons: Query<&Node, With<PlumeMenuButton>>,
    q_captions: Query<&Node, With<SelectCaption>>,
    q_listboxes: Query<&Node, With<ListBox>>,
    q_popups: Query<&Node, With<PlumeMenuPopup>>,
    q_check: Query<(), With<ListRowCheck>>,
    q_computed: Query<&ComputedNode>,
    mut commands: Commands,
) {
    fn val_px(val: Val) -> f32 {
        match val {
            Val::Px(v) => v,
            _ => 0.0,
        }
    }
    let width_of = |entity: Entity| {
        q_computed
            .get(entity)
            .map(|computed| computed.size().x * computed.inverse_scale_factor())
            .ok()
    };
    for select_ent in q_selects.iter() {
        let mut widest_row = 0.0f32;
        let mut widest_option = 0.0f32;
        let mut popup_chrome = 0.0f32;
        for descendant in q_children.iter_descendants(select_ent) {
            if let Ok(popup_node) = q_popups.get(descendant) {
                popup_chrome += val_px(popup_node.border.left)
                    + val_px(popup_node.border.right)
                    + val_px(popup_node.padding.left)
                    + val_px(popup_node.padding.right);
            }
            if let Ok(listbox_node) = q_listboxes.get(descendant) {
                popup_chrome += val_px(listbox_node.padding.right);
            }
            let Ok((row_node, row_children)) = q_rows.get(descendant) else {
                continue;
            };
            let gap = val_px(row_node.column_gap);
            // The check tick stays in the popup, so it counts toward the row but not the caption.
            let (mut row_width, mut option_width) = (0.0f32, 0.0f32);
            for &child in row_children.iter() {
                let Some(width) = width_of(child) else {
                    continue;
                };
                row_width += width + if row_width > 0.0 { gap } else { 0.0 };
                if !q_check.contains(child) {
                    option_width += width + if option_width > 0.0 { gap } else { 0.0 };
                }
            }
            if row_width > 0.0 {
                let padding = val_px(row_node.padding.left) + val_px(row_node.padding.right);
                widest_row = widest_row.max(row_width + padding);
                widest_option = widest_option.max(option_width);
            }
        }
        if widest_row <= 0.0 {
            continue;
        }
        let button_target = px((widest_row + popup_chrome).ceil());
        let caption_target = px(widest_option.ceil());
        for descendant in q_children.iter_descendants(select_ent) {
            if let Ok(button_node) = q_buttons.get(descendant)
                && button_node.width != button_target
            {
                let mut node = button_node.clone();
                node.width = button_target;
                commands.entity(descendant).insert(node);
            }
            if let Ok(caption_node) = q_captions.get(descendant)
                && caption_node.width != caption_target
            {
                let mut node = caption_node.clone();
                node.width = caption_target;
                commands.entity(descendant).insert(node);
            }
        }
    }
}

/// Plugin which runs the [`PlumeSelect`] control
pub struct SelectPlugin;

impl Plugin for SelectPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            Update,
            (
                sync_caption,
                focus_select_popup,
                sync_select_width,
                sync_select_button_width,
                sync_select_disabled,
            ),
        )
        .add_observer(select_on_set_selected);
    }
}
