//! Editable text field and its decorative container.
//!
//! A [`PlumeTextInput`] is a frame (background, border, sizing, focus ring) wrapping an inner
//! editable [`TextInputField`] child plus an optional, non-interactive suffix label (a unit such
//! as `px`, `%`, or `°`). The editable child is a taffy leaf — its glyphs come from the text
//! editor's own buffer, so the suffix cannot live inside it and must be a sibling under the frame.
use bevy_app::{Plugin, PreUpdate, PropagateOver};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    hierarchy::Children,
    lifecycle::RemovedComponents,
    query::{Added, Has, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
    template::template,
};
use bevy_input_focus::{InputFocus, tab_navigation::TabIndex};
use bevy_picking::{Pickable, PickingSystems};
use bevy_reflect::Reflect;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_scene::prelude::*;
use bevy_text::{
    EditableText, FontSourceTemplate, FontWeight, LineBreak, LineHeight, TextCursorStyle, TextFont,
    TextLayout,
};
use bevy_ui::{AlignItems, BorderRadius, InteractionDisabled, Node, UiRect, Val, px};

use crate::{
    constants::{fonts, size},
    cursor::EntityCursor,
    display::label_dim,
    theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemedText, UiTheme},
    tokens,
};

/// A single-line text input: a themed frame (background, border, focus ring, sizing) wrapping an
/// inner editable [`TextInputField`] and an optional suffix label.
///
/// This is spawnable by inheriting it as a "scene component" with optional [`PlumeTextInputProps`].
#[derive(SceneComponent, Default, Clone)]
#[scene(PlumeTextInputProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
pub struct PlumeTextInput;

/// Marker for the editable text entity nested inside a [`PlumeTextInput`] frame. This is the
/// entity that actually holds focus and the [`EditableText`] buffer; the frame styling systems
/// find it via the frame's children.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct TextInputField;

/// Props used to construct the [`PlumeTextInput`] scene.
#[derive(Default, Clone)]
pub struct PlumeTextInputProps {
    /// Visible width
    pub visible_width: Option<f32>,
    /// Max characters
    pub max_characters: Option<usize>,
    /// Optional non-editable suffix shown after the text (a unit such as `px`, `%`, or `°`).
    pub suffix: Option<String>,
}

impl PlumeTextInput {
    fn scene(props: PlumeTextInputProps) -> impl Scene {
        bsn! {
            text_input_frame()
            Children [
                (text_input_field(props.visible_width, props.max_characters)),
                {props.suffix.map(|suffix| bsn_list!(text_input_suffix(suffix)))}
            ]
        }
    }
}

/// The themed frame shared by [`PlumeTextInput`] and the number input: background, border, focus
/// ring, sizing, and the horizontal row that lays out the editable field beside its suffix. Carries
/// the [`PlumeTextInput`] marker so the frame styling systems drive it. Callers append the field
/// (and optional suffix) as children.
pub(crate) fn text_input_frame() -> impl Scene {
    bsn! {
        // Border + horizontal padding = GAP, so the text aligns with button captions; the row
        // centers the editable field and suffix on the cross axis.
        Node {
            height: size::ROW_HEIGHT,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(px(6.0)),
            border: size::CONTAINER_BORDER,
            border_radius: {BorderRadius::all(px(size::CORNER_RADIUS))},
            width: size::CONTROL_WIDTH,
        }
        PlumeTextInput
        ThemeBackgroundColor(tokens::TEXT_INPUT_BG)
        ThemeBorderColor(tokens::TEXT_INPUT_BORDER)
        // On the frame so the whole box (padding included) shows the text cursor; the cursor
        // resolver walks up from the hovered field to find it.
        EntityCursor::System(bevy_window::SystemCursorIcon::Text)
    }
}

/// The inner editable text entity: fills the frame, holds focus and the [`EditableText`] buffer.
pub(crate) fn text_input_field(
    visible_width: Option<f32>,
    max_characters: Option<usize>,
) -> impl Scene {
    bsn! {
        // Fills the frame's content box so the click/edit target is the whole row, not a thin strip.
        Node {
            flex_grow: 1.0,
            height: {Val::Percent(100.0)},
        }
        TextInputField
        // The field is the text entity, so the inheritable color (which only propagates to
        // descendants) would never reach it; set it directly.
        ThemeTextColor(tokens::TEXT_INPUT_TEXT)
        // Click-to-focus marker only; plume registers no Tab-key navigation.
        TabIndex(0)
        EditableText {
            cursor_width: 0.3,
            visible_width: {visible_width},
            max_characters: {max_characters},
        }
        ThemedText
        TextLayout {
            linebreak: LineBreak::NoWrap,
        }
        // Line height fills the content box exactly: a taller line overflows and makes the edit
        // viewport re-clamp (1px jitter) while typing.
        template(|_| Ok(LineHeight::Px(20.0)))
        TextFont {
            font: FontSourceTemplate::Handle(fonts::REGULAR),
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
        PropagateOver<TextFont>
        TextCursorStyle::default()
    }
}

/// A non-interactive dim suffix (unit) shown after the editable field.
pub(crate) fn text_input_suffix(text: impl Into<String>) -> impl Scene {
    bsn! {
        label_dim(text)
        Node {
            margin: {UiRect::left(size::GAP_TIGHT)},
        }
        // Never steal the click that focuses the field.
        Pickable::IGNORE
    }
}

fn update_text_cursor_color(
    mut q_field: Query<&mut TextCursorStyle, With<TextInputField>>,
    theme: Res<UiTheme>,
) {
    if theme.is_changed() {
        for mut cursor_style in q_field.iter_mut() {
            cursor_style.color = theme.color(&tokens::TEXT_INPUT_CURSOR);
            cursor_style.selection_color = theme.color(&tokens::TEXT_INPUT_SELECTION);
            cursor_style.unfocused_selection_color =
                theme.color(&tokens::TEXT_INPUT_SELECTION_UNFOCUSED);
        }
    }
}

fn update_text_input_styles(
    q_frames: Query<Entity, (With<PlumeTextInput>, Added<InteractionDisabled>)>,
    q_children: Query<&Children>,
    q_is_field: Query<(), With<TextInputField>>,
    q_bg: Query<&ThemeBackgroundColor>,
    q_border: Query<&ThemeBorderColor>,
    q_text: Query<&ThemeTextColor>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    for frame in q_frames.iter() {
        let Some(field) = field_of(frame, &q_children, &q_is_field) else {
            continue;
        };
        if focus.get() == Some(field) {
            focus.clear();
        }
        set_text_input_styles(
            frame, field, true, false, &q_bg, &q_border, &q_text, &mut commands,
        );
    }
}

fn update_text_input_styles_remove(
    q_frames: Query<(), With<PlumeTextInput>>,
    q_children: Query<&Children>,
    q_is_field: Query<(), With<TextInputField>>,
    q_bg: Query<&ThemeBackgroundColor>,
    q_border: Query<&ThemeBorderColor>,
    q_text: Query<&ThemeTextColor>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    focus: Res<InputFocus>,
    mut commands: Commands,
) {
    removed_disabled.read().for_each(|frame| {
        if q_frames.contains(frame)
            && let Some(field) = field_of(frame, &q_children, &q_is_field)
        {
            let focused = focus.get() == Some(field);
            set_text_input_styles(
                frame, field, false, focused, &q_bg, &q_border, &q_text, &mut commands,
            );
        }
    });
}

/// Restyle every text input when focus moves, so the edited one gets the active border.
fn update_text_input_styles_focus(
    q_frames: Query<(Entity, Has<InteractionDisabled>), With<PlumeTextInput>>,
    q_children: Query<&Children>,
    q_is_field: Query<(), With<TextInputField>>,
    q_bg: Query<&ThemeBackgroundColor>,
    q_border: Query<&ThemeBorderColor>,
    q_text: Query<&ThemeTextColor>,
    focus: Res<InputFocus>,
    mut commands: Commands,
) {
    if !focus.is_changed() {
        return;
    }
    for (frame, disabled) in q_frames.iter() {
        let Some(field) = field_of(frame, &q_children, &q_is_field) else {
            continue;
        };
        let focused = focus.get() == Some(field);
        set_text_input_styles(
            frame, field, disabled, focused, &q_bg, &q_border, &q_text, &mut commands,
        );
    }
}

/// The editable [`TextInputField`] child of a frame, or `None` while the frame's children are still
/// being spawned.
fn field_of(
    frame: Entity,
    q_children: &Query<&Children>,
    q_is_field: &Query<(), With<TextInputField>>,
) -> Option<Entity> {
    let children = q_children.get(frame).ok()?;
    children
        .iter()
        .find(|&&child| q_is_field.contains(child))
        .copied()
}

#[allow(clippy::too_many_arguments)]
fn set_text_input_styles(
    frame: Entity,
    field: Entity,
    disabled: bool,
    focused: bool,
    q_bg: &Query<&ThemeBackgroundColor>,
    q_border: &Query<&ThemeBorderColor>,
    q_text: &Query<&ThemeTextColor>,
    commands: &mut Commands,
) {
    let (bg_token, font_token, border_token) = match (disabled, focused) {
        (true, _) => (
            tokens::TEXT_INPUT_BG_DISABLED,
            tokens::TEXT_INPUT_TEXT_DISABLED,
            tokens::TEXT_INPUT_BORDER_DISABLED,
        ),
        (false, true) => (
            tokens::TEXT_INPUT_BG,
            tokens::TEXT_INPUT_TEXT_ACTIVE,
            tokens::TEXT_INPUT_BORDER_ACTIVE,
        ),
        (false, false) => (
            tokens::TEXT_INPUT_BG,
            tokens::TEXT_INPUT_TEXT,
            tokens::TEXT_INPUT_BORDER,
        ),
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Text,
    };

    // Background and border chrome live on the frame. Skip redundant re-inserts so a focus
    // change doesn't churn change detection on inputs that already have the right tokens.
    if !q_bg.get(frame).is_ok_and(|bg| bg.0 == bg_token) {
        commands
            .entity(frame)
            .insert(ThemeBackgroundColor(bg_token));
    }
    if !q_border.get(frame).is_ok_and(|border| border.0 == border_token) {
        commands
            .entity(frame)
            .insert(ThemeBorderColor(border_token));
    }

    // Text color lives on the editable field itself.
    if !q_text.get(field).is_ok_and(|text| text.0 == font_token) {
        commands.entity(field).insert(ThemeTextColor(font_token));
    }

    commands
        .entity(frame)
        .insert(EntityCursor::System(cursor_shape));

    // Without this a disabled input still acquires focus through the click-to-focus resolver
    // (`acquire_focus_tab_index`), and focus draws the blinking caret.
    if disabled {
        commands.entity(field).remove::<TabIndex>();
    } else {
        commands.entity(field).insert(TabIndex(0));
    }
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
                update_text_input_styles_focus,
            )
                .in_set(PickingSystems::Last),
        );
    }
}
