//! Response returned by every imm widget call: state flags plus builder methods.
//! Builders are gated by [`kind`] markers, so only the widget they belong to
//! exposes them.
use core::marker::PhantomData;
use core::ops::RangeInclusive;
use core::panic::Location;

use bevy::asset::AssetServer;
use bevy::color::Color;
use bevy::ecs::hierarchy::Children;
use bevy::ecs::{
    entity::Entity,
    hierarchy::ChildOf,
    world::{EntityWorldMut, World},
};
use bevy::picking::Pickable;
use bevy::scene::{EntityCommandsSceneExt, WorldSceneExt, bsn};
use bevy::text::{FontFeatureTag, FontFeatures, FontSize, FontSource, LineBreak, TextLayout};
use bevy::ui::{
    AlignItems, AlignSelf, BackgroundColor, BorderColor, Checkable, Checked, Node, Overflow,
    UiRect, Val,
};
use bevy::ui_widgets::{SliderPrecision, SliderRange, SliderStep, SliderValue};
use bevy_immediate::ImmId;
use bevy_immediate::{ImmEntity, imm_id, ui::disabled::ImmUiInteractionsDisabled};

use super::caps::ImmPlumeTooltip;
use super::{ImmEntityExt, PlumeCaps, Ui};
use crate::controls::ButtonOutline;
use crate::font_styles::InheritableFont;
use crate::imm::ImmPopup;
use crate::utils::numeric::Numeric;
use crate::{
    constants::size,
    containers::{PopupAnchor, SectionCollapsed, SectionCollapsible},
    controls::{
        ButtonVariant, NoSelectAllOnFocus, PlumeNumberInput, set_select_max_visible,
        text_input_placeholder, text_input_suffix,
    },
    display::{Tooltip, TooltipUi, tooltip_box, tooltip_chrome},
    rounded_corners::RoundedCorners,
    style::fonts,
    theme::{
        Flat, Inert, InheritableTextColor, InheritableThemeTextSlot, ThemeBackgroundSlot,
        ThemeBorderSlot, ThemeSlot, control_box_shadow,
    },
};

/// Zero-sized widget-kind markers for [`ImmResponse`]: each widget returns a
/// response typed to its kind, so kind-specific builders are compile-checked
/// (`ui.button(…).step(…)` doesn't exist). [`kind::Any`] is the default for widgets
/// with only the universal builders.
pub mod kind {
    /// Kinds whose value is a stepped/rounded number: slider, number input.
    pub trait Numeric {}
    /// Kinds built on the text-input frame: number input, text edit.
    pub trait Field {}
    /// Kinds that lay out direct children on a flex axis: row, column.
    /// Excludes frames with nested bodies (section, dialog).
    pub trait Container {}
    /// Kinds whose padding is layout rather than theming, so an app may set it:
    /// Excludes the themed containers (section, dialog).
    pub trait Padded {}
    /// Kinds an app may give a height: they either center their content (button,
    /// swatch) or hold whatever size they are handed (tabs, scroll area). Excludes
    /// controls whose height is font-driven or fixed geometry (caption, toggle,
    /// slider, checkbox, radio), where forcing one clips text or deforms the control.
    pub trait Heightable {}
    /// Kinds that derive nothing from either axis, so an app may set both: button,
    /// color swatch.
    pub trait Sizable: Heightable {}

    /// Default kind: universal builders only (caption, checkbox, toggle, radio).
    pub struct Any;
    /// `caption`.
    pub struct Caption;
    /// `button` / `icon_button` / `tool_button`.
    pub struct Button;
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
    /// `scroll_area`.
    pub struct ScrollArea;
    /// `horizontal`: children flow left-to-right, so its cross axis is vertical.
    pub struct Row;
    /// `vertical`: children flow top-to-bottom, so its cross axis is horizontal.
    pub struct Column;
    /// `screen`.
    pub struct Screen;
    /// `dialog`
    pub struct Dialog;

    impl Numeric for Slider {}
    impl Numeric for Number {}
    impl Field for Number {}
    impl Field for Text {}
    impl Container for Row {}
    impl Container for Column {}
    impl Container for Screen {}
    impl Padded for Row {}
    impl Padded for Column {}
    impl Padded for Screen {}
    impl Heightable for Button {}
    impl Heightable for Swatch {}
    impl Heightable for Tabs {}
    impl Heightable for ScrollArea {}
    impl Heightable for Row {}
    impl Heightable for Column {}
    impl Sizable for Button {}
    impl Sizable for Swatch {}
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
        }
    }

    /// Rich tooltip: `content` builds the panel body each frame while the
    /// tooltip is showing. Takes precedence over [`tooltip`](Self::tooltip) text.
    #[track_caller]
    pub fn tooltip_ui(mut self, content: impl FnOnce(&mut Ui<'w, 's>)) -> Self {
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
    // Guard shared by every value-carrying builder: true when `value` differs from
    // the last one stored under `Key` for this entity, so a builder only queues a
    // command when its argument actually changed. `Key` is a per-builder marker
    // type, giving each its own slot in the entity's hash memory.
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

    /// Tooltip shown after hovering the widget (or any descendant) for a delay.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        struct TooltipKey;
        let text = text.into();
        if self.key_changed::<TooltipKey>(&text) {
            self.e.entity_commands().insert(Tooltip(text));
        }
        self
    }

    /// Set `Flat` (not all controls use this but a lot have a gradient).
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

    /// Keep this widget's size however tight the container gets, so the squeeze
    /// falls on a sibling instead — for chrome that must stay usable.
    ///
    /// A [`grow`](Self::grow) sibling can't take it: growing zeroes `flex_basis`,
    /// and flex shrinks in proportion to it. Give instead with
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

    /// Establish the font size for this widget and everything below it.
    pub fn font_size(mut self, size: FontSize) -> Self {
        struct FontSizeKey;
        if self.key_changed::<FontSizeKey>(imm_for_font_size(size)) {
            self.set_inheritable_font(move |font, _| font.font_size = Some(size));
        }
        self
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
}

fn imm_for_font_size(size: FontSize) -> ImmId {
    let (str, value) = match size {
        FontSize::Px(v) => ("px", v),
        FontSize::Vw(v) => ("vw", v),
        FontSize::Vh(v) => ("vh", v),
        FontSize::VMin(v) => ("vmin", v),
        FontSize::VMax(v) => ("vmax", v),
        FontSize::Rem(v) => ("rem", v),
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
    /// Set both axes to `size` — the natural call for a square swatch or button.
    pub fn square(self, size: Val) -> Self {
        self.width(size).height(size)
    }
}

impl ImmResponse<'_, '_, '_, kind::Caption> {
    /// Keep the text on one line however long it runs, and let it be narrower than
    /// that line — so a bounded container cuts it off instead of the text wrapping
    /// and growing the row.
    ///
    /// Bevy takes a node's clip rect from its parent, never its own `overflow`, so
    /// this only bites when an ancestor clips: pair it with
    /// [`clip`](Self::clip) on the container. Unpaired, the text simply
    /// overflows in one line.
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

    /// Set caption to be small caps
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
}

impl ImmResponse<'_, '_, '_, kind::Button> {
    /// Set the button's color variant (the styling systems re-style on change).
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        struct VariantKey;
        if self.key_changed::<VariantKey>(format!("{variant:?}")) {
            self.e.entity_commands().insert(variant);
        }
        self
    }

    /// Sugar for [`Self::variant`]`(ButtonVariant::Primary)` — the confirm button.
    pub fn primary(self) -> Self {
        self.variant(ButtonVariant::Primary)
    }

    /// Don't respond to hover and press, so only the checked and disabled
    /// states move it.
    pub fn inert(mut self) -> Self {
        struct InertKey;
        if self.key_changed::<InertKey>(true) {
            self.e.entity_commands().insert(Inert);
        }
        self
    }

    /// Which corners the button rounds (fill and border alike).
    /// [`RoundedCorners::None`] squares it off for window chrome or a segmented group.
    pub fn corners(mut self, corners: RoundedCorners) -> Self {
        struct CornersKey;
        if self.key_changed::<CornersKey>(format!("{corners:?}")) {
            self.e
                .entity_commands()
                .queue(move |mut entity: EntityWorldMut| {
                    Self::set_button_corners(&mut entity, corners);
                });
        }
        self
    }

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

    /// Set checked state for a button marked as checkable.
    pub fn checked(mut self, checked: bool) -> Self {
        struct CheckedKey;
        if self.key_changed::<CheckedKey>(format!("{checked:?}")) {
            if checked {
                self.e.entity_commands().insert(Checked);
            } else {
                self.e.entity_commands().remove::<Checked>();
            }
        }
        self
    }

    /// Button is checkable, which will display as primary variant when checked is true.
    pub fn checkable(mut self) -> Self {
        struct CheckableKey;
        if self.key_changed::<CheckableKey>(true) {
            self.e.entity_commands().insert(Checkable);
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
                    if let Some(mut number) = entity.get_mut::<PlumeNumberInput>() {
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

    /// Whether taking focus selects the whole value. Default is `true`
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

impl ImmResponse<'_, '_, '_, kind::Text> {
    /// Dim hint shown while the field is empty and unfocused.
    /// Seeded on first spawn only — hints don't change, and the id doesn't track it.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        if self.will_be_spawned {
            let parent = self.entity;
            let placeholder = placeholder.into();
            self.e.commands().queue(move |world: &mut World| {
                if let Ok(mut child) = world.spawn_scene(text_input_placeholder(placeholder)) {
                    child.insert(ChildOf(parent));
                }
            });
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

impl ImmResponse<'_, '_, '_, kind::Section> {
    /// Seed the section collapsed on first spawn only; afterwards the retained
    /// entity owns its collapse state (a no-op on already-spawned widgets, so it
    /// never fights the user's expand/collapse).
    pub fn start_collapsed(mut self) -> Self {
        if self.will_be_spawned {
            self.e.entity_commands().insert(SectionCollapsed);
        }
        self
    }

    /// Whether the header folds the body when clicked (default true). App-owned
    /// config, so it reconciles every frame; the hash guard re-inserts only when
    /// the value actually changes rather than each frame.
    pub fn collapsible(mut self, collapsible: bool) -> Self {
        struct CollapsibleKey;
        if self.key_changed::<CollapsibleKey>(collapsible) {
            self.e
                .entity_commands()
                .insert(SectionCollapsible(collapsible));
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

    /// Cut off anything a child draws outside this container, rather than letting
    /// it spill. What gives [`no_wrap`](Self::no_wrap) captions their boundary,
    /// since bevy clips a node by its parent's `overflow`, not its own.
    pub fn clip(self) -> Self {
        struct ClipKey;
        self.set_node::<ClipKey, _>((), |node, ()| node.overflow = Overflow::clip())
    }

    /// Is this container pickable (`true`) or does it let pointer events fall
    /// through to whatever is behind (`false`). Default `true` for all containers
    /// except `screen`.
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

    /// Set the gap between children, overriding the container's default. Both
    /// `row_gap` and `column_gap` are set; plume containers are single-axis and
    /// don't wrap, so only the main-axis gap has any effect.
    pub fn gap(self, gap: Val) -> Self {
        struct GapKey;
        self.set_node::<GapKey, _>(gap, |node, gap| {
            node.row_gap = gap;
            node.column_gap = gap;
        })
    }
}

/// Builders for containers whose padding is layout, not theming.
impl<K: kind::Padded> ImmResponse<'_, '_, '_, K> {
    /// Set the container's padding; `UiRect::ZERO` for a flush, full-bleed
    /// surface such as a menu bar.
    pub fn padding<T: Into<UiRect> + core::fmt::Debug + Send + 'static>(self, padding: T) -> Self {
        struct PadKey;
        let padding = padding.into();
        self.set_node::<PadKey, _>(padding, |node, padding| node.padding = padding)
    }

    /// Paint the container's background; a plain row, column or screen has no
    /// fill of its own, unlike a section.
    pub fn background(mut self, color: Color) -> Self {
        struct BackgroundKey;
        if self.key_changed::<BackgroundKey>(format!("{color:?}")) {
            self.e.entity_commands().insert(BackgroundColor(color));
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
    pub fn corners(self, corners: RoundedCorners) -> Self {
        struct CornersKey;
        self.set_node::<CornersKey, _>(corners, |node, corners| {
            node.border_radius = corners.to_border_radius(size::CORNER_RADIUS);
        })
    }

    /// Lift the container off its surface with the standard themed control shadow.
    pub fn shadow(mut self) -> Self {
        struct ShadowKey;
        if self.key_changed::<ShadowKey>(true) {
            self.e.entity_commands().insert(control_box_shadow());
        }
        self
    }
}
