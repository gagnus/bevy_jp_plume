//! Radio button control.
use bevy_app::{Plugin, PreUpdate};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
};
use bevy_picking::PickingSystems;
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_scene::prelude::*;
use bevy_text::FontWeight;
use bevy_ui::{
    AlignItems, BorderRadius, Checked, Display, FlexDirection, InteractionDisabled, JustifyContent,
    LayoutConfig, Node, px,
};
use bevy_ui_widgets::RadioButton;

use crate::{
    constants::{fonts, size},
    cursor::EntityCursor,
    font_styles::InheritableFont,
    theme::{InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor},
    tokens,
};

/// A radio widget.
///
/// This is spawnable by inheriting it as a "scene component" with optional [`PlumeRadioProps`].
///
/// Emits [`bevy_ui_widgets::ValueChange<bool>`] (true) when checked, and the radio group emits
/// [`bevy_ui_widgets::ValueChange<Entity>`] with the newly selected radio.
#[derive(SceneComponent, Default, Clone)]
#[scene(PlumeRadioProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
pub struct PlumeRadio;

/// Props used to construct a [`PlumeRadio`] scene.
pub struct PlumeRadioProps {
    /// Label for this radio button. This can contain multiple entities, which will be contained
    /// in a flexbox.
    pub caption: Box<dyn SceneList>,
}

impl Default for PlumeRadioProps {
    fn default() -> Self {
        Self {
            caption: Box::new(bsn_list!()),
        }
    }
}

impl PlumeRadio {
    fn scene(props: PlumeRadioProps) -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Start,
                align_items: AlignItems::Center,
                column_gap: px(4),
                min_height: size::ROW_HEIGHT,
            }
            RadioButton
            EntityCursor::System(bevy_window::SystemCursorIcon::Pointer)
            InheritableThemeTextColor(tokens::RADIO_TEXT)
            InheritableFont {
                font: fonts::REGULAR,
                font_size: size::MEDIUM_FONT,
                weight: FontWeight::NORMAL,
            }
            Children [(
                Node {
                    display: Display::Flex,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    width: size::RADIO_SIZE,
                    height: size::RADIO_SIZE,
                    border: px(2),
                    border_radius: BorderRadius::MAX,
                }
                RadioOutline
                ThemeBorderColor(tokens::RADIO_BORDER)
                ThemeBackgroundColor(tokens::RADIO_BG)
                Children [(
                    Node {
                        width: px(12),
                        height: px(12),
                        border: px(2),
                        border_radius: BorderRadius::MAX,
                    }
                    LayoutConfig {
                        use_rounding: false,
                    }
                    RadioMark
                    ThemeBackgroundColor(tokens::RADIO_MARK)
                )]),
                {props.caption}
            ]
        }
    }
}

/// Marker for the radio outline
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct RadioOutline;

/// Marker for the radio check mark
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
struct RadioMark;

fn update_radio_styles(
    q_radioes: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            &InheritableThemeTextColor,
        ),
        (
            With<RadioButton>,
            Or<(Added<Checked>, Added<InteractionDisabled>)>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_outline: Query<(&ThemeBorderColor, &ThemeBackgroundColor), With<RadioOutline>>,
    mut q_mark: Query<&ThemeBackgroundColor, With<RadioMark>>,
    mut commands: Commands,
) {
    for (radio_ent, disabled, checked, font_color) in q_radioes.iter() {
        let Some(outline_ent) = q_children
            .iter_descendants(radio_ent)
            .find(|en| q_outline.contains(*en))
        else {
            continue;
        };
        let Some(mark_ent) = q_children
            .iter_descendants(radio_ent)
            .find(|en| q_mark.contains(*en))
        else {
            continue;
        };
        let (outline_border, outline_bg) = q_outline.get_mut(outline_ent).unwrap();
        let mark_color = q_mark.get_mut(mark_ent).unwrap();
        set_radio_styles(
            radio_ent,
            outline_ent,
            mark_ent,
            disabled,
            checked,
            outline_border,
            outline_bg,
            mark_color,
            font_color,
            &mut commands,
        );
    }
}

fn update_radio_styles_remove(
    q_radioes: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            &InheritableThemeTextColor,
        ),
        With<RadioButton>,
    >,
    q_children: Query<&Children>,
    mut q_outline: Query<(&ThemeBorderColor, &ThemeBackgroundColor), With<RadioOutline>>,
    mut q_mark: Query<&ThemeBackgroundColor, With<RadioMark>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_checked: RemovedComponents<Checked>,
    mut commands: Commands,
) {
    removed_disabled
        .read()
        .chain(removed_checked.read())
        .for_each(|ent| {
            if let Ok((radio_ent, disabled, checked, font_color)) = q_radioes.get(ent) {
                let Some(outline_ent) = q_children
                    .iter_descendants(radio_ent)
                    .find(|en| q_outline.contains(*en))
                else {
                    return;
                };
                let Some(mark_ent) = q_children
                    .iter_descendants(radio_ent)
                    .find(|en| q_mark.contains(*en))
                else {
                    return;
                };
                let (outline_border, outline_bg) = q_outline.get_mut(outline_ent).unwrap();
                let mark_color = q_mark.get_mut(mark_ent).unwrap();
                set_radio_styles(
                    radio_ent,
                    outline_ent,
                    mark_ent,
                    disabled,
                    checked,
                    outline_border,
                    outline_bg,
                    mark_color,
                    font_color,
                    &mut commands,
                );
            }
        });
}

fn set_radio_styles(
    radio_ent: Entity,
    outline_ent: Entity,
    mark_ent: Entity,
    disabled: bool,
    checked: bool,
    outline_border: &ThemeBorderColor,
    outline_bg: &ThemeBackgroundColor,
    mark_color: &ThemeBackgroundColor,
    font_color: &InheritableThemeTextColor,
    commands: &mut Commands,
) {
    let (outline_border_token, outline_bg_token) = match (checked, disabled) {
        (true, true) => (
            tokens::RADIO_BORDER_CHECKED_DISABLED,
            tokens::RADIO_BG_CHECKED_DISABLED,
        ),
        (true, false) => (tokens::RADIO_BORDER_CHECKED, tokens::RADIO_BG_CHECKED),
        (false, true) => (tokens::RADIO_BORDER_DISABLED, tokens::RADIO_BG_DISABLED),
        (false, false) => (tokens::RADIO_BORDER, tokens::RADIO_BG),
    };

    let mark_token = match disabled {
        true => tokens::RADIO_MARK_DISABLED,
        false => tokens::RADIO_MARK,
    };

    let font_color_token = match disabled {
        true => tokens::RADIO_TEXT_DISABLED,
        false => tokens::RADIO_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Pointer,
    };

    // Change outline border or bg
    if outline_border.0 != outline_border_token {
        commands
            .entity(outline_ent)
            .insert(ThemeBorderColor(outline_border_token));
    }
    if outline_bg.0 != outline_bg_token {
        commands
            .entity(outline_ent)
            .insert(ThemeBackgroundColor(outline_bg_token));
    }

    // Change mark color
    if mark_color.0 != mark_token {
        commands
            .entity(mark_ent)
            .insert(ThemeBackgroundColor(mark_token));
    }

    // Change mark visibility
    commands.entity(mark_ent).insert(match checked {
        true => Visibility::Inherited,
        false => Visibility::Hidden,
    });

    // Change font color
    if font_color.0 != font_color_token {
        commands
            .entity(radio_ent)
            .insert(InheritableThemeTextColor(font_color_token));
    }

    // Change cursor shape
    commands
        .entity(radio_ent)
        .insert(EntityCursor::System(cursor_shape));
}

/// Plugin which registers the systems for updating the radio styles.
pub struct RadioPlugin;

impl Plugin for RadioPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (update_radio_styles, update_radio_styles_remove).in_set(PickingSystems::Last),
        );
    }
}
