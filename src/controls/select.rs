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
use bevy_ui::{ComputedNode, InteractionDisabled, Node, Selected, px, widget::Text};
use bevy_ui_widgets::{ListBox, SetSelected, ValueChange, listbox_update_selection};

use super::listview::{PlumeListRow, PlumeListView};
use super::menu::{PlumeMenu, PlumeMenuButton, PlumeMenuPopup};
use crate::{display::caption, rounded_corners::RoundedCorners};

const SELECT_ROW_PX: f32 = 28.0;

/// Select control which spawns a menu popup with a list of string options
/// # Emitted events
/// * [`ValueChange<Entity>`](bevy_ui_widgets::ValueChange) when the selected option is changed.
#[derive(SceneComponent, Default, Clone)]
#[scene(PlumeSelectProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
pub struct PlumeSelect;

/// Entirely optional component to store a usize on a `PlumeListRow`
/// Added by [`list_rows_from_strings`] so there's a value
/// on a string based select you can use to work out which of the array
/// of strings was selected
#[derive(Component, Default, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct OptionIndex(pub usize);

/// Convert an iterator of strings into `PlumeListRow` scenes with `OptionIndex`
/// on each one containing its index, optionally mark one selected
pub fn list_rows_from_strings(
    options: impl IntoIterator<Item: AsRef<str>>,
    selected: Option<usize>,
) -> Box<dyn SceneList> {
    Box::new(
        options
            .into_iter()
            .enumerate()
            .map(|(i, label)| -> Box<dyn SceneList> {
                let label: String = label.as_ref().into();
                if Some(i) == selected {
                    bsn! { @PlumeListRow Selected OptionIndex(i) Children [ caption(label) ] }
                        .into()
                } else {
                    bsn! { @PlumeListRow OptionIndex(i) Children [ caption(label) ] }.into()
                }
            })
            .collect::<Vec<_>>(),
    )
}

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
    q_text: Query<&Text, Without<SelectCaption>>,
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
                sync_select_disabled,
            ),
        )
        .add_observer(select_on_set_selected);
    }
}
