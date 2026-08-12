//! Widget methods on the immediate-mode context; each wraps a retained plume scene.
use core::marker::PhantomData;
use core::ops::RangeInclusive;
use core::panic::Location;

use bevy::camera::visibility::Visibility;
use bevy::color::Color;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EntityEvent;
use bevy::ecs::hierarchy::Children;
use bevy::ecs::observer::On;
use bevy::ecs::system::Commands;
use bevy::ecs::world::EntityWorldMut;
use bevy::scene::{Scene, bsn, bsn_list, on, template_value};
use bevy::ui::widget::Text;
use bevy::ui::{JustifyContent, Node, UiRect, Val};
use bevy::ui_widgets::RequestClose;
use bevy_immediate::ui::activated::ImmUiActivated;
use bevy_immediate::ui::disabled::ImmUiInteractionsDisabled;
use bevy_immediate::ui::interaction::ImmUiInteraction;
use bevy_immediate::{ImmEntity, ImmId, ImmIdBuilder, imm_id};

use super::caps::{
    ImmPlumeChecked, ImmPlumeColor, ImmPlumeDialog, ImmPlumeMenu, ImmPlumeSelect, ImmPlumeSplit,
    ImmPlumeText, ImmPlumeValue, PlumeOccurrences,
};
use super::{ImmEntityExt, ImmResponse, PlumeCaps, Ui, kind};
use crate::constants::{FaIcon, size};
use crate::containers::{
    CloseRequested, DialogChrome, DialogHeader, DismissScope, PlumeDialogBody, PlumePopup,
    PopupAnchor, PopupDismiss, PopupPlacement, ScrollAxis, SplitAxis, SplitCollapsible, SplitPane,
    column, dialog_frame, flex_spacer, popup_socket, row, screen, scroll_content, scroll_frame,
    scroll_viewport, scrollbar, section_body, section_frame, separator, space, splitter_divider,
    splitter_frame, splitter_pane, tab_body, tab_button, tab_chrome, tab_strip, tab_strip_frame,
    tabs_frame,
};
use crate::controls::{
    ColorSwatchValue, MenuButtonRole, PlumeButton, PlumeCheckbox, PlumeColorEdit, PlumeColorPicker,
    PlumeColorSwatch, PlumeDisclosure, PlumeMenuBar, PlumeNumberInput, PlumeNumberInputProps,
    PlumeRadio, PlumeSelect, PlumeSlider, PlumeTextInput, PlumeToggleSwitch, PlumeToolButton,
    SelectedIndex, imm_menu_anchor, imm_menu_frame,
};
use crate::display::{caption, caption_large, caption_small_caps, fa_icon};
use crate::utils::numeric::Numeric;

/// Widget calls for immediate-mode systems. Implemented by [`Ui`]; import it
/// wherever imm systems are written.
pub trait PlumeImm<'w, 's> {
    /// Themed text in the current container's font and color. The response's
    /// `clicked`/`changed` are always false (text has no activation behavior);
    /// the builders and `hovered` work as usual.
    fn caption(&mut self, text: &str) -> ImmResponse<'_, 'w, 's, kind::Caption>;

    /// Hairline rule across the container: a horizontal line in a
    /// [`Self::vertical`], a vertical one in a [`Self::horizontal`]. Spans the
    /// container's content box; chain `.full_bleed()` to run edge to edge
    /// through the padding. The response's `clicked`/`changed` are always false.
    fn separator(&mut self) -> ImmResponse<'_, 'w, 's, kind::Separator>;

    /// Non-interactive color preview: a themed, bordered rounded box filled with
    /// `color`. Defaults to a [`ROW_HEIGHT`](crate::constants::size::ROW_HEIGHT)
    /// square; chain `.square()`/`.width()`/`.height()` to resize.
    fn color_swatch(&mut self, color: Color) -> ImmResponse<'_, 'w, 's, kind::Swatch>;

    /// Interactive HSV color picker: a saturation/value plane, a hue bar and a
    /// preview swatch. Two-way bound to `color`; `.changed` on the response fires
    /// when the user drags to a new color. The layout is fixed by the control.
    fn color_picker(&mut self, color: &mut Color) -> ImmResponse<'_, 'w, 's>;

    /// Editable color swatch: a swatch that opens a color-picker popup on click,
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
    fn slider<T: Numeric>(
        &mut self,
        value: &mut T,
        range: RangeInclusive<T>,
    ) -> ImmResponse<'_, 'w, 's, kind::Slider>;

    /// Numeric input field bound to `value`.
    fn number<T: Numeric>(&mut self, value: &mut T) -> ImmResponse<'_, 'w, 's, kind::Number>;

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

    /// Horizontal menu bar strip; `f` declares its drop-down menus on the
    /// [`ImmMenuBar`] context. Menus open on click, switch on hover while one
    /// is open, and close themselves when an item is picked, on Escape, or when
    /// focus leaves — the bodies only run while their menu is open.
    fn menu_bar(&mut self, f: impl FnOnce(&mut ImmMenuBar<'_, 'w, 's>)) -> ImmResponse<'_, 'w, 's>;

    /// Headerless floating surface — a
    /// [`dialog`](crate::imm::PlumeRoot::dialog) with no title bar (and so no
    /// title, ✕ or drag). Positioned chrome only; the caller controls whether it's
    /// drawn. Unlike the two root surfaces this is useful nested, pinned to the
    /// container it is declared in — see [`ImmPanel::at_corner`].
    ///
    /// Not inside a [`scroll_area_vertical`](Self::scroll_area_vertical), which does not float
    /// over but scrolls with, stretching the range. Declare it beside one instead.
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

    /// Tab container: a header strip over a body showing one tab at a time.
    /// `f` declares the tabs on the [`ImmTabs`] collector; only the selected tab's
    /// `.body()` closure runs, so hidden tabs cost nothing. Tabs that all finish
    /// with `.no_body()` leave the container a bare strip. A strip with more tabs than room
    /// squeezes them toward [`size::TAB_MIN_WIDTH`], then scrolls.
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
    fn scroll_area_vertical(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::ScrollArea>;

    /// [`scroll_area_vertical`](Self::scroll_area_vertical) on its side: the content stacks
    /// like a [`Self::horizontal`] and scrolls sideways once it outgrows the width
    /// set via `.max_width()`/`.width()`.
    fn scroll_area_horizontal(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::ScrollArea>;

    /// Two panes side by side, with a divider the user drags to re-proportion
    /// them.
    ///
    /// `fraction` is the first pane's share of the width, `0.0..=1.0`, written
    /// back as the divider moves — so it is the app's to keep and to save.
    /// `.changed` on the response reports a move. Chain
    /// [`min_panes`](ImmResponse::min_panes) to stop either pane getting too
    /// small.
    ///
    /// Two closures rather than a collector: a split has exactly two panes, and
    /// that is worth saying in the signature rather than in the docs.
    fn split_horizontal(
        &mut self,
        fraction: &mut f32,
        first: impl FnOnce(&mut Ui<'w, 's>),
        second: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Split>;

    /// [`split_horizontal`](Self::split_horizontal) with the panes stacked and
    /// the divider across them; `fraction` is the top pane's share of the height.
    fn split_vertical(
        &mut self,
        fraction: &mut f32,
        first: impl FnOnce(&mut Ui<'w, 's>),
        second: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Split>;

    /// Hosts a retained scene inside an immediate pass: `f` builds it the frame
    /// its entity is first spawned and never again, applied to that entity, so
    /// what the scene spawns belongs to it.
    ///
    /// The bridge for content an immediate pass must not rebuild every frame —
    /// a canvas holding a layout the user drags around, a subtree some other
    /// plugin owns and reconciles. Only the imm layer's own entities are
    /// tracked, so a hosted scene is never reconciled against: it persists
    /// untouched for as long as the call site keeps running, and is despawned
    /// with its host when that stops. Keep the entity from the response if the
    /// scene has to be found again.
    ///
    /// The host sizes it (`.grow()`, `.width()`, `.height()`); the scene styles
    /// itself, since an imm pass cannot know what it built.
    fn scene<S: Scene>(&mut self, f: impl FnOnce() -> S) -> ImmResponse<'_, 'w, 's, kind::Scene>;

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

// The two surfaces that open a pass, handed out by [`PlumeRoot`] alone. Nested
// they would measure against their parent rather than the viewport, be clipped by
// any ancestor that clips, and nest a `TabGroup` inside the one they opened in.
impl<'w, 's> Ui<'w, 's> {
    #[track_caller]
    pub(crate) fn screen(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Screen> {
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(screen)
            .add_ui(f);
        respond(entity, false)
    }

    #[track_caller]
    pub(crate) fn dialog<'a>(
        &'a mut self,
        title: &str,
        open: &'a mut bool,
    ) -> ImmDialog<'a, 'w, 's> {
        ImmDialog {
            ui: self,
            caller: Location::caller(),
            title: title.to_owned(),
            open,
            layout: DialogLayout {
                width: Val::Auto,
                height: Val::Auto,
                max_height: Val::Auto,
                inset: UiRect {
                    left: size::DEFAULT_DIALOG_POS.x,
                    top: size::DEFAULT_DIALOG_POS.y,
                    ..UiRect::AUTO
                },
                closable: true,
                movable: true,
            },
        }
    }
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
            integral: false,
            e: entity,
            kind: PhantomData,
        }
    }

    #[track_caller]
    fn separator(&mut self) -> ImmResponse<'_, 'w, 's, kind::Separator> {
        let mut entity = self.ch_loc(loc_id(())).on_spawn_apply_scene(separator);
        let hovered = entity.hovered();
        let will_be_spawned = entity.will_be_spawned();
        ImmResponse {
            clicked: false,
            changed: false,
            hovered,
            entity: entity.entity(),
            will_be_spawned,
            integral: false,
            e: entity,
            kind: PhantomData,
        }
    }

    #[track_caller]
    fn color_swatch(&mut self, color: Color) -> ImmResponse<'_, 'w, 's, kind::Swatch> {
        // Identity is the call site, not the color: an animating value reconciles its
        // ColorSwatchValue in place rather than respawning the box every change.
        let mut entity = self.ch_loc(loc_id(())).on_spawn_apply_scene(move || {
            bsn! {
                @PlumeColorSwatch
                ColorSwatchValue(color)
            }
        });
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
            integral: false,
            e: entity,
            kind: PhantomData,
        }
    }

    #[track_caller]
    fn color_picker(&mut self, color: &mut Color) -> ImmResponse<'_, 'w, 's> {
        // Identity is the call site: the picker retains its working HSV, so the
        // scene seeds the color once and the capability syncs it thereafter.
        let initial = *color;
        let mut changed = false;
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(move || bsn! { @PlumeColorPicker { @initial_color: initial } })
            .plume_color(color, &mut changed);
        respond(entity, changed)
    }

    #[track_caller]
    fn color_edit(&mut self, color: &mut Color) -> ImmResponse<'_, 'w, 's> {
        let initial = *color;
        let mut changed = false;
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(move || bsn! { @PlumeColorEdit { @initial_color: initial } })
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
                bsn! {
                    @PlumeButton {
                        @caption: bsn_list![
                            fa_icon(icon),
                            caption(label_owned),
                        ],
                    }
                }
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
            integral: false,
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
                bsn! {
                    @PlumeButton
                    Node { justify_content: JustifyContent::Start }
                }
            })
            .add_ui(f);
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
    fn slider<T: Numeric>(
        &mut self,
        value: &mut T,
        range: RangeInclusive<T>,
    ) -> ImmResponse<'_, 'w, 's, kind::Slider> {
        let (min, max) = (range.start().to_f32(), range.end().to_f32());
        // An integer value snaps: whole-number drags and single-unit arrow keys,
        // rather than a continuous sweep that narrows to the same integer.
        let (step, precision) = if T::INTEGRAL {
            (Some(1.0), Some(0))
        } else {
            (None, None)
        };
        let mut changed = false;
        let entity = self
            .ch_loc(loc_id((min.to_bits(), max.to_bits())))
            .on_spawn_apply_scene(move || {
                bsn! {
                    @PlumeSlider {
                        @min: min,
                        @max: max,
                        @step: step,
                        @precision: precision,
                    }
                }
            })
            .plume_value(value, &mut changed);
        respond_numeric::<T, _>(entity, changed)
    }

    #[track_caller]
    fn number<T: Numeric>(&mut self, value: &mut T) -> ImmResponse<'_, 'w, 's, kind::Number> {
        let initial = value.to_f32();
        // An integer field shows whole numbers and clamps to the type's own
        // limits, so a typed value can't silently saturate on the way back.
        let PlumeNumberInputProps {
            precision,
            min,
            max,
            ..
        } = PlumeNumberInputProps::default();
        let (precision, min, max) = if T::INTEGRAL {
            (0, T::MIN.to_f32(), T::MAX.to_f32())
        } else {
            (precision, min, max)
        };
        let mut changed = false;
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(move || {
                bsn! {
                    @PlumeNumberInput {
                        @value: initial,
                        @precision: precision,
                        @min: min,
                        @max: max,
                    }
                }
            })
            .plume_value(value, &mut changed);
        respond_numeric::<T, _>(entity, changed)
    }

    #[track_caller]
    fn text_edit(&mut self, text: &mut String) -> ImmResponse<'_, 'w, 's, kind::Text> {
        let initial = text.clone();
        let mut changed = false;
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(move || bsn! { @PlumeTextInput { @value: initial } })
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
        let labels: Vec<(String, bool)> = options
            .iter()
            .map(|option| (option.label.clone(), option.enabled))
            .collect();
        let mut changed = false;
        // The labels key the widget: the options are seeded at spawn, so an edited
        // option list — a renamed label or a flipped `enabled` — has to respawn
        // rather than keep stale rows.
        let entity = self
            .ch_loc(loc_id(&labels))
            .on_spawn_apply_scene(move || {
                bsn! { @PlumeSelect { @options: labels, @selected: initial } }
            })
            .plume_select(&mut index, &mut changed);
        if changed && let Some(option) = options.into_iter().nth(index) {
            *selected = option.key;
        }
        respond(entity, changed)
    }

    #[track_caller]
    fn menu_bar(&mut self, f: impl FnOnce(&mut ImmMenuBar<'_, 'w, 's>)) -> ImmResponse<'_, 'w, 's> {
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(|| bsn! { @PlumeMenuBar })
            .add_ui(|ui| f(&mut ImmMenuBar { ui }));
        respond(entity, false)
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
                inset: UiRect {
                    left: size::DEFAULT_DIALOG_POS.x,
                    top: size::DEFAULT_DIALOG_POS.y,
                    ..UiRect::AUTO
                },
                closable: false,
                movable: false,
            },
        }
    }

    #[track_caller]
    fn horizontal(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Row> {
        let entity = self.ch_loc(loc_id(())).on_spawn_apply_scene(row).add_ui(f);
        respond(entity, false)
    }

    #[track_caller]
    fn vertical(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Column> {
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(column)
            .add_ui(f);
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
                        bsn_list![caption_small_caps(header_owned)],
                        true,
                        bsn_list![]
                    )
                }
            })
            .add_ui(|ui| {
                ui.ch_id("section_body")
                    .on_spawn_apply_scene(section_body)
                    .add_ui(f);
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
            .on_spawn_apply_scene(
                move || bsn! { tabs_frame() template_value(SelectedIndex(initial)) },
            )
            .plume_select(&mut index, &mut changed);

        // A bare strip: no body node taking room the caller gave the container.
        let has_bodies = entries.iter().any(|entry| entry.body.is_some());
        let mut strip_items = Vec::with_capacity(entries.len());
        let mut selected_body = None;
        for (slot, entry) in entries.into_iter().enumerate() {
            let TabEntry {
                key,
                header,
                enabled,
                min_width,
                body,
            } = entry;
            strip_items.push(TabStripItem {
                header,
                enabled,
                min_width,
            });
            if slot == index {
                selected_body = body;
                if changed {
                    *selected = key;
                }
            }
        }

        let entity = entity.add_ui(move |ui| {
            let strip_frame = ui
                .ch_id("strip_frame")
                .on_spawn_apply_scene(tab_strip_frame);
            tab_strip_body(strip_frame, strip_items);
            if !has_bodies {
                return;
            }
            let body = ui.ch_id("tab_body").on_spawn_apply_scene(tab_body);
            if let Some(selected_body) = selected_body {
                body.add_ui(selected_body);
            }
        });
        respond(entity, changed)
    }

    #[track_caller]
    fn scroll_area_vertical(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::ScrollArea> {
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(|| scroll_frame(ScrollAxis::Vertical));
        scroll_body(entity, ScrollAxis::Vertical, f)
    }

    #[track_caller]
    fn scroll_area_horizontal(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::ScrollArea> {
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(|| scroll_frame(ScrollAxis::Horizontal));
        scroll_body(entity, ScrollAxis::Horizontal, f)
    }

    #[track_caller]
    fn split_horizontal(
        &mut self,
        fraction: &mut f32,
        first: impl FnOnce(&mut Ui<'w, 's>),
        second: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Split> {
        split(self, SplitAxis::Horizontal, fraction, first, second)
    }

    #[track_caller]
    fn split_vertical(
        &mut self,
        fraction: &mut f32,
        first: impl FnOnce(&mut Ui<'w, 's>),
        second: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Split> {
        split(self, SplitAxis::Vertical, fraction, first, second)
    }

    #[track_caller]
    fn scene<S: Scene>(&mut self, f: impl FnOnce() -> S) -> ImmResponse<'_, 'w, 's, kind::Scene> {
        // Identity is the call site, as everywhere else: the builder runs once,
        // so a scene whose inputs change is *not* rebuilt — that is the point of
        // hosting one. Key the call site with `push_id` to swap it for another.
        respond(self.ch_loc(loc_id(())).on_spawn_apply_scene(f), false)
    }

    #[track_caller]
    fn flex_spacer(&mut self) {
        self.ch_loc(loc_id(())).on_spawn_apply_scene(flex_spacer);
    }

    fn push_id<R>(&mut self, id: impl core::hash::Hash, f: impl FnOnce(&mut Ui<'w, 's>) -> R) -> R {
        let mut scope = self.with_add_id_pref(id);
        f(Ui::wrap_mut(&mut scope))
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
    enabled: bool,
}

impl<T> ImmSelect<T> {
    /// Declare an option standing for `key`, labeled `label`.
    pub fn option(&mut self, key: T, label: &str) -> ImmSelectOption<'_, T> {
        self.options.push(SelectOption {
            key,
            label: label.to_owned(),
            enabled: true,
        });
        ImmSelectOption {
            option: self
                .options
                .last_mut()
                .expect("the option was just pushed onto options"),
        }
    }
}

/// Handle to a just-declared option, for its per-option settings.
pub struct ImmSelectOption<'a, T> {
    option: &'a mut SelectOption<T>,
}

impl<T> ImmSelectOption<'_, T> {
    /// Default is `true`. If `false` then gray the option out and refuse picks on it.
    pub fn enabled(self, enabled: bool) -> Self {
        self.option.enabled = enabled;
        self
    }
}

/// Menu-bar context handed to [`PlumeImm::menu_bar`]'s closure: one
/// [`menu`](Self::menu) per top-level menu.
pub struct ImmMenuBar<'a, 'w, 's> {
    ui: &'a mut Ui<'w, 's>,
}

impl<'w, 's> ImmMenuBar<'_, 'w, 's> {
    /// A top-level menu button; `f` builds its drop-down on the [`ImmMenu`]
    /// context while the menu is open.
    #[track_caller]
    pub fn menu(
        &mut self,
        label: &str,
        f: impl FnOnce(&mut ImmMenu<'_, 'w, 's>),
    ) -> ImmResponse<'_, 'w, 's> {
        let label_owned = label.to_owned();
        let anchor = self.ui.ch_loc(loc_id(label)).on_spawn_apply_scene(move || {
            imm_menu_anchor(MenuButtonRole::Bar, label_owned, None, false)
        });
        imm_menu_popup(anchor, MenuButtonRole::Bar, f)
    }
}

/// Menu context handed to a [`menu`](ImmMenuBar::menu) or
/// [`submenu`](Self::submenu) closure while that menu is open.
pub struct ImmMenu<'a, 'w, 's> {
    ui: &'a mut Ui<'w, 's>,
}

impl<'w, 's> ImmMenu<'_, 'w, 's> {
    /// A pickable row; `.clicked` fires once when it is picked (the menu closes
    /// itself). Chain `.shortcut()`/`.enabled()`.
    #[track_caller]
    pub fn item(&mut self, label: &str) -> ImmResponse<'_, 'w, 's, kind::MenuItem> {
        let label_owned = label.to_owned();
        let entity = self.ui.ch_loc(loc_id(label)).on_spawn_apply_scene(move || {
            imm_menu_anchor(MenuButtonRole::Item, label_owned, None, false)
        });
        respond(entity, false)
    }

    /// A checkable row bound to `value`: picking it toggles the check (shown in
    /// the leading gutter) and closes the menu.
    #[track_caller]
    pub fn item_toggle(
        &mut self,
        label: &str,
        value: &mut bool,
    ) -> ImmResponse<'_, 'w, 's, kind::MenuItem> {
        let label_owned = label.to_owned();
        let mut changed = false;
        let entity = self
            .ui
            .ch_loc(loc_id(label))
            .on_spawn_apply_scene(move || {
                imm_menu_anchor(MenuButtonRole::Item, label_owned, None, true)
            })
            .plume_checked(value, &mut changed);
        respond(entity, changed)
    }

    /// A row that opens a nested menu beside itself (marked with a ▸); `f`
    /// builds it while open — on hover, click, or ArrowRight.
    #[track_caller]
    pub fn submenu(
        &mut self,
        label: &str,
        f: impl FnOnce(&mut ImmMenu<'_, 'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::MenuItem> {
        let label_owned = label.to_owned();
        let anchor = self.ui.ch_loc(loc_id(label)).on_spawn_apply_scene(move || {
            imm_menu_anchor(MenuButtonRole::Submenu, label_owned, None, false)
        });
        imm_menu_popup(anchor, MenuButtonRole::Submenu, f)
    }

    /// Hairline rule between item groups.
    #[track_caller]
    pub fn separator(&mut self) {
        self.ui.separator();
    }
}

// Builds a menu button's popup while its retained `MenuOpen` state says open.
// The popup is unrooted (see `ImmPopup`); the frame's `MenuAnchorLink` routes
// events and ancestor walks back to the anchor. After a close, one extra pass
// builds the popup hidden, so a picked item's pending activation still reaches
// its imm call site before the subtree is dropped.
fn imm_menu_popup<'r, 'w, 's, K>(
    mut anchor: ImmEntity<'r, 'w, 's, PlumeCaps>,
    role: MenuButtonRole,
    f: impl FnOnce(&mut ImmMenu<'_, 'w, 's>),
) -> ImmResponse<'r, 'w, 's, K> {
    let open = anchor.menu_open();
    struct MenuWasOpen;
    let was_open = anchor.hash_get_typ::<MenuWasOpen>() == Some(imm_id(true));
    anchor.hash_set_typ::<MenuWasOpen>(imm_id(open.is_some()));
    let grace = open.is_none() && was_open;
    if open.is_none() && !grace {
        return respond(anchor, false);
    }
    let anchor_entity = anchor.entity();
    let nav = open.flatten();
    anchor = anchor.unrooted_ui("menu_popup", |ui| {
        ui.ch_id("socket")
            .on_spawn_apply_scene(popup_socket)
            .on_spawn_insert(move || PopupAnchor(anchor_entity))
            .add_ui(|ui| {
                let mut frame = ui
                    .ch_id("frame")
                    .on_spawn_apply_scene(move || imm_menu_frame(anchor_entity, role, nav));
                if grace {
                    frame.entity_commands().insert(Visibility::Hidden);
                }
                frame.add_ui(|ui| f(&mut ImmMenu { ui }));
            });
    });
    respond(anchor, false)
}

/// Header kinds for [`ImmTab`]: which per-tab builders the handle carries.
pub mod tab_header {
    /// An [`ImmTabs::tab`](super::ImmTabs::tab) header — a label plus an optional icon.
    pub struct Labeled;
    /// An [`ImmTabs::tab_container`](super::ImmTabs::tab_container) header — content
    /// built by a closure, so plume contributes no label or icon of its own.
    pub struct Custom;
}

/// Tab collector handed to [`PlumeImm::tabs`]'s closure: declare one
/// [`tab`](Self::tab) or [`tab_container`](Self::tab_container) per tab, in strip
/// order.
pub struct ImmTabs<'t, 'w, 's, T> {
    entries: Vec<TabEntry<'t, 'w, 's, T>>,
}

// One declared tab. Header and body are boxed because every tab's closures are
// built while only the selected one's body is called.
struct TabEntry<'t, 'w, 's, T> {
    key: T,
    header: TabHeader<'t, 'w, 's>,
    enabled: bool,
    min_width: Option<Val>,
    body: Option<Box<dyn FnOnce(&mut Ui<'w, 's>) + 't>>,
}

enum TabHeader<'t, 'w, 's> {
    Label { label: String, icon: Option<FaIcon> },
    Content(Box<dyn FnOnce(&mut Ui<'w, 's>) + 't>),
}

impl<'t, 'w, 's, T> ImmTabs<'t, 'w, 's, T> {
    /// Declare a tab standing for `key`, labeled `label`. Chain `.icon()`/
    /// `.enabled()` on the returned handle, then finish it with `.body()` or
    /// `.no_body()`.
    pub fn tab(&mut self, key: T, label: &str) -> ImmTab<'_, 't, 'w, 's, T, tab_header::Labeled> {
        self.push(
            key,
            TabHeader::Label {
                label: label.to_owned(),
                icon: None,
            },
        )
    }

    /// Declare a tab standing for `key` whose header `header` builds instead of a
    /// label — a dirty marker, a badge, a close button, which keeps its own clicks.
    ///
    /// Every header is built each frame, so several cannot each hold a `&mut` to
    /// the same value; share a `Cell` to report back.
    pub fn tab_container(
        &mut self,
        key: T,
        header: impl FnOnce(&mut Ui<'w, 's>) + 't,
    ) -> ImmTab<'_, 't, 'w, 's, T, tab_header::Custom> {
        self.push(key, TabHeader::Content(Box::new(header)))
    }

    fn push<H>(&mut self, key: T, header: TabHeader<'t, 'w, 's>) -> ImmTab<'_, 't, 'w, 's, T, H> {
        self.entries.push(TabEntry {
            key,
            header,
            enabled: true,
            min_width: None,
            body: None,
        });
        ImmTab {
            entry: self
                .entries
                .last_mut()
                .expect("the entry was just pushed onto entries"),
            header: PhantomData,
        }
    }
}

/// Handle to a just-declared tab, for its per-tab options. `H` is the header's
/// [`tab_header`] kind, so `.icon()` only exists where plume owns the label.
///
/// A tab must end by saying what it shows: [`body`](Self::body) or
/// [`no_body`](Self::no_body), either of which consumes the handle.
#[must_use = "a tab has to say what it shows: finish it with .body(…) or .no_body()"]
pub struct ImmTab<'a, 't, 'w, 's, T, H = tab_header::Labeled> {
    entry: &'a mut TabEntry<'t, 'w, 's, T>,
    header: PhantomData<H>,
}

impl<'t, 'w, 's, T, H> ImmTab<'_, 't, 'w, 's, T, H> {
    /// Contents shown while this is the selected tab.
    pub fn body(self, body: impl FnOnce(&mut Ui<'w, 's>) + 't) {
        self.entry.body = Some(Box::new(body));
    }

    /// Finish a tab that shows nothing of its own. A strip whose tabs all say this
    /// has no body at all, and only reports which tab is picked.
    pub fn no_body(self) {}

    /// Gray the tab out and ignore clicks on it. A disabled tab that is
    /// nonetheless selected still shows its body.
    pub fn enabled(self, enabled: bool) -> Self {
        self.entry.enabled = enabled;
        self
    }

    /// How far a crowded strip may squeeze this tab before it scrolls instead
    /// (default [`size::TAB_MIN_WIDTH`]). What does not fit is clipped, so a header
    /// holding more than a label wants a floor that keeps the rest reachable.
    pub fn min_width(self, min_width: Val) -> Self {
        self.entry.min_width = Some(min_width);
        self
    }
}

impl<T> ImmTab<'_, '_, '_, '_, T, tab_header::Labeled> {
    /// Leading FontAwesome icon, before the label.
    pub fn icon(self, icon: FaIcon) -> Self {
        if let TabHeader::Label { icon: slot, .. } = &mut self.entry.header {
            *slot = Some(icon);
        }
        self
    }
}

/// Deferred dialog configuration returned by
/// [`PlumeRoot::dialog`](crate::imm::PlumeRoot::dialog); it only exists once
/// [`Self::show`] runs.
#[must_use = "a dialog does nothing until .show(|ui| …) builds it"]
pub struct ImmDialog<'a, 'w, 's> {
    ui: &'a mut Ui<'w, 's>,
    caller: &'static Location<'static>,
    title: String,
    open: &'a mut bool,
    layout: DialogLayout,
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
        self.layout.inset = UiRect {
            left,
            top,
            ..UiRect::AUTO
        };
        self
    }

    /// Initial position from corner, `x` and `y` in from its two edges.
    pub fn at_corner(mut self, corner: Corner, x: Val, y: Val) -> Self {
        self.layout.inset = corner.inset(x, y);
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
    pub fn show(
        self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> Option<ImmResponse<'e, 'w, 's, kind::Dialog>> {
        if !*self.open {
            return None;
        }
        let id = ImmIdBuilder::Hierarchy(ImmId::new((self.caller, self.title.as_str())));
        let (title, layout) = (self.title, self.layout);
        let mut entity = self
            .ui
            .ch_loc(id)
            .on_spawn_apply_scene(move || imm_dialog_scene(title, layout));
        if entity.close_requested() {
            *self.open = false;
            entity.entity_commands().despawn();
            return None;
        }
        Some(reconcile_frame_body(entity, layout, f))
    }
}

// The dialog's frame-level props, split out so they travel to the scene as one
// value instead of eight positional arguments.
#[derive(Clone, Copy)]
struct DialogLayout {
    width: Val,
    height: Val,
    max_height: Val,
    inset: UiRect,
    closable: bool,
    movable: bool,
}

/// Corner a floating surface pins to, via [`ImmPanel::at_corner`] or
/// [`ImmDialog::at_corner`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Corner {
    /// Top-left.
    #[default]
    TopLeft,
    /// Top-right.
    TopRight,
    /// Bottom-left.
    BottomLeft,
    /// Bottom-right.
    BottomRight,
}

impl Corner {
    /// On the surface's right edge, so a panel pinned here grows leftwards and
    /// the control nearest the corner is the last one added to its row.
    pub fn is_right(self) -> bool {
        matches!(self, Self::TopRight | Self::BottomRight)
    }

    /// On the surface's bottom edge, so a panel pinned here grows upwards.
    pub fn is_bottom(self) -> bool {
        matches!(self, Self::BottomLeft | Self::BottomRight)
    }

    /// Offsets from each edge of the surrounding surface placing a frame `x`, `y`
    /// in from this corner, as
    /// [`PlumeDialogProps::inset`](crate::retained::PlumeDialogProps).
    pub fn inset(self, x: Val, y: Val) -> UiRect {
        let (left, right) = if self.is_right() {
            (Val::Auto, x)
        } else {
            (x, Val::Auto)
        };
        let (top, bottom) = if self.is_bottom() {
            (Val::Auto, y)
        } else {
            (y, Val::Auto)
        };
        UiRect {
            left,
            right,
            top,
            bottom,
        }
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

    /// Position, in from the top-left of whatever the panel is declared in — the
    /// surrounding container, or the viewport at root scope. Spawn-time only.
    pub fn at(mut self, left: Val, top: Val) -> Self {
        self.layout.inset = UiRect {
            left,
            top,
            ..UiRect::AUTO
        };
        self
    }

    /// Pin the panel to a corner of whatever it is declared in — the surrounding
    /// container, or the viewport at root scope — `x` and `y` in from its edges.
    /// Inside a pane it follows that pane, a splitter's say, with nothing to wire.
    ///
    /// An ordinary absolute child, so a clipping container cuts it off at its own
    /// edge; insets that keep it inside are unaffected.
    pub fn at_corner(mut self, corner: Corner, x: Val, y: Val) -> Self {
        self.layout.inset = corner.inset(x, y);
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

/// Deferred popup configuration returned by [`ImmResponse::popup`]; the popup
/// only exists once [`Self::show`] runs while `*open`.
#[must_use = "a popup does nothing until .show(|ui| …) builds it"]
pub struct ImmPopup<'r, 'a, 'w, 's, K> {
    pub(crate) anchor: ImmResponse<'r, 'w, 's, K>,
    pub(crate) caller: &'static Location<'static>,
    pub(crate) open: &'a mut bool,
    pub(crate) placement: PopupPlacement,
    pub(crate) movable: bool,
    pub(crate) close_on_click_outside: bool,
}

impl<'r, 'w, 's, K> ImmPopup<'r, '_, 'w, 's, K> {
    /// Open beside the anchor, centered, trying right, left, above, below in
    /// that order. Default is to popup under.
    pub fn beside(mut self) -> Self {
        self.placement = PopupPlacement::Beside;
        self
    }

    /// If `true` then let background drags move the popup; a reopen re-anchors it.
    pub fn movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self
    }

    /// `false` keeps the popup open through outside clicks, leaving `open` as the
    /// only way to close it (default true).
    pub fn close_on_click_outside(mut self, close: bool) -> Self {
        self.close_on_click_outside = close;
        self
    }

    /// Toggle `open` when the anchor is clicked. Safe as a toggle because the anchor
    /// is exempt from outside-press dismissal, so its click only lands here.
    pub fn toggle_on_click(self) -> Self {
        if self.anchor.clicked {
            *self.open = !*self.open;
        }
        self
    }

    /// Build the popup and its body; an outside click writes back through `open`.
    /// Returns the anchor's response, so further builders can chain off it.
    pub fn show(self, f: impl FnOnce(&mut Ui<'w, 's>)) -> ImmResponse<'r, 'w, 's, K> {
        let Self {
            mut anchor,
            caller,
            open,
            placement,
            movable,
            close_on_click_outside,
        } = self;
        if !*open {
            return anchor;
        }
        let dismiss = match close_on_click_outside {
            true => PopupDismiss::OutsideClick,
            false => PopupDismiss::Explicit,
        };
        let anchor_entity = anchor.entity;
        let mut closed = false;
        // Unrooted, so the popup is a UI root rather than a descendant of the
        // anchor: picking bubbles up the hierarchy and `Hovered` is set on every
        // ancestor of the hit, so a popup inside its anchor would re-fire the
        // anchor's clicks and hold it highlighted. The socket carries the anchor
        // as [`PopupAnchor`] instead — it has no parent to fall back on. Ids stay
        // unique per anchor, since the scope hangs off the anchor's own id.
        anchor.e = anchor.e.unrooted_ui(caller, |ui| {
            ui.ch_id("socket")
                .on_spawn_apply_scene(popup_socket)
                .on_spawn_insert(move || PopupAnchor(anchor_entity))
                .add_ui(|ui| {
                    let mut popup = ui.ch_id("popup").on_spawn_apply_scene(move || {
                        imm_popup_scene(anchor_entity, placement, dismiss, movable)
                    });
                    if popup.close_requested() {
                        closed = true;
                        popup.entity_commands().despawn();
                    } else {
                        popup.add_ui(f);
                    }
                });
        });
        if closed {
            *open = false;
        }
        anchor
    }
}

fn imm_popup_scene(
    anchor: Entity,
    placement: PopupPlacement,
    dismiss: PopupDismiss,
    movable: bool,
) -> impl Scene {
    bsn! {
        @PlumePopup {
            @placement: placement,
            @dismiss: dismiss,
            @movable: movable,
        }
        template_value(DismissScope(anchor))
    }
}

// Reconcile a dialog/panel frame's app-owned size and fill its body, wrapping the
// content in the scrolling machinery when a height knob bounds it. Position is not
// re-applied — the user's dragging owns it after spawn.
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
    let entity = entity.add_ui(move |ui| {
        let body = ui
            .ch_id("dialog_body")
            .on_spawn_apply_scene(|| bsn! { @PlumeDialogBody });
        if layout.height == Val::Auto && layout.max_height == Val::Auto {
            body.add_ui(f);
        } else {
            body.add_ui(move |ui| {
                let frame = ui
                    .ch_id("scroll_frame")
                    .on_spawn_apply_scene(|| scroll_frame(ScrollAxis::Vertical));
                scroll_body(frame, ScrollAxis::Vertical, f);
            });
        }
    });
    respond(entity, false)
}

// One tab as the strip builder needs it, once the collector's entries have been
// split between the strip and the selected body.
struct TabStripItem<'t, 'w, 's> {
    header: TabHeader<'t, 'w, 's>,
    enabled: bool,
    min_width: Option<Val>,
}

// The strip's tabs inside its already-created [`tab_strip_frame`], so a strip with
// more tabs than room scrolls instead of putting them out of reach.
fn tab_strip_body<'w, 's>(
    frame: ImmEntity<'_, 'w, 's, PlumeCaps>,
    items: Vec<TabStripItem<'_, 'w, 's>>,
) {
    scroll_viewport_with_scrollbar(frame, ScrollAxis::Horizontal, move |ui| {
        ui.ch_id("tab_strip")
            .on_spawn_apply_scene(tab_strip)
            .add_ui(move |ui| {
                for (slot, item) in items.into_iter().enumerate() {
                    let TabStripItem {
                        header,
                        enabled,
                        min_width,
                    } = item;
                    let mut tab = match header {
                        // The label and glyph key the tab: a renamed tab
                        // respawns rather than keeping the old caption at
                        // the same slot.
                        TabHeader::Label { label, icon } => ui
                            .ch_id(("tab", slot, &label, icon.map(FaIcon::glyph)))
                            .on_spawn_apply_scene(move || tab_button(label, icon)),
                        // Keyed on the slot alone; the content reconciles
                        // itself, as it would anywhere else.
                        TabHeader::Content(content) => ui
                            .ch_id(("tab_container", slot))
                            .on_spawn_apply_scene(tab_chrome)
                            .add_ui(content),
                    };
                    struct TabMinWidth;
                    if let Some(min_width) = min_width
                        && tab
                            .hash_update_typ::<TabMinWidth>(Some(imm_id(format!("{min_width:?}"))))
                    {
                        tab.entity_commands()
                            .queue(move |mut entity: EntityWorldMut| {
                                if let Some(mut node) = entity.get_mut::<Node>() {
                                    node.min_width = min_width;
                                }
                            });
                    }
                    tab.interactions_enabled(enabled);
                }
            });
    });
}

// The scrolling content and its scrollbar inside an already-created [`scroll_frame`].
fn scroll_body<'r, 'w, 's>(
    frame: ImmEntity<'r, 'w, 's, PlumeCaps>,
    axis: ScrollAxis,
    f: impl FnOnce(&mut Ui<'w, 's>),
) -> ImmResponse<'r, 'w, 's, kind::ScrollArea> {
    let entity = scroll_viewport_with_scrollbar(frame, axis, move |ui| {
        ui.ch_id("scroll_content")
            .on_spawn_apply_scene(move || scroll_content(axis))
            .add_ui(f);
    });
    respond(entity, false)
}

// The scrolling viewport that `content` fills, plus the scrollbar driving it. Built
// piecewise rather than spawned as one scene because the scrollbar needs the
// viewport's entity, which `bsn!` names for a retained scene and imm must hand over.
fn scroll_viewport_with_scrollbar<'r, 'w, 's>(
    frame: ImmEntity<'r, 'w, 's, PlumeCaps>,
    axis: ScrollAxis,
    content: impl FnOnce(&mut Ui<'w, 's>),
) -> ImmEntity<'r, 'w, 's, PlumeCaps> {
    frame.add_ui(move |ui| {
        // The entity is known before its spawn command flushes, so the scrollbar
        // can point at the viewport it drives.
        let viewport = ui
            .ch_id("scroll_viewport")
            .on_spawn_apply_scene(move || scroll_viewport(axis))
            .add_ui(content)
            .entity();
        ui.ch_id("scrollbar")
            .on_spawn_apply_scene(move || scrollbar(viewport, axis));
    })
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

// Salt folded into a repeated id's suffix; a fixed tag so a disambiguated id can
// never coincide with a genuine `(location, key)` base.
const OCCURRENCE_SALT: u32 = 0x506c_756d; // "Plum"

// Child creation with plume-side occurrence disambiguation.
//
// Unpatched `bevy_immediate` maps each hierarchy id to exactly one entity, so two
// widgets built from the same call site — a helper called in a loop — would land
// on the same entity. This threads a per-`(parent, base id)` occurrence counter
// (held in [`PlumeOccurrences`]): the first use keeps the plain id (so a widget
// that appears once, or a conditional sibling, never shifts the others) and each
// repeat takes a distinct suffix. Keeping it here lets plume track upstream
// `bevy_immediate` with no `resolve`-time patch.
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

// Set the `glyph` on a tool button's `fa_icon` `Text` child. The font stays as
// spawned, since the face keys the button's identity.
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

// Both split directions, which differ only in the axis they hand down. The
// panes are imm children of the splitter's own frame rather than scene props:
// their contents are immediate, and only a `Ui` can build those.
#[track_caller]
fn split<'r, 'w, 's>(
    ui: &'r mut Ui<'w, 's>,
    axis: SplitAxis,
    fraction: &mut f32,
    first: impl FnOnce(&mut Ui<'w, 's>),
    second: impl FnOnce(&mut Ui<'w, 's>),
) -> ImmResponse<'r, 'w, 's, kind::Split> {
    let initial = *fraction;
    let mut changed = false;
    let entity = ui
        .ch_loc(loc_id(()))
        .on_spawn_apply_scene(move || splitter_frame(axis, initial))
        .plume_split(fraction, &mut changed);
    // A collapsed pane's contents are not built at all: nothing measures, so
    // nothing can prop the pane back open, and its imm state is released.
    let collapse = entity
        .cap_get_component::<SplitCollapsible>()
        .ok()
        .flatten()
        .copied()
        .unwrap_or_default();
    let split = *fraction;
    let entity = entity.add_ui(|ui| {
        let pane = ui
            .ch_id("split_first")
            .on_spawn_apply_scene(|| splitter_pane(SplitPane::First));
        if !(collapse.first && split == 0.0) {
            pane.add_ui(first);
        }
        ui.ch_id("split_divider")
            .on_spawn_apply_scene(move || splitter_divider(axis));
        let pane = ui
            .ch_id("split_second")
            .on_spawn_apply_scene(|| splitter_pane(SplitPane::Second));
        if !(collapse.second && split == 1.0) {
            pane.add_ui(second);
        }
    });
    respond(entity, changed)
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
        integral: false,
        e: entity,
        kind: PhantomData,
    }
}

// A value widget's response, tagged with whether the type it is bound to is an
// integer, so `precision` can reject a setting that type can't express.
fn respond_numeric<'r, 'w, 's, T: Numeric, K>(
    entity: ImmEntity<'r, 'w, 's, PlumeCaps>,
    changed: bool,
) -> ImmResponse<'r, 'w, 's, K> {
    ImmResponse {
        integral: T::INTEGRAL,
        ..respond(entity, changed)
    }
}

fn imm_dialog_scene(title: String, layout: DialogLayout) -> impl Scene {
    let DialogLayout {
        width,
        height,
        max_height,
        inset,
        closable,
        movable,
    } = layout;
    bsn! {
        // Empty body: the imm layer reconciles the body itself.
        dialog_frame(DialogChrome {
            name: format!("PlumeDialog({title})").into(),
            body: Box::new(bsn_list![]),
            header: Some(DialogHeader {
                title: Box::new(bsn_list![caption_large(title)]),
                closable,
                movable,
            }),
            width,
            height,
            max_height,
            inset,
        })
        on(|close: On<RequestClose>, mut commands: Commands| {
            commands.entity(close.event_target()).insert(CloseRequested);
        })
    }
}

fn imm_panel_scene(layout: DialogLayout) -> impl Scene {
    let DialogLayout {
        width,
        height,
        max_height,
        inset,
        ..
    } = layout;
    // Headerless: `header: None` drops the title bar (and so the ✕ and drag handle),
    // and there is no `RequestClose` observer, since a panel has no ✕.
    bsn! {
        dialog_frame(DialogChrome {
            name: "PlumePanel".into(),
            body: Box::new(bsn_list![]),
            header: None,
            width,
            height,
            max_height,
            inset,
        })
    }
}

#[cfg(test)]
mod tests {
    use bevy::MinimalPlugins;
    use bevy::app::{App, Update};
    use bevy::ecs::resource::Resource;
    use bevy::ecs::system::ResMut;

    use super::*;
    use crate::imm::{ImmPlugin, PlumeRoot};

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
