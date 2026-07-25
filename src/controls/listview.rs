//! Internal scrolling list view for options.
use accesskit::Role;
use bevy::a11y::AccessibilityNode;
use bevy::app::{Plugin, PostUpdate, PreUpdate};
use bevy::camera::visibility::Visibility;
use bevy::ecs::{
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
use bevy::input_focus::{InputFocus, InputFocusVisible, tab_navigation::TabIndex};
use bevy::picking::{PickingSystems, hover::Hovered};
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::scene::{Scene, SceneComponent, SceneList, bsn, bsn_list, template_value};
use bevy::text::FontWeight;
use bevy::ui::{
    AlignItems, BorderRadius, Display, FlexDirection, InteractionDisabled, JustifyContent, Node,
    Overflow, PositionType, Selected, UiRect, Val, px,
};
use bevy::ui_widgets::{ActiveDescendant, ControlOrientation, ListBox, ListItem, ScrollArea};

use crate::{
    constants::{font_awesome, fonts, size},
    controls::{PlumeScrollbar, ScrollbarGutter},
    cursor::EntityCursor,
    display::{caption, fa_icon},
    font_styles::InheritableFont,
    theme::{InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor},
    tokens,
};

// TODO:SELECT this is only used by select and doesn't seem to have anything targeting it much?
/// A container that displays a scrolling list of items
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(PlumeSelectOptionsProps)]
#[reflect(Component, Clone, Default)]
pub struct PlumeSelectOptions;

/// Props used to construct a [`PlumeListView`] scene.
pub struct PlumeSelectOptionsProps {
    /// The list of items to be displayed in the list view.
    pub options: Box<dyn SceneList>,
}

impl Default for PlumeSelectOptionsProps {
    fn default() -> Self {
        Self {
            options: Box::new(bsn_list!()),
        }
    }
}

impl PlumeSelectOptions {
    /// Scene function for list view.
    pub fn scene(props: PlumeSelectOptionsProps) -> impl Scene {
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
                        {props.options}
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
pub struct PlumeSelectOption;

impl PlumeSelectOption {
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
            InheritableThemeTextColor(tokens::OPTION_TEXT)
            ThemeBackgroundColor(tokens::OPTION_BG)
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
                SelectOptionCheck
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
pub struct SelectOptionIndex(pub usize);

/// Convert an iterator of strings into `PlumeListRow` scenes with `OptionIndex`
/// on each one containing its index, optionally mark one selected
pub fn options_from_strings(
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
                    @PlumeSelectOption
                    SelectOptionIndex(i)
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
pub struct SelectOptionCheck;

fn update_option_styles(
    q_options: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Selected>,
            &Hovered,
            &ThemeBackgroundColor,
            &InheritableThemeTextColor,
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
            &ThemeBackgroundColor,
            &InheritableThemeTextColor,
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
    bg_color: &ThemeBackgroundColor,
    font_color: &InheritableThemeTextColor,
    commands: &mut Commands,
) {
    // Background shows hover only; selection is the tick.
    let outline_bg_token = match (disabled, hovered) {
        (false, true) => tokens::OPTION_BG_HOVER,
        _ => tokens::OPTION_BG,
    };

    let font_color_token = match disabled {
        true => tokens::OPTION_TEXT_DISABLED,
        false => tokens::OPTION_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy::window::SystemCursorIcon::NotAllowed,
        false => bevy::window::SystemCursorIcon::Pointer,
    };

    if bg_color.0 != outline_bg_token {
        commands
            .entity(option_ent)
            .insert(ThemeBackgroundColor(outline_bg_token));
    }

    if font_color.0 != font_color_token {
        commands
            .entity(option_ent)
            .insert(InheritableThemeTextColor(font_color_token));
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
                left: Val::ZERO,
                right: Val::ZERO,
                top: Val::ZERO,
                bottom: Val::ZERO,
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(size::CORNER_RADIUS),
                ..Default::default()
            },
            ThemeBorderColor(tokens::FOCUS_RING),
            ActiveRowOutline,
        ));
    }
}

// TODO:SELECT - doesn't need a separate plugin when combined...
/// Plugin which registers the systems for updating the listrow styles.
pub struct SelectOptionsPlugin;

impl Plugin for SelectOptionsPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            (update_option_styles, update_option_styles_remove).in_set(PickingSystems::Last),
        );
        app.add_systems(PostUpdate, update_active_row_outline);
    }
}
