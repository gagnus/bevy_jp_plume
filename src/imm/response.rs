//! Response returned by every imm widget call: state flags plus builder methods,
//! gated by [`kind`] markers so only the widget they belong to exposes them.
use core::marker::PhantomData;
use core::ops::RangeInclusive;
use core::panic::Location;

use bevy::asset::AssetServer;
use bevy::camera::visibility::Visibility;
use bevy::color::Color;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::{ChildOf, Children};
use bevy::ecs::world::{EntityWorldMut, World};
use bevy::picking::Pickable;
use bevy::scene::{EntityCommandsSceneExt, WorldSceneExt, bsn, bsn_list};
use bevy::text::{FontFeatureTag, FontFeatures, FontSource, LineBreak, TextLayout};
use bevy::ui::widget::Text;
use bevy::ui::{
    AlignItems, AlignSelf, BackgroundColor, BorderColor, BorderRadius, Checkable, Display,
    GlobalZIndex, Node, Overflow, UiRect, UiTransform, Val, Val2,
};
use bevy::ui_widgets::{SliderPrecision, SliderRange, SliderStep, SliderValue};
use bevy_immediate::ui::disabled::ImmUiInteractionsDisabled;
use bevy_immediate::{ImmEntity, ImmId, imm_id};

use super::caps::{ImmPlumeChecked, ImmPlumeFocus, ImmPlumeMenu, ImmPlumeTooltip};
use super::widgets::{ImmMenu, imm_menu_popup_on};
use super::{ImmEntityExt, ImmPopup, PlumeCaps, Ui};
use crate::constants::{size, z_order};
use crate::containers::{
    BodyGap, BodyPadding, PopupAnchor, SectionCollapsed, SectionCollapsible, SeparatorBleed,
    SplitCollapsible, SplitDividerAutoHide, SplitMin, SplitPane, SplitSized,
};
use crate::controls::{
    ButtonCheckableVariant, ButtonOutline, ButtonVariant, MenuButtonRole, MenuShortcutText, NoDrag,
    NoSelectAllOnFocus, NumberInputFrame, ScrollbarHidden, TextInputField, menu_anchor_base,
    set_select_max_visible, text_input_placeholder, text_input_prefix_container, text_input_suffix,
    text_input_suffix_container,
};
use crate::display::{Tooltip, TooltipUi, tooltip_box, tooltip_chrome};
use crate::font_styles::{InheritableFont, PlumeFontSize};
use crate::rounded_corners::RoundedCorners;
use crate::style::fonts;
use crate::theme::{
    Flat, Inert, InheritableTextColor, InheritableThemeTextSlot, ThemeBackgroundSlot,
    ThemeBorderSlot, ThemeId, ThemeSlot,
};
use crate::utils::numeric::Numeric;

/// Zero-sized widget-kind markers for [`ImmResponse`]: each widget returns a response
/// typed to its kind, so kind-specific builders are compile-checked.
pub mod kind {
    use core::marker::PhantomData;

    /// Kinds whose value is a stepped/rounded number: slider, number input.
    pub trait Numeric {}
    /// Kinds built on the text-input frame: number input, text edit.
    pub trait Field {}
    /// Kinds spacing children the app supplied, so the gap between them is the
    /// app's to set.
    pub trait Gapped {}
    /// Kinds that lay out direct children on a flex axis: row, column.
    /// Excludes frames with nested bodies (section, dialog).
    pub trait Container: Gapped {}
    /// Kinds whose padding is layout rather than theming, so an app may set it.
    /// Excludes the themed containers (section, dialog).
    pub trait Padded {}
    /// Padded kinds that paint nothing of their own, leaving the fill, border,
    /// corners and shadow to the app. Excludes the controls that theme their own.
    pub trait Surface: Padded {}
    /// Kinds an app may give a height. Excludes controls whose height is font-driven
    /// or fixed geometry (caption, toggle, slider, checkbox, radio).
    pub trait Heightable {}
    /// Kinds that derive nothing from either axis, so an app may set both: button,
    /// color swatch.
    pub trait Sizable: Heightable {}
    /// Kinds built on the button frame, checkable or not: they share its chrome
    /// builders but not its variant, which narrows once the button is checkable.
    pub trait ButtonLike {}
    /// Kinds an app may lift out of the pick path: containers and decorations.
    /// Excludes controls, where it would kill the interaction but not the styling.
    pub trait PickThrough {}

    /// Default kind: universal builders only (caption, checkbox, toggle, radio).
    pub struct Any;
    /// `caption`.
    pub struct Caption;
    /// `icon`.
    pub struct Icon;
    /// What fills a button: its own label, laid out by the control.
    pub struct Label;
    /// What fills a button: children the app supplied, so their padding and gap
    /// are the app's to set.
    pub struct Content;
    /// `button` / `icon_button` / `tool_button`; `Button<Content>` for
    /// `button_container`.
    pub struct Button<C = Label>(PhantomData<C>);
    /// A button whose variant is set: its emphasis is spent, so it can no longer be
    /// made [`checkable`](super::ImmResponse::checkable). Call `checkable` first.
    pub struct StyledButton<C = Label>(PhantomData<C>);
    /// A button after [`checkable`](super::ImmResponse::checkable): its variant is
    /// narrowed to the ones with chrome left to spend on a checked state.
    pub struct CheckableButton<C = Label>(PhantomData<C>);
    /// `color_swatch`.
    pub struct Swatch;
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
    /// `tabs`.
    pub struct Tabs;
    /// `scroll_area_vertical` / `scroll_area_horizontal`.
    pub struct ScrollArea;
    /// `horizontal`: children flow left-to-right, so its cross axis is vertical.
    pub struct Row;
    /// `vertical`: children flow top-to-bottom, so its cross axis is horizontal.
    pub struct Column;
    /// `screen`.
    pub struct Screen;
    /// `separator`.
    pub struct Separator;
    /// `dialog`.
    pub struct Dialog;
    /// `scene`.
    pub struct Scene;
    /// `split_horizontal` / `split_vertical`.
    pub struct Split;
    /// A menu's rows: `item` / `item_toggle` / `submenu`.
    pub struct MenuItem;
    /// `reorderable`.
    pub struct Reorderable;

    impl Numeric for Slider {}
    impl Numeric for Number {}
    impl Field for Number {}
    impl Field for Text {}
    impl Gapped for Row {}
    impl Gapped for Column {}
    impl Gapped for Screen {}
    impl Gapped for Button<Content> {}
    impl Gapped for StyledButton<Content> {}
    impl Gapped for CheckableButton<Content> {}
    impl Container for Row {}
    impl Container for Column {}
    impl Container for Screen {}
    impl Padded for Row {}
    impl Padded for Column {}
    impl Padded for Screen {}
    impl Padded for Button<Content> {}
    impl Padded for StyledButton<Content> {}
    impl Padded for CheckableButton<Content> {}
    impl Surface for Row {}
    impl Surface for Column {}
    impl Surface for Screen {}
    impl<C> Heightable for Button<C> {}
    impl<C> Heightable for StyledButton<C> {}
    impl<C> Heightable for CheckableButton<C> {}
    impl Heightable for Swatch {}
    impl Heightable for Tabs {}
    impl Heightable for ScrollArea {}
    impl Heightable for Row {}
    impl Heightable for Column {}
    impl Heightable for Scene {}
    impl Heightable for Split {}
    impl<C> Sizable for Button<C> {}
    impl<C> Sizable for StyledButton<C> {}
    impl<C> Sizable for CheckableButton<C> {}
    impl Sizable for Swatch {}
    impl<C> ButtonLike for Button<C> {}
    impl<C> ButtonLike for StyledButton<C> {}
    impl<C> ButtonLike for CheckableButton<C> {}
    impl PickThrough for Row {}
    impl PickThrough for Column {}
    impl PickThrough for Screen {}
    impl PickThrough for Caption {}
    impl PickThrough for Icon {}
    impl PickThrough for Separator {}
    impl Gapped for Reorderable {}
    impl PickThrough for Reorderable {}
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
    pub(crate) will_be_spawned: bool,
    // Set by the numeric widgets from their bound `T`. Carried on the response
    // rather than as a marker component because the builders run in the same
    // frame as the spawn, before any inserted marker would be visible.
    pub(crate) integral: bool,
    pub(crate) e: ImmEntity<'r, 'w, 's, PlumeCaps>,
    pub(crate) kind: PhantomData<K>,
}

/// Anchoring, available on every kind.
impl<'r, 'w, 's, K> ImmResponse<'r, 'w, 's, K> {
    // Retype for a builder that narrows what the chain accepts next. Nothing about the
    // entity changes, only which builders stay in scope.
    fn into_kind<K2>(self) -> ImmResponse<'r, 'w, 's, K2> {
        ImmResponse {
            clicked: self.clicked,
            changed: self.changed,
            hovered: self.hovered,
            entity: self.entity,
            will_be_spawned: self.will_be_spawned,
            integral: self.integral,
            e: self.e,
            kind: PhantomData,
        }
    }

    /// Popup floating under this widget (or [`ImmPopup::beside`] it), filled by
    /// [`ImmPopup::show`]. While `*open` the popup exists; a click outside (or
    /// Escape) writes back through `open`.
    #[track_caller]
    pub fn popup<'a>(self, open: &'a mut bool) -> ImmPopup<'r, 'a, 'w, 's, K> {
        ImmPopup {
            anchor: self,
            caller: Location::caller(),
            open,
            placement: Default::default(),
            movable: false,
            close_on_click_outside: true,
            style: Default::default(),
        }
    }

    /// Drop-down menu anchored to this widget, filled by `f` — the standalone form of
    /// the menus [`menu_bar`](super::PlumeImm::menu_bar) holds, on the same
    /// [`ImmMenu`] rows. It owns its open state, so there is no `&mut bool` to keep.
    ///
    /// Anchoring makes the widget activate on press rather than release: opening the
    /// menu moves focus, which would close it before a release landed.
    #[track_caller]
    pub fn menu(mut self, f: impl FnOnce(&mut ImmMenu<'_, 'w, 's>)) -> Self {
        struct MenuAnchor;
        if self.key_changed::<MenuAnchor>(true) {
            self.e.entity_commands().apply_scene(menu_anchor_base());
        }
        self.e = imm_menu_popup_on(self.e, MenuButtonRole::Bar, Location::caller(), f);
        self
    }

    /// Whether the [`menu`](Self::menu) hung off this widget is open. Readable after
    /// the `menu` call, so hover-revealed chrome can stay while the menu is up.
    pub fn menu_open(&self) -> bool {
        self.e.menu_open().is_some()
    }

    /// Whether keyboard focus is in this widget — itself or a direct child.
    pub fn focused(&self) -> bool {
        self.e.focused()
    }

    /// Rich tooltip: `content` builds the panel body each frame while the
    /// tooltip is showing. Takes precedence over [`tooltip`](Self::tooltip) text.
    #[track_caller]
    pub fn tooltip_container(mut self, content: impl FnOnce(&mut Ui<'w, 's>)) -> Self {
        struct TooltipUiKey;
        if self.key_changed::<TooltipUiKey>(true) {
            self.e.entity_commands().insert(TooltipUi);
        }
        if !self.e.tooltip_showing() {
            return self;
        }
        let anchor_entity = self.entity;
        // Unrooted like an imm popup, so the panel is not a descendant of its
        // anchor; the box carries the anchor as `PopupAnchor` instead.
        self.e = self.e.unrooted_ui(Location::caller(), |ui| {
            ui.ch_id("tooltip_box")
                .on_spawn_apply_scene(tooltip_box)
                .on_spawn_insert(move || PopupAnchor(anchor_entity))
                .add_ui(|ui| {
                    ui.ch_id("tooltip_panel")
                        .on_spawn_apply_scene(tooltip_chrome)
                        .add_ui(content);
                });
        });
        self
    }
}

/// Universal builders, available on every kind.
impl<K> ImmResponse<'_, '_, '_, K> {
    // True when `value` differs from the last stored under `Key`, so a builder only
    // queues a command when its argument changed. `Key` is a per-builder marker type,
    // giving each its own slot in the entity's hash memory.
    fn key_changed<Key: 'static>(&mut self, value: impl core::hash::Hash) -> bool {
        self.e.hash_update_typ::<Key>(Some(imm_id(value)))
    }

    // Write one `Node` field, guarded by `Key`. Layout values (`Val`, `UiRect`) hold
    // floats and so aren't `Hash`; their `Debug` form keys them instead.
    fn set_node<Key: 'static, T: core::fmt::Debug + Send + 'static>(
        mut self,
        value: T,
        set: impl FnOnce(&mut Node, T) + Send + 'static,
    ) -> Self {
        if self.key_changed::<Key>(format!("{value:?}")) {
            self.e
                .entity_commands()
                .queue(move |mut entity: EntityWorldMut| {
                    if let Some(mut node) = entity.get_mut::<Node>() {
                        set(&mut node, value);
                    }
                });
        }
        self
    }

    // Write one `InheritableFont` field, merging into whatever the widget already
    // carries.
    fn set_inheritable_font(
        &mut self,
        set: impl FnOnce(&mut InheritableFont, &AssetServer) + Send + 'static,
    ) {
        self.e
            .entity_commands()
            .queue(move |mut entity: EntityWorldMut| {
                // Cloned (an `Arc` bump) so the resource borrow ends before `entry`
                // takes the entity mutably.
                let assets = entity.resource::<AssetServer>().clone();
                let mut font = entity.entry::<InheritableFont>().or_default();
                set(&mut font.get_mut(), &assets);
            });
    }

    /// Enable or disable the control (manages [`bevy::ui::InteractionDisabled`]).
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.e = self.e.interactions_enabled(enabled);
        self
    }

    /// Move, scale or rotate the widget without touching layout — the geometry
    /// pass applies a [`UiTransform`], so this is how chrome animates per frame.
    /// Skipped while the transform is unchanged, so a resting widget costs nothing.
    pub fn transform(mut self, transform: UiTransform) -> Self {
        struct TransformKey;
        if self.key_changed::<TransformKey>(format!("{transform:?}")) {
            self.e.entity_commands().insert(transform);
        }
        self
    }

    /// Paint the widget, or hide it while it holds its place in the layout. Hidden,
    /// it and everything under it take no clicks and leave the Tab order.
    pub fn visible(mut self, visible: bool) -> Self {
        struct VisibleKey;
        if self.key_changed::<VisibleKey>(visible) {
            self.e.entity_commands().insert(visibility(visible));
        }
        self
    }

    /// [`visible`](Self::visible), except a hidden widget also gives up its
    /// space and the container closes up around it.
    pub fn displayed(mut self, displayed: bool) -> Self {
        struct DisplayedKey;
        if self.key_changed::<DisplayedKey>(displayed) {
            self.e
                .entity_commands()
                .queue(move |mut entity: EntityWorldMut| {
                    if let Some(mut node) = entity.get_mut::<Node>() {
                        node.display = match displayed {
                            true => Display::Flex,
                            false => Display::None,
                        };
                    }
                    entity.insert(visibility(displayed));
                });
        }
        self
    }

    /// Tooltip shown after hovering the widget (or any descendant) for a delay.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        struct TooltipKey;
        let text = text.into();
        if self.key_changed::<TooltipKey>(&text) {
            self.e.entity_commands().insert(Tooltip(text));
        }
        self
    }

    /// Set `Flat`: the themed fill renders without its gradient shading.
    pub fn flat(mut self) -> Self {
        struct FlatKey;
        if self.key_changed::<FlatKey>(true) {
            self.e.entity_commands().insert(Flat);
        }
        self
    }

    /// Fill the remaining space along the container's main axis (`flex_grow` from
    /// a zero `flex_basis`).
    pub fn grow(self) -> Self {
        struct GrowKey;
        self.set_node::<GrowKey, _>(true, |node, _| {
            node.flex_basis = Val::ZERO;
            node.flex_grow = 1.0;
        })
    }

    /// Fill the remaining space like [`Self::grow`], but from the content size
    /// rather than zero — so a container that hugs its children still reserves
    /// room for this one.
    pub fn grow_from_content(self) -> Self {
        struct GrowFromContentKey;
        self.set_node::<GrowFromContentKey, _>(true, |node, _| node.flex_grow = 1.0)
    }

    /// Keep this widget's size however tight the container gets, so the squeeze falls
    /// on a sibling. A [`grow`](Self::grow) sibling can't take it — growing zeroes
    /// `flex_basis`, which flex shrinks in proportion to; use
    /// [`grow_from_content`](Self::grow_from_content) + [`min_width`](Self::min_width).
    pub fn no_shrink(self) -> Self {
        struct NoShrinkKey;
        self.set_node::<NoShrinkKey, _>(true, |node, _| node.flex_shrink = 0.0)
    }

    /// Override the control's minimum width. `Val::ZERO` lets a container shrink
    /// below its content — what makes [`clip`](Self::clip) actually clip.
    pub fn min_width(self, min_width: Val) -> Self {
        struct MinWidthKey;
        self.set_node::<MinWidthKey, _>(min_width, |node, min_width| node.min_width = min_width)
    }

    /// Place this one child on its container's cross axis, overriding the
    /// container's `align_items`.
    ///
    /// `AlignSelf::Start` in a column is how to stop a control stretching to full
    /// width; `Stretch` only bites on children that don't fix their own size.
    pub fn align_self(self, align: AlignSelf) -> Self {
        struct AlignSelfKey;
        self.set_node::<AlignSelfKey, _>(align, |node, align| node.align_self = align)
    }

    /// Override the control's width. Written into the retained `Node` only when
    /// the value changes.
    pub fn width(self, width: Val) -> Self {
        struct WidthKey;
        self.set_node::<WidthKey, _>(width, |node, width| node.width = width)
    }

    /// Establish the font size for this widget and everything below it: a bare
    /// `f32` is logical pixels.
    pub fn font_size(mut self, size: impl Into<PlumeFontSize>) -> Self {
        let size = size.into();
        struct FontSizeKey;
        if self.key_changed::<FontSizeKey>(imm_for_plume_font_size(size)) {
            self.set_inheritable_font(move |font, _| font.font_size = Some(size));
        }
        self
    }

    /// Establish the font size as a multiple of the inherited size — CSS's `em`.
    /// `1.25` on a header caption tracks whatever scale its dialog is at.
    pub fn font_scale(self, factor: f32) -> Self {
        self.font_size(PlumeFontSize::Em(factor))
    }

    /// Establish a one-off text color for this widget and everything below it.
    /// State-styled controls re-assert their own — color containers/captions.
    pub fn text_color(mut self, color: Color) -> Self {
        struct TextColorKey;
        if self.key_changed::<TextColorKey>(format!("{color:?}")) {
            self.e.entity_commands().insert(InheritableTextColor(color));
        }
        self
    }

    /// [`text_color`](Self::text_color), but from a theme slot.
    pub fn text_color_slot(mut self, slot: ThemeSlot) -> Self {
        struct TextColorSlotKey;
        if self.key_changed::<TextColorSlotKey>(&slot) {
            self.e
                .entity_commands()
                .insert(InheritableThemeTextSlot(slot));
        }
        self
    }

    /// Render this widget and everything below it in the theme registered under
    /// `id` (see [`UiTheme::set_palette`](crate::theme::UiTheme::set_palette)).
    /// Only ever sets: back to the default theme means not calling this at all.
    pub fn theme(mut self, id: ThemeId) -> Self {
        struct ThemeKey;
        if self.key_changed::<ThemeKey>(&id) {
            self.e
                .entity_commands()
                .insert(bevy::app::Propagate(id.clone()));
        }
        self
    }
}

fn visibility(visible: bool) -> Visibility {
    match visible {
        true => Visibility::Inherited,
        false => Visibility::Hidden,
    }
}

fn imm_for_plume_font_size(size: PlumeFontSize) -> ImmId {
    let (str, value) = match size {
        PlumeFontSize::Px(v) => ("px", v),
        PlumeFontSize::Em(v) => ("em", v),
        PlumeFontSize::Rem(v) => ("rem", v),
    };
    imm_id((str, value.to_bits()))
}

impl<K: kind::Heightable> ImmResponse<'_, '_, '_, K> {
    /// Fix the control's height instead of letting it hug its content. On a kind
    /// that scrolls, content past it scrolls: prefer `max_height` there unless the
    /// region should hold its size while near-empty.
    pub fn height(self, height: Val) -> Self {
        struct HeightKey;
        self.set_node::<HeightKey, _>(height, |node, height| node.height = height)
    }

    /// Containers default to `Val::ZERO`, the flex default (`auto`) resolves to
    /// the content size, which stops any nested scrolling region working.
    /// Set a floor here for chrome that has to keep its size.
    pub fn min_height(self, min_height: Val) -> Self {
        struct MinHeightKey;
        self.set_node::<MinHeightKey, _>(min_height, |node, min_height| {
            node.min_height = min_height
        })
    }
}

impl<K: kind::Sizable> ImmResponse<'_, '_, '_, K> {
    /// Set width and height.
    pub fn size(self, size: Val2) -> Self {
        self.width(size.x).height(size.y)
    }
}

impl ImmResponse<'_, '_, '_, kind::Caption> {
    /// Keep the text on one line however long it runs, and let it be narrower than
    /// that line, so a bounded container cuts it off instead of growing the row.
    /// Pair with [`clip`](Self::clip) on the container; unpaired, the text overflows.
    pub fn no_wrap(mut self) -> Self {
        struct NoWrapKey;
        if self.key_changed::<NoWrapKey>(()) {
            self.e.entity_commands().apply_scene(bsn! {
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                }
            });
        }
        // Flex resolves `min_width: auto` to the content size, which for unwrapped
        // text is the whole string — the row would grow rather than clip.
        struct NoWrapMinKey;
        self.set_node::<NoWrapMinKey, _>((), |node, ()| node.min_width = Val::ZERO)
    }

    /// Render the caption in small caps.
    pub fn small_caps(mut self) -> Self {
        struct SmallCapsKey;
        if self.key_changed::<SmallCapsKey>(()) {
            self.set_inheritable_font(|font, _| {
                font.font_features = Some(FontFeatures::from([
                    FontFeatureTag::SMALL_CAPS,
                    FontFeatureTag::CAPS_TO_SMALL_CAPS,
                ]));
            });
        }
        self
    }

    /// Pin the caption to the monospace face; size and features still inherit.
    pub fn monospace(mut self) -> Self {
        struct MonospaceKey;
        if self.key_changed::<MonospaceKey>(()) {
            self.set_inheritable_font(|font, assets| {
                font.font = Some(FontSource::Handle(assets.load(fonts::MONOSPACE)));
            });
        }
        self
    }

    /// Pin the caption to the bold face; size and features still inherit.
    pub fn bold(mut self) -> Self {
        struct BoldKey;
        if self.key_changed::<BoldKey>(()) {
            self.set_inheritable_font(|font, assets| {
                font.font = Some(FontSource::Handle(assets.load(fonts::BOLD)));
            });
        }
        self
    }

    /// Pin the text color to [`ThemeSlot::Text0`].
    pub fn bright(self) -> Self {
        self.text_color_slot(ThemeSlot::Text0)
    }
}

/// Builders shared by the button kinds, checkable or not.
impl<K: kind::ButtonLike> ImmResponse<'_, '_, '_, K> {
    /// Don't respond to hover and press, so only the checked and disabled
    /// states move it.
    pub fn inert(mut self) -> Self {
        struct InertKey;
        if self.key_changed::<InertKey>(true) {
            self.e.entity_commands().insert(Inert);
        }
        self
    }

    // Body of each kind's `corners`, which stays concrete: an inherent `corners` on a
    // bounded `K` would read as a duplicate of the container one whatever the bounds say.
    fn corners_impl(mut self, corners: RoundedCorners) -> Self {
        // Re-round a spawned button: the fill's radius lives on the button node, the
        // border's on its [`ButtonOutline`] overlay, and both have to agree.
        fn set_button_corners(button: &mut EntityWorldMut, corners: RoundedCorners) {
            let radius = corners.to_border_radius(size::CORNER_RADIUS);
            if let Some(mut node) = button.get_mut::<Node>() {
                node.border_radius = radius;
            }
            let children: Vec<Entity> = button
                .get::<Children>()
                .map(|children| children.iter().copied().collect())
                .unwrap_or_default();
            button.world_scope(|world| {
                for child in children {
                    if world.get::<ButtonOutline>(child).is_some()
                        && let Some(mut node) = world.get_mut::<Node>(child)
                    {
                        node.border_radius = radius;
                    }
                }
            });
        }

        struct CornersKey;
        if self.key_changed::<CornersKey>(format!("{corners:?}")) {
            self.e
                .entity_commands()
                .queue(move |mut entity: EntityWorldMut| {
                    set_button_corners(&mut entity, corners);
                });
        }
        self
    }
}

impl<'r, 'w, 's, C> ImmResponse<'r, 'w, 's, kind::Button<C>> {
    /// Which corners the button rounds (fill and border alike).
    /// [`RoundedCorners::None`] squares it off for window chrome or a segmented group.
    pub fn corners(self, corners: RoundedCorners) -> Self {
        self.corners_impl(corners)
    }

    /// Set the button's color variant. This spends the button's emphasis, so
    /// [`checkable`](Self::checkable) is no longer in reach.
    pub fn variant(
        mut self,
        variant: ButtonVariant,
    ) -> ImmResponse<'r, 'w, 's, kind::StyledButton<C>> {
        struct VariantKey;
        if self.key_changed::<VariantKey>(format!("{variant:?}")) {
            self.e.entity_commands().insert(variant);
        }
        self.into_kind()
    }

    /// Sugar for [`Self::variant`]`(ButtonVariant::Primary)` — the confirm button.
    pub fn primary(self) -> ImmResponse<'r, 'w, 's, kind::StyledButton<C>> {
        self.variant(ButtonVariant::Primary)
    }

    /// Sugar for [`Self::variant`]`(ButtonVariant::Danger)` — the confirm button for a
    /// destructive action.
    pub fn danger(self) -> ImmResponse<'r, 'w, 's, kind::StyledButton<C>> {
        self.variant(ButtonVariant::Danger)
    }

    /// Turn the button into a two-state toggle bound to `value`: activating it flips
    /// `value` and `.changed` fires. Call it before choosing chrome.
    pub fn checkable(
        mut self,
        value: &mut bool,
    ) -> ImmResponse<'r, 'w, 's, kind::CheckableButton<C>> {
        struct CheckableKey;
        if self.key_changed::<CheckableKey>(true) {
            self.e.entity_commands().insert((
                Checkable,
                ButtonVariant::from(ButtonCheckableVariant::default()),
            ));
        }
        let mut changed = false;
        self.e = self.e.plume_checked(value, &mut changed);
        self.changed |= changed;
        self.into_kind()
    }
}

impl<C> ImmResponse<'_, '_, '_, kind::StyledButton<C>> {
    /// Which corners the button rounds (fill and border alike).
    /// [`RoundedCorners::None`] squares it off for window chrome or a segmented group.
    pub fn corners(self, corners: RoundedCorners) -> Self {
        self.corners_impl(corners)
    }
}

impl<C> ImmResponse<'_, '_, '_, kind::CheckableButton<C>> {
    /// Which corners the toggle rounds (fill and border alike).
    /// [`RoundedCorners::None`] squares it off for window chrome or a segmented group.
    pub fn corners(self, corners: RoundedCorners) -> Self {
        self.corners_impl(corners)
    }

    /// Set the toggle's rest-state chrome; the checked state accents whichever
    /// surface that variant leads with.
    pub fn variant(mut self, variant: ButtonCheckableVariant) -> Self {
        struct ToggleVariantKey;
        if self.key_changed::<ToggleVariantKey>(format!("{variant:?}")) {
            self.e
                .entity_commands()
                .insert(ButtonVariant::from(variant));
        }
        self
    }
}

/// Builders shared by the numeric-valued kinds (slider, number input).
impl<K: kind::Numeric> ImmResponse<'_, '_, '_, K> {
    /// Set the increment applied by arrow keys (slider) or Up/Down in the field
    /// (number input). A slider defaults to 1% of its range, a number input to 1.
    pub fn step<T: Numeric>(mut self, step: T) -> Self {
        struct StepKey;
        if self.key_changed::<StepKey>(step.to_f32().to_bits()) {
            self.e.entity_commands().insert(SliderStep(step.to_f32()));
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::Slider> {
    /// Decimal places drag values are rounded to (`0` = integer). Ignored when the
    /// bound value is an integer type — with a warning if a non-zero was asked for.
    #[track_caller]
    pub fn precision(mut self, precision: usize) -> Self {
        struct PrecisionKey;
        if self.key_changed::<PrecisionKey>(precision) {
            if self.integral {
                warn_integral_precision(precision, Location::caller());
                return self;
            }
            self.e
                .entity_commands()
                .insert(SliderPrecision(precision as i32));
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::Number> {
    /// Whether dragging horizontally scrubs the value. Default is `true`;
    /// disabling leaves click-to-type as the only pointer interaction.
    pub fn draggable(mut self, draggable: bool) -> Self {
        struct DraggableKey;
        if self.key_changed::<DraggableKey>(draggable) {
            match draggable {
                true => self.e.entity_commands().remove::<NoDrag>(),
                false => self.e.entity_commands().insert(NoDrag),
            };
        }
        self
    }

    /// Set the committable range (clamps typed/stepped values).
    pub fn range<T: Numeric>(mut self, range: RangeInclusive<T>) -> Self {
        let (min, max) = (range.start().to_f32(), range.end().to_f32());
        struct RangeKey;
        if self.key_changed::<RangeKey>((min.to_bits(), max.to_bits())) {
            self.e.entity_commands().insert(SliderRange::new(min, max));
        }
        self
    }

    /// Set the displayed/committed decimal precision (`0` = integer). Reprints the
    /// value. Ignored when the bound value is an integer type — with a warning if a
    /// non-zero was asked for.
    #[track_caller]
    pub fn precision(mut self, precision: usize) -> Self {
        struct PrecisionKey;
        if self.key_changed::<PrecisionKey>(precision) {
            if self.integral {
                warn_integral_precision(precision, Location::caller());
                return self;
            }
            self.e
                .entity_commands()
                .queue(move |mut entity: EntityWorldMut| {
                    if let Some(mut number) = entity.get_mut::<NumberInputFrame>() {
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

fn warn_integral_precision(precision: usize, caller: &Location<'static>) {
    if precision > 0 {
        bevy::log::warn!(
            precision,
            %caller,
            "`precision` ignored: the bound value is an integer type, which always rounds to whole numbers"
        );
    }
}

/// Builders shared by the text-input-frame kinds (number input, text edit).
impl<K: kind::Field> ImmResponse<'_, '_, '_, K> {
    /// Append a dim, non-editable unit suffix to the input (e.g. `m/s`).
    /// Seeded on first spawn only — units don't change, and the id doesn't track it.
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        if self.will_be_spawned {
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

    /// Whether taking focus selects the whole value. Default is `true`.
    pub fn select_on_focus(mut self, select_on_focus: bool) -> Self {
        struct SelectOnFocusKey;
        if self.key_changed::<SelectOnFocusKey>(select_on_focus) {
            match select_on_focus {
                true => self.e.entity_commands().remove::<NoSelectAllOnFocus>(),
                false => self.e.entity_commands().insert(NoSelectAllOnFocus),
            };
        }
        self
    }
}

impl<'w, 's> ImmResponse<'_, 'w, 's, kind::Text> {
    /// Dim hint shown while the field is empty and unfocused.
    /// Seeded on first spawn only — hints don't change, and the id doesn't track it.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        if self.will_be_spawned {
            let parent = self.entity;
            let placeholder = placeholder.into();
            self.e.commands().queue(move |world: &mut World| {
                // The hint hangs off the editable field, so it starts where the
                // text would, whatever adornments sit ahead of the field.
                let field = world.get::<Children>(parent).and_then(|children| {
                    children
                        .iter()
                        .copied()
                        .find(|&child| world.get::<TextInputField>(child).is_some())
                });
                match field {
                    Some(field) => {
                        if let Ok(mut child) =
                            world.spawn_scene(text_input_placeholder(placeholder))
                        {
                            child.insert(ChildOf(field));
                        }
                    }
                    // Seeded first-spawn-only, so a miss would be permanent and
                    // silent without this.
                    None => bevy::log::warn!(
                        "text_edit placeholder dropped: no field child at spawn time"
                    ),
                }
            });
        }
        self
    }

    /// Content ahead of the text, inside the frame — a clear button, an icon.
    /// `f` builds it each frame, like any imm container body.
    #[track_caller]
    pub fn prefix_container(mut self, f: impl FnOnce(&mut Ui<'w, 's>)) -> Self {
        self.e = self.e.add_ui(|ui| {
            ui.ch_id("text_input_prefix")
                .on_spawn_apply_scene(|| text_input_prefix_container(Box::new(bsn_list![])))
                .add_ui(f);
        });
        self
    }

    /// Content after the text, inside the frame. The trailing slot holds a unit
    /// label or content, never both: exclusive with [`suffix`](Self::suffix).
    #[track_caller]
    pub fn suffix_container(mut self, f: impl FnOnce(&mut Ui<'w, 's>)) -> Self {
        self.e = self.e.add_ui(|ui| {
            ui.ch_id("text_input_suffix")
                .on_spawn_apply_scene(|| text_input_suffix_container(Box::new(bsn_list![])))
                .add_ui(f);
        });
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::Screen> {
    /// Where this screen sits in the window's stack, back to front.
    ///
    /// Clamped to [`z_order::APP_MAX`]: everything above belongs to plume's.
    pub fn z_index(mut self, z: i32) -> Self {
        struct ZIndexKey;
        let z = z.min(z_order::APP_MAX);
        if self.key_changed::<ZIndexKey>(z) {
            self.e.entity_commands().insert(GlobalZIndex(z));
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::Separator> {
    /// Run the rule edge to edge through the container's padding instead of
    /// stopping at its content box.
    pub fn full_bleed(mut self) -> Self {
        struct FullBleedKey;
        if self.key_changed::<FullBleedKey>(true) {
            self.e.entity_commands().insert(SeparatorBleed);
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::Select> {
    /// Cap the popup at `max_visible` rows before it scrolls (default 8).
    pub fn max_visible(mut self, max_visible: usize) -> Self {
        struct MaxVisibleKey;
        if self.key_changed::<MaxVisibleKey>(max_visible) {
            let select_ent = self.entity;
            self.e.commands().queue(move |world: &mut World| {
                set_select_max_visible(world, select_ent, max_visible);
            });
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::MenuItem> {
    /// Right-aligned shortcut hint on the item row. Display only — handling the
    /// key is the app's business.
    pub fn shortcut(mut self, text: impl Into<String>) -> Self {
        struct ShortcutKey;
        let text = text.into();
        if self.key_changed::<ShortcutKey>(&text) {
            self.e
                .entity_commands()
                .queue(move |mut e: EntityWorldMut| {
                    let children: Vec<Entity> = e
                        .get::<Children>()
                        .map(|children| children.iter().copied().collect())
                        .unwrap_or_default();
                    e.world_scope(|world| {
                        for child in children {
                            if world.get::<MenuShortcutText>(child).is_none() {
                                continue;
                            }
                            if let Some(mut node) = world.get_mut::<Node>(child) {
                                node.display = if text.is_empty() {
                                    Display::None
                                } else {
                                    Display::Flex
                                };
                            }
                            if let Some(mut caption) = world.get_mut::<Text>(child) {
                                caption.0 = text;
                            }
                            break;
                        }
                    });
                });
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::Section> {
    /// Seed the section collapsed on first spawn only; afterwards the retained entity
    /// owns its collapse state, so this never fights the user.
    pub fn start_collapsed(mut self) -> Self {
        if self.will_be_spawned {
            self.e.entity_commands().insert(SectionCollapsed);
        }
        self
    }

    /// Whether the header folds the body when clicked (default true).
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        struct CollapsibleKey;
        if self.key_changed::<CollapsibleKey>(collapsible) {
            self.e
                .entity_commands()
                .insert(SectionCollapsible(collapsible));
        }
        self
    }

    /// Set the gap between the items the body stacks, overriding the default
    /// [`size::SPACE_TIGHT`].
    pub fn gap(mut self, gap: Val) -> Self {
        struct SectionBodyGapKey;
        if self.key_changed::<SectionBodyGapKey>(format!("{gap:?}")) {
            self.e.entity_commands().insert(BodyGap(gap));
        }
        self
    }

    /// Set the body's padding, overriding the default [`size::SPACE`].
    pub fn padding(mut self, padding: impl Into<UiRect>) -> Self {
        struct SectionBodyPadKey;
        let padding = padding.into();
        if self.key_changed::<SectionBodyPadKey>(format!("{padding:?}")) {
            self.e.entity_commands().insert(BodyPadding(padding));
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::Tabs> {
    /// Set the gap between the items the body stacks, overriding the default
    /// [`size::SPACE`]. Container-wide: every tab's body reads as the same surface.
    pub fn gap(mut self, gap: Val) -> Self {
        struct TabBodyGapKey;
        if self.key_changed::<TabBodyGapKey>(format!("{gap:?}")) {
            self.e.entity_commands().insert(BodyGap(gap));
        }
        self
    }

    /// Set the body's padding, overriding the default [`size::SPACE`]. A tab wanting
    /// none of it takes [`Val::ZERO`] here and pads the others' content from inside.
    pub fn padding(mut self, padding: impl Into<UiRect>) -> Self {
        struct TabBodyPadKey;
        let padding = padding.into();
        if self.key_changed::<TabBodyPadKey>(format!("{padding:?}")) {
            self.e.entity_commands().insert(BodyPadding(padding));
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::Split> {
    /// How small each pane may get. [`Val::Auto`] — the default — is the pane's own
    /// content minimum; any other `Val` also stops the drag. A splitter too small for
    /// both floors gives the first pane its own and lets the second give.
    pub fn min_panes(mut self, first: Val, second: Val) -> Self {
        struct MinPanes;
        if self.key_changed::<MinPanes>(format!("{first:?}{second:?}")) {
            self.e.entity_commands().insert(SplitMin { first, second });
        }
        self
    }

    /// Which pane the bound [`SplitSize`](crate::containers::SplitSize) describes.
    /// The other pane flexes into the rest, so it alone absorbs resizes of the
    /// splitter itself. Defaults to the first pane.
    pub fn sized_pane(mut self, pane: SplitPane) -> Self {
        struct SizedPaneKey;
        if self.key_changed::<SizedPaneKey>(pane) {
            self.e.entity_commands().insert(SplitSized(pane));
        }
        self
    }

    /// Let a drag snap a pane fully closed.
    pub fn collapsible(mut self, first: bool, second: bool) -> Self {
        struct CollapsibleKey;
        if self.key_changed::<CollapsibleKey>((first, second)) {
            self.e
                .entity_commands()
                .insert(SplitCollapsible { first, second });
        }
        self
    }

    /// Paint the divider only while it is in use.
    pub fn auto_hide_divider(mut self, auto_hide: bool) -> Self {
        struct AutoHideKey;
        if self.key_changed::<AutoHideKey>(auto_hide) {
            self.e
                .entity_commands()
                .insert(SplitDividerAutoHide(auto_hide));
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::ScrollArea> {
    /// Cap the region's height: it hugs its content until it would exceed
    /// `max_height`, then stops growing and scrolls. The natural bound when the
    /// area sits in an auto-height surface.
    pub fn max_height(self, max_height: Val) -> Self {
        struct MaxHeightKey;
        self.set_node::<MaxHeightKey, _>(max_height, |node, max_height| {
            node.max_height = max_height
        })
    }

    /// [`max_height`](Self::max_height) on the other axis — the bound a
    /// `scroll_area_horizontal` needs in an auto-width surface.
    pub fn max_width(self, max_width: Val) -> Self {
        struct MaxWidthKey;
        self.set_node::<MaxWidthKey, _>(max_width, |node, max_width| node.max_width = max_width)
    }

    /// Set the gap between the items the region stacks.
    pub fn gap(mut self, gap: Val) -> Self {
        struct ScrollContentGapKey;
        if self.key_changed::<ScrollContentGapKey>(format!("{gap:?}")) {
            self.e.entity_commands().insert(BodyGap(gap));
        }
        self
    }

    /// Set the content's padding, which scrolls with it rather than framing the
    /// region — so it also spaces the two ends of the scroll.
    pub fn padding(mut self, padding: impl Into<UiRect>) -> Self {
        struct ScrollContentPadKey;
        let padding = padding.into();
        if self.key_changed::<ScrollContentPadKey>(format!("{padding:?}")) {
            self.e.entity_commands().insert(BodyPadding(padding));
        }
        self
    }

    /// Scroll with no scrollbar drawn and no gutter reserved for one, leaving the
    /// wheel as the only hint that there is more to see. For chrome with no room.
    pub fn hide_scrollbar(mut self) -> Self {
        struct HideScrollbarKey;
        if self.key_changed::<HideScrollbarKey>(true) {
            self.e.entity_commands().insert(ScrollbarHidden);
        }
        self
    }
}

/// Builders shared by the container kinds (row, column, screen).
impl<K: kind::Container> ImmResponse<'_, '_, '_, K> {
    /// Place every child on the container's cross axis — `Start` means top on a
    /// [`Row`](kind::Row), left on a [`Column`](kind::Column).
    ///
    /// Columns default to `Stretch` (what `.grow()` in nested rows resolves
    /// against), rows to `Center`. For a single child, see [`Self::align_self`].
    pub fn align_items(self, align: AlignItems) -> Self {
        struct AlignItemsKey;
        self.set_node::<AlignItemsKey, _>(align, |node, align| node.align_items = align)
    }

    /// Cut off anything a child draws outside this container. Bevy clips a node by its
    /// parent's `overflow`, not its own, so this is what bounds a
    /// [`no_wrap`](Self::no_wrap) caption.
    pub fn clip(self) -> Self {
        struct ClipKey;
        self.set_node::<ClipKey, _>((), |node, ()| node.overflow = Overflow::clip())
    }
}

/// Builders for the kinds that need not take pointer events.
impl<K: kind::PickThrough> ImmResponse<'_, '_, '_, K> {
    /// Does this take pointer events (`true`) or let them fall through to
    /// whatever is behind (`false`). Default `true` everywhere but `screen`.
    ///
    /// Per-entity and depth-based, so it says nothing about the children: a row
    /// lifted out of the path still hands its buttons their picks.
    pub fn pickable(mut self, pickable: bool) -> Self {
        struct PickableKey;
        if self.key_changed::<PickableKey>(pickable) {
            let component = if pickable {
                Pickable::default()
            } else {
                Pickable::IGNORE
            };
            self.e.entity_commands().insert(component);
        }
        self
    }
}

/// Builders for kinds spacing app-supplied children.
impl<K: kind::Gapped> ImmResponse<'_, '_, '_, K> {
    /// Set the gap between children, overriding the container's default.
    pub fn gap(self, gap: Val) -> Self {
        struct GapKey;
        self.set_node::<GapKey, _>(gap, |node, gap| {
            node.row_gap = gap;
            node.column_gap = gap;
        })
    }
}

/// Builders for kinds whose padding is layout, not theming.
impl<K: kind::Padded> ImmResponse<'_, '_, '_, K> {
    /// Set the container's padding; `UiRect::ZERO` for a flush, full-bleed
    /// surface such as a menu bar.
    pub fn padding<T: Into<UiRect> + core::fmt::Debug + Send + 'static>(self, padding: T) -> Self {
        struct PadKey;
        let padding = padding.into();
        self.set_node::<PadKey, _>(padding, |node, padding| node.padding = padding)
    }
}

/// Builders for containers that paint no chrome of their own.
impl<K: kind::Surface> ImmResponse<'_, '_, '_, K> {
    /// Paint the container's background; a plain row, column or screen has no
    /// fill of its own, unlike a section.
    pub fn background(mut self, color: Color) -> Self {
        struct BackgroundKey;
        if self.key_changed::<BackgroundKey>(format!("{color:?}")) {
            self.e
                .entity_commands()
                .insert(BackgroundColor(color))
                .remove::<ThemeBackgroundSlot>();
        }
        self
    }

    /// Paint the container's background from a theme slot.
    pub fn background_slot(mut self, slot: ThemeSlot) -> Self {
        struct BackgroundSlotKey;
        if self.key_changed::<BackgroundSlotKey>(&slot) {
            self.e.entity_commands().insert(ThemeBackgroundSlot(slot));
        }
        self
    }

    /// Draw a border of `width` in `color` around the container. Width and color
    /// travel together because either alone paints nothing.
    pub fn border(mut self, width: impl Into<UiRect>, color: Color) -> Self {
        let width = width.into();
        struct BorderKey;
        if self.key_changed::<BorderKey>(format!("{width:?}{color:?}")) {
            self.e
                .entity_commands()
                .queue(move |mut entity: EntityWorldMut| {
                    if let Some(mut node) = entity.get_mut::<Node>() {
                        node.border = width;
                    }
                    entity.insert(BorderColor::all(color));
                });
        }
        self
    }

    /// Draw a border of `width` in a theme slot's color.
    pub fn border_slot(mut self, width: impl Into<UiRect>, slot: ThemeSlot) -> Self {
        let width = width.into();
        struct BorderSlotKey;
        if self.key_changed::<BorderSlotKey>((format!("{width:?}"), &slot)) {
            self.e
                .entity_commands()
                .queue(move |mut entity: EntityWorldMut| {
                    if let Some(mut node) = entity.get_mut::<Node>() {
                        node.border = width;
                    }
                    entity.insert(ThemeBorderSlot(slot));
                });
        }
        self
    }

    /// Round the container's corners; the background and border follow it.
    pub fn border_radius<T: Into<BorderRadius>>(self, border_radius: T) -> Self {
        let border_radius = border_radius.into();
        struct BorderRadiusKey;
        self.set_node::<BorderRadiusKey, _>(border_radius, |node, border_radius| {
            node.border_radius = border_radius;
        })
    }
}
