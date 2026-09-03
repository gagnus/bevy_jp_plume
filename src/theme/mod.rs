//! A framework for theming.
use alloc::collections::BTreeMap;

use bevy::app::{
    App, HierarchyPropagatePlugin, Inherited, Plugin, PostUpdate, PropagateOver, PropagateSet,
};
use bevy::color::{Alpha, Color, Oklcha};
use bevy::ecs::change_detection::DetectChanges;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::query::{Changed, Has, Or, With, Without};
use bevy::ecs::reflect::{ReflectComponent, ReflectResource};
use bevy::ecs::resource::Resource;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::system::{Commands, Local, Query, Res};
use bevy::log::warn_once;
use bevy::platform::collections::HashMap;
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::text::{EditableText, TextColor, TextFont};
use bevy::ui::widget::Text;
use bevy::ui::{BorderColor, UiSystems};
use bevy_immediate::ImmediateSystemSet;
use rand::RngExt;
use smol_str::SmolStr;

use crate::imm::PlumeCaps;
use crate::tokens::ThemeToken;

/// Names a [`UiTheme`] palette and, on an entity, the theme it renders with.
/// Propagates from a `Propagate(ThemeId)` source; no source means the default.
// Small strings stay inline up to 23 bytes, so propagation's per-node clone
// never allocates for short names.
#[derive(Component, Reflect, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
#[reflect(Component, Default, Debug)]
pub struct ThemeId(SmolStr);

impl ThemeId {
    /// The id named `name`; the empty string is the default theme's id.
    pub fn new(name: impl Into<SmolStr>) -> Self {
        Self(name.into())
    }

    /// The id's name; empty for the default theme.
    pub fn name(&self) -> &str {
        &self.0
    }
}

// One theme's baked slot colors plus the parametric palette they came from.
#[derive(Clone, Debug, Reflect)]
struct ThemeEntry {
    resolved: ThemeResolvedPalette,
    editable: ThemeEditablePalette,
}

impl From<ThemeEditablePalette> for ThemeEntry {
    fn from(palette: ThemeEditablePalette) -> Self {
        Self {
            resolved: palette.resolve(),
            editable: palette,
        }
    }
}

/// The user interface themes, one palette per [`ThemeId`]. Overwriting a
/// palette via [`Self::set_palette`] restyles every entity on that theme.
#[derive(Resource, Reflect, Debug)]
#[reflect(Resource, Default, Debug)]
pub struct UiTheme {
    tokens: HashMap<ThemeToken, ThemeSlot>,
    themes: BTreeMap<ThemeId, ThemeEntry>,
}

impl Default for UiTheme {
    fn default() -> Self {
        dark_theme::default_dark_palette().into()
    }
}

impl From<ThemeEditablePalette> for UiTheme {
    fn from(value: ThemeEditablePalette) -> Self {
        Self {
            tokens: slots::DEFAULT_TOKEN_SLOTS.iter().cloned().collect(),
            themes: BTreeMap::from([(ThemeId::default(), ThemeEntry::from(value))]),
        }
    }
}

impl UiTheme {
    // The entry for `id`: `None` and unregistered ids (which warn) fall back
    // to the default theme.
    fn entry(&self, id: Option<&ThemeId>) -> &ThemeEntry {
        if let Some(id) = id {
            match self.themes.get(id) {
                Some(entry) => return entry,
                None => warn_once!(
                    "Theme \"{}\" has no palette; using the default theme.",
                    id.name()
                ),
            }
        }
        self.themes
            .get(&ThemeId::default())
            .expect("the default theme always has a palette")
    }

    /// The resolved color of a palette slot in theme `id`. `None` is the
    /// default theme.
    pub fn slot_color(&self, id: Option<&ThemeId>, slot: ThemeSlot) -> Color {
        self.entry(id).resolved[slot]
    }

    /// The parametric palette theme `id` was generated from, for an editor to show.
    pub fn editable_palette(&self, id: Option<&ThemeId>) -> &ThemeEditablePalette {
        &self.entry(id).editable
    }

    // Lookup a color by design token (tokens are plume-internal). If the theme does
    // not have an entry for that token, logs a warning and returns an error color.
    pub(crate) fn token_color(&self, id: Option<&ThemeId>, token: &ThemeToken) -> Color {
        match self.tokens.get(token) {
            Some(slot) => self.entry(id).resolved[*slot],
            None => {
                warn_once!("Theme color {} not found.", token);
                // Return a bright obnoxious color to make the error obvious.
                bevy::color::palettes::basic::FUCHSIA.into()
            }
        }
    }

    /// Set or replace theme `id`'s palette, re-resolving every slot color.
    pub fn set_palette(&mut self, id: ThemeId, palette: ThemeEditablePalette) {
        self.themes.insert(id, ThemeEntry::from(palette));
    }

    /// Drop theme `id`'s palette; entities on it fall back to the default
    /// theme, which cannot be removed.
    pub fn remove_palette(&mut self, id: &ThemeId) {
        if *id != ThemeId::default() {
            self.themes.remove(id);
        }
    }

    /// The registered theme ids, the default theme first.
    pub fn theme_ids(&self) -> impl Iterator<Item = &ThemeId> {
        self.themes.keys()
    }
}

// Recolors themed borders and text: everything on a palette change, per entity
// on a `ThemeId` change. Backgrounds are `resolve_backgrounds`' job — it owns
// the gradient-or-flat decision.
fn update_theme(
    mut q_border: Query<
        (
            &mut BorderColor,
            Option<&ThemeBorderToken>,
            Option<&ThemeBorderSlot>,
            Option<&ThemeId>,
        ),
        Or<(With<ThemeBorderToken>, With<ThemeBorderSlot>)>,
    >,
    mut q_text_color: Query<
        (
            &mut TextColor,
            Option<&ThemeTextToken>,
            Option<&ThemeTextSlot>,
            Option<&ThemeId>,
        ),
        Or<(With<ThemeTextToken>, With<ThemeTextSlot>)>,
    >,
    q_inherit: Query<
        (
            Entity,
            Option<&InheritableThemeTextToken>,
            Option<&InheritableThemeTextSlot>,
            Option<&ThemeId>,
            (Has<Text>, Has<EditableText>),
            Has<ThemeTextSlot>,
            Has<ThemeTextToken>,
        ),
        Or<(
            With<InheritableThemeTextToken>,
            With<InheritableThemeTextSlot>,
        )>,
    >,
    q_id_changed: Query<Entity, Changed<ThemeId>>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    fn apply_border(
        theme: &UiTheme,
        (mut border, token, slot, id): (
            bevy::ecs::change_detection::Mut<BorderColor>,
            Option<&ThemeBorderToken>,
            Option<&ThemeBorderSlot>,
            Option<&ThemeId>,
        ),
    ) {
        if let Some(slot) = slot {
            border.set_all(theme.slot_color(id, slot.0));
        } else if let Some(token) = token {
            border.set_all(theme.token_color(id, &token.0));
        }
    }

    fn apply_text(
        theme: &UiTheme,
        (mut text_color, token, slot, id): (
            bevy::ecs::change_detection::Mut<TextColor>,
            Option<&ThemeTextToken>,
            Option<&ThemeTextSlot>,
            Option<&ThemeId>,
        ),
    ) {
        if let Some(slot) = slot {
            text_color.0 = theme.slot_color(id, slot.0);
        } else if let Some(token) = token {
            text_color.0 = theme.token_color(id, &token.0);
        }
    }

    fn apply_inherit(
        theme: &UiTheme,
        commands: &mut Commands,
        (
            entity,
            inherit_token,
            inherit_slot,
            id,
            (has_text, has_editable),
            has_direct_slot,
            has_direct_token,
        ): (
            Entity,
            Option<&InheritableThemeTextToken>,
            Option<&InheritableThemeTextSlot>,
            Option<&ThemeId>,
            (bool, bool),
            bool,
            bool,
        ),
    ) {
        let color = if let Some(inherit_slot) = inherit_slot {
            theme.slot_color(id, inherit_slot.0)
        } else if let Some(inherit_token) = inherit_token {
            theme.token_color(id, &inherit_token.0)
        } else {
            return;
        };
        // Same self-write rule as the insert observers; the direct forms are
        // refreshed by the border/text sweeps.
        apply_inheritable_color(
            commands,
            entity,
            color,
            (has_text || has_editable) && !has_direct_slot && !has_direct_token,
        );
    }

    if theme.is_changed() {
        for row in q_border.iter_mut() {
            apply_border(&theme, row);
        }
        for row in q_text_color.iter_mut() {
            apply_text(&theme, row);
        }
        for row in &q_inherit {
            apply_inherit(&theme, &mut commands, row);
        }
        return;
    }

    // A `ThemeId` that landed or changed this frame re-resolves just its entity —
    // including the frame a themed subtree spawns, before its first paint.
    for entity in &q_id_changed {
        if let Ok(row) = q_border.get_mut(entity) {
            apply_border(&theme, row);
        }
        if let Ok(row) = q_text_color.get_mut(entity) {
            apply_text(&theme, row);
        }
        if let Ok(row) = q_inherit.get(entity) {
            apply_inherit(&theme, &mut commands, row);
        }
    }
}

// Regression net: text that resolves no color or font sits under no
// establishing surface (`screen`, a dialog, a popup).
fn warn_unstyled_text(
    // `PropagateOver<C>` exempts self-styled text (the direct color forms and
    // the `Inheritable*` sources on text carriers).
    q_no_color: Query<
        (),
        (
            Or<(With<Text>, With<EditableText>)>,
            Without<Inherited<TextColor>>,
            Without<PropagateOver<TextColor>>,
        ),
    >,
    q_no_font: Query<
        (),
        (
            Or<(With<Text>, With<EditableText>)>,
            Without<Inherited<TextFont>>,
            Without<PropagateOver<TextFont>>,
        ),
    >,
    mut color_suspect_last_frame: Local<bool>,
    mut font_suspect_last_frame: Local<bool>,
) {
    // Propagation lands the frame after an entity is parented, so text unresolved
    // this frame may still be in flight.
    let no_color = !q_no_color.is_empty();
    if no_color && *color_suspect_last_frame {
        warn_once!(
            "Text resolved no color and falls back to white. Text must sit under a surface \
             that establishes the style, as `screen`, dialogs and popups do; opt out with \
             `PlumeIgnore`."
        );
    }
    *color_suspect_last_frame = no_color;

    let no_font = !q_no_font.is_empty();
    if no_font && *font_suspect_last_frame {
        warn_once!(
            "Text resolved no font and falls back to the engine default. Text must sit under \
             a surface that establishes the style, as `screen`, dialogs and popups do; opt out \
             with `PlumeIgnore`."
        );
    }
    *font_suspect_last_frame = no_font;
}

// Installs [`UiTheme`], the style and `ThemeId` propagation channels, the
// refresh systems, and the token-change observers.
pub(crate) struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        // Unfiltered channels: styles reach every descendant, self-styled text
        // opts out with `PropagateOver`, and the per-node `TextFont` feeds
        // bevy's `EmSize` sync, so `Val::Em` chrome just works.
        app.init_resource::<UiTheme>()
            .add_plugins((
                HierarchyPropagatePlugin::<TextColor>::new(PostUpdate),
                HierarchyPropagatePlugin::<TextFont>::new(PostUpdate),
                HierarchyPropagatePlugin::<ThemeId>::new(PostUpdate),
            ))
            // Fonts must be current before `measure_text_system` and
            // `detect_text_needs_rerender` run in `UiSystems::Content`.
            .configure_sets(
                PostUpdate,
                PropagateSet::<TextFont>::default().in_set(UiSystems::Propagate),
            )
            // Ids the imm build inserts this frame propagate before the theme
            // systems read them: a themed subtree resolves in its spawn frame.
            .configure_sets(
                PostUpdate,
                PropagateSet::<ThemeId>::default()
                    .after(ImmediateSystemSet::<PlumeCaps>::default()),
            )
            // After the imm reconciler, so its cleanup despawns apply before the
            // repaint commands queued here could target a dead entity.
            .add_systems(
                PostUpdate,
                (update_theme, resolve_backgrounds)
                    .after(ImmediateSystemSet::<PlumeCaps>::default())
                    .after(PropagateSet::<ThemeId>::default()),
            )
            // After propagation, so text parented this frame has had its chance.
            .add_systems(
                PostUpdate,
                warn_unstyled_text
                    .after(PropagateSet::<TextColor>::default())
                    .after(PropagateSet::<TextFont>::default()),
            )
            .add_observer(on_changed_border_token)
            .add_observer(on_changed_border_slot)
            .add_observer(on_changed_inheritable_text_token)
            .add_observer(on_changed_inheritable_text_slot)
            .add_observer(on_changed_inheritable_text_color)
            .add_observer(on_changed_text_token)
            .add_observer(on_changed_text_slot);
    }
}

// One `Color` per `ThemeSlot`, baked from a `ThemeEditablePalette`.
#[derive(Clone, Debug, Default, Reflect)]
pub(crate) struct ThemeResolvedPalette([Color; ThemeSlot::COUNT]);

impl core::ops::Index<ThemeSlot> for ThemeResolvedPalette {
    type Output = Color;
    fn index(&self, slot: ThemeSlot) -> &Color {
        &self.0[slot as usize]
    }
}

/// Represents a set of [`Oklcha`] colors which have same hue and chroma but different lightnesses.
#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct OklchaArray<const N: usize> {
    /// Hue of the colors.
    pub hue: f32,
    /// Chroma of the colors.
    pub chroma: f32,
    /// N lightness values.
    pub l: [f32; N],
}

impl<const N: usize> OklchaArray<N> {
    fn to_color(&self, index: usize) -> Color {
        Color::oklcha(self.l[index], self.chroma, self.hue, 1.0)
    }
    fn to_array(&self) -> [Color; N] {
        core::array::from_fn(|index| self.to_color(index))
    }
}

/// Hue of the danger ramp, pinned across every palette so a destructive action is
/// always the same red — near the sRGB red primary's Oklch hue.
pub const DANGER_HUE: f32 = 29.0;

/// Chroma of the danger ramp.
pub const DANGER_CHROMA: f32 = 0.20;

/// The theme's parametric palette.
#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct ThemeEditablePalette {
    /// Neutral ramp; forms [`ThemeSlot::Neutral0`]..=[`ThemeSlot::Neutral6`].
    pub neutrals: OklchaArray<7>,

    /// Accent ramp; forms [`ThemeSlot::Accent0`]..=[`ThemeSlot::Accent3`]
    /// (and [`ThemeSlot::FocusRing`], derived from `accent[0]`, plus
    /// [`ThemeSlot::Danger0`]..=[`ThemeSlot::Danger2`], which borrow its first three
    /// lightnesses).
    pub accent: OklchaArray<4>,

    /// Foreground on accent-filled components (white in most themes); forms [`ThemeSlot::Contrast`].
    pub contrast: Oklcha,

    /// Text ramp; forms [`ThemeSlot::Text0`]..=[`ThemeSlot::Text1`].
    pub text: OklchaArray<2>,

    /// Disabled ramp; forms [`ThemeSlot::Disabled0`]..=[`ThemeSlot::Disabled1`]:
    /// text over a disabled fill, then the fill/chrome tone itself.
    pub disabled: OklchaArray<2>,

    /// RGB axis colors; form [`ThemeSlot::XAxis`], [`ThemeSlot::YAxis`], [`ThemeSlot::ZAxis`].
    pub axes: [Oklcha; 3],
}

impl ThemeEditablePalette {
    /// A palette on a random hue, usually dark and sometimes complementary-neutral.
    pub fn random() -> Self {
        loop {
            let hue = rand::rng().random_range(0.0..360.0);
            let complementary_neutral = rand::rng().random_bool(0.5);
            let boosted_neutral_chroma = rand::rng().random_bool(0.25);
            let dark = rand::rng().random_bool(0.666);
            let palette = if dark {
                dark_theme::dark_palette(hue, complementary_neutral, boosted_neutral_chroma)
            } else {
                light_theme::light_palette(hue, complementary_neutral, boosted_neutral_chroma)
            };
            if !palette.is_accent_close_to_danger() {
                return palette;
            }
        }
    }

    pub(crate) fn is_accent_close_to_danger(&self) -> bool {
        (self.accent.hue - DANGER_HUE).abs() < 15.
            && (self.accent.chroma - DANGER_CHROMA).abs() < 0.05
    }

    // Bake the parametric palette into one resolved color per `ThemeSlot`.
    //
    // The `copy_from_slice` blocks below rely on each ramp's variants being
    // contiguous and in order within `ThemeSlot` (Neutral0..=Neutral6, etc.).
    pub(crate) fn resolve(&self) -> ThemeResolvedPalette {
        let neutral = self.neutrals.to_array();
        let accent = self.accent.to_array();
        let danger = self.danger_ramp().to_array();
        let text = self.text.to_array();
        let disabled = self.disabled.to_array();
        let axes: [Color; 3] = self.axes.map(Into::into);

        let mut c = [Color::NONE; ThemeSlot::COUNT];
        c[ThemeSlot::Neutral0 as usize..=ThemeSlot::Neutral6 as usize].copy_from_slice(&neutral);
        c[ThemeSlot::Text0 as usize..=ThemeSlot::Text1 as usize].copy_from_slice(&text);
        c[ThemeSlot::Disabled0 as usize..=ThemeSlot::Disabled1 as usize].copy_from_slice(&disabled);
        c[ThemeSlot::Accent0 as usize..=ThemeSlot::Accent3 as usize].copy_from_slice(&accent);
        c[ThemeSlot::Danger0 as usize..=ThemeSlot::Danger2 as usize].copy_from_slice(&danger);
        c[ThemeSlot::Contrast as usize] = self.contrast.into();
        c[ThemeSlot::FocusRing as usize] = accent[0].with_alpha(0.5);
        c[ThemeSlot::XAxis as usize..=ThemeSlot::ZAxis as usize].copy_from_slice(&axes);
        // ThemeSlot::Transparent stays Color::NONE.
        ThemeResolvedPalette(c)
    }

    /// Neutral ramp stop `index`.
    pub fn neutral(&self, index: usize) -> Color {
        self.neutrals.to_color(index)
    }
    /// Accent ramp stop `index`.
    pub fn accent(&self, index: usize) -> Color {
        self.accent.to_color(index)
    }
    /// Danger ramp stop `index`.
    pub fn danger(&self, index: usize) -> Color {
        self.danger_ramp().to_color(index)
    }
    /// Text ramp stop `index`.
    pub fn text(&self, index: usize) -> Color {
        self.text.to_color(index)
    }
    /// Disabled ramp stop `index`.
    pub fn disabled(&self, index: usize) -> Color {
        self.disabled.to_color(index)
    }
    /// Axis color `index`, in X, Y, Z order.
    pub fn axis(&self, index: usize) -> Color {
        self.axes[index].into()
    }

    // The danger ramp is not an editable input: only its lightnesses vary.
    fn danger_ramp(&self) -> OklchaArray<3> {
        OklchaArray {
            hue: DANGER_HUE,
            chroma: DANGER_CHROMA,
            l: [self.accent.l[0], self.accent.l[1], self.accent.l[2]],
        }
    }
}

/// Get the 3 default axis colors (RGB).
pub fn default_axis_colors() -> [Oklcha; 3] {
    [
        Oklcha::new(0.5232, 0.1404, 13.84, 1.0),
        Oklcha::new(0.5866, 0.1543, 129.84, 1.0),
        Oklcha::new(0.4847, 0.1249, 253.08, 1.0),
    ]
}

pub(crate) mod components;

mod dark_theme;
mod editor;
mod light_theme;
mod slots;

pub(crate) mod tokens;

pub use components::PlumeIgnore;
pub(crate) use components::*;
pub use editor::{theme_editor, theme_editor_tabs};
pub use slots::ThemeSlot;

/// The built-in parametric palettes, ready to hand to [`UiTheme::from`].
pub mod palettes {
    pub use super::dark_theme::{
        DEFAULT_DARK_COMPLEMENTARY_NEUTRAL, DEFAULT_DARK_HUE, dark_palette, default_dark_palette,
    };
    pub use super::light_theme::{
        DEFAULT_LIGHT_COMPLEMENTARY_NEUTRAL, DEFAULT_LIGHT_HUE, default_light_palette,
        light_palette,
    };
}
