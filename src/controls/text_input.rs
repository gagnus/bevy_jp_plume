//! Editable text field and its decorative container.
use bevy::app::{Plugin, PreUpdate};
use bevy::camera::visibility::Visibility;
use bevy::ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    observer::On,
    query::{Added, Changed, Has, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
};
use bevy::input_focus::{InputFocus, tab_navigation::TabIndex};
use bevy::picking::{Pickable, PickingSystems};
use bevy::reflect::Reflect;
use bevy::reflect::std_traits::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::text::{
    EditableText, EditableTextFilter, LineBreak, LineHeight, TextCursorStyle, TextEdit, TextLayout, TextReadWriteMode,
};
use bevy::ui::{
    AlignItems, ComputedUiRenderTargetInfo, InteractionDisabled, Node, PositionType, UiRect, Val,
    widget::Text,
};
use bevy::ui_widgets::TextInput;

use crate::{
    constants::size,
    controls::DefaultWidth,
    cursor::EntityCursor,
    focus::FocusWithinIndicator,
    font_styles::TextStyleRelay,
    theme::{ThemeBackgroundToken, ThemeBorderToken, ThemeTextToken, ThemedText, UiTheme},
    tokens,
};

/// Horizontal inset of the field content (border + padding = GAP, aligning the text
/// with button captions); the placeholder overlay must match it.
const TEXT_INPUT_PAD_X: Val = match size::GAP.try_sub(size::CONTAINER_BORDER) {
    Ok(inset) => inset,
    Err(_) => unreachable!(),
};

/// A single-line text input: a themed frame (background, border, focus ring, sizing) wrapping an
/// inner editable field and an optional suffix label.
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

/// The field's text, mirrored onto the [`PlumeTextInput`] frame root: the scene
/// seeds it as the initial value, and every buffer edit is reflected back into it
/// (the imm layer reads widget state from roots only). Write through
/// [`SetTextInputValue`], not by re-inserting this.
#[derive(Component, Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[reflect(Component, Default)]
pub struct TextInputValue(pub String);

/// Programmatically replace a [`PlumeTextInput`]'s text; the field's buffer and
/// [`TextInputValue`] both follow.
#[derive(EntityEvent, Reflect)]
pub struct SetTextInputValue {
    /// The [`PlumeTextInput`] frame root.
    pub entity: Entity,
    /// Replacement text.
    pub text: String,
}

/// Props used to construct the [`PlumeTextInput`] scene.
#[derive(Default, Clone)]
pub struct PlumeTextInputProps {
    /// Initial text.
    pub value: String,
    /// Visible width
    pub visible_width: Option<f32>,
    /// Max characters
    pub max_characters: Option<usize>,
    /// Optional per-character filter rejecting disallowed input.
    pub filter: Option<EditableTextFilter>,
    /// Optional dim hint shown while the field is empty and unfocused.
    pub placeholder: Option<String>,
    /// Optional non-editable suffix shown after the text (a unit such as `px`, `%`, or `°`).
    pub suffix: Option<String>,
}

impl PlumeTextInput {
    fn scene(props: PlumeTextInputProps) -> impl Scene {
        bsn! {
            text_input_frame()
            TextInputValue({props.value})
            Children [
                (
                    text_input_field(props.visible_width, props.max_characters)
                    {props.filter.map(|filter| bsn!(template_value(filter)))}
                ),
                {props.placeholder.map(|placeholder| bsn_list!(text_input_placeholder(placeholder)))},
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
            padding: UiRect::new(TEXT_INPUT_PAD_X, TEXT_INPUT_PAD_X, Val::Px(1.0), Val::ZERO),
            border: size::CONTAINER_BORDER,
            border_radius: size::CORNER_RADIUS,
            // Two glyphs' worth plus padding, in em — the ballpark, not layout math.
            min_width: {size::em_from_px(40.0)},
        }
        // An empty field measures nothing, so `width: auto` would collapse it.
        DefaultWidth({size::em_from_px(124.0)})
        PlumeTextInput
        TextStyleRelay
        // Ring around the frame while the inner field holds focus.
        FocusWithinIndicator
        ThemeBackgroundToken(tokens::TEXT_INPUT_BG)
        ThemeBorderToken(tokens::TEXT_INPUT_BORDER)
        // On the frame so the whole box (padding included) shows the text cursor; the cursor
        // resolver walks up from the hovered field to find it.
        EntityCursor::System(bevy::window::SystemCursorIcon::Text)
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
        ThemeTextToken(tokens::TEXT_INPUT_TEXT)
        TabIndex(0)
        TextInput
        template_value(TextReadWriteMode::Editable)
        EditableText {
            cursor_width: 0.3,
            visible_width: {visible_width},
            max_characters: {max_characters},
        }
        ThemedText
        TextLayout {
            linebreak: LineBreak::NoWrap,
        }
        template_value(LineHeight::RelativeToFont(20.0 / size::MEDIUM_FONT_PX))
        TextCursorStyle::default()
    }
}

/// Replace the buffer contents (select-all + insert) when they differ.
pub(crate) fn set_editable_text(editable_text: &mut EditableText, replacement: String) {
    if editable_text.value() != replacement.as_str() {
        editable_text.queue_edit(TextEdit::SelectAll);
        editable_text.queue_edit(TextEdit::Insert(replacement.into()));
    }
}

// Push a scene-seeded [`TextInputValue`] into the field's buffer. `Added` fires only
// for the scene insert, so this never fights the mirror below.
fn seed_text_input_value(
    q_seeded: Query<(Entity, &TextInputValue), (With<PlumeTextInput>, Added<TextInputValue>)>,
    q_children: Query<&Children>,
    mut q_fields: Query<&mut EditableText, With<TextInputField>>,
) {
    for (frame_ent, value) in q_seeded.iter() {
        let Ok(children) = q_children.get(frame_ent) else {
            continue;
        };
        let Some(field_ent) = children.iter().copied().find(|&c| q_fields.contains(c)) else {
            continue;
        };
        if let Ok(mut editable_text) = q_fields.get_mut(field_ent) {
            set_editable_text(&mut editable_text, value.0.clone());
        }
    }
}

// Reflect every buffer change into the frame's [`TextInputValue`] mirror.
fn mirror_text_input_value(
    q_changed: Query<(&ChildOf, &EditableText), (With<TextInputField>, Changed<EditableText>)>,
    q_frames: Query<Option<&TextInputValue>, With<PlumeTextInput>>,
    mut commands: Commands,
) {
    for (child_of, editable_text) in q_changed.iter() {
        // A queued (unapplied) edit means the buffer is stale — mirroring it now
        // would clobber the mirror with the pre-edit text for a frame.
        if !editable_text.pending_edits.is_empty() {
            continue;
        }
        let frame_ent = child_of.parent();
        let Ok(mirror) = q_frames.get(frame_ent) else {
            continue;
        };
        let text = editable_text.value().to_string();
        if mirror.is_none_or(|mirror| mirror.0 != text) {
            commands.entity(frame_ent).insert(TextInputValue(text));
        }
    }
}

fn text_input_on_set_value(
    ev: On<SetTextInputValue>,
    q_frames: Query<(), With<PlumeTextInput>>,
    q_children: Query<&Children>,
    mut q_fields: Query<&mut EditableText, With<TextInputField>>,
) {
    if !q_frames.contains(ev.entity) {
        return;
    }
    let Ok(children) = q_children.get(ev.entity) else {
        return;
    };
    let Some(field_ent) = children.iter().copied().find(|&c| q_fields.contains(c)) else {
        return;
    };
    if let Ok(mut editable_text) = q_fields.get_mut(field_ent) {
        set_editable_text(&mut editable_text, ev.text.clone());
    }
}

// Marker for a placeholder hint inside a text-input frame.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct TextInputPlaceholder;

// Dim hint overlaying the field; absolute with auto vertical insets, so the frame's
// align_items centers it without displacing the field.
pub(crate) fn text_input_placeholder(text: impl Into<String>) -> impl Scene {
    bsn! {
        Text(text)
        ThemeTextToken(tokens::TEXT_DIM)
        Node {
            position_type: PositionType::Absolute,
            left: TEXT_INPUT_PAD_X,
        }
        TextInputPlaceholder
        Pickable::IGNORE
    }
}

/// A non-interactive dim suffix (unit) shown after the editable field.
pub(crate) fn text_input_suffix(text: impl Into<String>) -> impl Scene {
    bsn! {
        Text(text)
        ThemeTextToken(tokens::TEXT_DIM)
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
    q_bg: Query<&ThemeBackgroundToken>,
    q_border: Query<&ThemeBorderToken>,
    q_text: Query<&ThemeTextToken>,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    for frame_ent in q_frames.iter() {
        let Some(field_ent) = get_field_ent(frame_ent, &q_children, &q_is_field) else {
            continue;
        };
        if focus.get() == Some(field_ent) {
            focus.clear();
        }
        set_text_input_styles(
            frame_ent,
            field_ent,
            true,
            false,
            &q_bg,
            &q_border,
            &q_text,
            &mut commands,
        );
    }
}

fn update_text_input_styles_remove(
    q_frames: Query<(), With<PlumeTextInput>>,
    q_children: Query<&Children>,
    q_is_field: Query<(), With<TextInputField>>,
    q_bg: Query<&ThemeBackgroundToken>,
    q_border: Query<&ThemeBorderToken>,
    q_text: Query<&ThemeTextToken>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    focus: Res<InputFocus>,
    mut commands: Commands,
) {
    removed_disabled.read().for_each(|frame_ent| {
        if q_frames.contains(frame_ent)
            && let Some(field_ent) = get_field_ent(frame_ent, &q_children, &q_is_field)
        {
            let focused = focus.get() == Some(field_ent);
            set_text_input_styles(
                frame_ent,
                field_ent,
                false,
                focused,
                &q_bg,
                &q_border,
                &q_text,
                &mut commands,
            );
        }
    });
}

/// Restyle every text input when focus moves, so the edited one gets the active border.
fn update_text_input_styles_focus(
    q_frames: Query<(Entity, Has<InteractionDisabled>), With<PlumeTextInput>>,
    q_children: Query<&Children>,
    q_is_field: Query<(), With<TextInputField>>,
    q_bg: Query<&ThemeBackgroundToken>,
    q_border: Query<&ThemeBorderToken>,
    q_text: Query<&ThemeTextToken>,
    focus: Res<InputFocus>,
    mut commands: Commands,
) {
    if !focus.is_changed() {
        return;
    }
    for (frame_ent, disabled) in q_frames.iter() {
        let Some(field_ent) = get_field_ent(frame_ent, &q_children, &q_is_field) else {
            continue;
        };
        let focused = focus.get() == Some(field_ent);
        set_text_input_styles(
            frame_ent,
            field_ent,
            disabled,
            focused,
            &q_bg,
            &q_border,
            &q_text,
            &mut commands,
        );
    }
}

/// The editable [`TextInputField`] child of a frame, or `None` while the frame's children are still
/// being spawned.
fn get_field_ent(
    frame_ent: Entity,
    q_children: &Query<&Children>,
    q_is_field: &Query<(), With<TextInputField>>,
) -> Option<Entity> {
    let children = q_children.get(frame_ent).ok()?;
    children
        .iter()
        .find(|&&child| q_is_field.contains(child))
        .copied()
}

#[allow(clippy::too_many_arguments)]
fn set_text_input_styles(
    frame_ent: Entity,
    field_ent: Entity,
    disabled: bool,
    focused: bool,
    q_bg: &Query<&ThemeBackgroundToken>,
    q_border: &Query<&ThemeBorderToken>,
    q_text: &Query<&ThemeTextToken>,
    commands: &mut Commands,
) {
    let (bg_token, font_token, border_token) = match (disabled, focused) {
        (true, _) => (
            tokens::TEXT_INPUT_BG_DISABLED,
            tokens::TEXT_INPUT_TEXT_DISABLED,
            tokens::TEXT_INPUT_BORDER_DISABLED,
        ),
        (false, true) => (
            tokens::TEXT_INPUT_BG_ACTIVE,
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
        true => bevy::window::SystemCursorIcon::NotAllowed,
        false => bevy::window::SystemCursorIcon::Text,
    };

    // Background and border chrome live on the frame. Skip redundant re-inserts so a focus
    // change doesn't churn change detection on inputs that already have the right tokens.
    if !q_bg.get(frame_ent).is_ok_and(|bg| bg.0 == bg_token) {
        commands
            .entity(frame_ent)
            .insert(ThemeBackgroundToken(bg_token));
    }
    if !q_border
        .get(frame_ent)
        .is_ok_and(|border| border.0 == border_token)
    {
        commands
            .entity(frame_ent)
            .insert(ThemeBorderToken(border_token));
    }

    // Text color lives on the editable field itself.
    if !q_text.get(field_ent).is_ok_and(|text| text.0 == font_token) {
        commands
            .entity(field_ent)
            .insert(ThemeTextToken(font_token));
    }

    commands
        .entity(frame_ent)
        .insert(EntityCursor::System(cursor_shape));

    // Without this a disabled input still acquires focus through the click-to-focus resolver
    // (`acquire_focus_tab_index`), and focus draws the blinking caret.
    if disabled {
        commands.entity(field_ent).remove::<TabIndex>();
    } else {
        commands.entity(field_ent).insert(TabIndex(0));
    }
}

/// Show each placeholder only while its sibling field is empty and unfocused.
fn update_text_input_placeholders(
    mut q_placeholders: Query<(&ChildOf, &mut Visibility), With<TextInputPlaceholder>>,
    q_fields: Query<&EditableText, With<TextInputField>>,
    q_changed_fields: Query<(), (Changed<EditableText>, With<TextInputField>)>,
    q_children: Query<&Children>,
    focus: Res<InputFocus>,
) {
    if !focus.is_changed() && q_changed_fields.is_empty() {
        return;
    }
    for (child_of, mut visibility) in q_placeholders.iter_mut() {
        let Ok(children) = q_children.get(child_of.parent()) else {
            continue;
        };
        let Some(field_ent) = children.iter().copied().find(|&c| q_fields.contains(c)) else {
            continue;
        };
        let Ok(editable_text) = q_fields.get(field_ent) else {
            continue;
        };
        let show = editable_text.value() == "" && focus.get() != Some(field_ent);
        let wanted = match show {
            true => Visibility::Inherited,
            false => Visibility::Hidden,
        };
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}

/// Plugin which registers the systems for updating the text input styles.
pub(crate) struct TextInputPlugin;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            (
                update_text_cursor_color,
                reapply_field_justify,
                update_text_input_styles,
                update_text_input_styles_remove,
                update_text_input_styles_focus,
                update_text_input_placeholders,
                seed_text_input_value,
                mirror_text_input_value,
            )
                .in_set(PickingSystems::Last),
        )
        .add_observer(text_input_on_set_value);
    }
}

// A BSN-spawned field's `TextLayout` change fires before it is layout-eligible, so
// upstream drops the justify; re-touch it once the field is realized.
fn reapply_field_justify(
    mut q_fields: Query<&mut TextLayout, (With<TextInputField>, Added<ComputedUiRenderTargetInfo>)>,
) {
    for mut layout in q_fields.iter_mut() {
        layout.set_changed();
    }
}
