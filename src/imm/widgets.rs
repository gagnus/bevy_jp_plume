//! Widget methods on the immediate-mode context; each wraps a retained plume scene.
use core::marker::PhantomData;
use core::ops::RangeInclusive;
use core::panic::Location;

use bevy::camera::visibility::Visibility;
use bevy::color::Color;
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EntityEvent;
use bevy::ecs::observer::On;
use bevy::ecs::system::Commands;
use bevy::ecs::world::EntityWorldMut;
use bevy::scene::{Scene, bsn, bsn_list, on};
use bevy::ui::widget::Text;
use bevy::ui::{BackgroundColor, JustifyContent, Node, UiRect, Val};
use bevy::ui_widgets::{ModalDialog, RequestClose};
use bevy_immediate::ui::activated::ImmUiActivated;
use bevy_immediate::ui::disabled::ImmUiInteractionsDisabled;
use bevy_immediate::ui::interaction::ImmUiInteraction;
use bevy_immediate::{ImmEntity, ImmId, ImmIdBuilder, imm_id};

use super::caps::{
    ImmPlumeChecked, ImmPlumeColor, ImmPlumeDialog, ImmPlumeMenu, ImmPlumeReorder, ImmPlumeSelect,
    ImmPlumeSplit, ImmPlumeText, ImmPlumeValue, PlumeOccurrences,
};
use super::{ImmEntityExt, ImmResponse, PaneUi, PlumeCaps, Ui, kind};
use crate::body::{BodyGap, BodyPadding};
use crate::constants::{Icon, size};
use crate::containers::{
    CloseRequested, DialogChrome, DialogHeader, PlacementAnchor, PopupAnchor, PopupDismiss,
    PopupPlacement, ScrollAxis, SectionHeaderCaption, SplitAxis, SplitPane, SplitSize, column,
    dialog_body, dialog_frame, imm_popup_scene, modal_barrier, popup_socket, reorderable_frame,
    reorderable_grip, reorderable_item, row, screen, scroll_content, scroll_frame, scroll_viewport,
    scrollbar, section_body, section_frame, splitter_divider, splitter_frame, splitter_pane,
    tab_body, tab_button, tab_chrome, tab_strip, tab_strip_frame, tabs_frame,
};
use crate::controls::{
    ColorSwatchValue, MenuButtonRole, PlumeButton, PlumeCheckbox, PlumeColorEdit, PlumeColorPicker,
    PlumeColorSwatch, PlumeDisclosure, PlumeMenuBar, PlumeNumberInput, PlumeNumberInputProps,
    PlumeRadio, PlumeSelect, PlumeSlider, PlumeTextInput, PlumeToggleSwitch, PlumeToolButton,
    SelectedIndex, imm_menu_anchor, imm_menu_frame, set_icon_glyph,
};
use crate::display;
use crate::display::{caption, flex_spacer, separator, space};
use crate::font_styles::{InheritableFont, small_caps};
use crate::theme::ThemeSlot;
use crate::theme::components::{ThemeBackgroundSlot, ThemeBackgroundToken};
use crate::utils::numeric::Numeric;

/// Widget calls for immediate-mode systems. Implemented by [`Ui`]; import it
/// wherever imm systems are written.
pub trait PlumeImm<'w, 's> {
    /// Themed text in the current container's font and color. The response's
    /// `clicked`/`changed` are always false; `hovered` and the builders work.
    fn caption(&mut self, text: &str) -> ImmResponse<'_, 'w, 's, kind::Caption>;

    /// Hairline rule across the container: horizontal in a [`Self::vertical`],
    /// vertical in a [`Self::horizontal`]. Chain `.full_bleed()` to cross the padding.
    fn separator(&mut self) -> ImmResponse<'_, 'w, 's, kind::Separator>;

    /// Non-interactive color preview filled with `color`. Defaults to a
    /// [`ROW_HEIGHT`](crate::constants::size::ROW_HEIGHT) square; resize with `.size()`.
    fn color_swatch(&mut self, color: Color) -> ImmResponse<'_, 'w, 's, kind::Swatch>;

    /// [`Self::color_swatch`] painting `color` opaque, its alpha ignored.
    fn color_swatch_rgb(&mut self, color: Color) -> ImmResponse<'_, 'w, 's, kind::Swatch>;

    /// Interactive HSV color picker, two-way bound to `color`; `.changed` fires when
    /// the user drags to a new color. The layout is fixed by the control.
    fn color_picker(&mut self, color: &mut Color) -> ImmResponse<'_, 'w, 's>;

    /// [`Self::color_picker`] without the alpha bar and field, for a color whose
    /// alpha is not the user's to edit; the alpha it arrived with passes through.
    fn color_picker_rgb(&mut self, color: &mut Color) -> ImmResponse<'_, 'w, 's>;

    /// Editable color: a select-style button (swatch plus arrow) opening a
    /// color-picker popup on click, dismissed by clicking outside. Two-way bound
    /// to `color`. Fires `.changed` when the user edits it.
    fn color_edit(&mut self, color: &mut Color) -> ImmResponse<'_, 'w, 's>;

    /// [`Self::color_edit`] whose swatch paints opaque and whose popup edits RGB
    /// only; the alpha the color arrived with passes through.
    fn color_edit_rgb(&mut self, color: &mut Color) -> ImmResponse<'_, 'w, 's>;

    /// Fixed gap along the container's main axis - `length` of width in a
    /// [`Self::horizontal`], of height in a [`Self::vertical`]. For a gap that
    /// absorbs whatever is left over instead, use [`Self::flex_spacer`].
    fn space(&mut self, length: Val);

    /// Push button; `.clicked` on the response fires once per activation.
    fn button(&mut self, label: &str) -> ImmResponse<'_, 'w, 's, kind::Button>;

    /// Push button with a leading icon before the label.
    fn icon_button(&mut self, icon: Icon, label: &str) -> ImmResponse<'_, 'w, 's, kind::Button>;

    /// Compact icon-only button (tighter padding, square min-width) for headers/toolbars.
    fn tool_button(&mut self, icon: Icon) -> ImmResponse<'_, 'w, 's, kind::Button>;

    /// A non-interactive icon glyph in the current text color - the icon
    /// counterpart to [`Self::caption`].
    fn icon(&mut self, icon: Icon) -> ImmResponse<'_, 'w, 's, kind::Icon>;

    /// A push button whose content is built by `f` instead of a single label.
    /// The row it lays that content out in is the app's: chain `.padding()` and
    /// `.gap()`, which a labelled button doesn't take.
    fn button_container(
        &mut self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Button<kind::Content>>;

    /// Labeled checkbox bound to `value`.
    fn checkbox(&mut self, value: &mut bool, label: &str) -> ImmResponse<'_, 'w, 's>;

    /// Disclosure toggle bound to `open`.
    fn disclosure(&mut self, open: &mut bool) -> ImmResponse<'_, 'w, 's>;

    /// Labeled radio button: checked while `*value == variant`; clicking it (or
    /// Space on focus) writes `variant` into `value`. A group is just several
    /// radios bound to the same `value` - no container needed.
    fn radio<T: PartialEq>(
        &mut self,
        value: &mut T,
        variant: T,
        label: &str,
    ) -> ImmResponse<'_, 'w, 's>;

    /// Bare toggle switch bound to `value` (no label - the surrounding row owns it).
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
    /// order. Value-keyed like [`Self::radio`]; no match falls back to the first.
    fn select<T: PartialEq>(
        &mut self,
        selected: &mut T,
        f: impl FnOnce(&mut ImmSelect<T>),
    ) -> ImmResponse<'_, 'w, 's, kind::Select>;

    /// Horizontal menu bar strip; `f` declares its drop-down menus on the
    /// [`ImmMenuBar`] context. Bodies only run while their menu is open.
    fn menu_bar(&mut self, f: impl FnOnce(&mut ImmMenuBar<'_, 'w, 's>)) -> ImmResponse<'_, 'w, 's>;

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

    /// Tab container: a header strip over a body showing one tab at a time. `f`
    /// declares the tabs on the [`ImmTabs`] collector; only the selected tab's
    /// `.body()` runs. A crowded strip squeezes toward [`size::TAB_MIN_WIDTH`], then scrolls.
    ///
    /// Selection is value-keyed like [`Self::radio`]; a `selected` matching no tab
    /// falls back to the first.
    fn tabs<'t, T: PartialEq>(
        &mut self,
        selected: &mut T,
        f: impl FnOnce(&mut ImmTabs<'t, 'w, 's, T>),
    ) -> ImmResponse<'_, 'w, 's, kind::Tabs>;

    /// Collapsible section with a small-caps `header`; `f` builds its body.
    /// Collapse state persists across frames. Chain `.start_collapsed()`, or
    /// `.small_caps(false)` for a header whose casing matters.
    fn section(
        &mut self,
        header: &str,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Section>;

    /// Vertically scrolling region: `f` builds the content, which scrolls once it
    /// outgrows the height set via `.max_height()`/`.height()`. Unbounded it stacks
    /// like a [`Self::vertical`].
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

    /// Two panes side by side, with a divider the user drags to re-proportion them.
    ///
    /// `split` holds the sized pane's width (first pane unless
    /// [`sized_pane`](ImmResponse::sized_pane) says otherwise), written back as
    /// the divider moves; `.changed` reports a move. A `Percent` size keeps the
    /// split proportional when the splitter resizes; `Px`/`Em` anchor the sized
    /// pane so only the other pane absorbs the change. Chain
    /// [`min_panes`](ImmResponse::min_panes) to stop either pane getting too small.
    ///
    /// Give each pane one container of its own: a pane is bare layout with no
    /// direction or spacing, so a second child lands beside the first.
    fn split_horizontal(
        &mut self,
        split: &mut SplitSize,
        first: impl FnOnce(&mut PaneUi<'_, 'w, 's>),
        second: impl FnOnce(&mut PaneUi<'_, 'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Split>;

    /// [`split_horizontal`](Self::split_horizontal) with the panes stacked and
    /// the divider across them; `split` holds the sized pane's height.
    fn split_vertical(
        &mut self,
        split: &mut SplitSize,
        first: impl FnOnce(&mut PaneUi<'_, 'w, 's>),
        second: impl FnOnce(&mut PaneUi<'_, 'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Split>;

    /// Hosts a retained scene inside an immediate pass: `f` builds it the frame its
    /// entity is first spawned and never again.
    ///
    /// The scene is never reconciled against - it persists untouched while the call
    /// site keeps running, and is despawned with its host when that stops. Keep the
    /// entity from the response if it has to be found again.
    ///
    /// The host sizes it (`.grow()`, `.width()`); the scene styles itself.
    fn scene<S: Scene>(&mut self, f: impl FnOnce() -> S) -> ImmResponse<'_, 'w, 's, kind::Scene>;

    /// Invisible filler that absorbs a row's spare width (pushes what follows to
    /// the trailing edge).
    fn flex_spacer(&mut self);

    /// Scope child ids by `id`, making widget identity follow the key instead of call
    /// order - for entries that reorder, where occurrence indices are positional.
    fn push_id<R>(&mut self, id: impl core::hash::Hash, f: impl FnOnce(&mut Ui<'w, 's>) -> R) -> R;

    /// Vertical list the user drags to reorder by the grip plume draws at each
    /// item's left; `f` fills the rest of the item's row. `key` must be stable
    /// and unique across items - identity follows it, not position, so a step
    /// moves entities rather than respawning them (hash an id, never the item
    /// itself). A move is written into `items` before `f` runs; `.changed`
    /// reports it.
    fn reorderable<T, K: core::hash::Hash>(
        &mut self,
        items: &mut [T],
        key: impl Fn(&T) -> K,
        f: impl FnMut(&mut Ui<'w, 's>, &mut T),
    ) -> ImmResponse<'_, 'w, 's, kind::Reorderable>;
}

// The two surfaces that open a pass, handed out by [`PlumeRoot`] alone: nested, they
// would measure against their parent and nest a `TabGroup` inside the one they opened.
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
    pub(crate) fn panel(&mut self) -> ImmDialog<'_, 'w, 's, Panel> {
        ImmDialog {
            ui: self,
            caller: Location::caller(),
            // A panel has no title bar to show one in; it still keys the
            // surface id, where an empty one leaves the call site to do it.
            title: String::new(),
            open: None,
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
            style: FrameStyle::default(),
            anchor: None,
            mode: PhantomData,
        }
    }

    #[track_caller]
    pub(crate) fn modal<'a>(
        &'a mut self,
        title: &str,
        open: &'a mut bool,
    ) -> ImmDialog<'a, 'w, 's, Modal> {
        ImmDialog {
            ui: self,
            caller: Location::caller(),
            title: title.to_owned(),
            open: Some(open),
            layout: DialogLayout {
                width: Val::Auto,
                height: Val::Auto,
                // A modal is centred on its barrier, so its ceiling is that box
                // and it leaves the inset for the barrier's flex to resolve.
                max_height: Val::Percent(100.0),
                inset: UiRect::AUTO,
                closable: true,
                movable: false,
            },
            style: FrameStyle::default(),
            anchor: None,
            mode: PhantomData,
        }
    }

    #[track_caller]
    pub(crate) fn dialog<'a>(
        &'a mut self,
        title: &str,
        open: &'a mut bool,
    ) -> ImmDialog<'a, 'w, 's, Floating> {
        ImmDialog {
            ui: self,
            caller: Location::caller(),
            title: title.to_owned(),
            open: Some(open),
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
            style: FrameStyle::default(),
            anchor: None,
            mode: PhantomData,
        }
    }
}

impl<'w, 's> PlumeImm<'w, 's> for Ui<'w, 's> {
    #[track_caller]
    fn caption(&mut self, text: &str) -> ImmResponse<'_, 'w, 's, kind::Caption> {
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
        imm_color_swatch(self, color, true)
    }

    #[track_caller]
    fn color_swatch_rgb(&mut self, color: Color) -> ImmResponse<'_, 'w, 's, kind::Swatch> {
        imm_color_swatch(self, color, false)
    }

    #[track_caller]
    fn color_picker(&mut self, color: &mut Color) -> ImmResponse<'_, 'w, 's> {
        // The picker retains its working HSV, so the scene seeds the color once and
        // the capability syncs it thereafter.
        let initial = *color;
        let mut changed = false;
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(move || bsn! { @PlumeColorPicker { @initial_color: initial } })
            .plume_color(color, &mut changed);
        respond(entity, changed)
    }

    #[track_caller]
    fn color_picker_rgb(&mut self, color: &mut Color) -> ImmResponse<'_, 'w, 's> {
        let initial = *color;
        let mut changed = false;
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(
                move || bsn! { @PlumeColorPicker { @initial_color: initial, @alpha: false } },
            )
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
    fn color_edit_rgb(&mut self, color: &mut Color) -> ImmResponse<'_, 'w, 's> {
        let initial = *color;
        let mut changed = false;
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(
                move || bsn! { @PlumeColorEdit { @initial_color: initial, @alpha: false } },
            )
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
            move || bsn! { @PlumeButton { @caption: bsn! { @caption(label_owned) } } },
        );
        respond(entity, false)
    }

    #[track_caller]
    fn icon_button(&mut self, icon: Icon, label: &str) -> ImmResponse<'_, 'w, 's, kind::Button> {
        let label_owned = label.to_owned();
        // Not keyed on the glyph: it updates in place below. The icon is the first
        // `Text` child, ahead of the label, so `set_icon_glyph` lands on it.
        let mut entity = self.ch_loc(loc_id(label)).on_spawn_apply_scene(move || {
            bsn! {
                @PlumeButton {
                    @caption: bsn_list! {
                        @display::icon(icon)
                        --
                        @caption(label_owned)
                    },
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
    fn tool_button(&mut self, icon: Icon) -> ImmResponse<'_, 'w, 's, kind::Button> {
        // Not keyed on the glyph: it updates in place below.
        let mut entity = self.ch_loc(loc_id(())).on_spawn_apply_scene(move || {
            bsn! { @PlumeToolButton { @caption: bsn! { @display::icon(icon) } } }
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
    fn icon(&mut self, icon: Icon) -> ImmResponse<'_, 'w, 's, kind::Icon> {
        // Not keyed on the glyph: it updates in place below.
        let mut entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(move || display::icon(icon));
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
    ) -> ImmResponse<'_, 'w, 's, kind::Button<kind::Content>> {
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
                move || bsn! { @PlumeCheckbox { @caption: bsn! { @caption(label_owned) } } },
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
                move || bsn! { @PlumeRadio { @caption: bsn! { @caption(label_owned) } } },
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
        // The labels key the widget: options are seeded at spawn, so an edited list
        // must respawn rather than keep stale rows.
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
                    @section_frame(
                        bsn! {
                            @caption(header_owned)
                            @small_caps()
                            SectionHeaderCaption
                        },
                        true,
                        ()
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
                                width,
                            } = item;
                            let mut tab = match header {
                                // The label and glyph key the tab: a renamed tab respawns.
                                TabHeader::Label { label, icon } => ui
                                    .ch_id(("tab", slot, &label, icon.map(Icon::glyph)))
                                    .on_spawn_apply_scene(move || tab_button(label, icon)),
                                TabHeader::Content(content) => ui
                                    .ch_id(("tab_container", slot))
                                    .on_spawn_apply_scene(tab_chrome)
                                    .add_ui(content),
                            };
                            // Both sizes in one command: guarded separately, a change to
                            // either would keep the other's last write.
                            struct TabSizing;
                            if tab.hash_update_typ::<TabSizing>(Some(imm_id(format!(
                                "{width:?}{min_width:?}"
                            )))) {
                                tab.entity_commands()
                                    .queue(move |mut entity: EntityWorldMut| {
                                        if let Some(mut node) = entity.get_mut::<Node>() {
                                            node.width = width.unwrap_or(Val::Auto);
                                            node.min_width =
                                                min_width.unwrap_or(size::TAB_MIN_WIDTH);
                                            // A width with no floor under it is absolute:
                                            // the strip scrolls instead of squeezing.
                                            node.flex_shrink = match (width, min_width) {
                                                (Some(_), None) => 0.0,
                                                _ => 1.0,
                                            };
                                        }
                                    });
                            }
                            tab.interactions_enabled(enabled);
                        }
                    });
            });
        }

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
            .on_spawn_apply_scene(move || {
                bsn! {
                    @tabs_frame()
                    SelectedIndex(initial)
                }
            })
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
                width,
                body,
            } = entry;
            strip_items.push(TabStripItem {
                header,
                enabled,
                min_width,
                width,
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
        split_size: &mut SplitSize,
        first: impl FnOnce(&mut PaneUi<'_, 'w, 's>),
        second: impl FnOnce(&mut PaneUi<'_, 'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Split> {
        split(self, SplitAxis::Horizontal, split_size, first, second)
    }

    #[track_caller]
    fn split_vertical(
        &mut self,
        split_size: &mut SplitSize,
        first: impl FnOnce(&mut PaneUi<'_, 'w, 's>),
        second: impl FnOnce(&mut PaneUi<'_, 'w, 's>),
    ) -> ImmResponse<'_, 'w, 's, kind::Split> {
        split(self, SplitAxis::Vertical, split_size, first, second)
    }

    #[track_caller]
    fn scene<S: Scene>(&mut self, f: impl FnOnce() -> S) -> ImmResponse<'_, 'w, 's, kind::Scene> {
        // The builder runs once, so a scene whose inputs change is *not* rebuilt.
        // Key the call site with `push_id` to swap it for another.
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

    #[track_caller]
    fn reorderable<T, K: core::hash::Hash>(
        &mut self,
        items: &mut [T],
        key: impl Fn(&T) -> K,
        mut f: impl FnMut(&mut Ui<'w, 's>, &mut T),
    ) -> ImmResponse<'_, 'w, 's, kind::Reorderable> {
        let mut changed = false;
        // Before the items build, so this pass declares the moved order and the
        // layout-order sort agrees with the swap the grip already made.
        let entity = self
            .ch_loc(loc_id(()))
            .on_spawn_apply_scene(reorderable_frame)
            .plume_reorder(items, &mut changed);
        let entity = entity.add_ui(|ui| {
            for item in items.iter_mut() {
                ui.push_id(key(item), |ui| {
                    ui.ch_id("reorderable_item")
                        .on_spawn_apply_scene(reorderable_item)
                        .add_ui(|ui| {
                            ui.ch_id("reorderable_grip")
                                .on_spawn_apply_scene(reorderable_grip);
                            f(ui, item);
                        });
                });
            }
        });
        respond(entity, changed)
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
    /// Gray the option out and refuse picks on it. Default is `true`.
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
    /// builds it while open - on hover, click, or ArrowRight.
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

// The anchors plume builds itself, which have no response of their own yet. One
// menu per anchor here, so they all share the one popup id.
fn imm_menu_popup<'r, 'w, 's, K>(
    anchor: ImmEntity<'r, 'w, 's, PlumeCaps>,
    role: MenuButtonRole,
    f: impl FnOnce(&mut ImmMenu<'_, 'w, 's>),
) -> ImmResponse<'r, 'w, 's, K> {
    respond(imm_menu_popup_on(anchor, role, "menu_popup", f), false)
}

// Builds a menu button's popup while its retained `MenuOpen` says open. After a
// close, one extra pass builds it hidden, so a picked item's pending activation
// still reaches its imm call site before the subtree is dropped.
//
// A bar menu's popup is unrooted (see `ImmPopup`) and reaches its anchor through
// the frame's `MenuAnchorLink`. A submenu's cannot be: bevy closes a `MenuPopup`
// whose focused entity is not in its `ChildOf` subtree, and pressing an item in
// an unrooted submenu takes the focus out of the bar menu that owns it - closing
// the stack on the press, before the release that would have activated the item.
// So a submenu hangs under its own row, which is where the retained path puts it.
pub(crate) fn imm_menu_popup_on<'r, 'w, 's>(
    mut anchor: ImmEntity<'r, 'w, 's, PlumeCaps>,
    role: MenuButtonRole,
    id: impl core::hash::Hash,
    f: impl FnOnce(&mut ImmMenu<'_, 'w, 's>),
) -> ImmEntity<'r, 'w, 's, PlumeCaps> {
    let open = anchor.menu_open();
    struct MenuWasOpen;
    let was_open = anchor.hash_get_typ::<MenuWasOpen>() == Some(imm_id(true));
    anchor.hash_set_typ::<MenuWasOpen>(imm_id(open.is_some()));
    let grace = open.is_none() && was_open;
    if open.is_none() && !grace {
        return anchor;
    }
    let anchor_entity = anchor.entity();
    let nav = open.flatten();
    let build = move |ui: &mut Ui<'w, 's>| {
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
    };
    anchor = match role {
        MenuButtonRole::Submenu => anchor.add_ui(build),
        _ => anchor.unrooted_ui(id, build),
    };
    anchor
}

/// Header kinds for [`ImmTab`]: which per-tab builders the handle carries.
pub mod tab_header {
    /// An [`ImmTabs::tab`](super::ImmTabs::tab) header - a label plus an optional icon.
    pub struct Labeled;
    /// An [`ImmTabs::tab_container`](super::ImmTabs::tab_container) header - content
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
    width: Option<Val>,
    body: Option<Box<dyn FnOnce(&mut Ui<'w, 's>) + 't>>,
}

enum TabHeader<'t, 'w, 's> {
    Label { label: String, icon: Option<Icon> },
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
    /// label - a dirty marker, a badge, a close button.
    ///
    /// Every header is built each frame, so several cannot each hold a `&mut` to the
    /// same value; share a `Cell` to report back.
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
            width: None,
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

    /// Fix the tab's width. On its own it is absolute - a crowded strip scrolls
    /// rather than squeezing the tab; with [`min_width`](Self::min_width) the tab
    /// squeezes to that floor first.
    pub fn width(self, width: Val) -> Self {
        self.entry.width = Some(width);
        self
    }
}

impl<T> ImmTab<'_, '_, '_, '_, T, tab_header::Labeled> {
    /// Leading icon, before the label.
    pub fn icon(self, icon: Icon) -> Self {
        if let TabHeader::Label { icon: slot, .. } = &mut self.entry.header {
            *slot = Some(icon);
        }
        self
    }
}

/// A dialog the app places and the user drags - [`ImmDialog`]'s default mode.
pub struct Floating;

/// A dialog centred on a barrier that blocks the app behind it, answered before
/// the app behind it can be touched.
pub struct Modal;

/// A headerless surface floating over the app - no title bar, ✕ or drag. The one
/// mode that can hang off another entity's rect, via [`Self::at_corner_of`].
pub struct Panel;

/// Modes the app positions itself. Excludes [`Modal`], which its barrier centres.
pub trait Placed {}
impl Placed for Floating {}
impl Placed for Panel {}

/// Modes with a title bar, which is where a ✕ would go. Excludes [`Panel`].
pub trait Titled {}
impl Titled for Floating {}
impl Titled for Modal {}

/// Deferred configuration for one of plume's top-level surfaces.
#[must_use = "a surface does nothing until .show(|ui| …) builds it"]
pub struct ImmDialog<'a, 'w, 's, M = Floating> {
    ui: &'a mut Ui<'w, 's>,
    caller: &'static Location<'static>,
    title: String,
    open: Option<&'a mut bool>,
    layout: DialogLayout,
    style: FrameStyle,
    anchor: Option<Entity>,
    mode: PhantomData<M>,
}

// What every mode configures.
impl<'e, 'w, 's, M> ImmDialog<'e, 'w, 's, M> {
    /// Fix the width (default `Val::Auto` hugs the content). Rows that
    /// distribute space (`.grow()`, `flex_spacer`) need one to resolve against.
    pub fn width(mut self, width: Val) -> Self {
        self.layout.width = width;
        self
    }

    /// Fix the outer height, title bar included, scrolling the body once the
    /// content outgrows it. Prefer [`Self::max_height`] unless it should hold
    /// its size while near-empty.
    pub fn height(mut self, height: Val) -> Self {
        self.layout.height = height;
        self
    }

    /// Cap the outer height: it hugs its content until it would exceed
    /// `max_height`, then stops growing and scrolls the body.
    pub fn max_height(mut self, max_height: Val) -> Self {
        self.layout.max_height = max_height;
        self
    }

    /// Set the gap between the items the body stacks, overriding the default
    /// [`size::SPACE`].
    pub fn gap(mut self, gap: Val) -> Self {
        self.style.gap = Some(gap);
        self
    }

    /// Set the body's padding, overriding the default [`size::SPACE`]. A surface
    /// whose content is one full-bleed fill takes [`Val::ZERO`] here.
    pub fn padding(mut self, padding: impl Into<UiRect>) -> Self {
        self.style.padding = Some(padding.into());
        self
    }
}

// Placement. Not a modal's: it would work - the frame is absolute inside its
// barrier - but one placed off-centre is a different widget, and the mode is
// what holds that rule.
impl<'e, 'w, 's, M: Placed> ImmDialog<'e, 'w, 's, M> {
    /// Position from the viewport's top-left (default `120, 120` for a dialog).
    /// Spawn-time only where the user can drag it - after that the drag owns it.
    pub fn at(mut self, left: Val, top: Val) -> Self {
        self.layout.inset = UiRect {
            left,
            top,
            ..UiRect::AUTO
        };
        self
    }

    /// Pin to a corner of the viewport, `x` and `y` in from its two edges.
    pub fn at_corner(mut self, corner: Corner, x: Val, y: Val) -> Self {
        self.layout.inset = corner.inset(x, y);
        self
    }
}

// The ✕, which lives in a title bar, so only the modes that have one.
impl<'e, 'w, 's, M: Titled> ImmDialog<'e, 'w, 's, M> {
    /// `false` omits the ✕ button, for a surface dismissed only by an action
    /// button. On a [`Modal`] it drops the barrier click and Escape with it,
    /// leaving the body's own buttons as the only answer.
    pub fn closable(mut self, closable: bool) -> Self {
        self.layout.closable = closable;
        self
    }
}

impl<'e, 'w, 's> ImmDialog<'e, 'w, 's, Floating> {
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
        fn scene(title: String, layout: DialogLayout) -> impl Scene {
            bsn! {
                @dialog_frame_scene(title, layout, SurfaceChrome::Dialog)
                on(|close: On<RequestClose>, mut commands: Commands| {
                    commands.entity(close.event_target()).insert(CloseRequested);
                })
            }
        }

        let open = self.open?;
        if !*open {
            return None;
        }
        let id = surface_id(self.caller, &self.title, self.anchor.is_some());
        let (title, layout, style) = (self.title, self.layout, self.style);
        let mut entity = self
            .ui
            .ch_loc(id)
            .on_spawn_apply_scene(move || scene(title, layout));
        if entity.close_requested() {
            *open = false;
            entity.entity_commands().despawn();
            return None;
        }
        Some(reconcile_frame_body(entity, layout, style, f))
    }
}

impl<'e, 'w, 's> ImmDialog<'e, 'w, 's, Modal> {
    /// Build the modal and its body. While `*open` the modal exists and `f`
    /// fills its body; a dismissal writes back through `open`.
    pub fn show(
        self,
        f: impl FnOnce(&mut Ui<'w, 's>),
    ) -> Option<ImmResponse<'e, 'w, 's, kind::Dialog>> {
        fn barrier_scene(closable: bool) -> impl Scene {
            bsn! {
                @modal_barrier()
                // Every dismissal - the ✕, a barrier click, Escape - arrives
                // here, since `RequestClose` propagates up out of the frame.
                @{closable.then_some(bsn! {
                    on(|close: On<RequestClose>, mut commands: Commands| {
                        commands.entity(close.event_target()).insert(CloseRequested);
                    })
                })}
            }
        }

        let open = self.open?;
        if !*open {
            return None;
        }
        let id = surface_id(self.caller, &self.title, self.anchor.is_some());
        let (title, layout, style) = (self.title, self.layout, self.style);
        let closable = layout.closable;
        let mut barrier = self
            .ui
            .ch_loc(id)
            .on_spawn_apply_scene(move || barrier_scene(closable));
        if barrier.close_requested() {
            *open = false;
            barrier.entity_commands().despawn();
            return None;
        }
        // The frame is the barrier's child, which is what centres it, so the body
        // reconciles a level below the entity this mode owns.
        let barrier = barrier.add_ui(move |ui| {
            let frame = ui.ch_id("modal_frame").on_spawn_apply_scene(move || {
                dialog_frame_scene(title, layout, SurfaceChrome::Modal)
            });
            reconcile_frame_body(frame, layout, style, f);
        });
        Some(respond(barrier, false))
    }
}

impl<'e, 'w, 's> ImmDialog<'e, 'w, 's, Panel> {
    /// Pin the panel to a corner of `target`'s rect rather than the viewport's,
    /// `x` and `y` in from that rect's edges. It floats over `target` without
    /// becoming its child, so a clipping or scrolling ancestor of `target`
    /// neither cuts the panel off nor counts it as content.
    ///
    /// The rect is the one last frame's layout settled, so the panel trails a
    /// resize by a frame.
    pub fn at_corner_of(mut self, target: Entity, corner: Corner, x: Val, y: Val) -> Self {
        self.anchor = Some(target);
        self.layout.inset = corner.inset(x, y);
        self
    }

    /// Paint the panel from a theme slot in place of the dialog background.
    /// A panel's only, since a dialog's header keeps its own fill: a body
    /// recoloured under a themed header is a different widget.
    pub fn background_slot(mut self, slot: ThemeSlot) -> Self {
        self.style.background = Some(FrameBackground::Slot(slot));
        self
    }

    /// Paint the panel a fixed color in place of the dialog background. It
    /// stands outside the theme, so a palette swap leaves it be.
    pub fn background(mut self, color: Color) -> Self {
        self.style.background = Some(FrameBackground::Color(color));
        self
    }

    /// Build the panel and its body. Nothing can dismiss a panel, so unlike the
    /// other two modes it has no flag to write back through and always builds.
    pub fn show(self, f: impl FnOnce(&mut Ui<'w, 's>)) -> ImmResponse<'e, 'w, 's, kind::Dialog> {
        let id = surface_id(self.caller, &self.title, self.anchor.is_some());
        let (title, layout, style) = (self.title, self.layout, self.style);
        let Some(target) = self.anchor else {
            let entity = self.ui.ch_loc(id).on_spawn_apply_scene(move || {
                dialog_frame_scene(title, layout, SurfaceChrome::Panel)
            });
            return reconcile_frame_body(entity, layout, style, f);
        };
        // Anchored: the panel hangs in a socket that tracks `target`'s rect, so
        // its inset is measured from that rect. The same primitive a popup sits
        // over its control with, and it takes no picks of its own.
        let socket = self
            .ui
            .ch_loc(id)
            .on_spawn_apply_scene(popup_socket)
            .on_spawn_insert(move || (PopupAnchor(target), PlacementAnchor));
        let socket = socket.add_ui(move |ui| {
            let frame = ui.ch_id("panel_frame").on_spawn_apply_scene(move || {
                dialog_frame_scene(title, layout, SurfaceChrome::Panel)
            });
            reconcile_frame_body(frame, layout, style, f);
        });
        respond(socket, false)
    }
}

// Keyed on call site and title, so several surfaces built in one system stay
// distinct - and on whether it is anchored, because the two forms are different
// entities. An anchored panel hangs in a socket applied *on spawn*, so a panel
// that gains or loses its anchor has to be rebuilt: reusing the entity would
// leave the socket unapplied and nest a second frame inside the first.
fn surface_id(caller: &'static Location<'static>, title: &str, anchored: bool) -> ImmIdBuilder {
    ImmIdBuilder::Hierarchy(ImmId::new((caller, title, anchored)))
}

// Which surface a frame is being built for. The chrome is otherwise identical:
// what differs - the inset a modal leaves `Auto` for its barrier to centre on,
// the drag a pinned surface does without - is already settled in `layout`.
#[derive(Clone, Copy, PartialEq)]
enum SurfaceChrome {
    Dialog,
    Modal,
    Panel,
}

// The frame every mode mounts. The modal is marked so it stays out of the
// floating stack's z and its `TabGroup` turns modal; the panel goes headerless,
// which drops the ✕ and the drag handle with the title bar.
fn dialog_frame_scene(title: String, layout: DialogLayout, chrome: SurfaceChrome) -> impl Scene {
    let DialogLayout {
        width,
        height,
        max_height,
        inset,
        closable,
        movable,
    } = layout;
    let name = match chrome {
        SurfaceChrome::Dialog => format!("PlumeDialog({title})"),
        SurfaceChrome::Modal => format!("PlumeModal({title})"),
        SurfaceChrome::Panel => "PlumePanel".to_owned(),
    };
    let header = match chrome {
        SurfaceChrome::Panel => None,
        _ => Some(DialogHeader {
            title: Box::new(bsn! {
                @caption(title)
                InheritableFont { font_size: size::DIALOG_HEADER_TEXT_SIZE }
            }),
            closable,
            movable,
        }),
    };
    bsn! {
        // Empty body: the imm layer reconciles the body itself.
        @dialog_frame(DialogChrome {
            name: name.into(),
            body: Box::new(bsn_list! {}),
            header,
            width,
            height,
            max_height,
            inset,
        })
        @{(chrome == SurfaceChrome::Modal).then_some(bsn! { ModalDialog })}
    }
}

// A floating surface's styling as its builder chain collected it: the body's
// spacing and the frame's fill. Applied to the surface's own frame; the widget's
// relay is what puts the spacing on the right node.
#[derive(Clone, Copy, Default)]
pub(crate) struct FrameStyle {
    gap: Option<Val>,
    padding: Option<UiRect>,
    background: Option<FrameBackground>,
}

// The fill a caller asked for in place of the frame's own `DIALOG_BG`.
#[derive(Clone, Copy, Debug)]
enum FrameBackground {
    Slot(ThemeSlot),
    Color(Color),
}

impl FrameStyle {
    // Insert whichever the caller set onto `entity`; an unset one is left absent, so
    // the scene's own value stands. One guard keyed on all, as in the tab sizing.
    fn apply(self, entity: &mut ImmEntity<'_, '_, '_, PlumeCaps>) {
        struct FrameStyleKey;
        let Self {
            gap,
            padding,
            background,
        } = self;
        if (gap.is_some() || padding.is_some() || background.is_some())
            && entity.hash_update_typ::<FrameStyleKey>(Some(imm_id(format!(
                "{gap:?}{padding:?}{background:?}"
            ))))
        {
            let mut commands = entity.entity_commands();
            if let Some(gap) = gap {
                commands.insert(BodyGap(gap));
            }
            if let Some(padding) = padding {
                commands.insert(BodyPadding(padding));
            }
            match background {
                Some(FrameBackground::Slot(slot)) => {
                    commands.insert(ThemeBackgroundSlot(slot));
                }
                // A fixed color is outside the theme: the sources go too, or the
                // next palette swap paints over it.
                Some(FrameBackground::Color(color)) => {
                    commands
                        .insert(BackgroundColor(color))
                        .remove::<(ThemeBackgroundToken, ThemeBackgroundSlot)>();
                }
                None => {}
            }
        }
    }
}

// The dialog's frame-level props, as one value the scene builders destructure.
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

    /// Offsets from each edge of the surrounding surface placing a frame `left`, `top`
    /// in from this corner, as
    /// [`PlumeDialogProps::inset`](crate::retained::PlumeDialogProps).
    pub fn inset(self, left: Val, top: Val) -> UiRect {
        let (left, right) = if self.is_right() {
            (Val::Auto, left)
        } else {
            (left, Val::Auto)
        };
        let (top, bottom) = if self.is_bottom() {
            (Val::Auto, top)
        } else {
            (top, Val::Auto)
        };
        UiRect {
            left,
            right,
            top,
            bottom,
        }
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
    pub(crate) style: FrameStyle,
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

    /// Set the gap between the items the popup stacks, overriding the default
    /// [`size::SPACE`].
    pub fn gap(mut self, gap: Val) -> Self {
        self.style.gap = Some(gap);
        self
    }

    /// Set the popup's padding, overriding the default [`size::SPACE_TIGHT`].
    pub fn padding(mut self, padding: impl Into<UiRect>) -> Self {
        self.style.padding = Some(padding.into());
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
            style,
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
        // Unrooted, so the popup is a UI root rather than a descendant of the anchor:
        // picking bubbles, so a popup inside its anchor would re-fire the anchor's
        // clicks and hold it hovered. The socket carries [`PopupAnchor`] instead.
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
                        style.apply(&mut popup);
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

// Reconcile a dialog/panel frame's app-owned size and fill its body, wrapping the
// content in the scrolling machinery when a height knob bounds it. Position is not
// re-applied - the user's dragging owns it after spawn.
fn reconcile_frame_body<'e, 'w, 's>(
    mut entity: ImmEntity<'e, 'w, 's, PlumeCaps>,
    layout: DialogLayout,
    style: FrameStyle,
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
    // Keyed on the asked-for inset alone, so a surface the user has dragged is
    // only moved when its caller asks for somewhere new - a stack re-slotting
    // its panels, say.
    struct FrameInsetKey;
    if entity.hash_update_typ::<FrameInsetKey>(Some(imm_id(format!("{:?}", layout.inset)))) {
        entity
            .entity_commands()
            .queue(move |mut entity: EntityWorldMut| {
                if let Some(mut node) = entity.get_mut::<Node>() {
                    node.left = layout.inset.left;
                    node.top = layout.inset.top;
                    node.right = layout.inset.right;
                    node.bottom = layout.inset.bottom;
                }
            });
    }
    // On the frame, not the body: `relay_dialog_body_style` is what knows which node
    // stacks the content, and that changes with the height knobs above.
    style.apply(&mut entity);
    let entity = entity.add_ui(move |ui| {
        let body = ui.ch_id("dialog_body").on_spawn_apply_scene(dialog_body);
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
    width: Option<Val>,
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
        let viewport = ui
            .ch_id("scroll_viewport")
            .on_spawn_apply_scene(move || scroll_viewport(axis))
            .add_ui(content)
            .entity();
        ui.ch_id("scrollbar")
            .on_spawn_apply_scene(move || scrollbar(viewport, axis));
    })
}

// Widget identity is the call site: `#[track_caller]` bubbles the location through
// the widget methods to the user's source line, so a widget whose bound value moves
// reconciles in place rather than respawning. Folding in `key` makes a change to
// that key respawn instead. Repeated call sites collide; `ch_loc` resolves those.
#[track_caller]
fn loc_id(key: impl core::hash::Hash) -> ImmIdBuilder {
    ImmIdBuilder::Hierarchy(ImmId::new((Location::caller(), key)))
}

// Salt folded into a repeated id's suffix; a fixed tag so a disambiguated id can
// never coincide with a genuine `(location, key)` base.
const OCCURRENCE_SALT: u32 = 0x506c_756d; // "Plum"

// Child creation with plume-side occurrence disambiguation. `bevy_immediate` maps
// each hierarchy id to one entity, so two widgets from the same call site - a helper
// called in a loop - would collide. The first use of a `(parent, base id)` keeps the
// plain id, so a widget appearing once never shifts; each repeat takes a suffix.
// Doing it here rather than in the id resolver keeps upstream unpatched.
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

// Both split directions, which differ only in the axis they hand down. The
// panes are imm children of the splitter's own frame rather than scene props:
// their contents are immediate, and only a `Ui` can build those.
#[track_caller]
fn split<'r, 'w, 's>(
    ui: &'r mut Ui<'w, 's>,
    axis: SplitAxis,
    split_size: &mut SplitSize,
    first: impl FnOnce(&mut PaneUi<'_, 'w, 's>),
    second: impl FnOnce(&mut PaneUi<'_, 'w, 's>),
) -> ImmResponse<'r, 'w, 's, kind::Split> {
    let initial = *split_size;
    let mut changed = false;
    let entity = ui
        .ch_loc(loc_id(()))
        .on_spawn_apply_scene(move || splitter_frame(axis, initial, SplitPane::First))
        .plume_split(split_size, &mut changed);
    let split = *split_size;
    let entity = entity.add_ui(|ui| {
        let pane = ui
            .ch_id("split_first")
            .on_spawn_apply_scene(|| splitter_pane(SplitPane::First));
        // A closed pane's contents are not built at all: nothing measures, so
        // nothing can prop the pane back open, and its imm state is released.
        if split.closed != Some(SplitPane::First) {
            pane.add_ui(|ui| first(&mut PaneUi { ui, split }));
        }
        ui.ch_id("split_divider")
            .on_spawn_apply_scene(move || splitter_divider(axis));
        let pane = ui
            .ch_id("split_second")
            .on_spawn_apply_scene(|| splitter_pane(SplitPane::Second));
        if split.closed != Some(SplitPane::Second) {
            pane.add_ui(|ui| second(&mut PaneUi { ui, split }));
        }
    });
    respond(entity, changed)
}

// `color_swatch` and `color_swatch_rgb`: one spawn, keyed so a call site that
// switches variants respawns, with the color pushed on change thereafter.
#[track_caller]
fn imm_color_swatch<'r, 'w, 's>(
    ui: &'r mut Ui<'w, 's>,
    color: Color,
    alpha: bool,
) -> ImmResponse<'r, 'w, 's, kind::Swatch> {
    let mut entity = ui.ch_loc(loc_id(alpha)).on_spawn_apply_scene(move || {
        bsn! {
            @PlumeColorSwatch {
                @initial_color: color,
                @alpha: alpha,
            }
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

#[cfg(test)]
mod tests {
    use bevy::MinimalPlugins;
    use bevy::app::{App, Update};
    use bevy::ecs::entity::Entity;
    use bevy::ecs::resource::Resource;
    use bevy::ecs::system::ResMut;

    use super::*;
    use crate::imm::{ImmPlugin, PlumeRoot};

    // Records the sibling entities produced each frame, so the test can check both
    // within-frame distinctness and across-frame stability.
    #[derive(Resource, Default)]
    struct Recorded(Vec<Vec<Entity>>);

    // Three widgets from ONE call site (the loop body) under one parent.
    fn three_siblings(mut root: PlumeRoot, mut recorded: ResMut<Recorded>) {
        let mut frame = Vec::new();
        root.push_id("sibling-test", |root| {
            for _ in 0..3 {
                frame.push(root.imm.ch_loc(loc_id(())).entity());
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
            "retained entities must be reused across frames - proof the mapping is \
             populated, so a real collision would surface as a duplicate",
        );
    }
}
