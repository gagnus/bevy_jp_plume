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
    constants::FaIcon,
    containers::{
        DialogCloseRequested, PlumeDialogBody, PlumeDialogProps, PlumeSectionProps, column,
        dialog_frame, flex_spacer, row, section_body, section_frame, separator,
    },
    controls::{
        PlumeButton, PlumeCheckbox, PlumeNumberInput, PlumeSelect, PlumeSlider, PlumeToggleSwitch,
        PlumeToolButton, list_rows_from_strings,
    },
    display::{caption, caption_small_caps, fa_icon},
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

    /// Push button with a leading FontAwesome icon before the label.
    fn icon_button(&mut self, icon: FaIcon, label: &str) -> ImmResponse<'_, 'w, 's>;

    /// Compact icon-only button (tighter padding, square min-width) for headers/toolbars.
    fn tool_button(&mut self, icon: FaIcon) -> ImmResponse<'_, 'w, 's>;

    /// Labeled checkbox bound to `value`.
    fn checkbox(&mut self, value: &mut bool, label: &str) -> ImmResponse<'_, 'w, 's>;

    /// Bare toggle switch bound to `value` (no label — the surrounding row owns it).
    fn toggle(&mut self, value: &mut bool) -> ImmResponse<'_, 'w, 's>;

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
    /// The response chains `.grow()`/`.width()` to size the row itself.
    fn horizontal(&mut self, f: impl FnOnce(&mut Ui<'w, 's>)) -> ImmResponse<'_, 'w, 's>;

    /// Vertical container; children stretch to its width. The response chains
    /// `.grow()`/`.width()` to size the column itself (e.g. equal-width columns).
    fn vertical(&mut self, f: impl FnOnce(&mut Ui<'w, 's>)) -> ImmResponse<'_, 'w, 's>;

    /// Collapsible section with a small-caps `header`; `f` builds its body.
    /// Collapse state persists across frames. Chain `.start_collapsed()`.
    fn section(&mut self, header: &str, f: impl FnOnce(&mut Ui<'w, 's>))
    -> ImmResponse<'_, 'w, 's>;

    /// Invisible filler that absorbs a row's spare width (pushes what follows to
    /// the trailing edge).
    fn flex_spacer(&mut self);

    /// Scope child ids by `id`. A widget-building helper called more than once
    /// needs this: every `#[track_caller]` call site inside the helper shares one
    /// source location, so without a distinct `id` per call the ids collide.
    fn push_id<R>(&mut self, id: impl core::hash::Hash, f: impl FnOnce(&mut Ui<'w, 's>) -> R) -> R;
}

impl<'w, 's> PlumeImm<'w, 's> for Ui<'w, 's> {
    #[track_caller]
    fn caption(&mut self, text: &str) -> ImmResponse<'_, 'w, 's> {
        let text_owned = text.to_owned();
        let mut entity = self
            .ch_with_manual_id(loc_id(text))
            .on_spawn_apply_scene(move || caption(text_owned));
        let hovered = entity.hovered();
        let spawned = entity.will_be_spawned();
        ImmResponse {
            clicked: false,
            changed: false,
            hovered,
            entity: entity.entity(),
            spawned,
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
    fn icon_button(&mut self, icon: FaIcon, label: &str) -> ImmResponse<'_, 'w, 's> {
        let label_owned = label.to_owned();
        let entity = self
            .ch_with_manual_id(loc_id((icon, label)))
            .on_spawn_apply_scene(move || {
                bsn! { @PlumeButton { @caption: bsn_list! { fa_icon(icon), caption(label_owned) } } }
            });
        respond(entity, false)
    }

    #[track_caller]
    fn tool_button(&mut self, icon: FaIcon) -> ImmResponse<'_, 'w, 's> {
        let entity = self
            .ch_with_manual_id(loc_id(icon))
            .on_spawn_apply_scene(move || {
                bsn! { @PlumeToolButton { @caption: bsn! { fa_icon(icon) } } }
            });
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
    fn toggle(&mut self, value: &mut bool) -> ImmResponse<'_, 'w, 's> {
        let mut changed = false;
        let entity = self
            .ch_with_manual_id(loc_id(()))
            .on_spawn_apply_scene(|| bsn! { @PlumeToggleSwitch })
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
    fn horizontal(&mut self, f: impl FnOnce(&mut Ui<'w, 's>)) -> ImmResponse<'_, 'w, 's> {
        let entity = self
            .ch_with_manual_id(loc_id(()))
            .on_spawn_apply_scene(row)
            .add(f);
        respond(entity, false)
    }

    #[track_caller]
    fn vertical(&mut self, f: impl FnOnce(&mut Ui<'w, 's>)) -> ImmResponse<'_, 'w, 's> {
        let entity = self
            .ch_with_manual_id(loc_id(()))
            .on_spawn_apply_scene(column)
            .add(f);
        respond(entity, false)
    }

    #[track_caller]
    fn section(
        &mut self,
        header: &str,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's> {
        let header_owned = header.to_owned();
        let entity = self
            .ch_with_manual_id(loc_id(header))
            .on_spawn_apply_scene(move || {
                bsn! {
                    section_frame(PlumeSectionProps {
                        header: Box::new(bsn_list!(caption_small_caps(header_owned))),
                        ..Default::default()
                    })
                }
            })
            .add(|ui| {
                ui.ch_id("section_body")
                    .on_spawn_apply_scene(section_body)
                    .add(f);
            });
        respond(entity, false)
    }

    #[track_caller]
    fn flex_spacer(&mut self) {
        self.ch_with_manual_id(loc_id(()))
            .on_spawn_apply_scene(flex_spacer);
    }

    fn push_id<R>(&mut self, id: impl core::hash::Hash, f: impl FnOnce(&mut Ui<'w, 's>) -> R) -> R {
        let mut scope = self.with_add_id_pref(id);
        f(&mut scope)
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
    let spawned = entity.will_be_spawned();
    ImmResponse {
        clicked,
        changed,
        hovered,
        entity: entity.entity(),
        spawned,
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
