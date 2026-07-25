//! Widget methods on the immediate-mode context; each wraps a retained plume scene.
use core::marker::PhantomData;
use core::ops::RangeInclusive;
use core::panic::Location;

use bevy::color::Color;
use bevy::ecs::{
    entity::Entity, event::EntityEvent, hierarchy::Children, observer::On, system::Commands,
    world::EntityWorldMut,
};
use bevy::scene::{Scene, bsn, bsn_list, on};
use bevy::ui::{JustifyContent, Node, UiRect, Val, widget::Text};
use bevy::ui_widgets::RequestClose;
use bevy_immediate::{
    ImmEntity, ImmId, ImmIdBuilder, imm_id,
    ui::{
        activated::ImmUiActivated, disabled::ImmUiInteractionsDisabled,
        interaction::ImmUiInteraction,
    },
};

use crate::{
    constants::{FaIcon, size},
    containers::{
        DialogChrome, DialogCloseRequested, DialogHeader, PlumeDialogBody, column, dialog_frame,
        flex_spacer, row, screen, scroll_frame, scroll_viewport, scrollbar, section_body,
        section_frame, separator, space, tab_body, tab_button, tab_strip, tabs_frame,
    },
    controls::{
        ColorSwatchValue, PlumeButton, PlumeCheckbox, PlumeColorEdit, PlumeColorPicker,
        PlumeColorSwatch, PlumeDisclosure, PlumeNumberInput, PlumeRadio, PlumeSelect, PlumeSlider,
        PlumeTextInput, PlumeToggleSwitch, PlumeToolButton, list_rows_from_strings,
    },
    display::{caption, caption_small_caps, fa_icon},
};

use super::{
    ImmResponse, PlumeCaps, Ui,
    caps::{
        ImmPlumeChecked, ImmPlumeColor, ImmPlumeDialog, ImmPlumeSelect, ImmPlumeText,
        ImmPlumeValue, PlumeOccurrences,
    },
    kind,
};

/// Widget calls for immediate-mode systems. Implemented by [`Ui`] (and therefore
/// available on [`super::PlumeUi`]); import it wherever imm systems are written.
pub trait PlumeImm<'w, 's> {
    /// Themed text in the current container's font and color. The response's
    /// `clicked`/`changed` are always false (text has no activation behavior);
    /// the builders and `hovered` work as usual.
    fn caption(&mut self, text: &str) -> ImmResponse<'_, 'w, 's, kind::Caption>;

    /// Hairline rule across the container: a horizontal line in a
    /// [`Self::vertical`], a vertical one in a [`Self::horizontal`].
    fn separator(&mut self);

    /// Non-interactive color preview: a themed, bordered rounded box filled with
    /// `color`. Defaults to a [`ROW_HEIGHT`](crate::constants::size::ROW_HEIGHT)
    /// square; chain `.square()`/`.width()`/`.height()` to resize.
    fn color_swatch(&mut self, color: Color) -> ImmResponse<'_, 'w, 's, kind::Swatch>;

    /// Interactive HSV colour picker: a saturation/value plane, a hue bar and a
    /// preview swatch. Two-way bound to `color`; `.changed` on the response fires
    /// when the user drags to a new colour. The layout is fixed by the control.
    fn color_picker(&mut self, color: &mut Color) -> ImmResponse<'_, 'w, 's>;

    /// Editable colour swatch: a swatch that opens a colour-picker popup on click,
    /// dismissed by clicking outside. Two-way bound to `color`; `.changed` fires
    /// when the user edits it.
    fn color_edit(&mut self, color: &mut Color) -> ImmResponse<'_, 'w, 's>;

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

    /// A non-interactive FontAwesome glyph in the current text color — the icon
    /// counterpart to [`Self::caption`].
    fn icon(&mut self, icon: FaIcon) -> ImmResponse<'_, 'w, 's>;

    /// A push button whose content is built by `f` instead of a single label.
    fn button_container(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Button>;

    /// Labeled checkbox bound to `value`.
    fn checkbox(&mut self, value: &mut bool, label: &str) -> ImmResponse<'_, 'w, 's>;

    /// Disclosure toggle bound to `open`.
    fn disclosure(&mut self, open: &mut bool) -> ImmResponse<'_, 'w, 's>;

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

    /// Dropdown: `f` declares the options on the [`ImmSelect`] collector, in popup
    /// order. Selection is value-keyed like [`Self::radio`] — each option names the
    /// value it stands for, and picking one writes that value into `selected`. A
    /// `selected` matching no option falls back to the first.
    fn select<T: PartialEq>(
        &mut self,
        selected: &mut T,
        f: impl FnOnce(&mut ImmSelect<T>),
    ) -> ImmResponse<'_, 'w, 's, kind::Select>;

    /// Movable floating dialog: configure via the returned [`ImmDialog`] and build
    /// the body with [`ImmDialog::show`]. While `*open`, the dialog exists; the ✕
    /// writes back through `open`. Dragged position persists.
    fn dialog<'a>(&'a mut self, title: &str, open: &'a mut bool) -> ImmDialog<'a, 'w, 's>;

    /// Headerless floating surface — a [`Self::dialog`] with no title bar (and so no
    /// title, ✕ or drag). Positioned chrome only; the caller controls whether it's
    /// drawn. Floats above a [`Self::screen`] like a dialog does.
    fn panel(&mut self) -> ImmPanel<'_, 'w, 's>;

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

    /// Full-screen root surface: a transparent, padded column establishing the
    /// standard font and text color, so bare text works at root scope. It
    /// overlays the scene behind it and lets picks fall through empty areas.
    fn screen(&mut self, f: impl FnOnce(&mut Ui<'w, 's>)) -> ImmResponse<'_, 'w, 's, kind::Screen>;

    /// Tab container: a header strip over a body showing one tab at a time.
    /// `f` declares the tabs on the [`ImmTabs`] collector; only the selected tab's
    /// body closure runs, so hidden tabs cost nothing.
    ///
    /// Selection is value-keyed like [`Self::radio`]: each tab names the value it
    /// stands for, and clicking one writes that value into `selected`. A `selected`
    /// matching no tab falls back to the first.
    fn tabs<'t, T: PartialEq>(
        &mut self,
        selected: &mut T,
        f: impl FnOnce(&mut ImmTabs<'t, 'w, 's, T>),
    ) -> ImmResponse<'_, 'w, 's, kind::Tabs>;

    /// Collapsible section with a small-caps `header`; `f` builds its body.
    /// Collapse state persists across frames. Chain `.start_collapsed()`.
    fn section(
        &mut self,
        header: &str,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Section>;

    /// Vertically scrolling region: `f` builds the content, which scrolls inside a
    /// managed viewport (with a self-hiding scrollbar) once it outgrows the height
    /// set via `.max_height()`/`.height()`. Unbounded it just stacks its content
    /// like a [`Self::vertical`]. Use it to scroll one part of a surface while the
    /// rest — headers, footers — stays pinned.
    fn scroll_area(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::ScrollArea>;

    /// Invisible filler that absorbs a row's spare width (pushes what follows to
    /// the trailing edge).
    fn flex_spacer(&mut self);

    /// Scope child ids by `id`, making widget identity follow the key instead of
    /// call order.
    ///
    /// Same-id repeats already auto-disambiguate by occurrence index, but that is
    /// positional — use this when entries reorder or a conditional sibling shifts them.
    fn push_id<R>(&mut self, id: impl core::hash::Hash, f: impl FnOnce(&mut Ui<'w, 's>) -> R) -> R;
}

impl<'w, 's> PlumeImm<'w, 's> for Ui<'w, 's> {
    #[track_caller]
    fn caption(&mut self, text: &str) -> ImmResponse<'_, 'w, 's, kind::Caption> {
        // Identity is the call site, not the text, so a caption whose text changes
        // (e.g. a live value read-out) reconciles its `Text` in place rather than
        // respawning an entity every frame the value moves.
        let mut entity = self.ch_loc(loc_id(())).on_spawn_apply_scene({
            let text = text.to_owned();
            move || caption(text)
        });
        struct CaptionText;
        if entity.hash_update_typ::<CaptionText>(Some(imm_id(text))) && !entity.will_be_spawned() {
            let text = text.to_owned();
            entity
                .entity_commands()
                .queue(move |mut e: EntityWorldMut| {
                    if let Some(mut node_text) = e.get_mut::<Text>() {
                        node_text.0 = text;
                    }
                });
        }
        let hovered = entity.hovered();
        let will_be_spawned = entity.will_be_spawned();
        ImmResponse {
            clicked: false,
            changed: false,
            hovered,
            entity: entity.entity(),
            will_be_spawned,
            e: entity,
            kind: PhantomData,
        }
    }

    #[track_caller]
    fn separator(&mut self) {
        self.ch_loc(loc_id(())).on_spawn_apply_scene(separator);
    }

    #[track_caller]
    fn color_swatch(&mut self, color: Color) -> ImmResponse<'_, 'w, 's, kind::Swatch> {
        // Identity is the call site, not the color: an animating value reconciles its
        // ColorSwatchValue in place rather than respawning the box every change.
        let mut entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(move || bsn! { @PlumeColorSwatch ColorSwatchValue({color}) });
        struct SwatchColor;
        let lin = color.to_linear();
        let key = (
            lin.red.to_bits(),
            lin.green.to_bits(),
            lin.blue.to_bits(),
            lin.alpha.to_bits(),
        );
        if entity.hash_update_typ::<SwatchColor>(Some(imm_id(key))) && !entity.will_be_spawned() {
            entity
                .entity_commands()
                .queue(move |mut e: EntityWorldMut| {
                    if let Some(mut value) = e.get_mut::<ColorSwatchValue>() {
                        value.0 = color;
                    }
                });
        }
        let hovered = entity.hovered();
        let will_be_spawned = entity.will_be_spawned();
        ImmResponse {
            clicked: false,
            changed: false,
            hovered,
            entity: entity.entity(),
            will_be_spawned,
            e: entity,
            kind: PhantomData,
        }
    }

    #[track_caller]
    fn color_picker(&mut self, color: &mut Color) -> ImmResponse<'_, 'w, 's> {
        // Identity is the call site: the picker retains its working HSV, so the
        // scene seeds the colour once and the capability syncs it thereafter.
        let initial = *color;
        let mut changed = false;
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(move || bsn! { @PlumeColorPicker { @initial_color: {initial} }})
            .plume_color(color, &mut changed);
        respond(entity, changed)
    }

    #[track_caller]
    fn color_edit(&mut self, color: &mut Color) -> ImmResponse<'_, 'w, 's> {
        let initial = *color;
        let mut changed = false;
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(move || bsn! { @PlumeColorEdit { @initial_color: {initial} }})
            .plume_color(color, &mut changed);
        respond(entity, changed)
    }

    #[track_caller]
    fn space(&mut self, length: Val) {
        self.ch_loc(loc_id(format!("{length:?}")))
            .on_spawn_apply_scene(move || space(length));
    }

    #[track_caller]
    fn button(&mut self, label: &str) -> ImmResponse<'_, 'w, 's, kind::Button> {
        let label_owned = label.to_owned();
        let entity = self.ch_loc(loc_id(label)).on_spawn_apply_scene(
            move || bsn! { @PlumeButton { @caption: bsn! { caption(label_owned) } } },
        );
        respond(entity, false)
    }

    #[track_caller]
    fn icon_button(&mut self, icon: FaIcon, label: &str) -> ImmResponse<'_, 'w, 's, kind::Button> {
        let label_owned = label.to_owned();
        // Keys on the face and label, not the glyph, so a glyph toggle reconciles in
        // place instead of respawning; see `tool_button`. The icon is the first
        // `Text` child, ahead of the label, so `set_icon_glyph` lands on it.
        let mut entity = self
            .ch_loc(loc_id((icon.face(), label)))
            .on_spawn_apply_scene(move || {
                bsn! { @PlumeButton { @caption: bsn_list! { fa_icon(icon), caption(label_owned) } } }
            });
        struct IconButtonGlyph;
        if entity.hash_update_typ::<IconButtonGlyph>(Some(imm_id(icon.glyph())))
            && !entity.will_be_spawned()
        {
            entity
                .entity_commands()
                .queue(move |mut e: EntityWorldMut| set_icon_glyph(&mut e, icon.glyph()));
        }
        respond(entity, false)
    }

    #[track_caller]
    fn tool_button(&mut self, icon: FaIcon) -> ImmResponse<'_, 'w, 's, kind::Button> {
        // Identity keys on the face, not the glyph, so toggling the glyph within a
        // face reconciles in place instead of respawning (a visible pop). The face
        // stays in the key because it selects the font, a runtime asset handle.
        let mut entity = self
            .ch_loc(loc_id(icon.face()))
            .on_spawn_apply_scene(move || {
                bsn! { @PlumeToolButton { @caption: bsn! { fa_icon(icon) } } }
            });
        struct ToolGlyph;
        if entity.hash_update_typ::<ToolGlyph>(Some(imm_id(icon.glyph())))
            && !entity.will_be_spawned()
        {
            entity
                .entity_commands()
                .queue(move |mut e: EntityWorldMut| set_icon_glyph(&mut e, icon.glyph()));
        }
        respond(entity, false)
    }

    #[track_caller]
    fn icon(&mut self, icon: FaIcon) -> ImmResponse<'_, 'w, 's> {
        // Keyed on the face (a runtime font handle), not the glyph, so toggling the
        // glyph within a face reconciles in place rather than respawning — as tool_button does.
        let mut entity = self
            .ch_loc(loc_id(icon.face()))
            .on_spawn_apply_scene(move || fa_icon(icon));
        struct IconGlyph;
        if entity.hash_update_typ::<IconGlyph>(Some(imm_id(icon.glyph())))
            && !entity.will_be_spawned()
        {
            entity
                .entity_commands()
                .queue(move |mut e: EntityWorldMut| {
                    if let Some(mut text) = e.get_mut::<Text>() {
                        text.0 = icon.glyph().to_owned();
                    }
                });
        }
        let hovered = entity.hovered();
        let will_be_spawned = entity.will_be_spawned();
        ImmResponse {
            clicked: false,
            changed: false,
            hovered,
            entity: entity.entity(),
            will_be_spawned,
            e: entity,
            kind: PhantomData,
        }
    }

    #[track_caller]
    fn button_container(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Button> {
        // Empty caption in the scene; the row's content comes from `f`. Left-aligned
        // so content packs from the leading edge rather than centering like a label.
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(|| {
                bsn! { @PlumeButton Node { justify_content: JustifyContent::Start } }
            })
            .add(f);
        respond(entity, false)
    }

    #[track_caller]
    fn checkbox(&mut self, value: &mut bool, label: &str) -> ImmResponse<'_, 'w, 's> {
        let label_owned = label.to_owned();
        let mut changed = false;
        let entity = self
            .ch_loc(loc_id(label))
            .on_spawn_apply_scene(
                move || bsn! { @PlumeCheckbox { @caption: bsn! { caption(label_owned) } } },
            )
            .plume_checked(value, &mut changed);
        respond(entity, changed)
    }

    #[track_caller]
    fn disclosure(&mut self, open: &mut bool) -> ImmResponse<'_, 'w, 's> {
        let mut changed = false;
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(|| bsn! { @PlumeDisclosure })
            .plume_checked(open, &mut changed);
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
        // The checked sync sees a derived bool, so a user click lands as a pending
        // true that is written back through `value` below.
        let mut checked = *value == variant;
        let mut changed = false;
        let entity = self
            .ch_loc(loc_id(label))
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
            .ch_loc(loc_id(()))
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
            .ch_loc(loc_id((min.to_bits(), max.to_bits())))
            .on_spawn_apply_scene(move || bsn! { @PlumeSlider { @min: {min}, @max: {max} } })
            .plume_value(value, &mut changed);
        respond(entity, changed)
    }

    #[track_caller]
    fn number(&mut self, value: &mut f32) -> ImmResponse<'_, 'w, 's, kind::Number> {
        let initial = *value;
        let mut changed = false;
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(move || bsn! { @PlumeNumberInput { @value: {initial} } })
            .plume_value(value, &mut changed);
        respond(entity, changed)
    }

    #[track_caller]
    fn text_edit(&mut self, text: &mut String) -> ImmResponse<'_, 'w, 's, kind::Text> {
        let initial = text.clone();
        let mut changed = false;
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(move || bsn! { @PlumeTextInput { @value: {initial} } })
            .plume_text(text, &mut changed);
        respond(entity, changed)
    }

    #[track_caller]
    fn select<T: PartialEq>(
        &mut self,
        selected: &mut T,
        f: impl FnOnce(&mut ImmSelect<T>),
    ) -> ImmResponse<'_, 'w, 's, kind::Select> {
        let mut collector = ImmSelect {
            options: Vec::new(),
        };
        f(&mut collector);
        let options = collector.options;

        // A `selected` naming no declared option (an empty select, or a value whose
        // option was conditionally dropped) shows the first without writing back.
        let mut index = options
            .iter()
            .position(|option| option.key == *selected)
            .unwrap_or(0);
        let initial = index;
        let labels: Vec<String> = options.iter().map(|option| option.label.clone()).collect();
        let mut changed = false;
        // The labels key the widget: the rows are seeded at spawn, so an edited
        // option list has to respawn rather than keep the stale popup.
        let entity = self
            .ch_loc(loc_id(&labels))
            .on_spawn_apply_scene(move || {
                bsn! { @PlumeSelect { @options: {list_rows_from_strings(labels, Some(initial))} } }
            })
            .plume_select(&mut index, &mut changed);
        if changed && let Some(option) = options.into_iter().nth(index) {
            *selected = option.key;
        }
        respond(entity, changed)
    }

    #[track_caller]
    fn dialog<'a>(&'a mut self, title: &str, open: &'a mut bool) -> ImmDialog<'a, 'w, 's> {
        ImmDialog {
            ui: self,
            caller: Location::caller(),
            title: title.to_owned(),
            icon: None,
            open,
            layout: DialogLayout {
                width: Val::Auto,
                height: Val::Auto,
                max_height: Val::Auto,
                left: size::DEFAULT_DIALOG_POS.x,
                top: size::DEFAULT_DIALOG_POS.y,
                closable: true,
                movable: true,
                body_padding: size::PAD.into(),
            },
        }
    }

    #[track_caller]
    fn panel(&mut self) -> ImmPanel<'_, 'w, 's> {
        ImmPanel {
            ui: self,
            caller: Location::caller(),
            layout: DialogLayout {
                width: Val::Auto,
                height: Val::Auto,
                max_height: Val::Auto,
                left: size::DEFAULT_DIALOG_POS.x,
                top: size::DEFAULT_DIALOG_POS.y,
                closable: false,
                movable: false,
                body_padding: size::PAD.into(),
            },
        }
    }

    #[track_caller]
    fn horizontal(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Row> {
        let entity = self.ch_loc(loc_id(())).on_spawn_apply_scene(row).add(f);
        respond(entity, false)
    }

    #[track_caller]
    fn vertical(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Column> {
        let entity = self.ch_loc(loc_id(())).on_spawn_apply_scene(column).add(f);
        respond(entity, false)
    }

    #[track_caller]
    fn screen(&mut self, f: impl FnOnce(&mut Ui<'w, 's>)) -> ImmResponse<'_, 'w, 's, kind::Screen> {
        let entity = self.ch_loc(loc_id(())).on_spawn_apply_scene(screen).add(f);
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
            .ch_loc(loc_id(header))
            .on_spawn_apply_scene(move || {
                bsn! {
                    section_frame(
                        bsn_list!(caption_small_caps(header_owned)),
                        true,
                        bsn_list!()
                    )
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
    fn tabs<'t, T: PartialEq>(
        &mut self,
        selected: &mut T,
        f: impl FnOnce(&mut ImmTabs<'t, 'w, 's, T>),
    ) -> ImmResponse<'_, 'w, 's, kind::Tabs> {
        let mut collector = ImmTabs {
            entries: Vec::new(),
        };
        f(&mut collector);
        let entries = collector.entries;

        // A `selected` naming no declared tab (an empty container, or a value whose
        // tab was conditionally dropped) shows the first tab without writing back.
        let mut index = entries
            .iter()
            .position(|entry| entry.key == *selected)
            .unwrap_or(0);
        let initial = index;
        let mut changed = false;
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(move || tabs_frame(initial))
            .plume_select(&mut index, &mut changed);

        let mut strip_items = Vec::with_capacity(entries.len());
        let mut selected_body = None;
        for (slot, entry) in entries.into_iter().enumerate() {
            let TabEntry {
                key,
                label,
                icon,
                enabled,
                body,
            } = entry;
            strip_items.push((label, icon, enabled));
            if slot == index {
                selected_body = Some(body);
                if changed {
                    *selected = key;
                }
            }
        }

        let entity = entity.add(move |ui| {
            ui.ch_id("tab_strip")
                .on_spawn_apply_scene(tab_strip)
                .add(move |ui| {
                    for (slot, (label, icon, enabled)) in strip_items.into_iter().enumerate() {
                        // The label and glyph key the tab: a renamed tab respawns
                        // rather than keeping the old caption at the same slot.
                        ui.ch_id(("tab", slot, &label, icon.map(FaIcon::glyph)))
                            .on_spawn_apply_scene(move || tab_button(label, icon))
                            .interactions_enabled(enabled);
                    }
                });
            let body = ui.ch_id("tab_body").on_spawn_apply_scene(tab_body);
            if let Some(selected_body) = selected_body {
                body.add(selected_body);
            }
        });
        respond(entity, changed)
    }

    #[track_caller]
    fn scroll_area(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::ScrollArea> {
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(scroll_frame)
            .add(move |ui| {
                // The viewport's entity is known before its spawn command flushes,
                // so the scrollbar can point at the viewport it drives.
                let viewport = ui
                    .ch_id("scroll_area")
                    .on_spawn_apply_scene(scroll_viewport)
                    .add(f)
                    .entity();
                ui.ch_id("scrollbar")
                    .on_spawn_apply_scene(move || scrollbar(viewport));
            });
        respond(entity, false)
    }

    #[track_caller]
    fn flex_spacer(&mut self) {
        self.ch_loc(loc_id(())).on_spawn_apply_scene(flex_spacer);
    }

    fn push_id<R>(&mut self, id: impl core::hash::Hash, f: impl FnOnce(&mut Ui<'w, 's>) -> R) -> R {
        let mut scope = self.with_add_id_pref(id);
        f(&mut scope)
    }
}

/// Option collector handed to [`PlumeImm::select`]'s closure: declare one
/// [`option`](Self::option) per row, in popup order.
pub struct ImmSelect<T> {
    options: Vec<SelectOption<T>>,
}

// One declared option: the value it stands for and the row's label.
struct SelectOption<T> {
    key: T,
    label: String,
}

impl<T> ImmSelect<T> {
    /// Declare an option standing for `key`, labeled `label`.
    pub fn option(&mut self, key: T, label: &str) {
        self.options.push(SelectOption {
            key,
            label: label.to_owned(),
        });
    }
}

/// Tab collector handed to [`PlumeImm::tabs`]'s closure: declare one
/// [`tab`](Self::tab) per tab, in strip order.
pub struct ImmTabs<'t, 'w, 's, T> {
    entries: Vec<TabEntry<'t, 'w, 's, T>>,
}

// One declared tab. The body is boxed because every tab's closure is built while
// only the selected one is called.
struct TabEntry<'t, 'w, 's, T> {
    key: T,
    label: String,
    icon: Option<FaIcon>,
    enabled: bool,
    body: Box<dyn FnOnce(&mut Ui<'w, 's>) + 't>,
}

impl<'t, 'w, 's, T> ImmTabs<'t, 'w, 's, T> {
    /// Declare a tab standing for `key`, labeled `label`, whose contents `body`
    /// builds while it is the selected tab. Chain `.icon()`/`.enabled()` on the
    /// returned handle.
    pub fn tab(
        &mut self,
        key: T,
        label: &str,
        body: impl FnOnce(&mut Ui<'w, 's>) + 't,
    ) -> ImmTab<'_, 't, 'w, 's, T> {
        self.entries.push(TabEntry {
            key,
            label: label.to_owned(),
            icon: None,
            enabled: true,
            body: Box::new(body),
        });
        ImmTab {
            entry: self
                .entries
                .last_mut()
                .expect("the entry was just pushed onto entries"),
        }
    }
}

/// Handle to a just-declared tab, for its per-tab options.
pub struct ImmTab<'a, 't, 'w, 's, T> {
    entry: &'a mut TabEntry<'t, 'w, 's, T>,
}

impl<T> ImmTab<'_, '_, '_, '_, T> {
    /// Leading FontAwesome icon, before the label.
    pub fn icon(self, icon: FaIcon) -> Self {
        self.entry.icon = Some(icon);
        self
    }

    /// Grey the tab out and ignore clicks on it. A disabled tab that is
    /// nonetheless selected still shows its body.
    pub fn enabled(self, enabled: bool) -> Self {
        self.entry.enabled = enabled;
        self
    }
}

/// Deferred dialog configuration returned by [`PlumeImm::dialog`]; the dialog only
/// exists once [`Self::show`] runs.
#[must_use = "a dialog does nothing until .show(|ui| …) builds it"]
pub struct ImmDialog<'a, 'w, 's> {
    ui: &'a mut Ui<'w, 's>,
    caller: &'static Location<'static>,
    title: String,
    icon: Option<FaIcon>,
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
    body_padding: UiRect,
}

impl DialogLayout {
    /// Whether the body needs the scrolling machinery: either height knob bounds
    /// the dialog, so its content can no longer be assumed to fit.
    fn scrolls(&self) -> bool {
        self.height != Val::Auto || self.max_height != Val::Auto
    }
}

impl<'e, 'w, 's> ImmDialog<'e, 'w, 's> {
    /// Fix the dialog's width (default `Val::Auto` hugs the content). Rows that
    /// distribute space (`.grow()`, `flex_spacer`) need one to resolve against.
    pub fn width(mut self, width: Val) -> Self {
        self.layout.width = width;
        self
    }

    /// Fix the dialog's outer height, title bar included, scrolling the body once
    /// the content outgrows it. Prefer [`Self::max_height`] unless the dialog
    /// should hold its size while near-empty.
    pub fn height(mut self, height: Val) -> Self {
        self.layout.height = height;
        self
    }

    /// Cap the dialog's outer height: it hugs its content until it would exceed
    /// `max_height`, then stops growing and scrolls the body.
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

    /// Leading FontAwesome icon in the title bar, before the title text.
    pub fn icon(mut self, icon: FaIcon) -> Self {
        self.icon = Some(icon);
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

    /// Set body padding.
    pub fn pad(mut self, pad: UiRect) -> Self {
        self.layout.body_padding = pad;
        self
    }

    /// Build the dialog and its body. While `*open` the dialog exists and `f`
    /// fills its body; the ✕ writes back through `open`.
    pub fn show(
        self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> Option<ImmResponse<'e, 'w, 's, kind::Dialog>> {
        if !*self.open {
            return None;
        }
        let id = ImmIdBuilder::Hierarchy(ImmId::new((self.caller, self.title.as_str())));
        let (title, icon, layout) = (self.title, self.icon, self.layout);
        let mut entity = self
            .ui
            .ch_loc(id)
            .on_spawn_apply_scene(move || imm_dialog_scene(title, icon, layout));
        if entity.close_requested() {
            *self.open = false;
            entity.entity_commands().despawn();
            return None;
        }
        Some(reconcile_frame_body(entity, layout, f))
    }
}

/// Deferred panel configuration returned by [`PlumeImm::panel`]; the panel only
/// exists once [`Self::show`] runs.
#[must_use = "a panel does nothing until .show(|ui| …) builds it"]
pub struct ImmPanel<'a, 'w, 's> {
    ui: &'a mut Ui<'w, 's>,
    caller: &'static Location<'static>,
    layout: DialogLayout,
}

impl<'e, 'w, 's> ImmPanel<'e, 'w, 's> {
    /// Fix the panel's width (default `Val::Auto` hugs the content).
    pub fn width(mut self, width: Val) -> Self {
        self.layout.width = width;
        self
    }

    /// Fix the panel's outer height, scrolling the body once the content outgrows it.
    pub fn height(mut self, height: Val) -> Self {
        self.layout.height = height;
        self
    }

    /// Cap the panel's outer height: it hugs its content until it would exceed
    /// `max_height`, then stops growing and scrolls the body.
    pub fn max_height(mut self, max_height: Val) -> Self {
        self.layout.max_height = max_height;
        self
    }

    /// Position (the panel is absolutely positioned). Spawn-time only.
    pub fn at(mut self, left: Val, top: Val) -> Self {
        self.layout.left = left;
        self.layout.top = top;
        self
    }

    /// Build the panel and its body.
    pub fn show(self, f: impl FnOnce(&mut Ui<'w, 's>)) -> ImmResponse<'e, 'w, 's, kind::Dialog> {
        let id = ImmIdBuilder::Hierarchy(ImmId::new(self.caller));
        let layout = self.layout;
        let entity = self
            .ui
            .ch_loc(id)
            .on_spawn_apply_scene(move || imm_panel_scene(layout));
        reconcile_frame_body(entity, layout, f)
    }
}

/// Reconcile a dialog/panel frame's app-owned size and fill its body, wrapping the
/// content in the scrolling machinery when a height knob bounds it. Position is not
/// re-applied — the user's dragging owns it after spawn.
fn reconcile_frame_body<'e, 'w, 's>(
    mut entity: ImmEntity<'e, 'w, 's, PlumeCaps>,
    layout: DialogLayout,
    f: impl FnOnce(&mut Ui<'w, 's>),
) -> ImmResponse<'e, 'w, 's, kind::Dialog> {
    struct FrameSizeKey;
    let size_key = format!(
        "{:?}{:?}{:?}",
        layout.width, layout.height, layout.max_height
    );
    if entity.hash_update_typ::<FrameSizeKey>(Some(imm_id(size_key))) {
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
    let entity = entity.add(move |ui| {
        let body = ui
            .ch_id("dialog_body")
            .on_spawn_apply_scene(|| bsn! { @PlumeDialogBody { @padding: {layout.body_padding} } });
        if !scrolls {
            body.add(f);
            return;
        }
        body.add(move |ui| {
            ui.ch_id("scroll_frame")
                .on_spawn_apply_scene(scroll_frame)
                .add(move |ui| {
                    // The scroll area's entity is known before its spawn command
                    // flushes, so the scrollbar can point at the viewport it drives.
                    let viewport = ui
                        .ch_id("scroll_area")
                        .on_spawn_apply_scene(scroll_viewport)
                        .add(f)
                        .entity();
                    ui.ch_id("scrollbar")
                        .on_spawn_apply_scene(move || scrollbar(viewport));
                });
        });
    });
    respond(entity, false)
}

// Combining the caller location with a key means label/options changes respawn the
// widget instead of leaving stale scene content. `#[track_caller]` bubbles the
// location through the (also `#[track_caller]`) widget methods to the user's call
// site, so the same widget at two source lines already gets distinct ids; only a
// repeated call site (e.g. a loop) collides, which [`PlumeChild::ch_loc`] resolves.
#[track_caller]
fn loc_id(key: impl core::hash::Hash) -> ImmIdBuilder {
    ImmIdBuilder::Hierarchy(ImmId::new((Location::caller(), key)))
}

/// Salt folded into a repeated id's suffix; a fixed tag so a disambiguated id can
/// never coincide with a genuine `(location, key)` base.
const OCCURRENCE_SALT: u32 = 0x506c_756d; // "Plum"

/// Child creation with plume-side occurrence disambiguation.
///
/// Unpatched `bevy_immediate` maps each hierarchy id to exactly one entity, so two
/// widgets built from the same call site — a helper called in a loop — would land
/// on the same entity. This threads a per-`(parent, base id)` occurrence counter
/// (held in [`PlumeOccurrences`]): the first use keeps the plain id (so a widget
/// that appears once, or a conditional sibling, never shifts the others) and each
/// repeat takes a distinct suffix. Keeping it here lets plume track upstream
/// `bevy_immediate` with no `resolve`-time patch.
trait PlumeChild<'w, 's> {
    fn ch_loc(&mut self, id: ImmIdBuilder) -> ImmEntity<'_, 'w, 's, PlumeCaps>;
}

impl<'w, 's> PlumeChild<'w, 's> for Ui<'w, 's> {
    fn ch_loc(&mut self, id: ImmIdBuilder) -> ImmEntity<'_, 'w, 's, PlumeCaps> {
        let ImmIdBuilder::Hierarchy(base) = id else {
            // Auto/Unique carry their own uniqueness contract; pass them through.
            return self.ch_with_manual_id(id);
        };
        let parent = self.current_imm_id();
        let occurrence = {
            let mut table = self
                .ctx_mut()
                .cap_resources
                .resources
                .get_mut::<PlumeOccurrences>()
                .expect("PlumeOccurrences is registered by CapabilityPlumeIds");
            let counter = table.0.entry(parent.with(base)).or_insert(0);
            let occurrence = *counter;
            *counter += 1;
            occurrence
        };
        let id = if occurrence == 0 {
            base
        } else {
            base.with((OCCURRENCE_SALT, occurrence))
        };
        self.ch_with_manual_id(ImmIdBuilder::Hierarchy(id))
    }
}

/// Set the `glyph` on a tool button's `fa_icon` `Text` child. The font stays as
/// spawned, since the face keys the button's identity.
fn set_icon_glyph(button: &mut EntityWorldMut, glyph: &'static str) {
    let children: Vec<Entity> = button
        .get::<Children>()
        .map(|children| children.iter().copied().collect())
        .unwrap_or_default();
    button.world_scope(|world| {
        for child in children {
            if let Some(mut text) = world.get_mut::<Text>(child) {
                text.0 = glyph.to_owned();
                break;
            }
        }
    });
}

fn respond<'r, 'w, 's, K>(
    mut entity: ImmEntity<'r, 'w, 's, PlumeCaps>,
    changed: bool,
) -> ImmResponse<'r, 'w, 's, K> {
    let clicked = entity.activated();
    let hovered = entity.hovered();
    let will_be_spawned = entity.will_be_spawned();
    ImmResponse {
        clicked,
        changed,
        hovered,
        entity: entity.entity(),
        will_be_spawned,
        e: entity,
        kind: PhantomData,
    }
}

fn imm_dialog_scene(title: String, icon: Option<FaIcon>, layout: DialogLayout) -> impl Scene {
    let DialogLayout {
        width,
        height,
        max_height,
        left,
        top,
        closable,
        movable,
        // Body padding lives on the reconciled `PlumeDialogBody`, not the frame.
        body_padding: _,
    } = layout;
    bsn! {
        // Empty body: the imm layer reconciles the body itself.
        dialog_frame(DialogChrome {
            body: Box::new(bsn_list!()),
            header: Some(DialogHeader {
                title: Box::new(bsn_list![ {icon.map(|icon| bsn! { fa_icon(icon) })}, caption(title)]),
                closable,
                movable,
            }),
            width,
            height,
            max_height,
            left,
            top,
        })
        on(|close: On<RequestClose>, mut commands: Commands| {
            commands.entity(close.event_target()).insert(DialogCloseRequested);
        })
    }
}

fn imm_panel_scene(layout: DialogLayout) -> impl Scene {
    let DialogLayout {
        width,
        height,
        max_height,
        left,
        top,
        ..
    } = layout;
    // Headerless: `header: None` drops the title bar (and so the ✕ and drag handle),
    // and there is no `RequestClose` observer, since a panel has no ✕.
    bsn! {
        dialog_frame(DialogChrome {
            body: Box::new(bsn_list!()),
            header: None,
            width,
            height,
            max_height,
            left,
            top,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imm::{ImmPlugin, PlumeRoot};
    use bevy::MinimalPlugins;
    use bevy::app::{App, Update};
    use bevy::ecs::resource::Resource;
    use bevy::ecs::system::ResMut;

    // Records the sibling entities produced each frame, so the test can check both
    // within-frame distinctness and across-frame stability.
    #[derive(Resource, Default)]
    struct Recorded(Vec<Vec<Entity>>);

    // Three widgets from ONE call site (the loop body) under one parent. With
    // occurrence disambiguation they map to three distinct, stable entities;
    // without it they collide onto a single entity once the id->entity mapping is
    // populated (i.e. from the second frame on).
    fn three_siblings(mut root: PlumeRoot, mut recorded: ResMut<Recorded>) {
        let mut frame = Vec::new();
        root.push_id("sibling-test", |ui| {
            for _ in 0..3 {
                frame.push(ui.ch_loc(loc_id(())).entity());
            }
        });
        recorded.0.push(frame);
    }

    #[test]
    fn repeated_call_site_gets_distinct_stable_entities() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(ImmPlugin)
            .init_resource::<Recorded>()
            .add_systems(Update, three_siblings);

        // Frame 1 spawns fresh; frame 2 resolves against the now-populated mapping,
        // which is where any id collision surfaces.
        app.update();
        app.update();

        let recorded = &app.world().resource::<Recorded>().0;
        assert_eq!(recorded.len(), 2, "system should have run twice");
        for (frame, e) in recorded.iter().enumerate() {
            assert_eq!(e.len(), 3);
            assert!(
                e[0] != e[1] && e[1] != e[2] && e[0] != e[2],
                "frame {frame}: siblings from one call site must be distinct, got {e:?}",
            );
        }
        assert_eq!(
            recorded[0], recorded[1],
            "retained entities must be reused across frames — proof the mapping is \
             populated, so a real collision would surface as a duplicate",
        );
    }
}
