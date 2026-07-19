//! Response returned by every imm widget call: state flags plus builder methods.
use bevy_ecs::{entity::Entity, world::EntityWorldMut};
use bevy_immediate::{ImmEntity, imm_id, ui::disabled::ImmUiInteractionsDisabled};
use bevy_ui::{Node, Val};

use super::PlumeCaps;

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
