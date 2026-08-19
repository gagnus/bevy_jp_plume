//! A framework for theming.
use bevy::app::{
    App, HierarchyPropagatePlugin, Inherited, Plugin, PostUpdate, PropagateOver, PropagateSet,
};
use bevy::color::{Alpha, Color, Oklcha};
use bevy::ecs::change_detection::DetectChanges;
use bevy::ecs::entity::Entity;
use bevy::ecs::query::{Has, Or, With, Without};
use bevy::ecs::reflect::ReflectResource;
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

use crate::imm::PlumeCaps;
use crate::tokens::ThemeToken;

/// The currently selected user interface theme. Overwriting this resource changes the theme.
#[derive(Resource, Reflect, Debug)]
#[reflect(Resource, Default, Debug)]
pub struct UiTheme {
    tokens: HashMap<ThemeToken, ThemeSlot>,
    resolved: ThemeResolvedPalette,
    editable: ThemeEditablePalette,
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
            resolved: value.resolve(),
            editable: value,
        }
    }
}

impl UiTheme {
    /// The resolved color of a palette slot — how an app reads a theme color.
    pub fn palette(&self, slot: ThemeSlot) -> Color {
        self.resolved[slot]
    }

    /// The parametric palette this theme was generated from, for an editor to show.
    pub fn editable(&self) -> &ThemeEditablePalette {
        &self.editable
    }

    // Lookup a color by design token (tokens are plume-internal). If the theme does
    // not have an entry for that token, logs a warning and returns an error color.
    pub(crate) fn color(&self, token: &ThemeToken) -> Color {
        let color = self.tokens.get(token).map(|slot| self.resolved[*slot]);
        match color {
            Some(c) => c,
            None => {
                warn_once!("Theme color {} not found.", token);
                // Return a bright obnoxious color to make the error obvious.
                bevy::color::palettes::basic::FUCHSIA.into()
            }
        }
    }

    /// Re-resolve every slot color from an editable palette.
    pub fn set_palette(&mut self, palette: &ThemeEditablePalette) {
        self.resolved = palette.resolve();
        self.editable = palette.clone();
    }
}

// Themed backgrounds are refreshed by `resolve_backgrounds`, which owns the
// gradient-or-flat decision and so has to handle the palette swap itself.
fn update_theme(
    mut q_border: Query<
        (
            &mut BorderColor,
            Option<&ThemeBorderToken>,
            Option<&ThemeBorderSlot>,
        ),
        Or<(With<ThemeBorderToken>, With<ThemeBorderSlot>)>,
    >,
    mut q_text_color: Query<
        (
            &mut TextColor,
            Option<&ThemeTextToken>,
            Option<&ThemeTextSlot>,
        ),
        Or<(With<ThemeTextToken>, With<ThemeTextSlot>)>,
    >,
    q_inherit: Query<
        (
            Entity,
            Option<&InheritableThemeTextToken>,
            Option<&InheritableThemeTextSlot>,
            (Has<Text>, Has<EditableText>),
            Has<ThemeTextSlot>,
            Has<ThemeTextToken>,
        ),
        Or<(
            With<InheritableThemeTextToken>,
            With<InheritableThemeTextSlot>,
        )>,
    >,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    if theme.is_changed() {
        for (mut border, theme_border_token, theme_border_slot) in q_border.iter_mut() {
            if let Some(theme_border_slot) = theme_border_slot {
                border.set_all(theme.palette(theme_border_slot.0));
            } else if let Some(theme_border_token) = theme_border_token {
                border.set_all(theme.color(&theme_border_token.0));
            }
        }

        for (mut text_color, theme_text_token, theme_text_slot) in q_text_color.iter_mut() {
            if let Some(theme_text_slot) = theme_text_slot {
                text_color.0 = theme.palette(theme_text_slot.0);
            } else if let Some(theme_text_token) = theme_text_token {
                text_color.0 = theme.color(&theme_text_token.0);
            }
        }

        for (
            entity,
            inherit_token,
            inherit_slot,
            (has_text, has_editable),
            has_direct_slot,
            has_direct_token,
        ) in &q_inherit
        {
            let color = if let Some(inherit_slot) = inherit_slot {
                theme.palette(inherit_slot.0)
            } else if let Some(inherit_token) = inherit_token {
                theme.color(&inherit_token.0)
            } else {
                continue;
            };
            // Same self-write rule as the insert observers; the direct forms
            // were refreshed by the loops above.
            apply_inheritable_color(
                &mut commands,
                entity,
                color,
                (has_text || has_editable) && !has_direct_slot && !has_direct_token,
            );
        }
    }
}

// Text styles reach every descendant of an establishing surface, so text that
// resolves no color or no font sits under no surface at all — the regression
// net for an app subtree mounted outside `screen`, a dialog or a popup.
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
             that establishes the style, as `screen`, dialogs and popups do; text styled by \
             hand opts out with `PropagateOver<TextColor>`."
        );
    }
    *color_suspect_last_frame = no_color;

    let no_font = !q_no_font.is_empty();
    if no_font && *font_suspect_last_frame {
        warn_once!(
            "Text resolved no font and falls back to the engine default. Text must sit under \
             a surface that establishes the style, as `screen`, dialogs and popups do; text \
             styled by hand opts out with `PropagateOver<TextFont>`."
        );
    }
    *font_suspect_last_frame = no_font;
}

// Installs the [`UiTheme`] resource, the theme refresh system, both themed
// text-style propagation channels, and the token-change observers.
pub(crate) struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        // Both text-style channels, unfiltered: styles reach every descendant, and
        // self-styled text opts out with `PropagateOver`. Landing a real `TextFont`
        // on every node also lets bevy's `sync_font_size_to_em_size` keep `EmSize`
        // current everywhere, so `Val::Em` chrome needs no plume bookkeeping.
        app.init_resource::<UiTheme>()
            .add_plugins((
                HierarchyPropagatePlugin::<TextColor>::new(PostUpdate),
                HierarchyPropagatePlugin::<TextFont>::new(PostUpdate),
            ))
            // Fonts must be current before `measure_text_system` and
            // `detect_text_needs_rerender` run in `UiSystems::Content`.
            .configure_sets(
                PostUpdate,
                PropagateSet::<TextFont>::default().in_set(UiSystems::Propagate),
            )
            // After the imm reconciler's cleanup despawns, so the repaint commands
            // `resolve_backgrounds` queues can't target an entity whose despawn is
            // already in an earlier buffer at the same sync point — the ordering
            // edge inserts a sync point that applies those despawns first.
            .add_systems(
                PostUpdate,
                (update_theme, resolve_backgrounds)
                    .after(ImmediateSystemSet::<PlumeCaps>::default()),
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
            let dark = rand::rng().random_bool(0.666);
            let palette = if dark {
                dark_theme::dark_palette(hue, complementary_neutral)
            } else {
                light_theme::light_palette(hue, complementary_neutral)
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

pub(crate) use components::*;
pub use editor::theme_editor;
pub use slots::ThemeSlot;

/// The built-in parametric palettes, ready to hand to [`UiTheme::from`].
pub mod palettes {
    pub use super::dark_theme::{dark_palette, default_dark_palette};
    pub use super::light_theme::{default_light_palette, light_palette};
}
