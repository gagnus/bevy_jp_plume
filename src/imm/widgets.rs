//! Widget methods on the immediate-mode context; each wraps a retained plume scene.
use core::ops::RangeInclusive;
use core::panic::Location;

use bevy_ecs::{event::EntityEvent, observer::On, system::Commands};
use bevy_immediate::{
    ImmEntity, ImmId, ImmIdBuilder,
    ui::{activated::ImmUiActivated, interaction::ImmUiInteraction},
};
use bevy_scene::{Scene, bsn, bsn_list, on};
use bevy_ui_widgets::RequestClose;

use crate::{
    containers::{
        DialogCloseRequested, PlumeDialogBody, PlumeDialogProps, dialog_frame, flex_spacer, row,
        separator,
    },
    controls::{
        PlumeButton, PlumeCheckbox, PlumeNumberInput, PlumeSelect, PlumeSlider,
        list_rows_from_strings,
    },
    display::caption,
};

use super::{
    ImmResponse, PlumeCaps, Ui,
    caps::{ImmPlumeChecked, ImmPlumeDialog, ImmPlumeSelect, ImmPlumeValue},
};

/// Widget calls for immediate-mode systems. Implemented by [`Ui`] (and therefore
/// available on [`super::PlumeUi`]); import it wherever imm systems are written.
pub trait PlumeImm<'w, 's> {
    /// Themed text in the current container's font and color. The response's
    /// `clicked`/`changed` are always false (text has no activation behavior);
    /// the builders and `hovered` work as usual.
    fn caption(&mut self, text: &str) -> ImmResponse<'_, 'w, 's>;

    /// Hairline horizontal rule.
    fn separator(&mut self);

    /// Push button; `.clicked` on the response fires once per activation.
    fn button(&mut self, label: &str) -> ImmResponse<'_, 'w, 's>;

    /// Labeled checkbox bound to `value`.
    fn checkbox(&mut self, value: &mut bool, label: &str) -> ImmResponse<'_, 'w, 's>;

    /// Slider bound to `value` over `range`.
    fn slider(&mut self, value: &mut f32, range: RangeInclusive<f32>) -> ImmResponse<'_, 'w, 's>;

    /// Numeric input field bound to `value`.
    fn number(&mut self, value: &mut f32) -> ImmResponse<'_, 'w, 's>;

    /// Dropdown bound to `index` into `options`.
    fn select(&mut self, index: &mut usize, options: &[&str]) -> ImmResponse<'_, 'w, 's>;

    /// Movable floating dialog. While `*open`, the dialog exists and `f` builds its
    /// body; the ✕ writes back through `open`. Dragged position persists.
    fn dialog(&mut self, title: &str, open: &mut bool, f: impl FnOnce(&mut Ui<'w, 's>));

    /// Horizontal, center-aligned container (label-beside-control). Children pack
    /// left; use [`Self::flex_spacer`] or [`ImmResponse::grow`] to distribute width.
    fn horizontal(&mut self, f: impl FnOnce(&mut Ui<'w, 's>));

    /// Invisible filler that absorbs a row's spare width (pushes what follows to
    /// the trailing edge).
    fn flex_spacer(&mut self);
}

impl<'w, 's> PlumeImm<'w, 's> for Ui<'w, 's> {
    #[track_caller]
    fn caption(&mut self, text: &str) -> ImmResponse<'_, 'w, 's> {
        let text_owned = text.to_owned();
        let mut entity = self
            .ch_with_manual_id(loc_id(text))
            .on_spawn_apply_scene(move || caption(text_owned));
        let hovered = entity.hovered();
        ImmResponse {
            clicked: false,
            changed: false,
            hovered,
            entity: entity.entity(),
            e: entity,
        }
    }

    #[track_caller]
    fn separator(&mut self) {
        self.ch_with_manual_id(loc_id(()))
            .on_spawn_apply_scene(separator);
    }

    #[track_caller]
    fn button(&mut self, label: &str) -> ImmResponse<'_, 'w, 's> {
        let label_owned = label.to_owned();
        let entity = self.ch_with_manual_id(loc_id(label)).on_spawn_apply_scene(
            move || bsn! { @PlumeButton { @caption: bsn! { caption(label_owned) } } },
        );
        respond(entity, false)
    }

    #[track_caller]
    fn checkbox(&mut self, value: &mut bool, label: &str) -> ImmResponse<'_, 'w, 's> {
        let label_owned = label.to_owned();
        let mut changed = false;
        let entity = self
            .ch_with_manual_id(loc_id(label))
            .on_spawn_apply_scene(
                move || bsn! { @PlumeCheckbox { @caption: bsn! { caption(label_owned) } } },
            )
            .plume_checked(value, &mut changed);
        respond(entity, changed)
    }

    #[track_caller]
    fn slider(&mut self, value: &mut f32, range: RangeInclusive<f32>) -> ImmResponse<'_, 'w, 's> {
        let (min, max) = (*range.start(), *range.end());
        let mut changed = false;
        let entity = self
            .ch_with_manual_id(loc_id((min.to_bits(), max.to_bits())))
            .on_spawn_apply_scene(move || bsn! { @PlumeSlider { @min: {min}, @max: {max} } })
            .plume_value(value, &mut changed);
        respond(entity, changed)
    }

    #[track_caller]
    fn number(&mut self, value: &mut f32) -> ImmResponse<'_, 'w, 's> {
        let initial = *value;
        let mut changed = false;
        let entity = self
            .ch_with_manual_id(loc_id(()))
            .on_spawn_apply_scene(move || bsn! { @PlumeNumberInput { @value: {initial} } })
            .plume_value(value, &mut changed);
        respond(entity, changed)
    }

    #[track_caller]
    fn select(&mut self, index: &mut usize, options: &[&str]) -> ImmResponse<'_, 'w, 's> {
        let options_owned: Vec<String> = options.iter().map(|option| option.to_string()).collect();
        *index = (*index).min(options.len().saturating_sub(1));
        let initial = *index;
        let mut changed = false;
        let entity = self
            .ch_with_manual_id(loc_id(options))
            .on_spawn_apply_scene(move || {
                bsn! { @PlumeSelect { @options: {list_rows_from_strings(options_owned, Some(initial))} } }
            })
            .plume_select(index, &mut changed);
        respond(entity, changed)
    }

    #[track_caller]
    fn dialog(&mut self, title: &str, open: &mut bool, f: impl FnOnce(&mut Ui<'w, 's>)) {
        if !*open {
            return;
        }
        let title_owned = title.to_owned();
        let mut entity = self
            .ch_with_manual_id(loc_id(title))
            .on_spawn_apply_scene(move || imm_dialog_scene(title_owned));
        if entity.close_requested() {
            *open = false;
            entity.entity_commands().despawn();
            return;
        }
        entity.add(|ui| {
            ui.ch_id("dialog_body")
                .on_spawn_apply_scene(|| bsn! { @PlumeDialogBody })
                .add(f);
        });
    }

    #[track_caller]
    fn horizontal(&mut self, f: impl FnOnce(&mut Ui<'w, 's>)) {
        self.ch_with_manual_id(loc_id(()))
            .on_spawn_apply_scene(row)
            .add(f);
    }

    #[track_caller]
    fn flex_spacer(&mut self) {
        self.ch_with_manual_id(loc_id(()))
            .on_spawn_apply_scene(flex_spacer);
    }
}

// Combining the caller location with a key means label/options changes respawn the
// widget instead of leaving stale scene content.
#[track_caller]
fn loc_id(key: impl core::hash::Hash) -> ImmIdBuilder {
    ImmIdBuilder::Hierarchy(ImmId::new((Location::caller(), key)))
}

fn respond<'r, 'w, 's>(
    mut entity: ImmEntity<'r, 'w, 's, PlumeCaps>,
    changed: bool,
) -> ImmResponse<'r, 'w, 's> {
    let clicked = entity.activated();
    let hovered = entity.hovered();
    ImmResponse {
        clicked,
        changed,
        hovered,
        entity: entity.entity(),
        e: entity,
    }
}

fn imm_dialog_scene(title: String) -> impl Scene {
    bsn! {
        // Empty contents: the imm layer reconciles the body itself.
        dialog_frame(PlumeDialogProps {
            title: Box::new(bsn_list!(caption(title))),
            ..Default::default()
        })
        on(|close: On<RequestClose>, mut commands: Commands| {
            commands.entity(close.event_target()).insert(DialogCloseRequested);
        })
    }
}
