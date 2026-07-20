//! Internal scrolling list view and its selectable rows.
use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{Plugin, PostUpdate, PreUpdate};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs as _,
    system::{Commands, Query, Res},
};
use bevy_input_focus::{InputFocus, InputFocusVisible, tab_navigation::TabIndex};
use bevy_picking::{PickingSystems, hover::Hovered};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::{Scene, SceneComponent, SceneList, bsn, bsn_list, template_value};
use bevy_text::FontWeight;
use bevy_ui::{
    AlignItems, BorderRadius, Display, FlexDirection, InteractionDisabled, JustifyContent, Node,
    Overflow, PositionType, Selected, UiRect, px,
};
use bevy_ui_widgets::{ActiveDescendant, ControlOrientation, ListBox, ListItem, ScrollArea};

use crate::{
    constants::{font_awesome, fonts, size},
    controls::{PlumeScrollbar, ScrollbarGutter},
    cursor::EntityCursor,
    display::{caption, fa_icon},
    font_styles::InheritableFont,
    theme::{InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor},
    tokens,
};

/// A container that displays a scrolling list of items
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeListViewProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeListView;

/// Props used to construct a [`PlumeListView`] scene.
pub struct PlumeListViewProps {
    /// The list of items to be displayed in the list view.
    pub rows: Box<dyn SceneList>,
}

impl Default for PlumeListViewProps {
    fn default() -> Self {
        Self {
            rows: Box::new(bsn_list!()),
        }
    }
}

impl PlumeListView {
    /// Scene function for list view.
    pub fn scene(props: PlumeListViewProps) -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Start,
            }
            template_value(ScrollbarGutter(size::SCROLLBAR_GUTTER.try_add(size::PAD).unwrap()))
            ListBox
            // Focusable for arrow-key selection; a hidden popup is skipped by Tab anyway
            // (tab gathering ignores invisible entities).
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
                    Children [
                        {props.rows}
                    ]
                ),

                @PlumeScrollbar {
                    @target: #inner,
                    @orientation: {ControlOrientation::Vertical}
                }
                Node {
                    position_type: PositionType::Absolute,
                    right: size::PAD,
                    top: px(2),
                    bottom: px(2),
                    width: size::SCROLLBAR_WIDTH,
                }
            ]
        }
    }
}

/// A selectable row in a list of items
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct PlumeListRow;

impl PlumeListRow {
    /// Scene function for list row.
    pub fn scene() -> impl Scene {
        bsn! {
            Node {
                min_height: size::ROW_HEIGHT,
                min_width: size::ROW_HEIGHT,
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Start,
                align_items: AlignItems::Center,
                column_gap: size::GAP,
                padding: UiRect::horizontal(size::GAP),
            }
            AccessibilityNode(accesskit::Node::new(Role::ListItem))
            InheritableThemeTextColor(tokens::LISTROW_TEXT)
            ThemeBackgroundColor(tokens::LISTROW_BG)
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::MEDIUM_FONT,
                weight: FontWeight::NORMAL,
            }
            Hovered
            ListItem
            Children [(
                // Hidden ticks still occupy layout, so every label shares the gutter.
                fa_icon(font_awesome::solid::CHECK)
                Node {
                    width: size::ICON_WIDTH
                }
                ListRowCheck
                Visibility::Hidden
            )]
        }
    }
}

/// Entirely optional component to store a usize on a `PlumeListRow`
/// Added by [`list_rows_from_strings`] so there's a value
/// on a string based select you can use to work out which of the array
/// of strings was selected
#[derive(Component, Default, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct ListRowIndex(pub usize);

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
            .map(|(i, label)| {
                let label: String = label.as_ref().into();
                bsn! {
                    @PlumeListRow
                    ListRowIndex(i)
                    {selected.is_some_and(|selected| selected == i).then(|| bsn! { Selected })}
                    Children [ caption(label) ]
                }
            })
            .collect::<Vec<_>>(),
    )
}

/// Marker for the selected-row tick.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct ListRowCheck;

fn update_listrow_styles(
    q_listrows: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Selected>,
            &Hovered,
            &ThemeBackgroundColor,
            &InheritableThemeTextColor,
        ),
        (
            With<PlumeListRow>,
            Or<(
                Changed<Hovered>,
                Added<Selected>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    q_children: Query<&Children>,
    q_check: Query<(), With<ListRowCheck>>,
    mut commands: Commands,
) {
    for (listrow_ent, disabled, selected, hovered, bg_color, font_color) in q_listrows.iter() {
        let check_ent = q_children
            .iter_descendants(listrow_ent)
            .find(|en| q_check.contains(*en));
        set_listrow_styles(
            listrow_ent,
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

fn update_listrow_styles_remove(
    q_listrows: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Selected>,
            &Hovered,
            &ThemeBackgroundColor,
            &InheritableThemeTextColor,
        ),
        With<PlumeListRow>,
    >,
    q_children: Query<&Children>,
    q_check: Query<(), With<ListRowCheck>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_selected: RemovedComponents<Selected>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_selected.read())
        .for_each(|ent| {
            if let Ok((listrow_ent, disabled, selected, hovered, bg_color, font_color)) =
                q_listrows.get(ent)
            {
                let check_ent = q_children
                    .iter_descendants(listrow_ent)
                    .find(|en| q_check.contains(*en));
                set_listrow_styles(
                    listrow_ent,
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

fn set_listrow_styles(
    listrow_ent: Entity,
    check_ent: Option<Entity>,
    disabled: bool,
    selected: bool,
    hovered: bool,
    bg_color: &ThemeBackgroundColor,
    font_color: &InheritableThemeTextColor,
    commands: &mut Commands,
) {
    // Background shows hover only; selection is the tick.
    let outline_bg_token = match (disabled, hovered) {
        (false, true) => tokens::LISTROW_BG_HOVER,
        _ => tokens::LISTROW_BG,
    };

    let font_color_token = match disabled {
        true => tokens::LISTROW_TEXT_DISABLED,
        false => tokens::LISTROW_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
    };

    if bg_color.0 != outline_bg_token {
        commands
            .entity(listrow_ent)
            .insert(ThemeBackgroundColor(outline_bg_token));
    }

    if font_color.0 != font_color_token {
        commands
            .entity(listrow_ent)
            .insert(InheritableThemeTextColor(font_color_token));
    }

    if let Some(check_ent) = check_ent {
        commands.entity(check_ent).insert(match selected {
            true => Visibility::Inherited,
            false => Visibility::Hidden,
        });
    }

    commands
        .entity(listrow_ent)
        .insert(EntityCursor::System(cursor_shape));
}

/// Marker for the keyboard-navigation highlight on a listbox's active row.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct ActiveRowOutline;

/// Outline the focused listbox's [`ActiveDescendant`] row (the arrow-key cursor,
/// distinct from the `Selected` row's tick) while keyboard focus is visible.
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
                left: px(0),
                right: px(0),
                top: px(0),
                bottom: px(0),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(size::CORNER_RADIUS),
                ..Default::default()
            },
            ThemeBorderColor(tokens::FOCUS_RING),
            ActiveRowOutline,
        ));
    }
}

/// Plugin which registers the systems for updating the listrow styles.
pub struct ListViewPlugin;

impl Plugin for ListViewPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_listrow_styles, update_listrow_styles_remove).in_set(PickingSystems::Last),
        );
        app.add_systems(PostUpdate, update_active_row_outline);
    }
}
