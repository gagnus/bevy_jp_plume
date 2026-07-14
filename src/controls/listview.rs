//! Internal scrolling list view and its selectable rows.
use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{Plugin, PreUpdate};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Changed, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs as _,
    system::{Commands, Query},
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_picking::{PickingSystems, hover::Hovered};
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::{Scene, SceneComponent, SceneList, bsn, bsn_list};
use bevy_text::FontWeight;
use bevy_ui::{
    AlignItems, Display, FlexDirection, InteractionDisabled, JustifyContent, Node, Overflow,
    PositionType, Selected, UiRect, px,
};
use bevy_ui_widgets::{ControlOrientation, ListBox, ListItem, ScrollArea};

use crate::{
    constants::{font_awesome, fonts, size},
    controls::{PlumeScrollbar, ScrollbarGutter},
    cursor::EntityCursor,
    display::fa_icon_solid,
    font_styles::InheritableFont,
    theme::{InheritableThemeTextColor, ThemeBackgroundColor},
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
            // Outer frame that holds the scrollbar
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Start,
            }
            ScrollbarGutter(size::SCROLLBAR_GUTTER)
            ListBox
            // Click-to-focus marker only; plume registers no Tab-key navigation.
            TabIndex(0)
            AccessibilityNode(accesskit::Node::new(Role::ListBox))
            Children [
                // Inner part that scrolls
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
                    right: px(0),
                    top: px(0),
                    bottom: px(0),
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
                column_gap: size::GAP_TIGHT,
                padding: UiRect::axes(size::GAP, px(2)),
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
                fa_icon_solid(font_awesome::FA_CHECK)
                ListRowCheck
                Visibility::Hidden
            )]
        }
    }
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

    // Change outline background
    if bg_color.0 != outline_bg_token {
        commands
            .entity(listrow_ent)
            .insert(ThemeBackgroundColor(outline_bg_token));
    }

    // Change font color
    if font_color.0 != font_color_token {
        commands
            .entity(listrow_ent)
            .insert(InheritableThemeTextColor(font_color_token));
    }

    // Change tick visibility
    if let Some(check_ent) = check_ent {
        commands.entity(check_ent).insert(match selected {
            true => Visibility::Inherited,
            false => Visibility::Hidden,
        });
    }

    // Change cursor shape
    commands
        .entity(listrow_ent)
        .insert(EntityCursor::System(cursor_shape));
}

/// Plugin which registers the systems for updating the listrow styles.
pub struct ListViewPlugin;

impl Plugin for ListViewPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_listrow_styles, update_listrow_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
