//! Response returned by every imm widget call: state flags plus builder methods.
//! Builders are gated by [`kind`] markers, so only the widget they belong to
//! exposes them.
use core::marker::PhantomData;
use core::ops::RangeInclusive;

use bevy_app::PropagateOver;
use bevy_color::Color;
use bevy_ecs::{
    entity::Entity,
    hierarchy::ChildOf,
    world::{EntityWorldMut, World},
};
use bevy_immediate::{ImmEntity, imm_id, ui::disabled::ImmUiInteractionsDisabled};
use bevy_scene::{EntityCommandsSceneExt, WorldSceneExt, bsn};
use bevy_text::{FontFeatureTag, FontFeatures, FontSourceTemplate, TextFont};
use bevy_ui::{AlignItems, AlignSelf, BackgroundColor, Checkable, Checked, Node, UiRect, Val};
use bevy_ui_widgets::{SliderPrecision, SliderRange, SliderStep, SliderValue};

use super::PlumeCaps;
use crate::{
    constants::{fonts, size},
    controls::{
        ButtonVariant, PlumeNumberInput, set_select_max_visible, text_input_placeholder,
        text_input_suffix,
    },
};
use crate::{
    containers::{SectionCollapsed, SectionCollapsible},
    theme::{Flat, Inert},
};

/// Zero-sized widget-kind markers for [`ImmResponse`]: each widget returns a
/// response typed to its kind, so kind-specific builders are compile-checked
/// (`ui.button(…).step(…)` doesn't exist). [`Any`] is the default for widgets
/// with only the universal builders.
pub mod kind {
    /// Kinds whose value is a stepped/rounded number: slider, number input.
    pub trait Numeric {}
    /// Kinds built on the text-input frame: number input, text edit.
    pub trait Field {}
    /// Kinds that lay out children on a flex axis: row, column, group, screen.
    pub trait Container {}
    /// Kinds whose padding is layout rather than theming, so an app may set it:
    /// row, column, screen. Excludes the themed containers (group, section).
    pub trait Padded {}
    /// Kinds an app may give a height: they either centre their content (button,
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
    /// `group`: a themed column, so its padding is not app-settable.
    pub struct Group;
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
    impl Container for Group {}
    impl Container for Screen {}
    impl Padded for Row {}
    impl Padded for Column {}
    impl Padded for Screen {}
    impl Heightable for Button {}
    impl Heightable for Swatch {}
    impl Heightable for Tabs {}
    impl Heightable for ScrollArea {}
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
    pub(crate) e: ImmEntity<'r, 'w, 's, PlumeCaps>,
    pub(crate) kind: PhantomData<K>,
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

    /// Enable or disable the control (manages [`bevy_ui::InteractionDisabled`]).
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.e = self.e.interactions_enabled(enabled);
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
}

impl<K: kind::Heightable> ImmResponse<'_, '_, '_, K> {
    /// Fix the control's height instead of letting it hug its content. On a kind
    /// that scrolls, content past it scrolls: prefer `max_height` there unless the
    /// region should hold its size while near-empty.
    pub fn height(self, height: Val) -> Self {
        struct HeightKey;
        self.set_node::<HeightKey, _>(height, |node, height| node.height = height)
    }
}

impl<K: kind::Sizable> ImmResponse<'_, '_, '_, K> {
    /// Set both axes to `size` — the natural call for a square swatch or button.
    pub fn square(self, size: Val) -> Self {
        self.width(size).height(size)
    }
}

impl ImmResponse<'_, '_, '_, kind::Caption> {
    /// Set caption to be small caps
    pub fn small_caps(mut self) -> Self {
        struct SmallCapsKey;
        if self.key_changed::<SmallCapsKey>(()) {
            self.e.entity_commands().apply_scene(bsn! {
                TextFont {
                    font: FontSourceTemplate::Handle(fonts::REGULAR),
                    font_size: size::MEDIUM_FONT,
                    font_features: FontFeatures::from([
                        FontFeatureTag::SMALL_CAPS,
                        FontFeatureTag::CAPS_TO_SMALL_CAPS,
                    ]),
                }
                PropagateOver<TextFont>
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

    /// Set checked on a button also marked as checkable
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

    /// Button is checkable, which will display as primary variant when checked is true
    /// also implies inert()
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
    pub fn step(mut self, step: f32) -> Self {
        struct StepKey;
        if self.key_changed::<StepKey>(step.to_bits()) {
            self.e.entity_commands().insert(SliderStep(step));
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::Slider> {
    /// Decimal places drag values are rounded to (`0` = integer).
    pub fn precision(mut self, precision: usize) -> Self {
        struct PrecisionKey;
        if self.key_changed::<PrecisionKey>(precision) {
            self.e
                .entity_commands()
                .insert(SliderPrecision(precision as i32));
        }
        self
    }
}

impl ImmResponse<'_, '_, '_, kind::Number> {
    /// Set the committable range (clamps typed/stepped values).
    pub fn range(mut self, range: RangeInclusive<f32>) -> Self {
        let (min, max) = (*range.start(), *range.end());
        struct RangeKey;
        if self.key_changed::<RangeKey>((min.to_bits(), max.to_bits())) {
            self.e.entity_commands().insert(SliderRange::new(min, max));
        }
        self
    }

    /// Set the displayed/committed decimal precision (`0` = integer).
    /// Reprints the value.
    pub fn precision(mut self, precision: usize) -> Self {
        struct PrecisionKey;
        if self.key_changed::<PrecisionKey>(precision) {
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

/// Builders shared by the container kinds (row, column).
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
}

/// Builders for containers whose padding is layout, not theming.
impl<K: kind::Padded> ImmResponse<'_, '_, '_, K> {
    /// Set the container's padding; `UiRect::ZERO` for a flush, full-bleed
    /// surface such as a menu bar.
    pub fn pad(self, padding: UiRect) -> Self {
        struct PadKey;
        self.set_node::<PadKey, _>(padding, |node, padding| node.padding = padding)
    }

    /// Paint the container's background; a plain row, column or screen has no
    /// fill of its own, unlike a group or section.
    pub fn background(mut self, color: Color) -> Self {
        struct BackgroundKey;
        if self.key_changed::<BackgroundKey>(format!("{color:?}")) {
            self.e.entity_commands().insert(BackgroundColor(color));
        }
        self
    }
}
