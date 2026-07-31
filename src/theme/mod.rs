//! A framework for theming.
use bevy::app::{
    App, HierarchyPropagatePlugin, Inherited, Plugin, PostUpdate, PropagateOver, PropagateSet,
};
use bevy::color::{Alpha, Color, Oklcha};
use bevy::ecs::query::Or;
use bevy::ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    query::{Has, With, Without},
    reflect::ReflectResource,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Local, Query, Res},
};
use bevy::log::warn_once;
use bevy::platform::collections::HashMap;
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::text::{EditableText, TextColor, TextFont};
use bevy::ui::widget::Text;
use bevy::ui::{BackgroundColor, BackgroundGradient, BorderColor};
use rand::RngExt;

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

fn update_theme(
    mut q_background: Query<
        (
            &mut BackgroundColor,
            Option<&ThemeBackgroundToken>,
            Option<&ThemeBackgroundSlot>,
        ),
        Or<(With<ThemeBackgroundToken>, With<ThemeBackgroundSlot>)>,
    >,
    mut q_gradient: Query<(&mut BackgroundGradient, &ThemeBackgroundGradient)>,
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
        for (mut bg, theme_bg_token, theme_bg_slot) in q_background.iter_mut() {
            if let Some(theme_bg_slot) = theme_bg_slot {
                bg.0 = theme.palette(theme_bg_slot.0);
            } else if let Some(theme_bg_token) = theme_bg_token {
                bg.0 = theme.color(&theme_bg_token.0);
            }
        }

        for (mut gradient, theme_grad) in q_gradient.iter_mut() {
            *gradient = theme_background_gradient(theme.color(&theme_grad.0), theme_grad.1);
        }

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

// Every plume wrapper relays the inherited text styles, so themed text that still
// resolves no color or no font sits under a broken chain — the regression net for
// a wrapper the relay sweep missed, or an app subtree with no establishing surface.
fn warn_unstyled_themed_text(
    // `PropagateOver<C>` exempts relays, the `Inheritable*` sources, and pinned
    // text (direct color forms, raw-colored captions, icons, small caps, input
    // fields) — all of which style their own text.
    q_no_color: Query<
        (),
        (
            With<ThemedText>,
            Without<Inherited<TextColor>>,
            Without<PropagateOver<TextColor>>,
        ),
    >,
    q_no_font: Query<
        (),
        (
            With<ThemedText>,
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
            "Themed text resolved no color and falls back to white. Text must sit under a \
             surface that establishes the style, as `screen`, dialogs and popups do."
        );
    }
    *color_suspect_last_frame = no_color;

    let no_font = !q_no_font.is_empty();
    if no_font && *font_suspect_last_frame {
        warn_once!(
            "Themed text resolved no font and falls back to the engine default. Text must sit \
             under a surface that establishes the style, as `screen`, dialogs and popups do."
        );
    }
    *font_suspect_last_frame = no_font;
}

/// Installs the [`UiTheme`] resource, the theme refresh system, the themed
/// text-color propagation, and the token-change observers.
pub(crate) struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiTheme>()
            .add_plugins(HierarchyPropagatePlugin::<TextColor, With<ThemedText>>::new(PostUpdate))
            .add_systems(PostUpdate, update_theme)
            // After propagation, so text parented this frame has had its chance.
            .add_systems(
                PostUpdate,
                warn_unstyled_themed_text
                    .after(PropagateSet::<TextColor>::default())
                    .after(PropagateSet::<TextFont>::default()),
            )
            .add_observer(on_changed_background_token)
            .add_observer(on_changed_background_slot)
            .add_observer(on_changed_gradient)
            .add_observer(on_changed_border_token)
            .add_observer(on_changed_border_slot)
            .add_observer(on_changed_inheritable_text_token)
            .add_observer(on_changed_inheritable_text_slot)
            .add_observer(on_changed_inheritable_text_color)
            .add_observer(on_changed_text_token)
            .add_observer(on_changed_text_slot)
            .add_observer(on_themed_text_inserted::<TextColor>);
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

/// Represents a set of [`Oklcha`] colors which have same hue and chroma but different lightnesses
#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct OklchaArray<const N: usize> {
    /// Hue of the colors
    pub hue: f32,
    /// Chroma of the colors
    pub chroma: f32,
    /// N lightness values
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

/// The theme's parametric palette.
#[derive(Clone, Debug, PartialEq, Reflect)]
pub struct ThemeEditablePalette {
    /// Neutral ramp; forms [`ThemeSlot::Neutral0`]..=[`ThemeSlot::Neutral6`].
    pub neutrals: OklchaArray<7>,

    /// Accent ramp; forms [`ThemeSlot::Accent0`]..=[`ThemeSlot::Accent3`]
    /// (and [`ThemeSlot::FocusRing`], derived from `accent[0]`).
    pub accent: OklchaArray<4>,

    /// Foreground on accent-filled components (white in most themes); forms [`ThemeSlot::Contrast`].
    pub contrast: Oklcha,

    /// Text ramp; forms [`ThemeSlot::Text0`]..=[`ThemeSlot::Text1`]
    /// (and [`ThemeSlot::TextDisabled0`]..=[`ThemeSlot::TextDisabled1`], derived).
    pub text: OklchaArray<2>,

    /// Alpha applied to `text` to derive [`ThemeSlot::TextDisabled0`]..=[`ThemeSlot::TextDisabled1`].
    pub disabled_text_alpha_modifier: f32,

    /// RGB axis colors; form [`ThemeSlot::XAxis`], [`ThemeSlot::YAxis`], [`ThemeSlot::ZAxis`].
    pub axes: [Oklcha; 3],
}

impl ThemeEditablePalette {
    /// A palette on a random hue, usually dark and sometimes complementary-neutral.
    pub fn random() -> Self {
        let hue = rand::rng().random_range(0.0..360.0);
        let complementary_neutral = rand::rng().random_bool(0.5);
        let dark = rand::rng().random_bool(0.666);
        if dark {
            dark_theme::dark_palette(hue, complementary_neutral)
        } else {
            light_theme::light_palette(hue, complementary_neutral)
        }
    }

    // Bake the parametric palette into one resolved color per `ThemeSlot`.
    //
    // The `copy_from_slice` blocks below rely on each ramp's variants being
    // contiguous and in order within `ThemeSlot` (Neutral0..=Neutral6, etc.).
    pub(crate) fn resolve(&self) -> ThemeResolvedPalette {
        let neutral = self.neutrals.to_array();
        let accent = self.accent.to_array();
        let text = self.text.to_array();
        let text_dim = text.map(|c| c.with_alpha(self.disabled_text_alpha_modifier));
        let axes: [Color; 3] = self.axes.map(Into::into);

        let mut c = [Color::NONE; ThemeSlot::COUNT];
        c[ThemeSlot::Neutral0 as usize..=ThemeSlot::Neutral6 as usize].copy_from_slice(&neutral);
        c[ThemeSlot::Text0 as usize..=ThemeSlot::Text1 as usize].copy_from_slice(&text);
        c[ThemeSlot::TextDisabled0 as usize..=ThemeSlot::TextDisabled1 as usize]
            .copy_from_slice(&text_dim);
        c[ThemeSlot::Accent0 as usize..=ThemeSlot::Accent3 as usize].copy_from_slice(&accent);
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
    /// Text ramp stop `index`.
    pub fn text(&self, index: usize) -> Color {
        self.text.to_color(index)
    }
    /// Text stop `index` with the disabled-alpha modifier applied.
    pub fn text_dim(&self, index: usize) -> Color {
        self.text(index)
            .with_alpha(self.disabled_text_alpha_modifier)
    }
    /// Axis color `index`, in X, Y, Z order.
    pub fn axis(&self, index: usize) -> Color {
        self.axes[index].into()
    }
}

/// Get the 3 default axis colors (RGB)
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

pub use editor::theme_editor;
pub use slots::ThemeSlot;

pub(crate) use components::*;

/// The built-in parametric palettes, ready to hand to [`UiTheme::from`].
pub mod palettes {
    pub use super::dark_theme::{dark_palette, default_dark_palette};
    pub use super::light_theme::{default_light_palette, light_palette};
}
