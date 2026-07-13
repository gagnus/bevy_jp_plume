//! Editable text field and its decorative container.
use bevy_app::{Plugin, PreUpdate, PropagateOver};
use bevy_asset::AssetServer;
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    lifecycle::RemovedComponents,
    query::{Added, Has, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
    template::template,
};
use bevy_input_focus::{InputFocus, tab_navigation::TabIndex};
use bevy_picking::PickingSystems;
use bevy_reflect::Reflect;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_scene::prelude::*;
use bevy_text::{
    EditableText, FontSource, FontWeight, LineBreak, LineHeight, TextCursorStyle, TextFont,
    TextLayout,
};
use bevy_ui::{BorderRadius, InteractionDisabled, Node, UiRect, px};

use crate::{
    constants::{fonts, size},
    cursor::EntityCursor,
    theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemedText, UiTheme},
    tokens,
};

/// A single-line editable text field, self-framed (background, border, and sizing
/// live on this entity — no wrapper container needed).
///
/// This is spawnable by inheriting it as a "scene component" with optional [`PlumeTextInputProps`].
#[derive(SceneComponent, Default, Clone)]
#[scene(PlumeTextInputProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
pub struct PlumeTextInput;

/// Props used to construct the [`PlumeTextInput`] scene.
#[derive(Default, Clone)]
pub struct PlumeTextInputProps {
    /// Visible width
    pub visible_width: Option<f32>,
    /// Max characters
    pub max_characters: Option<usize>,
}

impl PlumeTextInput {
    fn scene(props: PlumeTextInputProps) -> impl Scene {
        bsn! {
            // Line height fills the content box exactly (24 - 2*2 border): a taller
            // line overflows and makes the edit viewport re-clamp (1px jitter) while typing.
            Node {
                height: size::ROW_HEIGHT,
                padding: UiRect::horizontal(px(5.0)),
                border: px(2),
                border_radius: {BorderRadius::all(px(4.0))},
                width: px(180),
            }
            PlumeTextInput
            ThemeBackgroundColor(tokens::TEXT_INPUT_BG)
            ThemeBorderColor(tokens::TEXT_INPUT_BORDER)
            // The input is itself the text entity, so the inheritable color (which only
            // propagates to descendants) would never reach it.
            ThemeTextColor(tokens::TEXT_INPUT_TEXT)
            // Click-to-focus marker only; plume registers no Tab-key navigation.
            TabIndex(0)
            EditableText {
                cursor_width: 0.3,
                visible_width: {props.visible_width},
                max_characters: {props.max_characters},
            }
            ThemedText
            TextLayout {
                linebreak: LineBreak::NoWrap,
            }
            template(|_| Ok(LineHeight::Px(20.0)))
            template(|ctx| {
                Ok(TextFont {
                    font: FontSource::Handle(ctx.resource::<AssetServer>().load(fonts::REGULAR)),
                    font_size: size::COMPACT_FONT,
                    weight: FontWeight::NORMAL,
                    ..Default::default()
                })
            })
            PropagateOver<TextFont>
            EntityCursor::System(bevy_window::SystemCursorIcon::Text)
            TextCursorStyle::default()
        }
    }
}

fn update_text_cursor_color(
    mut q_text_input: Query<&mut TextCursorStyle, With<PlumeTextInput>>,
    theme: Res<UiTheme>,
) {
    if theme.is_changed() {
        for mut cursor_style in q_text_input.iter_mut() {
            cursor_style.color = theme.color(&tokens::TEXT_INPUT_CURSOR);
            cursor_style.selection_color = theme.color(&tokens::TEXT_INPUT_SELECTION);
            cursor_style.unfocused_selection_color =
                theme.color(&tokens::TEXT_INPUT_SELECTION_UNFOCUSED);
        }
    }
}

fn update_text_input_styles(
    q_inputs: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            &ThemeBackgroundColor,
            &ThemeTextColor,
        ),
        (With<PlumeTextInput>, Added<InteractionDisabled>),
    >,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    for (input_ent, disabled, bg_color, font_color) in q_inputs.iter() {
        if focus.get() == Some(input_ent) {
            focus.clear();
        }
        set_text_input_styles(input_ent, disabled, bg_color, font_color, &mut commands);
    }
}

fn update_text_input_styles_remove(
    q_inputs: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            &ThemeBackgroundColor,
            &ThemeTextColor,
        ),
        With<PlumeTextInput>,
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    removed_disabled.read().for_each(|ent| {
        if let Ok((input_ent, disabled, bg_color, font_color)) = q_inputs.get(ent) {
            set_text_input_styles(input_ent, disabled, bg_color, font_color, &mut commands);
        }
    });
}

fn set_text_input_styles(
    input_ent: Entity,
    disabled: bool,
    bg_color: &ThemeBackgroundColor,
    font_color: &ThemeTextColor,
    commands: &mut Commands,
) {
    let (bg_token, font_color_token) = match disabled {
        true => (
            tokens::TEXT_INPUT_BG_DISABLED,
            tokens::TEXT_INPUT_TEXT_DISABLED,
        ),
        false => (tokens::TEXT_INPUT_BG, tokens::TEXT_INPUT_TEXT),
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Text,
    };

    // Change background color
    if bg_color.0 != bg_token {
        commands
            .entity(input_ent)
            .insert(ThemeBackgroundColor(bg_token));
    }

    // Change font color
    if font_color.0 != font_color_token {
        commands
            .entity(input_ent)
            .insert(ThemeTextColor(font_color_token));
    }

    // Change cursor shape
    commands
        .entity(input_ent)
        .insert(EntityCursor::System(cursor_shape));
}

/// Plugin which registers the systems for updating the text input styles.
pub struct TextInputPlugin;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (
                update_text_cursor_color,
                update_text_input_styles,
                update_text_input_styles_remove,
            )
                .in_set(PickingSystems::Last),
        );
    }
}
