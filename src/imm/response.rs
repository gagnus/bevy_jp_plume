//! Response returned by every imm widget call: state flags plus builder methods.
//! Builders are gated by [`kind`] markers, so only the widget they belong to
//! exposes them.
use core::marker::PhantomData;
use core::ops::RangeInclusive;

use bevy_ecs::{
    entity::Entity,
    hierarchy::ChildOf,
    world::{EntityWorldMut, World},
};
use bevy_immediate::{ImmEntity, imm_id, ui::disabled::ImmUiInteractionsDisabled};
use bevy_scene::WorldSceneExt;
use bevy_ui::{AlignItems, AlignSelf, Node, Val};
use bevy_ui_widgets::{SliderPrecision, SliderRange, SliderStep, SliderValue};

use super::PlumeCaps;
use crate::containers::SectionCollapsed;
use crate::controls::{
    ButtonVariant, PlumeNumberInput, set_select_max_visible, text_input_placeholder,
    text_input_suffix,
};

/// Zero-sized widget-kind markers for [`ImmResponse`]: each widget returns a
/// response typed to its kind, so kind-specific builders are compile-checked
/// (`ui.button(…).step(…)` doesn't exist). [`Any`] is the default for widgets
/// with only the universal builders.
pub mod kind {
    /// Kinds whose value is a stepped/rounded number: slider, number input.
    pub trait Numeric {}
    /// Kinds built on the text-input frame: number input, text edit.
    pub trait Field {}
    /// Kinds that lay out children on a flex axis: row, column.
    pub trait Container {}

    /// Default kind: universal builders only (caption, checkbox, toggle, radio).
    pub struct Any;
    /// `button` / `icon_button` / `tool_button`.
    pub struct Button;
    /// `slider`.
    pub struct Slider;
    /// `number`.
    pub struct Number;
    /// `text_edit`.
    pub struct Text;
    /// `select`.
    pub struct Select;
    /// `section`.
    pub struct Section;
    /// `horizontal`: children flow left-to-right, so its cross axis is vertical.
    pub struct Row;
    /// `vertical` / `group` / `screen`: children flow top-to-bottom, so its cross
    /// axis is horizontal.
    pub struct Column;

    impl Numeric for Slider {}
    impl Numeric for Number {}
    impl Field for Number {}
    impl Field for Text {}
    impl Container for Row {}
    impl Container for Column {}
}

/// What a widget reported this frame, plus chainable builders for
/// composition-rule props (`enabled`, …). `K` is the widget's [`kind`].
pub struct ImmResponse<'r, 'w, 's, K = kind::Any> {
    /// Activated this frame (pointer click or keyboard).
    pub clicked: bool,
    /// The user changed the widget's value this frame (already written back).
    pub changed: bool,
    /// The pointer is over the widget.
    pub hovered: bool,
    /// The widget's root entity — the escape hatch to the retained layer.
    pub entity: Entity,
    pub(crate) spawned: bool,
    pub(crate) e: ImmEntity<'r, 'w, 's, PlumeCaps>,
    pub(crate) kind: PhantomData<K>,
}

/// Universal builders, available on every kind.
impl<K> ImmResponse<'_, '_, '_, K> {
    /// Enable or disable the control (manages [`bevy_ui::InteractionDisabled`]).
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.e = self.e.interactions_enabled(enabled);
        self
    }

    /// Fill the remaining space in a row (`flex_grow` from a zero basis), e.g. a
    /// slider spanning the middle of a caption/value row.
    pub fn grow(mut self) -> Self {
        struct GrowKey;
        if self.e.hash_update_typ::<GrowKey>(Some(imm_id(true))) {
            self.e
                .entity_commands()
                .queue(|mut entity: EntityWorldMut| {
                    if let Some(mut node) = entity.get_mut::<Node>() {
                        node.width = Val::ZERO;
                        node.flex_grow = 1.0;
                    }
                });
        }
        self
    }

    /// Place this one child on its container's cross axis, overriding the
    /// container's `align_items`.
    ///
    /// `AlignSelf::Start` in a column is how to stop a control stretching to full
    /// width; `Stretch` only bites on children that don't fix their own size.
    pub fn align_self(mut self, align: AlignSelf) -> Self {
        struct AlignSelfKey;
        if self
            .e
            .hash_update_typ::<AlignSelfKey>(Some(imm_id(format!("{align:?}"))))
        {
            self.e
                .entity_commands()
                .queue(move |mut entity: EntityWorldMut| {
                    if let Some(mut node) = entity.get_mut::<Node>() {
                        node.align_self = align;
                    }
                });
        }
        self
    }

    /// Override the control's width. Written into the retained `Node` only when
    /// the value changes.
    pub fn width(mut self, width: Val) -> Self {
        struct WidthKey;
        if self
            .e
            .hash_update_typ::<WidthKey>(Some(imm_id(format!("{width:?}"))))
        {
            self.e
                .entity_commands()
                .queue(move |mut entity: EntityWorldMut| {
                    if let Some(mut node) = entity.get_mut::<Node>() {
                        node.width = width;
                    }
                });
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::Button> {
    /// Set the button's color variant (the styling systems re-style on change).
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        struct VariantKey;
        if self
            .e
            .hash_update_typ::<VariantKey>(Some(imm_id(format!("{variant:?}"))))
        {
            self.e.entity_commands().insert(variant);
        }
        self
    }

    /// Sugar for [`Self::variant`]`(ButtonVariant::Primary)` — the confirm button.
    pub fn primary(self) -> Self {
        self.variant(ButtonVariant::Primary)
    }
}

/// Builders shared by the numeric-valued kinds (slider, number input).
impl<K: kind::Numeric> ImmResponse<'_, '_, '_, K> {
    /// Set the increment applied by arrow keys (slider) or Up/Down in the field
    /// (number input). A slider defaults to 1% of its range, a number input to 1.
    pub fn step(mut self, step: f32) -> Self {
        struct StepKey;
        if self
            .e
            .hash_update_typ::<StepKey>(Some(imm_id(step.to_bits())))
        {
            self.e.entity_commands().insert(SliderStep(step));
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::Slider> {
    /// Decimal places drag values are rounded to (`0` = integer).
    pub fn precision(mut self, precision: usize) -> Self {
        struct PrecisionKey;
        if self
            .e
            .hash_update_typ::<PrecisionKey>(Some(imm_id(precision)))
        {
            self.e
                .entity_commands()
                .insert(SliderPrecision(precision as i32));
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::Number> {
    /// Set the committable range (clamps typed/stepped values).
    pub fn range(mut self, range: RangeInclusive<f32>) -> Self {
        let (min, max) = (*range.start(), *range.end());
        struct RangeKey;
        if self
            .e
            .hash_update_typ::<RangeKey>(Some(imm_id((min.to_bits(), max.to_bits()))))
        {
            self.e.entity_commands().insert(SliderRange::new(min, max));
        }
        self
    }

    /// Set the displayed/committed decimal precision (`0` = integer).
    /// Reprints the value.
    pub fn precision(mut self, precision: usize) -> Self {
        struct PrecisionKey;
        if self
            .e
            .hash_update_typ::<PrecisionKey>(Some(imm_id(precision)))
        {
            self.e
                .entity_commands()
                .queue(move |mut entity: EntityWorldMut| {
                    if let Some(mut number) = entity.get_mut::<PlumeNumberInput>() {
                        number.precision = precision;
                    }
                    // Re-touch the value so the text reformats at the new precision.
                    if let Some(value) = entity.get::<SliderValue>().map(|value| value.0) {
                        entity.insert(SliderValue(value));
                    }
                });
        }
        self
    }
}

/// Builders shared by the text-input-frame kinds (number input, text edit).
impl<K: kind::Field> ImmResponse<'_, '_, '_, K> {
    /// Append a dim, non-editable unit suffix to the input (e.g. `m/s`).
    /// Seeded on first spawn only — units don't change, and the id doesn't track it.
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        if self.spawned {
            let parent = self.entity;
            let suffix = suffix.into();
            self.e.commands().queue(move |world: &mut World| {
                if let Ok(mut child) = world.spawn_scene(text_input_suffix(suffix)) {
                    child.insert(ChildOf(parent));
                }
            });
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::Text> {
    /// Dim hint shown while the field is empty and unfocused.
    /// Seeded on first spawn only — hints don't change, and the id doesn't track it.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        if self.spawned {
            let parent = self.entity;
            let placeholder = placeholder.into();
            self.e.commands().queue(move |world: &mut World| {
                if let Ok(mut child) = world.spawn_scene(text_input_placeholder(placeholder)) {
                    child.insert(ChildOf(parent));
                }
            });
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::Select> {
    /// Cap the popup at `max_visible` rows before it scrolls (default 8).
    pub fn max_visible(mut self, max_visible: usize) -> Self {
        struct MaxVisibleKey;
        if self
            .e
            .hash_update_typ::<MaxVisibleKey>(Some(imm_id(max_visible)))
        {
            let select_ent = self.entity;
            self.e.commands().queue(move |world: &mut World| {
                set_select_max_visible(world, select_ent, max_visible);
            });
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::Section> {
    /// Seed the section collapsed on first spawn only; afterwards the retained
    /// entity owns its collapse state (a no-op on already-spawned widgets, so it
    /// never fights the user's expand/collapse).
    pub fn start_collapsed(mut self) -> Self {
        if self.spawned {
            self.e.entity_commands().insert(SectionCollapsed);
        }
        self
    }
}

/// Builders shared by the container kinds (row, column).
impl<K: kind::Container> ImmResponse<'_, '_, '_, K> {
    /// Place every child on the container's cross axis — `Start` means top on a
    /// [`Row`](kind::Row), left on a [`Column`](kind::Column).
    ///
    /// Columns default to `Stretch` (what `.grow()` in nested rows resolves
    /// against), rows to `Center`. For a single child, see [`Self::align_self`].
    pub fn align_items(mut self, align: AlignItems) -> Self {
        struct AlignItemsKey;
        if self
            .e
            .hash_update_typ::<AlignItemsKey>(Some(imm_id(format!("{align:?}"))))
        {
            self.e
                .entity_commands()
                .queue(move |mut entity: EntityWorldMut| {
                    if let Some(mut node) = entity.get_mut::<Node>() {
                        node.align_items = align;
                    }
                });
        }
        self
    }
}
