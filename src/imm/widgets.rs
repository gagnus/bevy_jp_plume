//! Widget methods on the immediate-mode context; each wraps a retained plume scene.
use core::ops::RangeInclusive;
use core::panic::Location;

use bevy_ecs::{event::EntityEvent, observer::On, system::Commands, world::EntityWorldMut};
use bevy_immediate::{
    ImmEntity, ImmId, ImmIdBuilder, imm_id,
    ui::{activated::ImmUiActivated, interaction::ImmUiInteraction},
};
use bevy_scene::{Scene, bsn, bsn_list, on};
use bevy_ui::{Node, Val, px};
use bevy_ui_widgets::RequestClose;

use crate::{
    constants::FaIcon,
    containers::{
        DialogCloseRequested, PlumeDialogBody, PlumeDialogProps, PlumeSectionProps, column,
        dialog_frame, flex_spacer, row, section_body, section_frame, separator,
    },
    controls::{
        PlumeButton, PlumeCheckbox, PlumeNumberInput, PlumeRadio, PlumeSelect, PlumeSlider,
        PlumeTextInput, PlumeToggleSwitch, PlumeToolButton, list_rows_from_strings,
    },
    display::{caption, caption_small_caps, fa_icon},
};

use super::{
    ImmResponse, PlumeCaps, Ui,
    caps::{ImmPlumeChecked, ImmPlumeDialog, ImmPlumeSelect, ImmPlumeText, ImmPlumeValue},
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

    /// Labeled radio button: checked while `*value == variant`; clicking it (or
    /// Space on focus) writes `variant` into `value`. A group is just several
    /// radios bound to the same `value` — no container needed.
    fn radio<T: PartialEq>(
        &mut self,
        value: &mut T,
        variant: T,
        label: &str,
    ) -> ImmResponse<'_, 'w, 's>;

    /// Bare toggle switch bound to `value` (no label — the surrounding row owns it).
    fn toggle(&mut self, value: &mut bool) -> ImmResponse<'_, 'w, 's>;

    /// Slider bound to `value` over `range`.
    fn slider(&mut self, value: &mut f32, range: RangeInclusive<f32>) -> ImmResponse<'_, 'w, 's>;

    /// Numeric input field bound to `value`.
    fn number(&mut self, value: &mut f32) -> ImmResponse<'_, 'w, 's>;

    /// Single-line text input bound to `text` (synced per edit, not on commit).
    /// While the field is focused the widget's buffer wins; app writes land on
    /// blur. Chain [`ImmResponse::placeholder`]/[`ImmResponse::suffix`].
    fn text_edit(&mut self, text: &mut String) -> ImmResponse<'_, 'w, 's>;

    /// Dropdown bound to `index` into `options`.
    fn select(&mut self, index: &mut usize, options: &[&str]) -> ImmResponse<'_, 'w, 's>;

    /// Movable floating dialog: configure via the returned [`ImmDialog`] and build
    /// the body with [`ImmDialog::show`]. While `*open`, the dialog exists; the ✕
    /// writes back through `open`. Dragged position persists.
    fn dialog<'a>(&'a mut self, title: &str, open: &'a mut bool) -> ImmDialog<'a, 'w, 's>;

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
    fn radio<T: PartialEq>(
        &mut self,
        value: &mut T,
        variant: T,
        label: &str,
    ) -> ImmResponse<'_, 'w, 's> {
        let label_owned = label.to_owned();
        // The checked sync sees a derived bool: pushing app state unchecks a
        // sibling the frame after another radio wins; a user click lands as a
        // pending true, which is written back through `value` below.
        let mut checked = *value == variant;
        let mut changed = false;
        let entity = self
            .ch_with_manual_id(loc_id(label))
            .on_spawn_apply_scene(
                move || bsn! { @PlumeRadio { @caption: bsn! { caption(label_owned) } } },
            )
            .plume_checked(&mut checked, &mut changed);
        let selected = changed && checked;
        if selected {
            *value = variant;
        }
        respond(entity, selected)
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
    fn text_edit(&mut self, text: &mut String) -> ImmResponse<'_, 'w, 's> {
        let initial = text.clone();
        let mut changed = false;
        let entity = self
            .ch_with_manual_id(loc_id(()))
            .on_spawn_apply_scene(move || bsn! { @PlumeTextInput { @value: {initial} } })
            .plume_text(text, &mut changed);
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
    fn dialog<'a>(&'a mut self, title: &str, open: &'a mut bool) -> ImmDialog<'a, 'w, 's> {
        ImmDialog {
            ui: self,
            caller: Location::caller(),
            title: title.to_owned(),
            open,
            width: Val::Auto,
            left: px(120),
            top: px(120),
            closable: true,
            movable: true,
        }
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

/// Deferred dialog configuration returned by [`PlumeImm::dialog`]; the dialog only
/// exists once [`Self::show`] runs.
#[must_use = "a dialog does nothing until .show(|ui| …) builds it"]
pub struct ImmDialog<'a, 'w, 's> {
    ui: &'a mut Ui<'w, 's>,
    caller: &'static Location<'static>,
    title: String,
    open: &'a mut bool,
    width: Val,
    left: Val,
    top: Val,
    closable: bool,
    movable: bool,
}

impl<'w, 's> ImmDialog<'_, 'w, 's> {
    /// Fix the dialog's width (default `Val::Auto` hugs the content). Rows that
    /// distribute space (`.grow()`, `flex_spacer`) need a fixed width to resolve
    /// against. Applied live: a changed value re-sizes the open dialog.
    pub fn width(mut self, width: Val) -> Self {
        self.width = width;
        self
    }

    /// Initial position (default `120, 120`). Spawn-time only — once open, the
    /// user's dragging owns the position.
    pub fn at(mut self, left: Val, top: Val) -> Self {
        self.left = left;
        self.top = top;
        self
    }

    /// `false` omits the ✕ button, for dialogs dismissed only by an action button.
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// `false` omits the drag handle, pinning the dialog in place.
    pub fn movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self
    }

    /// Build the dialog and its body. While `*open` the dialog exists and `f`
    /// fills its body; the ✕ writes back through `open`.
    pub fn show(self, f: impl FnOnce(&mut Ui<'w, 's>)) {
        if !*self.open {
            return;
        }
        let id = ImmIdBuilder::Hierarchy(ImmId::new((self.caller, self.title.as_str())));
        let (title, width) = (self.title, self.width);
        let (left, top, closable, movable) = (self.left, self.top, self.closable, self.movable);
        let mut entity = self.ui.ch_with_manual_id(id).on_spawn_apply_scene(move || {
            imm_dialog_scene(title, width, left, top, closable, movable)
        });
        if entity.close_requested() {
            *self.open = false;
            entity.entity_commands().despawn();
            return;
        }
        // Width is app-owned even while open; position is not re-applied (the
        // user's dragging owns it after spawn).
        struct DialogWidthKey;
        if entity.hash_update_typ::<DialogWidthKey>(Some(imm_id(format!("{width:?}")))) {
            entity
                .entity_commands()
                .queue(move |mut entity: EntityWorldMut| {
                    if let Some(mut node) = entity.get_mut::<Node>() {
                        node.width = width;
                    }
                });
        }
        entity.add(|ui| {
            ui.ch_id("dialog_body")
                .on_spawn_apply_scene(|| bsn! { @PlumeDialogBody })
                .add(f);
        });
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

fn imm_dialog_scene(
    title: String,
    width: Val,
    left: Val,
    top: Val,
    closable: bool,
    movable: bool,
) -> impl Scene {
    bsn! {
        // Empty contents: the imm layer reconciles the body itself.
        dialog_frame(PlumeDialogProps {
            title: Box::new(bsn_list!(caption(title))),
            width,
            left,
            top,
            closable,
            movable,
            ..Default::default()
        })
        on(|close: On<RequestClose>, mut commands: Commands| {
            commands.entity(close.event_target()).insert(DialogCloseRequested);
        })
    }
}
