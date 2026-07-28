//! A framework for theming.
use bevy::app::{
    App, HierarchyPropagatePlugin, Inherited, Plugin, PostUpdate, Propagate, PropagateOver,
    PropagateSet, PropagateStop,
};
use bevy::color::{Alpha, Color, Luminance, Oklcha, Srgba, palettes};
use bevy::ecs::query::Or;
use bevy::ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    hierarchy::ChildOf,
    lifecycle::Insert,
    observer::On,
    query::{Changed, With, Without},
    reflect::{ReflectComponent, ReflectResource},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Local, Query, Res},
};
use bevy::log::warn_once;
use bevy::platform::collections::HashMap;
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::text::TextColor;
use bevy::ui::{
    BackgroundColor, BackgroundGradient, BorderColor, BoxShadow, ColorStop, Gradient,
    InterpolationColorSpace, LinearGradient, Val, percent,
};
use rand::RngExt;
use smol_str::SmolStr;

use crate::constants::size;
use crate::tokens::ThemeToken;
use slots::ThemeSlot;

/// The currently selected user interface theme. Overwriting this resource changes the theme.
#[derive(Resource, Reflect, Debug)]
#[reflect(Resource, Default, Debug)]
pub struct UiTheme {
    /// Which palette slot each design token draws from.
    pub tokens: HashMap<ThemeToken, ThemeSlot>,
    /// The resolved color of every slot.
    pub palette: ThemeResolvedPalette,
    /// Stored in case an editor wants to see values
    /// used to generate the resolved palette...
    pub generated_from: ThemeEditablePalette,
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
            palette: value.resolve(),
            generated_from: value,
        }
    }
}

impl UiTheme {
    /// Lookup a color by design token. If the theme does not have an entry for that token,
    /// logs a warning and returns an error color.
    pub fn color(&self, token: &ThemeToken) -> Color {
        let color = self.tokens.get(token).map(|slot| self.palette[*slot]);
        match color {
            Some(c) => c,
            None => {
                warn_once!("Theme color {} not found.", token);
                // Return a bright obnoxious color to make the error obvious.
                palettes::basic::FUCHSIA.into()
            }
        }
    }

    /// Associate a design token with a given palette slot.
    pub fn set_token(&mut self, token: &str, slot: ThemeSlot) {
        self.tokens
            .insert(ThemeToken::new(SmolStr::new(token)), slot);
    }

    /// Re-resolve every slot color from an editable palette.
    pub fn set_palette(&mut self, palette: &ThemeEditablePalette) {
        self.palette = palette.resolve();
    }
}

/// Component which causes the background color of an entity to be set based on a theme color.
#[derive(Component, Clone, Default)]
#[require(BackgroundColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBackgroundColor(pub ThemeToken);

/// Component which causes the background color of an entity to be set based on a theme slot.
/// Internal use `ThemeBackgroundColor` instead, external prefer this, this takes priority over `ThemeBackgroundColor`.
#[derive(Component, Clone, Default)]
#[require(BackgroundColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBackgroundSlot(pub ThemeSlot);

/// The standard luminance adjust (+-) for an active control's [`ThemeBackgroundGradient`].
pub const GRADIENT_AMOUNT: f32 = 0.05;

/// Opt-in marker: the entity's themed fills render flat (gradient amount 0).
/// Honored by the gradient-drawn elements (button, checkbox, radio, toggle,
/// slider thumb, section header); everything else ignores it.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct Flat;

/// Opt-in marker: the entity does not repond to hover and pressed. Currently only
/// respected by `PlumeButton`. Implied by [`bevy::ui::Checkable`].
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct Inert;

/// Component which fills an entity's background with a gentle top-to-bottom gradient derived
/// from a theme color.
#[derive(Component, Clone, Default)]
#[require(BackgroundGradient)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBackgroundGradient(pub ThemeToken, pub f32);

/// Build the vertical gradient a [`ThemeBackgroundGradient`] resolves to.
fn theme_background_gradient(base: Color, amount: f32) -> BackgroundGradient {
    BackgroundGradient(vec![Gradient::Linear(LinearGradient {
        angle: LinearGradient::TO_BOTTOM,
        stops: vec![
            ColorStop::new(base.lighter(amount), percent(0)),
            ColorStop::new(base.darker(amount), percent(100)),
        ],
        color_space: InterpolationColorSpace::LinearRgba,
    })])
}

pub fn control_box_shadow() -> BoxShadow {
    BoxShadow::new(
        Srgba::BLACK.with_alpha(0.4).into(),
        size::GAP / 8.0,
        size::GAP / 4.0,
        Val::ZERO,
        size::GAP / 2.0,
    )
}

/// Component which causes the border color of an entity to be set based on a theme color.
/// Only supports setting all borders to the same color.
#[derive(Component, Clone, Default)]
#[require(BorderColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBorderColor(pub ThemeToken);

/// Component which causes the border color of an entity to be set based on a theme slot.
/// Internal use `ThemeBorderColor` instead, external prefer this, this takes priority over `ThemeBorderColor`.
#[derive(Component, Clone, Default)]
#[require(BorderColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBorderSlot(pub ThemeSlot);

/// Component which causes the inherited text color of an entity to be set based on a theme color.
#[derive(Component, Clone, Default)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
#[require(ThemedText, PropagateOver::<TextColor>)]
pub struct InheritableThemeTextColor(pub ThemeToken);

/// Component which causes the color of a text span to be set based on a theme color. Unlike
/// [`InheritableThemeTextColor`], this can work when set directly on the text span entity, and is
/// not inherited.
// TODO: This is necessary because an entity with Propagate doesn't update itself, only its
// descendants.
#[derive(Component, Clone, Default)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
#[require(ThemedText, PropagateOver::<TextColor>)]
pub struct ThemeTextColor(pub ThemeToken);

/// A marker component that is used to indicate that the text entity wants to opt-in to using
/// inherited text styles.
#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
pub struct ThemedText;

/// Finish propagating `C` to a child that gained [`ThemedText`] after it was
/// parented.
///
/// Propagation copies `Inherited<C>` onto a new child when its `ChildOf` lands, but
/// only onto children already matching the `With<ThemedText>` filter. The imm
/// reconciler parents an entity before its scene applies the marker, so that copy is
/// missed and the text keeps the engine default font/color; this repeats it from the
/// other side. Entities whose parent's `Inherited<C>` is itself new are covered by
/// the ordinary downward pass.
pub(crate) fn on_themed_text_inserted<C: Component + Clone + PartialEq>(
    insert: On<Insert, ThemedText>,
    q_unresolved: Query<&ChildOf, (Without<Propagate<C>>, Without<Inherited<C>>)>,
    q_inherited: Query<&Inherited<C>, Without<PropagateStop<C>>>,
    mut commands: Commands,
) {
    let Ok(child_of) = q_unresolved.get(insert.entity) else {
        return;
    };
    if let Ok(inherited) = q_inherited.get(child_of.parent()) {
        commands.entity(insert.entity).insert(inherited.clone());
    }
}

fn update_theme(
    mut q_background: Query<
        (
            &mut BackgroundColor,
            Option<&ThemeBackgroundColor>,
            Option<&ThemeBackgroundSlot>,
        ),
        Or<(With<ThemeBackgroundColor>, With<ThemeBackgroundSlot>)>,
    >,
    mut q_gradient: Query<(&mut BackgroundGradient, &ThemeBackgroundGradient)>,
    mut q_border: Query<
        (
            &mut BorderColor,
            Option<&ThemeBorderColor>,
            Option<&ThemeBorderSlot>,
        ),
        Or<(With<ThemeBorderColor>, With<ThemeBorderSlot>)>,
    >,
    mut q_text_color: Query<(&mut TextColor, &ThemeTextColor)>,
    q_inherit: Query<(Entity, &InheritableThemeTextColor)>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    if theme.is_changed() {
        for (mut bg, theme_bg_token, theme_bg_slot) in q_background.iter_mut() {
            if let Some(theme_bg_slot) = theme_bg_slot {
                bg.0 = theme.palette[theme_bg_slot.0];
            } else if let Some(theme_bg_token) = theme_bg_token {
                bg.0 = theme.color(&theme_bg_token.0);
            }
        }

        for (mut gradient, theme_grad) in q_gradient.iter_mut() {
            *gradient = theme_background_gradient(theme.color(&theme_grad.0), theme_grad.1);
        }

        for (mut border, theme_border_token, theme_border_slot) in q_border.iter_mut() {
            if let Some(theme_border_slot) = theme_border_slot {
                border.set_all(theme.palette[theme_border_slot.0]);
            } else if let Some(theme_border_token) = theme_border_token {
                border.set_all(theme.color(&theme_border_token.0));
            }
        }

        for (mut text_color, theme_text_color) in q_text_color.iter_mut() {
            text_color.0 = theme.color(&theme_text_color.0);
        }

        for (entity, inherit) in &q_inherit {
            commands
                .entity(entity)
                .insert(Propagate(TextColor(theme.color(&inherit.0))));
        }
    }
}

fn on_changed_background_token(
    insert: On<Insert, ThemeBackgroundColor>,
    mut q_background: Query<
        (&mut BackgroundColor, &ThemeBackgroundColor),
        Changed<ThemeBackgroundColor>,
    >,
    theme: Res<UiTheme>,
) {
    if let Ok((mut bg, theme_bg)) = q_background.get_mut(insert.entity) {
        bg.0 = theme.color(&theme_bg.0);
    }
}

fn on_changed_background_slot(
    insert: On<Insert, ThemeBackgroundSlot>,
    mut q_background: Query<
        (&mut BackgroundColor, &ThemeBackgroundSlot),
        Changed<ThemeBackgroundSlot>,
    >,
    theme: Res<UiTheme>,
) {
    if let Ok((mut bg, theme_bg)) = q_background.get_mut(insert.entity) {
        bg.0 = theme.palette[theme_bg.0];
    }
}

fn on_changed_gradient(
    insert: On<Insert, ThemeBackgroundGradient>,
    mut q_gradient: Query<
        (&mut BackgroundGradient, &ThemeBackgroundGradient),
        Changed<ThemeBackgroundGradient>,
    >,
    theme: Res<UiTheme>,
) {
    if let Ok((mut gradient, theme_grad)) = q_gradient.get_mut(insert.entity) {
        *gradient = theme_background_gradient(theme.color(&theme_grad.0), theme_grad.1);
    }
}

fn on_changed_border(
    insert: On<Insert, ThemeBorderColor>,
    mut q_border: Query<(&mut BorderColor, &ThemeBorderColor), Changed<ThemeBorderColor>>,
    theme: Res<UiTheme>,
) {
    if let Ok((mut border, theme_border)) = q_border.get_mut(insert.entity) {
        border.set_all(theme.color(&theme_border.0));
    }
}

fn on_changed_border_slot(
    insert: On<Insert, ThemeBorderSlot>,
    mut q_border: Query<(&mut BorderColor, &ThemeBorderSlot), Changed<ThemeBorderSlot>>,
    theme: Res<UiTheme>,
) {
    if let Ok((mut border, theme_border)) = q_border.get_mut(insert.entity) {
        border.set_all(theme.palette[theme_border.0]);
    }
}

fn on_changed_text_color(
    insert: On<Insert, ThemeTextColor>,
    mut q_span: Query<(&mut TextColor, &ThemeTextColor), Changed<ThemeTextColor>>,
    theme: Res<UiTheme>,
) {
    if let Ok((mut text_color, theme_text_color)) = q_span.get_mut(insert.entity) {
        text_color.0 = theme.color(&theme_text_color.0);
    }
}

/// An observer which looks for changes to the [`InheritableThemeTextColor`] component on an entity,
/// and propagates downward the text color to all participating text entities.
fn on_changed_font_color(
    insert: On<Insert, InheritableThemeTextColor>,
    font_color: Query<&InheritableThemeTextColor>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    if let Ok(token) = font_color.get(insert.entity) {
        let color = theme.color(&token.0);
        commands
            .entity(insert.entity)
            .insert(Propagate(TextColor(color)));
    }
}

// Internal frames may break the propagation chain; only text that ends up with no
// color at all is a bug, so that is what this watches for.
fn warn_unstyled_themed_text(
    q_unstyled: Query<
        (),
        (
            With<ThemedText>,
            Without<Inherited<TextColor>>,
            Without<ThemeTextColor>,
            Without<InheritableThemeTextColor>,
        ),
    >,
    mut suspect_last_frame: Local<bool>,
) {
    // Propagation lands the frame after an entity is parented, so text unresolved
    // this frame may still be in flight.
    let unstyled = !q_unstyled.is_empty();
    let confirmed = unstyled && *suspect_last_frame;
    *suspect_last_frame = unstyled;
    if confirmed {
        warn_once!(
            "Themed text resolved no color and falls back to white. A container an app fills \
             with text must establish the style, as `row`/`column`/`scroll_content` do."
        );
    }
}

/// Installs the [`UiTheme`] resource, the theme refresh system, the themed
/// text-color propagation, and the token-change observers.
pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiTheme>()
            .add_plugins(HierarchyPropagatePlugin::<TextColor, With<ThemedText>>::new(PostUpdate))
            .add_systems(PostUpdate, update_theme)
            // After propagation, so text parented this frame has had its chance.
            .add_systems(
                PostUpdate,
                warn_unstyled_themed_text.after(PropagateSet::<TextColor>::default()),
            )
            .add_observer(on_changed_background_token)
            .add_observer(on_changed_background_slot)
            .add_observer(on_changed_gradient)
            .add_observer(on_changed_border)
            .add_observer(on_changed_border_slot)
            .add_observer(on_changed_font_color)
            .add_observer(on_changed_text_color)
            .add_observer(on_themed_text_inserted::<TextColor>);
    }
}

/// Fully-resolved colors: one [`Color`] per [`ThemeSlot`], built by [`ThemeEditablePalette::resolve`].
#[derive(Clone, Debug, Default, Reflect)]
pub struct ThemeResolvedPalette([Color; ThemeSlot::COUNT]);

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
/// Call [`Self::resolve`] to bake it into a [`ThemeResolvedPalette`].
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

    /// Bake the parametric palette into one resolved color per [`ThemeSlot`].
    ///
    /// The `copy_from_slice` blocks below rely on each ramp's variants being
    /// contiguous and in order within [`ThemeSlot`] (Neutral0..=Neutral6, etc.).
    pub fn resolve(&self) -> ThemeResolvedPalette {
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

    pub fn neutral(&self, index: usize) -> Color {
        self.neutrals.to_color(index)
    }
    pub fn accent(&self, index: usize) -> Color {
        self.accent.to_color(index)
    }
    pub fn text(&self, index: usize) -> Color {
        self.text.to_color(index)
    }
    /// Text stop `index` with the disabled-alpha modifier applied.
    pub fn text_dim(&self, index: usize) -> Color {
        self.text(index)
            .with_alpha(self.disabled_text_alpha_modifier)
    }
    pub fn axis(&self, index: usize) -> Color {
        self.axes[index].into()
    }

    /// Color for `token` via the *default* token→slot mapping (ignores any [`UiTheme`] remaps).
    pub fn token(&self, token: &ThemeToken) -> Color {
        let resolved = self.resolve();
        let lookup: HashMap<ThemeToken, ThemeSlot> =
            slots::DEFAULT_TOKEN_SLOTS.iter().cloned().collect();
        lookup
            .get(token)
            .map(|slot| resolved[*slot])
            .unwrap_or(Color::NONE)
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

pub mod dark_theme;
mod editor;
pub mod light_theme;
pub mod slots;
pub mod tokens;

pub use editor::theme_editor;
