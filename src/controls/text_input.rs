//! Editable text field and its decorative container.
use bevy::app::{Plugin, PostUpdate, PreUpdate};
use bevy::camera::visibility::Visibility;
use bevy::ecs::change_detection::{DetectChanges, DetectChangesMut};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EntityEvent;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::lifecycle::RemovedComponents;
use bevy::ecs::observer::On;
use bevy::ecs::query::{Added, Changed, Has, Or, With, Without};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Query, Res, ResMut};
use bevy::input::ButtonState;
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusedInput, InputFocus};
use bevy::picking::{Pickable, PickingSystems};
use bevy::reflect::Reflect;
use bevy::reflect::std_traits::ReflectDefault;
use bevy::scene::prelude::*;
use bevy::text::{
    EditableText, EditableTextFilter, LineBreak, LineHeight, TextCursorStyle, TextEdit, TextFont,
    TextLayout, TextReadWriteMode,
};
use bevy::ui::widget::Text;
use bevy::ui::{
    AlignItems, ComputedUiRenderTargetInfo, InteractionDisabled, Node, PositionType, UiRect,
    UiSystems, Val,
};
use bevy::ui_widgets::{SelectAllOnFocus, TextInput, ValueChange};

use crate::constants::size;
use crate::controls::{ButtonVariant, DefaultWidth, SetValue};
use crate::cursor::EntityCursor;
use crate::font_styles::TextStyleRelay;
use crate::theme::{ThemeBackgroundToken, ThemeBorderToken, ThemeTextToken, ThemedText, UiTheme};
use crate::tokens;
use crate::utils::hierarchy::nearest_with;

/// A single-line text input: a themed frame (background, border, sizing) wrapping an
/// inner editable field and an optional suffix label. Enter releases focus —
/// an app treating blur as its commit signal gets Enter-to-commit with it.
///
/// # Emitted events
/// * [`ValueChange<String>`](bevy::ui_widgets::ValueChange) on each keystroke while focused.
#[derive(SceneComponent, Default, Clone)]
#[scene(PlumeTextInputProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
pub struct PlumeTextInput;

// Marker for the editable text entity nested inside a [`PlumeTextInput`] frame.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct TextInputField;

/// The field's text, mirrored onto the [`PlumeTextInput`] frame root. Write through
/// [`SetValue<String>`](crate::retained::SetValue), not by re-inserting this.
#[derive(Component, Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[reflect(Component, Default)]
pub struct TextInputValue(pub String);

/// Props used to construct the [`PlumeTextInput`] scene.
#[derive(Default)]
pub struct PlumeTextInputProps {
    /// Initial text.
    pub value: String,
    /// Visible width.
    pub visible_width: Option<f32>,
    /// Max characters.
    pub max_characters: Option<usize>,
    /// Optional per-character filter rejecting disallowed input.
    pub filter: Option<EditableTextFilter>,
    /// Optional dim hint shown while the field is empty and unfocused.
    pub placeholder: Option<String>,
    /// Optional non-editable suffix shown after the text (a unit such as `px`, `%`, or `°`).
    /// Exclusive with `suffix_container`.
    pub suffix: Option<String>,
    /// Optional content ahead of the text, inside the frame (e.g. a clear button).
    pub prefix_container: Option<Box<dyn SceneList>>,
    /// Optional content after the text, inside the frame. Replaces `suffix` —
    /// the trailing slot holds a unit label or content, never both.
    pub suffix_container: Option<Box<dyn SceneList>>,
}

impl PlumeTextInput {
    fn scene(props: PlumeTextInputProps) -> impl Scene {
        debug_assert!(
            !(props.suffix.is_some() && props.suffix_container.is_some()),
            "`suffix` and `suffix_container` are exclusive: the trailing slot holds one thing"
        );
        bsn! {
            text_input_frame()
            TextInputValue({props.value})
            Children [
                text_input_outline(),
                {props.prefix_container.map(|content| bsn_list![
                    text_input_prefix_container(content),
                ])},
                (
                    text_input_field(props.visible_width, props.max_characters)
                    {props.filter.map(|filter| bsn! { template_value(filter) })}
                    on(text_input_on_enter)
                    Children [
                        {props.placeholder.map(|placeholder| bsn_list![
                            text_input_placeholder(placeholder),
                        ])},
                    ]
                ),
                {props.suffix.map(|suffix| bsn_list![text_input_suffix(suffix)])},
                {props.suffix_container.map(|content| bsn_list![
                    text_input_suffix_container(content),
                ])},
            ]
        }
    }
}

// The themed frame shared by [`PlumeTextInput`] and the number input. Callers append the field
// (and optional suffix) as children.
pub(crate) fn text_input_frame() -> impl Scene {
    bsn! {
        // Horizontal padding = SPACE, so the text aligns with button captions; the row
        // centers the editable field and suffix on the cross axis. The border is a
        // child overlay, not a node border, so it stays out of this content box.
        Node {
            height: size::ROW_HEIGHT,
            align_items: AlignItems::Center,
            padding: UiRect::new(size::SPACE, size::SPACE, size::em_from_px(1.5), Val::ZERO),
            border_radius: size::CORNER_RADIUS_SMALL,
            min_width: size::em_from_px(40.0),
        }
        DefaultWidth(size::em_from_px(124.0))
        TextInputFrame
        TextStyleRelay
        ThemeBackgroundToken(tokens::TEXT_INPUT_BG)
        EntityCursor::System(bevy::window::SystemCursorIcon::Text)
    }
}

// Plain root marker on every frame [`text_input_frame`] builds — the number input's
// included, since it composes the same frame. The systems key on this, not the
// [`PlumeTextInput`] scene component, which must not be inserted into a shared piece.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct TextInputFrame;

// Marker for a text-input frame's border overlay; the border tokens are swapped
// on this, not on the frame.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct TextInputOutline;

// The frame's border, on an absolutely-positioned overlay child (as `PlumeButton`
// does) rather than the frame node. A node border insets the content box, so a px
// hairline inside em padding would shift the text by a font-dependent amount.
pub(crate) fn text_input_outline() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: Val::ZERO,
            right: Val::ZERO,
            top: Val::ZERO,
            bottom: Val::ZERO,
            border: size::HAIRLINE,
            border_radius: size::CORNER_RADIUS_SMALL,
        }
        TextInputOutline
        // Em-sized chrome needs the chain's `EmSize`.
        TextStyleRelay
        Pickable::IGNORE
        ThemeBorderToken(tokens::TEXT_INPUT_BORDER)
    }
}

// The inner editable text entity: fills the frame.
pub(crate) fn text_input_field(
    visible_width: Option<f32>,
    max_characters: Option<usize>,
) -> impl Scene {
    bsn! {
        // Fills the frame's content box so the click/edit target is the whole row, not a thin strip.
        // Centering is for the placeholder child; the editable text lays itself out.
        Node {
            flex_grow: 1.0,
            height: {Val::Percent(100.0)},
            align_items: AlignItems::Center,
        }
        TextInputField
        // The field is the text entity, so the inheritable color (which only propagates to
        // descendants) would never reach it; set it directly.
        ThemeTextToken(tokens::TEXT_INPUT_TEXT)
        TabIndex(0)
        TextInput
        SelectAllOnFocus
        template_value(TextReadWriteMode::Editable)
        EditableText {
            cursor_width: 0.3,
            visible_width: visible_width,
            max_characters: max_characters,
        }
        ThemedText
        TextLayout {
            linebreak: LineBreak::NoWrap,
        }
        template_value(LineHeight::RelativeToFont(20.0 / size::MEDIUM_FONT_PX))
        TextCursorStyle::default()
    }
}

// Enter releases focus — a single-line field's "done". Only on the plain text
// input's field: the number input's own key handler commits and steps too.
fn text_input_on_enter(
    key_input: On<FocusedInput<KeyboardInput>>,
    query_fields: Query<&ChildOf, With<TextInputField>>,
    query_no_blur: Query<(), With<NoBlurOnEnter>>,
    mut focus: ResMut<InputFocus>,
) {
    if key_input.input.state != ButtonState::Pressed {
        return;
    }
    let Ok(child_of) = query_fields.get(key_input.event_target()) else {
        return;
    };
    // The frame's opt-out: an app whose own Enter handling may keep the field
    // in play (a filter, a prompt) must not find it blurred when it declines.
    if query_no_blur.contains(child_of.parent()) {
        return;
    }
    if matches!(
        key_input.input.key_code,
        KeyCode::Enter | KeyCode::NumpadEnter
    ) {
        focus.clear();
    }
}

// Replace the buffer contents (select-all + insert) when they differ.
pub(crate) fn set_editable_text(editable_text: &mut EditableText, replacement: String) {
    // A queued replacement has not reached `value()` yet, so comparing against it
    // would stack a second select-all + insert and write the text twice.
    if !editable_text.pending_edits.is_empty() {
        return;
    }
    if editable_text.value() != replacement.as_str() {
        editable_text.queue_edit(TextEdit::SelectAll);
        editable_text.queue_edit(TextEdit::Insert(replacement.into()));
    }
}

/// Opt-out marker on a text-input frame ([`PlumeTextInput`] or
/// [`PlumeNumberInput`](crate::controls::PlumeNumberInput)).
///
/// On the frame rather than the field, so callers need not reach into its children;
/// a system relays it down.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct NoSelectAllOnFocus;

/// Opt-out marker on a [`PlumeTextInput`] frame: Enter keeps focus in the field,
/// for an app whose own Enter handling may leave it in play (a filter, a prompt).
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct NoBlurOnEnter;

// Relay the frame's [`NoSelectAllOnFocus`] to its field, in both directions. Running as a system
// (not at insertion time) means the field is always spawned by the time the marker is read.
fn sync_select_all_on_focus(
    q_added: Query<Entity, (With<TextInputFrame>, Added<NoSelectAllOnFocus>)>,
    q_frames: Query<(), With<TextInputFrame>>,
    q_children: Query<&Children>,
    q_is_field: Query<(), With<TextInputField>>,
    mut removed: RemovedComponents<NoSelectAllOnFocus>,
    mut commands: Commands,
) {
    for frame_ent in q_added.iter() {
        if let Some(field_ent) = get_field_ent(frame_ent, &q_children, &q_is_field) {
            commands.entity(field_ent).remove::<SelectAllOnFocus>();
        }
    }
    removed.read().for_each(|frame_ent| {
        if q_frames.contains(frame_ent)
            && let Some(field_ent) = get_field_ent(frame_ent, &q_children, &q_is_field)
        {
            commands.entity(field_ent).insert(SelectAllOnFocus);
        }
    });
}

// Marks a frame whose scene-seeded value has been pushed into its buffer.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct TextInputSeeded;

// Push a scene-seeded [`TextInputValue`] into the field's buffer, once. The mirror
// below re-inserts `TextInputValue`, which re-fires `Added`; seeding again there
// would queue a second replacement on top of the first and double the text.
fn seed_text_input_value(
    q_seeded: Query<
        (Entity, &TextInputValue),
        (
            With<TextInputFrame>,
            Added<TextInputValue>,
            Without<TextInputSeeded>,
        ),
    >,
    q_children: Query<&Children>,
    mut q_fields: Query<&mut EditableText, With<TextInputField>>,
    mut commands: Commands,
) {
    for (frame_ent, value) in q_seeded.iter() {
        commands.entity(frame_ent).insert(TextInputSeeded);
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

// Reflect every buffer change into the frame's [`TextInputValue`] mirror, and
// announce it as a `ValueChange<String>` when the user is the one typing.
fn mirror_text_input_value(
    q_changed: Query<
        (Entity, &ChildOf, &EditableText),
        (With<TextInputField>, Changed<EditableText>),
    >,
    q_frames: Query<Option<&TextInputValue>, With<TextInputFrame>>,
    focus: Res<InputFocus>,
    mut commands: Commands,
) {
    for (field_ent, child_of, editable_text) in q_changed.iter() {
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
            commands
                .entity(frame_ent)
                .insert(TextInputValue(text.clone()));
            // Only a focused field can be the user typing; seeding and
            // `SetValue` moves the buffer too, and must stay silent.
            if focus.get() == Some(field_ent) {
                // The field has no separate commit step, so every keystroke is final.
                commands.trigger(ValueChange {
                    source: frame_ent,
                    value: text,
                    is_final: true,
                });
            }
        }
    }
}

fn text_input_on_set_value(
    ev: On<SetValue<String>>,
    q_frames: Query<(), With<TextInputFrame>>,
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
        set_editable_text(&mut editable_text, ev.value.clone());
    }
}

// Marker for a placeholder hint inside a text-input frame.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct TextInputPlaceholder;

// Marker for the dim, non-editable text in an input frame: the placeholder and the
// suffix. They are siblings of the field, not descendants, so nothing reaches them by
// inheritance and the frame's styler has to color them alongside it.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
struct TextInputDimText;

// Dim hint over the field — a child of it, not the frame, so it starts where the
// text it stands in for would, whatever adornments sit ahead of the field.
// Absolute with auto vertical insets, so the field's align_items centers it.
pub(crate) fn text_input_placeholder(text: impl Into<String>) -> impl Scene {
    bsn! {
        Text(text)
        ThemeTextToken(tokens::TEXT_DIM)
        Node {
            position_type: PositionType::Absolute,
            left: Val::ZERO,
        }
        TextInputPlaceholder
        TextInputDimText
        Pickable::IGNORE
    }
}

// A non-interactive dim suffix (unit) shown after the editable field.
pub(crate) fn text_input_suffix(text: impl Into<String>) -> impl Scene {
    bsn! {
        Text(text)
        ThemeTextToken(tokens::TEXT_DIM)
        Node {
            margin: UiRect::left(size::SPACE_TIGHT),
        }
        TextInputDimText
        // Never steal the click that focuses the field.
        Pickable::IGNORE
    }
}

// Marks a frame's leading content slot, kept ahead of the field wherever its
// spawner appended it.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct TextInputPrefix;

// Content ahead of the editable field, inside the frame — a clear button, an icon.
// Interactive, unlike the dim text pieces: clicks land on the content, not the field.
pub(crate) fn text_input_prefix_container(content: Box<dyn SceneList>) -> impl Scene {
    bsn! {
        Node {
            align_items: AlignItems::Center,
            margin: UiRect::right(size::SPACE_TIGHT),
            flex_shrink: 0.0,
        }
        TextInputPrefix
        TextStyleRelay
        Children [
            {content},
        ]
    }
}

// Marks a frame's trailing content slot, for the padding tightener below.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct TextInputSuffixContainer;

// Content after the editable field, inside the frame; the trailing counterpart
// of [`text_input_prefix_container`].
pub(crate) fn text_input_suffix_container(content: Box<dyn SceneList>) -> impl Scene {
    bsn! {
        Node {
            align_items: AlignItems::Center,
            margin: UiRect::left(size::SPACE_TIGHT),
            flex_shrink: 0.0,
        }
        TextInputSuffixContainer
        TextStyleRelay
        Children [
            {content},
        ]
    }
}

// A content adornment sits in the end padding meant to inset bare text; drop that
// end to SPACE_TIGHT. Child-list-driven, so late imm adornments re-derive it.
fn tighten_adorned_frame_padding(
    mut q_frames: Query<(&Children, &mut Node), (With<TextInputFrame>, Changed<Children>)>,
    q_prefix: Query<(), With<TextInputPrefix>>,
    q_suffix: Query<(), With<TextInputSuffixContainer>>,
) {
    for (children, mut node) in q_frames.iter_mut() {
        let end = |occupied: bool| match occupied {
            true => size::SPACE_TIGHT,
            false => size::SPACE,
        };
        let left = end(children.iter().any(|&child| q_prefix.contains(child)));
        let right = end(children.iter().any(|&child| q_suffix.contains(child)));
        if node.padding.left != left || node.padding.right != right {
            node.padding.left = left;
            node.padding.right = right;
        }
    }
}

// An imm-built prefix is appended after the scene's own children (the reconciler
// only parents, it does not order); move it ahead of the field on arrival.
fn order_text_input_prefixes(
    q_added: Query<(Entity, &ChildOf), Added<TextInputPrefix>>,
    q_frames: Query<&Children, With<TextInputFrame>>,
    q_is_field: Query<(), With<TextInputField>>,
    mut commands: Commands,
) {
    for (prefix_ent, child_of) in q_added.iter() {
        let Ok(children) = q_frames.get(child_of.parent()) else {
            continue;
        };
        let Some(field_index) = children
            .iter()
            .position(|&child| q_is_field.contains(child))
        else {
            continue;
        };
        let prefix_index = children.iter().position(|&child| child == prefix_ent);
        if prefix_index.is_some_and(|index| index > field_index) {
            commands
                .entity(child_of.parent())
                .insert_children(field_index, &[prefix_ent]);
        }
    }
}

// Theme the caret and selection of every field: on a theme change, and on each
// field as it appears.
fn update_text_cursor_color(
    mut q_field: Query<&mut TextCursorStyle, With<TextInputField>>,
    theme: Res<UiTheme>,
) {
    let theme_changed = theme.is_changed();
    for mut cursor_style in q_field.iter_mut() {
        if !theme_changed && !cursor_style.is_added() {
            continue;
        }
        let themed = TextCursorStyle {
            color: theme.color(&tokens::TEXT_INPUT_CURSOR),
            selection_color: theme.color(&tokens::TEXT_INPUT_SELECTION),
            unfocused_selection_color: theme.color(&tokens::TEXT_INPUT_SELECTION_UNFOCUSED),
            ..*cursor_style
        };
        // A `Changed` tick here re-extracts the node for rendering, so write
        // only a real difference.
        cursor_style.set_if_neq(themed);
    }
}

fn update_text_input_styles(
    q_frames: Query<Entity, (With<TextInputFrame>, Added<InteractionDisabled>)>,
    q_children: Query<&Children>,
    q_is_field: Query<(), With<TextInputField>>,
    q_is_outline: Query<(), With<TextInputOutline>>,
    q_bg: Query<&ThemeBackgroundToken>,
    q_border: Query<&ThemeBorderToken>,
    q_text: Query<&ThemeTextToken>,
    q_dim: Query<(), With<TextInputDimText>>,
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
            &q_children,
            &q_is_outline,
            &q_bg,
            &q_border,
            &q_text,
            &q_dim,
            &mut commands,
        );
    }
}

fn update_text_input_styles_remove(
    q_frames: Query<(), With<TextInputFrame>>,
    q_children: Query<&Children>,
    q_is_field: Query<(), With<TextInputField>>,
    q_is_outline: Query<(), With<TextInputOutline>>,
    q_bg: Query<&ThemeBackgroundToken>,
    q_border: Query<&ThemeBorderToken>,
    q_text: Query<&ThemeTextToken>,
    q_dim: Query<(), With<TextInputDimText>>,
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
                &q_children,
                &q_is_outline,
                &q_bg,
                &q_border,
                &q_text,
                &q_dim,
                &mut commands,
            );
        }
    });
}

// Restyle every text input when focus moves, so the edited one gets the active border.
fn update_text_input_styles_focus(
    q_frames: Query<(Entity, Has<InteractionDisabled>), With<TextInputFrame>>,
    q_children: Query<&Children>,
    q_is_field: Query<(), With<TextInputField>>,
    q_is_outline: Query<(), With<TextInputOutline>>,
    q_bg: Query<&ThemeBackgroundToken>,
    q_border: Query<&ThemeBorderToken>,
    q_text: Query<&ThemeTextToken>,
    q_dim: Query<(), With<TextInputDimText>>,
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
            &q_children,
            &q_is_outline,
            &q_bg,
            &q_border,
            &q_text,
            &q_dim,
            &mut commands,
        );
    }
}

// The editable [`TextInputField`] child of a frame, or `None` while the frame's children are still
// being spawned.
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

// The [`TextInputOutline`] child of a frame — the entity carrying the border, so
// the border tokens go here rather than on the frame.
fn get_outline_ent(
    frame_ent: Entity,
    q_children: &Query<&Children>,
    q_is_outline: &Query<(), With<TextInputOutline>>,
) -> Option<Entity> {
    let children = q_children.get(frame_ent).ok()?;
    children
        .iter()
        .find(|&&child| q_is_outline.contains(child))
        .copied()
}

#[allow(clippy::too_many_arguments)]
fn set_text_input_styles(
    frame_ent: Entity,
    field_ent: Entity,
    disabled: bool,
    focused: bool,
    q_children: &Query<&Children>,
    q_is_outline: &Query<(), With<TextInputOutline>>,
    q_bg: &Query<&ThemeBackgroundToken>,
    q_border: &Query<&ThemeBorderToken>,
    q_text: &Query<&ThemeTextToken>,
    q_dim: &Query<(), With<TextInputDimText>>,
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

    // Background lives on the frame, the border on its overlay child. Skip redundant
    // re-inserts so a focus change doesn't churn change detection on inputs that
    // already have the right tokens.
    if !q_bg.get(frame_ent).is_ok_and(|bg| bg.0 == bg_token) {
        commands
            .entity(frame_ent)
            .insert(ThemeBackgroundToken(bg_token));
    }
    if let Some(outline_ent) = get_outline_ent(frame_ent, q_children, q_is_outline)
        && !q_border
            .get(outline_ent)
            .is_ok_and(|border| border.0 == border_token)
    {
        commands
            .entity(outline_ent)
            .insert(ThemeBorderToken(border_token));
    }

    // Text color lives on the editable field itself.
    if !q_text.get(field_ent).is_ok_and(|text| text.0 == font_token) {
        commands
            .entity(field_ent)
            .insert(ThemeTextToken(font_token));
    }

    // Placeholder and suffix dim with the frame; the suffix is a frame child,
    // the placeholder the field's.
    let dim_token = if disabled {
        tokens::TEXT_INPUT_TEXT_DISABLED
    } else {
        tokens::TEXT_DIM
    };
    let dim_children = q_children
        .get(frame_ent)
        .into_iter()
        .flatten()
        .chain(q_children.get(field_ent).into_iter().flatten());
    for &child in dim_children.filter(|&&child| q_dim.contains(child)) {
        if !q_text.get(child).is_ok_and(|text| text.0 == dim_token) {
            commands
                .entity(child)
                .insert(ThemeTextToken(dim_token.clone()));
        }
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

// Mirror the frame's `InteractionDisabled` onto plume buttons inside its
// adornment containers, so a disabled input's embedded controls gray and refuse
// picks with it (the relay the select does for its internal button).
fn sync_adornment_disabled(
    q_newly_disabled: Query<Entity, (With<TextInputFrame>, Added<InteractionDisabled>)>,
    q_new_buttons: Query<Entity, Added<ButtonVariant>>,
    q_frames: Query<Has<InteractionDisabled>, With<TextInputFrame>>,
    q_childof: Query<&ChildOf>,
    q_children: Query<&Children>,
    q_container: Query<(), Or<(With<TextInputPrefix>, With<TextInputSuffixContainer>)>>,
    q_button: Query<(), With<ButtonVariant>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    fn relay(
        frame_ent: Entity,
        disabled: bool,
        q_children: &Query<&Children>,
        q_container: &Query<(), Or<(With<TextInputPrefix>, With<TextInputSuffixContainer>)>>,
        q_button: &Query<(), With<ButtonVariant>>,
        commands: &mut Commands,
    ) {
        for &container in q_children.get(frame_ent).into_iter().flatten() {
            if !q_container.contains(container) {
                continue;
            }
            for target in q_children.iter_descendants(container) {
                if q_button.contains(target) {
                    match disabled {
                        true => commands.entity(target).insert(InteractionDisabled),
                        false => commands.entity(target).remove::<InteractionDisabled>(),
                    };
                }
            }
        }
    }
    for frame_ent in q_newly_disabled.iter() {
        relay(
            frame_ent,
            true,
            &q_children,
            &q_container,
            &q_button,
            &mut commands,
        );
    }
    removed_disabled.read().for_each(|frame_ent| {
        if q_frames.contains(frame_ent) {
            relay(
                frame_ent,
                false,
                &q_children,
                &q_container,
                &q_button,
                &mut commands,
            );
        }
    });
    // A button spawned into an already-disabled frame's adornment (imm rebuilds
    // its container content) starts disabled too.
    for button_ent in q_new_buttons.iter() {
        let disabled = nearest_with(button_ent, &q_childof, &q_container)
            .and_then(|container| q_childof.get(container).ok())
            .and_then(|frame| q_frames.get(frame.parent()).ok())
            .unwrap_or(false);
        if disabled {
            commands.entity(button_ent).insert(InteractionDisabled);
        }
    }
}

// Show each placeholder only while its parent field is empty and unfocused.
fn update_text_input_placeholders(
    mut q_placeholders: Query<(&ChildOf, &mut Visibility), With<TextInputPlaceholder>>,
    q_fields: Query<&EditableText, With<TextInputField>>,
    q_changed_fields: Query<(), (Changed<EditableText>, With<TextInputField>)>,
    focus: Res<InputFocus>,
) {
    if !focus.is_changed() && q_changed_fields.is_empty() {
        return;
    }
    for (child_of, mut visibility) in q_placeholders.iter_mut() {
        let field_ent = child_of.parent();
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

// Plugin which registers the systems for updating the text input styles.
pub(crate) struct TextInputPlugin;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PreUpdate,
            (
                update_text_cursor_color,
                reapply_field_text_styles,
                update_text_input_styles,
                update_text_input_styles_remove,
                update_text_input_styles_focus,
                update_text_input_placeholders,
                sync_select_all_on_focus,
                sync_adornment_disabled,
                seed_text_input_value,
                mirror_text_input_value,
            )
                .in_set(PickingSystems::Last),
        )
        // Before layout, so an imm adornment spawned this frame is ordered and
        // padded before it is ever drawn, not one frame later.
        .add_systems(
            PostUpdate,
            (order_text_input_prefixes, tighten_adorned_frame_padding)
                .chain()
                .before(UiSystems::Layout),
        )
        .add_observer(text_input_on_set_value);
    }
}

// Upstream's editable-style sync misses `TextLayout`/`LineHeight` changes made
// before target info or a loading font arrives; re-touch both when a gate re-opens.
fn reapply_field_text_styles(
    mut q_fields: Query<
        (&mut TextLayout, &mut LineHeight),
        (
            With<TextInputField>,
            Or<(Added<ComputedUiRenderTargetInfo>, Changed<TextFont>)>,
        ),
    >,
) {
    for (mut layout, mut line_height) in q_fields.iter_mut() {
        layout.set_changed();
        line_height.set_changed();
    }
}
