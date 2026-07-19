//! Response returned by every imm widget call: state flags plus builder methods.
use core::ops::RangeInclusive;

use bevy_ecs::{
    entity::Entity,
    hierarchy::ChildOf,
    world::{EntityWorldMut, World},
};
use bevy_immediate::{ImmEntity, imm_id, ui::disabled::ImmUiInteractionsDisabled};
use bevy_scene::WorldSceneExt;
use bevy_ui::{AlignItems, Node, Val};
use bevy_ui_widgets::{SliderPrecision, SliderRange, SliderStep, SliderValue};

use super::PlumeCaps;
use crate::containers::SectionCollapsed;
use crate::controls::{ButtonVariant, PlumeNumberInput, PlumeSlider, text_input_suffix};

/// What a widget reported this frame, plus chainable builders for
/// composition-rule props (`enabled`, …).
pub struct ImmResponse<'r, 'w, 's> {
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
}

impl ImmResponse<'_, '_, '_> {
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

    /// Set a button's color variant (the styling systems re-style on change).
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

    /// Set a number input's committable range (clamps typed/stepped values).
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

    /// Set decimal precision (`0` = integer). A number input reprints its value;
    /// a slider rounds the values it commits while dragging.
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
                        // Re-touch the value so the text reformats at the new precision.
                        if let Some(value) = entity.get::<SliderValue>().map(|value| value.0) {
                            entity.insert(SliderValue(value));
                        }
                    } else if entity.contains::<PlumeSlider>() {
                        entity.insert(SliderPrecision(precision as i32));
                    }
                });
        }
        self
    }

    /// Append a dim, non-editable unit suffix to a number/text input (e.g. `m/s`).
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

    /// Top-align a container's children (`align_items: Start`), e.g. a
    /// [`horizontal`](super::PlumeImm::horizontal) row of unequal-height columns
    /// so a shorter one anchors to the top instead of centering.
    pub fn align_top(mut self) -> Self {
        struct AlignTopKey;
        if self.e.hash_update_typ::<AlignTopKey>(Some(imm_id(true))) {
            self.e
                .entity_commands()
                .queue(|mut entity: EntityWorldMut| {
                    if let Some(mut node) = entity.get_mut::<Node>() {
                        node.align_items = AlignItems::Start;
                    }
                });
        }
        self
    }

    /// Seed a section collapsed on first spawn only; afterwards the retained
    /// entity owns its collapse state (a no-op on already-spawned widgets, so it
    /// never fights the user's expand/collapse).
    pub fn start_collapsed(mut self) -> Self {
        if self.spawned {
            self.e.entity_commands().insert(SectionCollapsed);
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
