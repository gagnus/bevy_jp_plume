//! Dropdown select control over string options: a button plus a popup listbox
//! that only exists while open.
use accesskit::Role;
use bevy::a11y::AccessibilityNode;
use bevy::app::{Inherited, Plugin, PostUpdate, PreUpdate, Update};
use bevy::camera::visibility::Visibility;
use bevy::ecs::change_detection::DetectChanges;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EntityEvent;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::lifecycle::RemovedComponents;
use bevy::ecs::observer::On;
use bevy::ecs::query::{Added, Changed, Has, Or, With, Without};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs as _;
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::ecs::world::World;
use bevy::input_focus::tab_navigation::{NavAction, TabIndex};
use bevy::input_focus::{FocusCause, InputFocus, InputFocusVisible};
use bevy::log::warn;
use bevy::picking::hover::Hovered;
use bevy::picking::{Pickable, PickingSystems};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::text::{LineBreak, RemSize, TextFont, TextLayout};
use bevy::ui::widget::Text;
use bevy::ui::{
    AlignItems, BorderRadius, ComputedNode, Display, FlexDirection, InteractionDisabled,
    JustifyContent, Node, Overflow, PositionType, Selected, UiRect, Val, px,
};
use bevy::ui_widgets::{
    ActivateOnPress, ActiveDescendant, ControlOrientation, ListBox, ListItem, MenuAction,
    MenuButton, MenuEvent, MenuFocusState, ReselectListRow, ScrollArea, ValueChange,
    listbox_update_selection,
};

use crate::constants::{font_awesome, size};
use crate::containers::{
    PlumePopup, PopupDismiss, PopupPlacement, PopupSocket, close_popup, popup_socket,
};
use crate::controls::{ButtonVariant, PlumeButton, PlumeScrollbar, ScrollbarGutter, SetValue};
use crate::cursor::EntityCursor;
use crate::display::{caption, fa_icon};
use crate::font_styles::TextStyleRelay;
use crate::rounded_corners::RoundedCorners;
use crate::theme::{
    InheritableThemeTextToken, ThemeBackgroundToken, ThemeBorderToken, set_optional_background,
};
use crate::tokens;

/// Select control: a dropdown button over string options.
/// # Emitted events
/// * [`ValueChange<usize>`](bevy::ui_widgets::ValueChange) with the picked index.
#[derive(SceneComponent, Default, Clone)]
#[scene(PlumeSelectProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
pub struct PlumeSelect;

/// The selected option's index, held on the [`PlumeSelect`] root.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
#[reflect(Component)]
pub struct SelectedIndex(pub usize);

// Marker for the caption which changes with selected item
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct SelectCaption;

// Marker for the select's dropdown button (hosts the headless `MenuButton`).
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
struct PlumeSelectButton;

// The option labels, in popup order; the popup rows are rebuilt from these on
// every open.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct SelectOptions(Vec<(String, bool)>);

// Row cap before the popup scrolls; applied at popup spawn.
#[derive(Component, Default, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
struct SelectMaxVisible(usize);

/// Props for the control
pub struct PlumeSelectProps {
    /// Option labels in popup order, each with whether it can be picked. A
    /// disabled option still shows, grayed and inert.
    pub options: Vec<(String, bool)>,
    /// Index of the initially selected option.
    pub selected: usize,
    /// Corner roundedness
    pub corners: RoundedCorners,
    /// Maximum visible options before it scrolls
    pub max_visible: usize,
}

impl Default for PlumeSelectProps {
    fn default() -> Self {
        Self {
            options: Vec::new(),
            selected: 0,
            corners: Default::default(),
            max_visible: 8,
        }
    }
}

/// Collect option labels for [`PlumeSelectProps`]'s `options`, all enabled.
pub fn select_options(options: impl IntoIterator<Item: AsRef<str>>) -> Vec<(String, bool)> {
    options
        .into_iter()
        .map(|label| (label.as_ref().into(), true))
        .collect()
}

impl PlumeSelect {
    fn scene(props: PlumeSelectProps) -> impl Scene {
        let initial_caption = props
            .options
            .get(props.selected)
            .map(|(label, _)| label.clone())
            .unwrap_or_default();
        let ghost_rows: Vec<_> = props
            .options
            .iter()
            .map(|(label, _)| option_row(label.clone()))
            .collect();
        let options = props.options;
        let selected = props.selected;
        let max_visible = props.max_visible.max(1);

        bsn! {
            Node {
                height: size::ROW_HEIGHT,
                justify_content: JustifyContent::Stretch,
                align_items: AlignItems::Stretch,
            }
            PlumeSelect
            TextStyleRelay
            template_value(SelectOptions(options))
            template_value(SelectedIndex(selected))
            template_value(SelectMaxVisible(max_visible))
            on(on_menu_event)
            Children [
                (
                    @PlumeButton {
                        // The caption is pinned to the measured widest label, so a
                        // label with a space in it would wrap on a sub-pixel shortfall.
                        @caption: bsn! {
                            caption(initial_caption)
                            SelectCaption
                            TextLayout {
                                linebreak: LineBreak::NoWrap,
                            }
                        },
                        @variant: ButtonVariant::Normal,
                        @corners: {props.corners},
                    }
                    ActivateOnPress
                    MenuButton
                    PlumeSelectButton
                    Node {
                        flex_grow: 1.0,
                    }
                    Children [
                        Node {
                            flex_grow: 1.0,
                        },
                        fa_icon(font_awesome::solid::ANGLE_DOWN),
                    ]
                ),
                popup_socket(),
                // Ghost rows in a zero-size clipped overlay: laid out (so the labels
                // get real text measurement) without occupying space or taking picks;
                // despawned once `measure_select_width` has sized the button.
                (
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::ZERO,
                        height: Val::ZERO,
                        overflow: Overflow::clip(),
                    }
                    Pickable::IGNORE
                    Visibility::Hidden
                    SelectMeasure
                    // The ghost labels must inherit the real row font, or the
                    // measured widths bake in the engine default.
                    TextStyleRelay
                    Children [
                        {ghost_rows},
                    ]
                ),
            ]
        }
    }
}

// A single option row: check-tick gutter plus the label. `InteractionDisabled`
// is what `set_option_styles` grays and what makes the row refuse picks.
fn option_row(label: String) -> impl Scene {
    bsn! {
        @PlumeSelectOption
        Children [
            caption(label),
        ]
    }
}

// Popup rows for `options`, indexed, with the current pick marked.
fn option_rows(options: &[(String, bool)], selected: usize) -> Box<dyn SceneList> {
    Box::new(
        options
            .iter()
            .enumerate()
            .map(|(index, (label, enabled))| {
                let label = label.clone();
                let disabled = !*enabled;
                bsn! {
                    option_row(label)
                    SelectOptionIndex(index)
                    {disabled.then(|| bsn! { InteractionDisabled })}
                    {(index == selected).then(|| bsn! { Selected })}
                }
            })
            .collect::<Vec<_>>(),
    )
}

// Scrolling listbox holding the popup's option rows.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeSelectOptionsProps)]
#[reflect(Component, Clone, Default)]
struct PlumeSelectOptions;

// Props used to construct a [`PlumeSelectOptions`] scene.
struct PlumeSelectOptionsProps {
    // The list of option rows to display.
    options: Box<dyn SceneList>,
}

impl Default for PlumeSelectOptionsProps {
    fn default() -> Self {
        Self {
            options: Box::new(bsn_list![]),
        }
    }
}

impl PlumeSelectOptions {
    fn scene(props: PlumeSelectOptionsProps) -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Start,
            }
            template_value(ScrollbarGutter(size::SCROLLBAR_GUTTER.try_add(size::SPACE).unwrap()))
            ListBox
            TextStyleRelay
            // Focusable for arrow-key selection.
            TabIndex(0)
            AccessibilityNode(accesskit::Node::new(Role::ListBox))
            Children [
                (
                    #inner
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        justify_content: JustifyContent::Start,
                        overflow: Overflow::scroll_y(),
                    }
                    ScrollArea
                    TextStyleRelay
                    Children [
                        {props.options},
                    ]
                ),
                (
                    @PlumeScrollbar {
                        @target: #inner,
                        @orientation: ControlOrientation::Vertical,
                    }
                    Node {
                        position_type: PositionType::Absolute,
                        right: size::SPACE,
                        top: {size::SPACE_TIGHT / 2.0},
                        bottom: {size::SPACE_TIGHT / 2.0},
                        width: size::SCROLLBAR_WIDTH,
                    }
                ),
            ]
        }
    }
}

// A selectable row in the popup's list of options.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct PlumeSelectOption;

impl PlumeSelectOption {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                min_height: size::ROW_HEIGHT,
                min_width: size::ROW_HEIGHT,
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Start,
                align_items: AlignItems::Center,
                column_gap: size::SPACE,
                padding: UiRect::horizontal(size::SPACE),
            }
            AccessibilityNode(accesskit::Node::new(Role::ListItem))
            InheritableThemeTextToken(tokens::SELECT_OPTION_TEXT)
            TextStyleRelay
            Hovered
            ListItem
            Children [
                (
                    // Hidden ticks still occupy layout, so every label shares the gutter.
                    fa_icon(font_awesome::solid::CHECK)
                    Node {
                        width: size::ICON_WIDTH,
                    }
                    SelectOptionCheck
                    Visibility::Hidden
                ),
            ]
        }
    }
}

// Index of an option row among its select's options.
#[derive(Component, Default, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
struct SelectOptionIndex(usize);

// Marker for the selected-row tick.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct SelectOptionCheck;

// Marker for the popup ([`PlumePopup`] chrome + [`MenuFocusState`]); its
// existence is the select's open state.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
struct PlumeSelectPopup;

// Marker for the ghost-row overlay awaiting measurement.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct SelectMeasure;

// Change the row cap ([`PlumeSelectProps`]'s `max_visible` covers spawn time);
// an already-open popup keeps its cap until reopened.
pub(crate) fn set_select_max_visible(world: &mut World, select_ent: Entity, max_visible: usize) {
    if let Ok(mut entity) = world.get_entity_mut(select_ent)
        && let Some(mut cap) = entity.get_mut::<SelectMaxVisible>()
    {
        cap.0 = max_visible.max(1);
    }
}

// Spawn the popup (chrome + option rows) into the select's socket.
fn open_select_popup(
    select_ent: Entity,
    nav: NavAction,
    q_children: &Query<&Children>,
    q_socket: &Query<(), With<PopupSocket>>,
    q_select: &Query<
        (
            &SelectOptions,
            &SelectedIndex,
            &SelectMaxVisible,
            &ComputedNode,
        ),
        With<PlumeSelect>,
    >,
    commands: &mut Commands,
) {
    let Ok((options, selected, max_visible, computed)) = q_select.get(select_ent) else {
        return;
    };
    let Some(socket) = q_children
        .iter_descendants(select_ent)
        .find(|descendant| q_socket.contains(*descendant))
    else {
        warn!("Select popup socket not found");
        return;
    };
    let rows = option_rows(&options.0, selected.0);
    let max_height = size::ROW_HEIGHT * max_visible.0.max(1) as f32;
    // Seeded here so the popup opens at the select's width; `sync_select_width`
    // tracks later resizes.
    let min_width = px((computed.size().x * computed.inverse_scale_factor()).round());
    commands
        .spawn_scene(bsn! {
            @PlumePopup {
                @placement: PopupPlacement::Below,
                @place_very_close: true,
                @dismiss: PopupDismiss::FocusOut,
                @padding: UiRect::vertical(size::SPACE_TIGHT),
                @contents: bsn_list![
                    (
                        @PlumeSelectOptions {
                            @options: rows,
                        }
                        on(listbox_update_selection)
                        on(re_emit_listbox_value)
                        on(close_popup_on_reselect)
                        Node {
                            max_height: max_height,
                        }
                    ),
                ],
            }
            PlumeSelectPopup
            template_value(MenuFocusState::Opening(nav))
            // The select's own chrome tokens, over the generic popup ones.
            ThemeBackgroundToken(tokens::SELECT_BG)
            ThemeBorderToken(tokens::SELECT_BORDER)
            Node {
                min_width: min_width,
            }
        })
        .insert(ChildOf(socket));
}

// Despawn `popup`, returning focus to the select's button when focus was inside
// the select (e.g. on the popup's listbox).
fn close_select_popup(
    popup: Entity,
    q_parents: &Query<&ChildOf>,
    q_children: &Query<&Children>,
    q_is_select: &Query<(), With<PlumeSelect>>,
    q_button: &Query<(), With<PlumeSelectButton>>,
    focus: &mut InputFocus,
    commands: &mut Commands,
) {
    if let Some(select_ent) = q_parents
        .iter_ancestors(popup)
        .find(|ancestor| q_is_select.contains(*ancestor))
    {
        let focus_in_select = focus.get().is_some_and(|focused| {
            focused == select_ent || q_parents.iter_ancestors(focused).any(|a| a == select_ent)
        });
        if focus_in_select
            && let Some(button) = q_children
                .iter_descendants(select_ent)
                .find(|descendant| q_button.contains(*descendant))
        {
            focus.set(button, FocusCause::Navigated);
        }
    }
    close_popup(commands, popup);
}

#[allow(clippy::too_many_arguments)]
fn on_menu_event(
    mut ev: On<MenuEvent>,
    q_children: Query<&Children>,
    q_parents: Query<&ChildOf>,
    q_popup: Query<(), With<PlumeSelectPopup>>,
    q_socket: Query<(), With<PopupSocket>>,
    q_buttons: Query<(), With<PlumeSelectButton>>,
    q_is_select: Query<(), With<PlumeSelect>>,
    q_select: Query<
        (
            &SelectOptions,
            &SelectedIndex,
            &SelectMaxVisible,
            &ComputedNode,
        ),
        With<PlumeSelect>,
    >,
    mut commands: Commands,
    mut focus: ResMut<InputFocus>,
) {
    let popup = q_children
        .iter_descendants(ev.source)
        .find(|descendant| q_popup.contains(*descendant));
    match ev.event().action {
        MenuAction::Open(nav) => {
            ev.propagate(false);
            if popup.is_none() {
                open_select_popup(
                    ev.source,
                    nav,
                    &q_children,
                    &q_socket,
                    &q_select,
                    &mut commands,
                );
            }
        }
        MenuAction::Toggle => {
            ev.propagate(false);
            match popup {
                Some(popup) => close_select_popup(
                    popup,
                    &q_parents,
                    &q_children,
                    &q_is_select,
                    &q_buttons,
                    &mut focus,
                    &mut commands,
                ),
                None => open_select_popup(
                    ev.source,
                    NavAction::First,
                    &q_children,
                    &q_socket,
                    &q_select,
                    &mut commands,
                ),
            }
        }
        MenuAction::CloseAll => {
            if let Some(popup) = popup {
                ev.propagate(false);
                close_select_popup(
                    popup,
                    &q_parents,
                    &q_children,
                    &q_is_select,
                    &q_buttons,
                    &mut focus,
                    &mut commands,
                );
            }
        }
        MenuAction::FocusRoot => {
            for descendant in q_children.iter_descendants(ev.source) {
                if q_buttons.contains(descendant) {
                    ev.propagate(false);
                    focus.set(descendant, FocusCause::Navigated);
                    break;
                }
            }
        }
    }
}

// Event is sent on the options listbox.
fn close_popup_on_reselect(
    ev: On<ReselectListRow>,
    q_popup: Query<(), With<PlumeSelectPopup>>,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    q_is_select: Query<(), With<PlumeSelect>>,
    q_button: Query<(), With<PlumeSelectButton>>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    if let Some(popup) = q_parents
        .iter_ancestors(ev.event_target())
        .find(|ancestor| q_popup.contains(*ancestor))
    {
        close_select_popup(
            popup,
            &q_parents,
            &q_children,
            &q_is_select,
            &q_button,
            &mut focus,
            &mut commands,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn re_emit_listbox_value(
    ev: On<ValueChange<Entity>>,
    q_select: Query<(), With<PlumeSelect>>,
    q_option_index: Query<&SelectOptionIndex>,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    q_popup: Query<(), With<PlumeSelectPopup>>,
    q_button: Query<(), With<PlumeSelectButton>>,
    mut focus: ResMut<InputFocus>,
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

    if let Some(select_ent) = select_ent
        && let Ok(index) = q_option_index.get(ev.value)
    {
        commands.trigger(ValueChange {
            source: select_ent,
            value: index.0,
            is_final: true,
        });
    };

    if let Some(popup_ent) = popup_ent {
        close_select_popup(
            popup_ent,
            &q_parents,
            &q_children,
            &q_select,
            &q_button,
            &mut focus,
            &mut commands,
        );
    }
}

fn sync_selected_index(
    q_newly_selected: Query<
        (Entity, &SelectOptionIndex),
        (Added<Selected>, With<PlumeSelectOption>),
    >,
    q_parents: Query<&ChildOf>,
    mut q_select: Query<&mut SelectedIndex, With<PlumeSelect>>,
) {
    for (row, row_index) in q_newly_selected.iter() {
        let Some(select_ent) = q_parents
            .iter_ancestors(row)
            .find(|ancestor| q_select.contains(*ancestor))
        else {
            continue;
        };
        if let Ok(mut index) = q_select.get_mut(select_ent)
            && index.0 != row_index.0
        {
            index.0 = row_index.0;
        }
    }
}

fn select_on_set_selected_index(
    ev: On<SetValue<usize>>,
    mut q_select: Query<(&SelectOptions, &mut SelectedIndex), With<PlumeSelect>>,
) {
    let Ok((options, mut index)) = q_select.get_mut(ev.entity) else {
        return;
    };
    if ev.value < options.0.len() && index.0 != ev.value {
        index.0 = ev.value;
    }
}

// The button caption always shows the selected option's label.
fn sync_caption(
    q_selects: Query<
        (Entity, &SelectedIndex, &SelectOptions),
        (
            With<PlumeSelect>,
            Or<(Changed<SelectedIndex>, Changed<SelectOptions>)>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_caption: Query<&mut Text, With<SelectCaption>>,
) {
    for (select_ent, index, options) in q_selects.iter() {
        let label = options
            .0
            .get(index.0)
            .map(|(label, _)| label.clone())
            .unwrap_or_default();
        for descendant in q_children.iter_descendants(select_ent) {
            if let Ok(mut caption) = q_caption.get_mut(descendant) {
                if caption.0 != label {
                    caption.0 = label;
                }
                break;
            }
        }
    }
}

// An open popup's `Selected` row follows the root's `SelectedIndex` (covers
// programmatic writes; row clicks already set both).
fn sync_rows_from_index(
    q_changed: Query<(Entity, &SelectedIndex), (With<PlumeSelect>, Changed<SelectedIndex>)>,
    q_children: Query<&Children>,
    q_rows: Query<(&SelectOptionIndex, Has<Selected>), With<PlumeSelectOption>>,
    mut commands: Commands,
) {
    for (select_ent, index) in q_changed.iter() {
        for descendant in q_children.iter_descendants(select_ent) {
            let Ok((row_index, selected)) = q_rows.get(descendant) else {
                continue;
            };
            match (row_index.0 == index.0, selected) {
                (true, false) => {
                    commands.entity(descendant).insert(Selected);
                }
                (false, true) => {
                    commands.entity(descendant).remove::<Selected>();
                }
                _ => {}
            }
        }
    }
}

// The headless `MenuButton` checks `InteractionDisabled` on itself, so the marker on the
// select root must be mirrored onto the internal menu button (which also restyles it).
#[allow(clippy::too_many_arguments)]
fn sync_select_disabled(
    q_newly_disabled: Query<Entity, (With<PlumeSelect>, Added<InteractionDisabled>)>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    q_is_select: Query<(), With<PlumeSelect>>,
    q_children: Query<&Children>,
    q_parents: Query<&ChildOf>,
    q_button: Query<(), With<PlumeSelectButton>>,
    q_popup: Query<(), With<PlumeSelectPopup>>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    for select_ent in q_newly_disabled.iter() {
        for descendant in q_children.iter_descendants(select_ent) {
            if q_button.contains(descendant) {
                commands.entity(descendant).insert(InteractionDisabled);
            } else if q_popup.contains(descendant) {
                close_select_popup(
                    descendant,
                    &q_parents,
                    &q_children,
                    &q_is_select,
                    &q_button,
                    &mut focus,
                    &mut commands,
                );
            }
        }
    }
    removed_disabled.read().for_each(|ent| {
        if q_is_select.contains(ent) {
            for descendant in q_children.iter_descendants(ent) {
                if q_button.contains(descendant) {
                    commands.entity(descendant).remove::<InteractionDisabled>();
                }
            }
        }
    });
}

// Keep an open popup at least as wide as its select.
fn sync_select_width(
    q_selects: Query<(Entity, &ComputedNode), With<PlumeSelect>>,
    q_children: Query<&Children>,
    q_popup: Query<(), With<PlumeSelectPopup>>,
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

// Size the button (and pin the caption) to the widest option from the measured
// ghost rows, so picking never resizes the control; the ghosts then despawn.
fn measure_select_width(
    q_measures: Query<(Entity, &ChildOf, Option<&Inherited<TextFont>>), With<SelectMeasure>>,
    q_children: Query<&Children>,
    q_labels: Query<(&Text, Option<&TextFont>), Without<SelectOptionCheck>>,
    q_button: Query<(), With<PlumeSelectButton>>,
    q_caption: Query<(), With<SelectCaption>>,
    q_computed: Query<&ComputedNode>,
    rem_size: Res<RemSize>,
    mut q_nodes: Query<&mut Node>,
    mut commands: Commands,
) {
    // The chrome constants are em, so they only become widths against the em
    // size of the select they belong to.
    fn val_px(val: Val, em_px: f32) -> f32 {
        match val {
            Val::Px(v) => v,
            Val::Em(v) => v * em_px,
            _ => 0.0,
        }
    }
    let width_of = |entity: Entity| {
        q_computed
            .get(entity)
            .map(|computed| computed.size().x * computed.inverse_scale_factor())
            .unwrap_or(0.0)
    };
    'measures: for (measure_ent, child_of, inherited) in q_measures.iter() {
        // The effective font at this select; the overlay is a relay, so it
        // arrives with normal propagation.
        let Some(effective) = inherited else {
            continue;
        };
        let rows = q_children.get(measure_ent).ok();
        let (mut widest_row, mut widest_label) = (0.0f32, 0.0f32);
        for row in rows.iter().flat_map(|rows| rows.iter()) {
            let Some((label_ent, text, text_font)) =
                q_children.get(*row).ok().and_then(|row_children| {
                    row_children.iter().find_map(|child| {
                        q_labels
                            .get(*child)
                            .ok()
                            .map(|(text, font)| (*child, text, font))
                    })
                })
            else {
                continue;
            };
            if text.0.is_empty() {
                continue;
            }
            // Unmeasured until the row font has propagated (a default-font width
            // would bake in the wrong size) and the glyphs have a real layout.
            if text_font.is_none_or(|font| font.font_size != effective.0.font_size) {
                continue 'measures;
            }
            let label_width = width_of(label_ent);
            if label_width <= 0.0 {
                continue 'measures;
            }
            widest_label = widest_label.max(label_width);
            widest_row = widest_row.max(width_of(*row));
        }
        if widest_row > 0.0 {
            // The popup doesn't exist to measure, so its horizontal chrome is
            // reconstructed: border both sides plus the options' scrollbar gutter.
            // Bevy caches the overlay's resolved em size on its `ComputedNode`;
            // before layout has reached it, em falls back to rem exactly as
            // `Val::Em` resolution would.
            let em_px = q_computed
                .get(measure_ent)
                .map_or(rem_size.0, |computed| computed.em_size.0);
            let chrome = 2.0 * val_px(size::HAIRLINE, em_px)
                + val_px(size::SCROLLBAR_GUTTER, em_px)
                + val_px(size::SPACE, em_px);
            let button_target = px((widest_row + chrome).ceil());
            let caption_target = px(widest_label.ceil());
            for descendant in q_children.iter_descendants(child_of.parent()) {
                if q_button.contains(descendant) {
                    if let Ok(mut node) = q_nodes.get_mut(descendant) {
                        node.width = button_target;
                    }
                } else if q_caption.contains(descendant)
                    && let Ok(mut node) = q_nodes.get_mut(descendant)
                {
                    node.width = caption_target;
                }
            }
        }
        commands.entity(measure_ent).despawn();
    }
}

fn update_option_styles(
    q_options: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Selected>,
            &Hovered,
            Option<&ThemeBackgroundToken>,
            &InheritableThemeTextToken,
        ),
        (
            With<PlumeSelectOption>,
            Or<(
                Changed<Hovered>,
                Added<Selected>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    q_check: Query<(), With<SelectOptionCheck>>,
    mut commands: Commands,
) {
    for (option_ent, disabled, selected, hovered, bg_color, font_color) in q_options.iter() {
        let check_ent = q_children
            .iter_descendants(option_ent)
            .find(|en| q_check.contains(*en));
        set_option_styles(
            option_ent,
            check_ent,
            disabled,
            selected,
            hovered.0,
            bg_color,
            font_color,
            &mut commands,
        );
    }
}

fn update_option_styles_remove(
    q_options: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Selected>,
            &Hovered,
            Option<&ThemeBackgroundToken>,
            &InheritableThemeTextToken,
        ),
        With<PlumeSelectOption>,
    >,
    q_children: Query<&Children>,
    q_check: Query<(), With<SelectOptionCheck>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_selected: RemovedComponents<Selected>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_selected.read())
        .for_each(|ent| {
            if let Ok((option_ent, disabled, selected, hovered, bg_color, font_color)) =
                q_options.get(ent)
            {
                let check_ent = q_children
                    .iter_descendants(option_ent)
                    .find(|en| q_check.contains(*en));
                set_option_styles(
                    option_ent,
                    check_ent,
                    disabled,
                    selected,
                    hovered.0,
                    bg_color,
                    font_color,
                    &mut commands,
                );
            }
        });
}

fn set_option_styles(
    option_ent: Entity,
    check_ent: Option<Entity>,
    disabled: bool,
    selected: bool,
    hovered: bool,
    bg_color: Option<&ThemeBackgroundToken>,
    font_color: &InheritableThemeTextToken,
    commands: &mut Commands,
) {
    // Background shows hover only; selection is the tick.
    let bg_token = (!disabled && hovered).then_some(tokens::SELECT_OPTION_BG_HOVER);

    let font_color_token = match disabled {
        true => tokens::SELECT_OPTION_TEXT_DISABLED,
        false => tokens::SELECT_OPTION_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy::window::SystemCursorIcon::NotAllowed,
        false => bevy::window::SystemCursorIcon::Pointer,
    };

    set_optional_background(commands, option_ent, bg_color, bg_token);

    if font_color.0 != font_color_token {
        commands
            .entity(option_ent)
            .insert(InheritableThemeTextToken(font_color_token));
    }

    if let Some(check_ent) = check_ent {
        commands.entity(check_ent).insert(match selected {
            true => Visibility::Inherited,
            false => Visibility::Hidden,
        });
    }

    commands
        .entity(option_ent)
        .insert(EntityCursor::System(cursor_shape));
}

// Marker for the keyboard-navigation highlight on a listbox's active row.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ActiveRowOutline;

// Outline the focused listbox's `ActiveDescendant` row (the arrow-key cursor,
// distinct from the `Selected` row's tick) while keyboard focus is visible.
fn update_active_row_outline(
    focus: Res<InputFocus>,
    focus_visible: Res<InputFocusVisible>,
    q_active_changed: Query<(), (With<ListBox>, Changed<ActiveDescendant>)>,
    q_listbox: Query<&ActiveDescendant, With<ListBox>>,
    q_row_outline: Query<(Entity, &ChildOf), With<ActiveRowOutline>>,
    mut commands: Commands,
) {
    if !focus.is_changed() && !focus_visible.is_changed() && q_active_changed.is_empty() {
        return;
    }

    let active_row = focus
        .get()
        .filter(|_| focus_visible.0)
        .and_then(|focused| q_listbox.get(focused).ok())
        .and_then(|active_descendant| active_descendant.0);

    let mut needs_spawn = active_row.is_some();
    for (outline_ent, child_of) in q_row_outline.iter() {
        if Some(child_of.parent()) == active_row {
            needs_spawn = false;
        } else {
            commands.entity(outline_ent).despawn();
        }
    }

    if let Some(row_ent) = active_row
        && needs_spawn
    {
        commands.entity(row_ent).with_child((
            Node {
                position_type: PositionType::Absolute,
                left: Val::ZERO,
                right: Val::ZERO,
                top: Val::ZERO,
                bottom: Val::ZERO,
                border: UiRect::all(size::FOCUS_RING_WIDTH),
                border_radius: BorderRadius::all(size::CORNER_RADIUS),
                ..Default::default()
            },
            ThemeBorderToken(tokens::FOCUS_RING),
            ActiveRowOutline,
            // Em-sized chrome needs the chain's `EmSize`.
            TextStyleRelay,
        ));
    }
}

// Plugin which runs the [`PlumeSelect`] control
pub(crate) struct SelectPlugin;

impl Plugin for SelectPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            (update_option_styles, update_option_styles_remove).in_set(PickingSystems::Last),
        )
        .add_systems(
            Update,
            (
                sync_caption,
                sync_selected_index,
                sync_rows_from_index,
                sync_select_width,
                measure_select_width,
                sync_select_disabled,
            ),
        )
        .add_systems(PostUpdate, update_active_row_outline)
        .add_observer(select_on_set_selected_index);
    }
}
