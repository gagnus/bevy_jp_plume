//! Widget methods on the immediate-mode context; each wraps a retained plume scene.
use core::marker::PhantomData;
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
        DialogCloseRequested, PlumeDialogBody, PlumeDialogProps, PlumeGroup, PlumeSectionProps,
        column, dialog_frame, dialog_scroll_area, dialog_scroll_frame, dialog_scrollbar,
        flex_spacer, row, screen, section_body, section_frame, separator, space,
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
    kind,
};

/// Widget calls for immediate-mode systems. Implemented by [`Ui`] (and therefore
/// available on [`super::PlumeUi`]); import it wherever imm systems are written.
pub trait PlumeImm<'w, 's> {
    /// Themed text in the current container's font and color. The response's
    /// `clicked`/`changed` are always false (text has no activation behavior);
    /// the builders and `hovered` work as usual.
    fn caption(&mut self, text: &str) -> ImmResponse<'_, 'w, 's>;

    /// Hairline rule across the container: a horizontal line in a
    /// [`Self::vertical`], a vertical one in a [`Self::horizontal`].
    fn separator(&mut self);

    /// Fixed gap along the container's main axis — `length` of width in a
    /// [`Self::horizontal`], of height in a [`Self::vertical`]. For a gap that
    /// absorbs whatever is left over instead, use [`Self::flex_spacer`].
    fn space(&mut self, length: Val);

    /// Push button; `.clicked` on the response fires once per activation.
    fn button(&mut self, label: &str) -> ImmResponse<'_, 'w, 's, kind::Button>;

    /// Push button with a leading FontAwesome icon before the label.
    fn icon_button(&mut self, icon: FaIcon, label: &str) -> ImmResponse<'_, 'w, 's, kind::Button>;

    /// Compact icon-only button (tighter padding, square min-width) for headers/toolbars.
    fn tool_button(&mut self, icon: FaIcon) -> ImmResponse<'_, 'w, 's, kind::Button>;

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
    fn slider(
        &mut self,
        value: &mut f32,
        range: RangeInclusive<f32>,
    ) -> ImmResponse<'_, 'w, 's, kind::Slider>;

    /// Numeric input field bound to `value`.
    fn number(&mut self, value: &mut f32) -> ImmResponse<'_, 'w, 's, kind::Number>;

    /// Single-line text input bound to `text` (synced per edit, not on commit).
    /// While the field is focused the widget's buffer wins; app writes land on
    /// blur. Chain `.placeholder()`/`.suffix()`.
    fn text_edit(&mut self, text: &mut String) -> ImmResponse<'_, 'w, 's, kind::Text>;

    /// Dropdown bound to `index` into `options`.
    fn select(
        &mut self,
        index: &mut usize,
        options: &[&str],
    ) -> ImmResponse<'_, 'w, 's, kind::Select>;

    /// Movable floating dialog: configure via the returned [`ImmDialog`] and build
    /// the body with [`ImmDialog::show`]. While `*open`, the dialog exists; the ✕
    /// writes back through `open`. Dragged position persists.
    fn dialog<'a>(&'a mut self, title: &str, open: &'a mut bool) -> ImmDialog<'a, 'w, 's>;

    /// Horizontal, center-aligned container (label-beside-control). Children pack
    /// left; use [`Self::flex_spacer`] or `.grow()` to distribute width.
    /// The response chains `.grow()`/`.width()` to size the row itself.
    fn horizontal(&mut self, f: impl FnOnce(&mut Ui<'w, 's>))
    -> ImmResponse<'_, 'w, 's, kind::Row>;

    /// Vertical container; children stretch to its width. The response chains
    /// `.grow()`/`.width()` to size the column itself (e.g. equal-width columns).
    fn vertical(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Column>;

    /// Full-screen root surface for top-level content: a transparent, padded
    /// column (a dialog body sized to the viewport) that establishes the standard
    /// font and text color, so bare `caption`/text works at root scope. Children
    /// stretch to its width and pack from the top; it overlays whatever renders
    /// behind it and lets picks fall through its empty areas. Wrap a system's
    /// top-level widgets in one call.
    fn screen(&mut self, f: impl FnOnce(&mut Ui<'w, 's>)) -> ImmResponse<'_, 'w, 's, kind::Column>;

    /// Filled box visually grouping related controls; children stretch to its
    /// width (a themed [`Self::vertical`]).
    fn group(&mut self, f: impl FnOnce(&mut Ui<'w, 's>)) -> ImmResponse<'_, 'w, 's, kind::Column>;

    /// Collapsible section with a small-caps `header`; `f` builds its body.
    /// Collapse state persists across frames. Chain `.start_collapsed()`.
    fn section(
        &mut self,
        header: &str,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Section>;

    /// Invisible filler that absorbs a row's spare width (pushes what follows to
    /// the trailing edge).
    fn flex_spacer(&mut self);

    /// Scope child ids by `id`, making widget identity follow the key instead of
    /// call order. Repeated calls of a widget-building helper already
    /// auto-disambiguate (each repeat of the same id under one parent gets an
    /// occurrence index), but that identity is positional: use `push_id` when
    /// entries can reorder (e.g. rows of a sortable `Vec`) or when a conditional
    /// sibling would shift the repeats that follow it, losing their widget state.
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
            kind: PhantomData,
        }
    }

    #[track_caller]
    fn separator(&mut self) {
        self.ch_with_manual_id(loc_id(()))
            .on_spawn_apply_scene(separator);
    }

    #[track_caller]
    fn space(&mut self, length: Val) {
        self.ch_with_manual_id(loc_id(format!("{length:?}")))
            .on_spawn_apply_scene(move || space(length));
    }

    #[track_caller]
    fn button(&mut self, label: &str) -> ImmResponse<'_, 'w, 's, kind::Button> {
        let label_owned = label.to_owned();
        let entity = self.ch_with_manual_id(loc_id(label)).on_spawn_apply_scene(
            move || bsn! { @PlumeButton { @caption: bsn! { caption(label_owned) } } },
        );
        respond(entity, false)
    }

    #[track_caller]
    fn icon_button(&mut self, icon: FaIcon, label: &str) -> ImmResponse<'_, 'w, 's, kind::Button> {
        let label_owned = label.to_owned();
        let entity = self
            .ch_with_manual_id(loc_id((icon, label)))
            .on_spawn_apply_scene(move || {
                bsn! { @PlumeButton { @caption: bsn_list! { fa_icon(icon), caption(label_owned) } } }
            });
        respond(entity, false)
    }

    #[track_caller]
    fn tool_button(&mut self, icon: FaIcon) -> ImmResponse<'_, 'w, 's, kind::Button> {
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
    fn slider(
        &mut self,
        value: &mut f32,
        range: RangeInclusive<f32>,
    ) -> ImmResponse<'_, 'w, 's, kind::Slider> {
        let (min, max) = (*range.start(), *range.end());
        let mut changed = false;
        let entity = self
            .ch_with_manual_id(loc_id((min.to_bits(), max.to_bits())))
            .on_spawn_apply_scene(move || bsn! { @PlumeSlider { @min: {min}, @max: {max} } })
            .plume_value(value, &mut changed);
        respond(entity, changed)
    }

    #[track_caller]
    fn number(&mut self, value: &mut f32) -> ImmResponse<'_, 'w, 's, kind::Number> {
        let initial = *value;
        let mut changed = false;
        let entity = self
            .ch_with_manual_id(loc_id(()))
            .on_spawn_apply_scene(move || bsn! { @PlumeNumberInput { @value: {initial} } })
            .plume_value(value, &mut changed);
        respond(entity, changed)
    }

    #[track_caller]
    fn text_edit(&mut self, text: &mut String) -> ImmResponse<'_, 'w, 's, kind::Text> {
        let initial = text.clone();
        let mut changed = false;
        let entity = self
            .ch_with_manual_id(loc_id(()))
            .on_spawn_apply_scene(move || bsn! { @PlumeTextInput { @value: {initial} } })
            .plume_text(text, &mut changed);
        respond(entity, changed)
    }

    #[track_caller]
    fn select(
        &mut self,
        index: &mut usize,
        options: &[&str],
    ) -> ImmResponse<'_, 'w, 's, kind::Select> {
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
            layout: DialogLayout {
                width: Val::Auto,
                height: Val::Auto,
                max_height: Val::Auto,
                left: px(120),
                top: px(120),
                closable: true,
                movable: true,
            },
        }
    }

    #[track_caller]
    fn horizontal(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Row> {
        let entity = self
            .ch_with_manual_id(loc_id(()))
            .on_spawn_apply_scene(row)
            .add(f);
        respond(entity, false)
    }

    #[track_caller]
    fn vertical(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Column> {
        let entity = self
            .ch_with_manual_id(loc_id(()))
            .on_spawn_apply_scene(column)
            .add(f);
        respond(entity, false)
    }

    #[track_caller]
    fn screen(&mut self, f: impl FnOnce(&mut Ui<'w, 's>)) -> ImmResponse<'_, 'w, 's, kind::Column> {
        let entity = self
            .ch_with_manual_id(loc_id(()))
            .on_spawn_apply_scene(screen)
            .add(f);
        respond(entity, false)
    }

    #[track_caller]
    fn group(&mut self, f: impl FnOnce(&mut Ui<'w, 's>)) -> ImmResponse<'_, 'w, 's, kind::Column> {
        let entity = self
            .ch_with_manual_id(loc_id(()))
            .on_spawn_apply_scene(|| bsn! { @PlumeGroup })
            .add(f);
        respond(entity, false)
    }

    #[track_caller]
    fn section(
        &mut self,
        header: &str,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Section> {
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
    layout: DialogLayout,
}

/// The dialog's frame-level props, split out so they travel to the scene as one
/// value instead of eight positional arguments.
#[derive(Clone, Copy)]
struct DialogLayout {
    width: Val,
    height: Val,
    max_height: Val,
    left: Val,
    top: Val,
    closable: bool,
    movable: bool,
}

impl DialogLayout {
    /// Whether the body needs the scrolling machinery: either height knob bounds
    /// the dialog, so its content can no longer be assumed to fit.
    fn scrolls(&self) -> bool {
        self.height != Val::Auto || self.max_height != Val::Auto
    }
}

impl<'w, 's> ImmDialog<'_, 'w, 's> {
    /// Fix the dialog's width (default `Val::Auto` hugs the content). Rows that
    /// distribute space (`.grow()`, `flex_spacer`) need a fixed width to resolve
    /// against. Applied live: a changed value re-sizes the open dialog.
    pub fn width(mut self, width: Val) -> Self {
        self.layout.width = width;
        self
    }

    /// Fix the dialog's outer height, title bar included, and scroll the body
    /// vertically once the content outgrows it. Applied live, like
    /// [`Self::width`].
    ///
    /// Prefer [`Self::max_height`] unless the dialog should hold its size while
    /// near-empty. A height below the title bar's own is clamped away rather
    /// than honored.
    pub fn height(mut self, height: Val) -> Self {
        self.layout.height = height;
        self
    }

    /// Cap the dialog's outer height: it hugs its content as usual until it
    /// would exceed `max_height`, then stops growing and scrolls the body.
    ///
    /// The usual choice for a settings dialog whose length depends on how many
    /// sections happen to be expanded.
    pub fn max_height(mut self, max_height: Val) -> Self {
        self.layout.max_height = max_height;
        self
    }

    /// Initial position (default `120, 120`). Spawn-time only — once open, the
    /// user's dragging owns the position.
    pub fn at(mut self, left: Val, top: Val) -> Self {
        self.layout.left = left;
        self.layout.top = top;
        self
    }

    /// `false` omits the ✕ button, for dialogs dismissed only by an action button.
    pub fn closable(mut self, closable: bool) -> Self {
        self.layout.closable = closable;
        self
    }

    /// `false` omits the drag handle, pinning the dialog in place.
    pub fn movable(mut self, movable: bool) -> Self {
        self.layout.movable = movable;
        self
    }

    /// Build the dialog and its body. While `*open` the dialog exists and `f`
    /// fills its body; the ✕ writes back through `open`.
    pub fn show(self, f: impl FnOnce(&mut Ui<'w, 's>)) {
        if !*self.open {
            return;
        }
        let id = ImmIdBuilder::Hierarchy(ImmId::new((self.caller, self.title.as_str())));
        let (title, layout) = (self.title, self.layout);
        let mut entity = self
            .ui
            .ch_with_manual_id(id)
            .on_spawn_apply_scene(move || imm_dialog_scene(title, layout));
        if entity.close_requested() {
            *self.open = false;
            entity.entity_commands().despawn();
            return;
        }
        // Size is app-owned even while open; position is not re-applied (the
        // user's dragging owns it after spawn).
        struct DialogSizeKey;
        let size_key = format!(
            "{:?}{:?}{:?}",
            layout.width, layout.height, layout.max_height
        );
        if entity.hash_update_typ::<DialogSizeKey>(Some(imm_id(size_key))) {
            entity
                .entity_commands()
                .queue(move |mut entity: EntityWorldMut| {
                    if let Some(mut node) = entity.get_mut::<Node>() {
                        node.width = layout.width;
                        node.height = layout.height;
                        node.max_height = layout.max_height;
                    }
                });
        }
        let scrolls = layout.scrolls();
        entity.add(move |ui| {
            let body = ui
                .ch_id("dialog_body")
                .on_spawn_apply_scene(|| bsn! { @PlumeDialogBody });
            if !scrolls {
                body.add(f);
                return;
            }
            body.add(move |ui| {
                ui.ch_id("scroll_frame")
                    .on_spawn_apply_scene(dialog_scroll_frame)
                    .add(move |ui| {
                        // The scroll area's entity is known before its spawn
                        // command flushes, so the scrollbar beside it can point
                        // at the viewport it drives.
                        let viewport = ui
                            .ch_id("scroll_area")
                            .on_spawn_apply_scene(dialog_scroll_area)
                            .add(f)
                            .entity();
                        ui.ch_id("scrollbar")
                            .on_spawn_apply_scene(move || dialog_scrollbar(viewport));
                    });
            });
        });
    }
}

// Combining the caller location with a key means label/options changes respawn the
// widget instead of leaving stale scene content.
#[track_caller]
fn loc_id(key: impl core::hash::Hash) -> ImmIdBuilder {
    ImmIdBuilder::Hierarchy(ImmId::new((Location::caller(), key)))
}

fn respond<'r, 'w, 's, K>(
    mut entity: ImmEntity<'r, 'w, 's, PlumeCaps>,
    changed: bool,
) -> ImmResponse<'r, 'w, 's, K> {
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
        kind: PhantomData,
    }
}

fn imm_dialog_scene(title: String, layout: DialogLayout) -> impl Scene {
    let DialogLayout {
        width,
        height,
        max_height,
        left,
        top,
        closable,
        movable,
    } = layout;
    bsn! {
        // Empty contents: the imm layer reconciles the body itself.
        dialog_frame(PlumeDialogProps {
            title: Box::new(bsn_list!(caption(title))),
            width,
            height,
            max_height,
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
