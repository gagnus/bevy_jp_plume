//! A framework for theming.
use bevy_app::{App, HierarchyPropagatePlugin, Plugin, PostUpdate, Propagate, PropagateOver};
use bevy_color::{Alpha, Color, Oklcha, palettes};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    lifecycle::Insert,
    observer::On,
    query::{Changed, With},
    reflect::{ReflectComponent, ReflectResource},
    resource::Resource,
    system::{Commands, Query, Res},
};
use bevy_log::warn_once;
use bevy_platform::collections::HashMap;
use bevy_reflect::{Reflect, prelude::ReflectDefault};
use bevy_text::TextColor;
use bevy_ui::{BackgroundColor, BorderColor};
use smol_str::SmolStr;

/// A design token for the theme. This serves as the lookup key for the theme properties.
#[derive(Clone, PartialEq, Eq, Hash, Reflect, Default)]
pub struct ThemeToken(SmolStr);

impl ThemeToken {
    /// Construct a new [`ThemeToken`] from a [`SmolStr`].
    pub const fn new(text: SmolStr) -> Self {
        Self(text)
    }

    /// Construct a new [`ThemeToken`] from a static string.
    pub const fn new_static(text: &'static str) -> Self {
        Self(SmolStr::new_static(text))
    }
}

impl core::fmt::Display for ThemeToken {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::fmt::Debug for ThemeToken {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ThemeToken({:?})", self.0)
    }
}

/// One token per pointer-interaction state; see [`InteractionTokens::pick`].
#[derive(Clone, Reflect)]
pub struct InteractionTokens {
    /// Rest state.
    pub base: ThemeToken,
    /// Pointer over the control.
    pub hover: ThemeToken,
    /// Pointer pressed on the control.
    pub pressed: ThemeToken,
    /// Interaction disabled.
    pub disabled: ThemeToken,
}

impl InteractionTokens {
    /// Token for the given interaction state (disabled > pressed > hover > base).
    pub fn pick(&self, disabled: bool, pressed: bool, hovered: bool) -> ThemeToken {
        if disabled {
            self.disabled.clone()
        } else if pressed {
            self.pressed.clone()
        } else if hovered {
            self.hover.clone()
        } else {
            self.base.clone()
        }
    }
}

/// A collection of properties that make up a theme.
#[derive(Default, Clone, Reflect, Debug)]
#[reflect(Default, Debug)]
pub struct ThemeProps {
    /// Map of design tokens to colors.
    pub color: HashMap<ThemeToken, Color>,
    // Other style property types to be added later.
}

/// The currently selected user interface theme. Overwriting this resource changes the theme.
#[derive(Resource, Default, Reflect, Debug)]
#[reflect(Resource, Default, Debug)]
pub struct UiTheme(pub ThemeProps);

impl UiTheme {
    /// Lookup a color by design token. If the theme does not have an entry for that token,
    /// logs a warning and returns an error color.
    pub fn color(&self, token: &ThemeToken) -> Color {
        let color = self.0.color.get(token);
        match color {
            Some(c) => *c,
            None => {
                warn_once!("Theme color {} not found.", token);
                // Return a bright obnoxious color to make the error obvious.
                palettes::basic::FUCHSIA.into()
            }
        }
    }

    /// Associate a design token with a given color.
    pub fn set_color(&mut self, token: &str, color: Color) {
        self.0
            .color
            .insert(ThemeToken::new(SmolStr::new(token)), color);
    }
}

/// Component which causes the background color of an entity to be set based on a theme color.
#[derive(Component, Clone, Default)]
#[require(BackgroundColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBackgroundColor(pub ThemeToken);

/// Component which causes the border color of an entity to be set based on a theme color.
/// Only supports setting all borders to the same color.
#[derive(Component, Clone, Default)]
#[require(BorderColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBorderColor(pub ThemeToken);

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

/// Installs the [`UiTheme`] resource, the theme refresh system, the themed
/// text-color propagation, and the token-change observers.
pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiTheme>()
            .add_plugins(HierarchyPropagatePlugin::<TextColor, With<ThemedText>>::new(PostUpdate))
            .add_systems(PostUpdate, update_theme)
            .add_observer(on_changed_background)
            .add_observer(on_changed_border)
            .add_observer(on_changed_font_color)
            .add_observer(on_changed_text_color);
    }
}

fn update_theme(
    mut q_background: Query<(&mut BackgroundColor, &ThemeBackgroundColor)>,
    mut q_border: Query<(&mut BorderColor, &ThemeBorderColor)>,
    mut q_text_color: Query<(&mut TextColor, &ThemeTextColor)>,
    q_inherit: Query<(Entity, &InheritableThemeTextColor)>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    if theme.is_changed() {
        // Update all background colors
        for (mut bg, theme_bg) in q_background.iter_mut() {
            bg.0 = theme.color(&theme_bg.0);
        }

        // Update all border colors
        for (mut border, theme_border) in q_border.iter_mut() {
            border.set_all(theme.color(&theme_border.0));
        }

        // Update all direct text span colors
        for (mut text_color, theme_text_color) in q_text_color.iter_mut() {
            text_color.0 = theme.color(&theme_text_color.0);
        }

        // Re-propagate inheritable text colors (buttons, menus, list rows, etc.)
        for (entity, inherit) in &q_inherit {
            commands
                .entity(entity)
                .insert(Propagate(TextColor(theme.color(&inherit.0))));
        }
    }
}

fn on_changed_background(
    insert: On<Insert, ThemeBackgroundColor>,
    mut q_background: Query<
        (&mut BackgroundColor, &ThemeBackgroundColor),
        Changed<ThemeBackgroundColor>,
    >,
    theme: Res<UiTheme>,
) {
    // Update background colors where the design token has changed.
    if let Ok((mut bg, theme_bg)) = q_background.get_mut(insert.entity) {
        bg.0 = theme.color(&theme_bg.0);
    }
}

fn on_changed_border(
    insert: On<Insert, ThemeBorderColor>,
    mut q_border: Query<(&mut BorderColor, &ThemeBorderColor), Changed<ThemeBorderColor>>,
    theme: Res<UiTheme>,
) {
    // Update background colors where the design token has changed.
    if let Ok((mut border, theme_border)) = q_border.get_mut(insert.entity) {
        border.set_all(theme.color(&theme_border.0));
    }
}

fn on_changed_text_color(
    insert: On<Insert, ThemeTextColor>,
    mut q_span: Query<(&mut TextColor, &ThemeTextColor), Changed<ThemeTextColor>>,
    theme: Res<UiTheme>,
) {
    // Update background colors where the design token has changed.
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

// [`EditablePalette`] is the *parametric* form an editor manipulates
// [`EditablePalette::resolve`] bakes it into a [`ResolvedPalette`] — one [`Color`]
// per [`ThemeSlot`]. [`build_theme`] then maps each theme token to a slot using
// a mapping which can be got from [`default_token_slots`] and looks up its color

/// A single semantic color role, these are all the colors that make up
/// a theme.
#[derive(Component, Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum ThemeSlot {
    /// Window background: `WINDOW_BG`.
    #[default]
    Neutral0,

    /// Surface bodies & menus: `SUBPANE_BODY_BG`, `DIALOG_BG`, `MENU_BG`, `TEXT_INPUT_BG_DISABLED`,
    /// `TEXT_INPUT_BG`.
    Neutral1,

    /// Raised container headers & borders: `SUBPANE_HEADER_BG`, `GROUP_BG`/`_BORDER`,
    /// `DIALOG_HEADER_BG`.
    Neutral2,

    /// Weak fills & disabled chrome: `SLIDER_BG*`, `SCROLLBAR_BG`, `LISTROW_BG_HOVER`, `BUTTON_BG_DISABLED`,
    /// `BUTTON_PRIMARY_BG_DISABLED`, plus every
    /// disabled border/mark/knob on checkbox/radio/switch/text input (`*_BORDER_DISABLED`,
    /// `*_BORDER_CHECKED_DISABLED`, `*_MARK_DISABLED`, `SWITCH_SLIDE_*_DISABLED`).
    Neutral3,

    /// Control rest bg & borders + selected row: `BUTTON_BG`, `CHECKBOX_BORDER`, `RADIO_BORDER`,
    /// `SWITCH_BG`/`_BORDER`, `LISTROW_BG_SELECTED`, `SLIDER_BAR_DISABLED`, `SLIDER_THUMB_DISABLED`,
    /// `SUBPANE_HEADER_BORDER`, `SUBPANE_BODY_BORDER`, `DIALOG_BORDER`, `MENU_BORDER`, `TEXT_INPUT_BORDER`.
    /// Checkbox/radio/switch have no hover/pressed variants (Radix-style: they
    /// change only with checked state).
    Neutral4,

    /// Control hover: `BUTTON_BG_HOVER`, `BUTTON_PLAIN_BG_HOVER`.
    Neutral5,

    /// Control pressed: `BUTTON_BG_PRESSED`, `BUTTON_PLAIN_BG_PRESSED`.
    Neutral6,

    /// Bright on-surface labels & unchecked switch knob: `BUTTON_TEXT`, `TEXT_INPUT_TEXT`,
    /// `LISTROW_TEXT`, `SUBPANE_HEADER_TEXT`, `DIALOG_HEADER_TEXT`,
    /// `SWITCH_SLIDE_BG`/`_BORDER`.
    Text0,

    /// Body text: `TEXT_MAIN`, `DIALOG_TEXT`, `CHECKBOX_TEXT`, `RADIO_TEXT`.
    Text1,

    /// Disabled bright text + dimmed text: `BUTTON_TEXT_DISABLED`, `BUTTON_PRIMARY_TEXT_DISABLED`,
    /// `TEXT_INPUT_TEXT_DISABLED`, `LISTROW_TEXT_DISABLED`, `TEXT_DIM`.
    TextDim0,

    /// Disabled body-text labels: `CHECKBOX_TEXT_DISABLED`, `RADIO_TEXT_DISABLED`.
    TextDim1,

    /// Base call-to-action: `BUTTON_PRIMARY_BG`, `SLIDER_BAR`, `SLIDER_THUMB`, `SCROLLBAR_THUMB`,
    /// `*_BG_CHECKED`/`*_BORDER_CHECKED` (checkbox/radio/switch), `TEXT_INPUT_SELECTION`.
    Accent0,

    /// Call-to-action hover: `BUTTON_PRIMARY_BG_HOVER`, `SLIDER_BAR_HOVER`,
    /// `SLIDER_THUMB_HOVER`, `SCROLLBAR_THUMB_HOVER`.
    Accent1,

    /// Call-to-action pressed: `BUTTON_PRIMARY_BG_PRESSED`, `SLIDER_BAR_PRESSED`, `SLIDER_THUMB_PRESSED`,
    /// `SCROLLBAR_THUMB_PRESSED`.
    Accent2,

    /// Brightest accent: `TEXT_INPUT_CURSOR`.
    Accent3,

    /// Foreground over accent-filled components: `BUTTON_PRIMARY_TEXT`, `CHECKBOX_MARK`, `RADIO_MARK`,
    /// `SWITCH_SLIDE_BG_CHECKED`/`SWITCH_SLIDE_BORDER_CHECKED`.
    Contrast,

    /// Focus/selection ring color (reserved; no token maps here yet).
    FocusRing,

    /// Red axis (reserved for axis-colored widgets).
    XAxis,

    /// Green axis (reserved for axis-colored widgets).
    YAxis,

    /// Blue axis (reserved for axis-colored widgets).
    ZAxis,

    /// Always [`Color::NONE`]; used by tokens that paint nothing: `BUTTON_PLAIN_BG`/`_DISABLED`,
    /// unchecked `RADIO_BG`/`CHECKBOX_BG`, `*_BG_DISABLED`/`*_BG_CHECKED_DISABLED`
    /// (checkbox/radio/switch), `TEXT_INPUT_SELECTION_UNFOCUSED`,
    /// `LISTROW_BG`.
    Transparent,
}

impl ThemeSlot {
    /// Every slot, in discriminant order (matches [`ResolvedPalette`] storage).
    pub const ALL: [ThemeSlot; 21] = [
        ThemeSlot::Neutral0,
        ThemeSlot::Neutral1,
        ThemeSlot::Neutral2,
        ThemeSlot::Neutral3,
        ThemeSlot::Neutral4,
        ThemeSlot::Neutral5,
        ThemeSlot::Neutral6,
        ThemeSlot::Text0,
        ThemeSlot::Text1,
        ThemeSlot::TextDim0,
        ThemeSlot::TextDim1,
        ThemeSlot::Accent0,
        ThemeSlot::Accent1,
        ThemeSlot::Accent2,
        ThemeSlot::Accent3,
        ThemeSlot::Contrast,
        ThemeSlot::FocusRing,
        ThemeSlot::XAxis,
        ThemeSlot::YAxis,
        ThemeSlot::ZAxis,
        ThemeSlot::Transparent,
    ];

    /// Number of slots — the backing size of [`ResolvedPalette`].
    pub const COUNT: usize = Self::ALL.len();

    /// Human-readable name, for editor UI / pickers.
    pub fn label(self) -> &'static str {
        match self {
            ThemeSlot::Neutral0 => "Neutral 0",
            ThemeSlot::Neutral1 => "Neutral 1",
            ThemeSlot::Neutral2 => "Neutral 2",
            ThemeSlot::Neutral3 => "Neutral 3",
            ThemeSlot::Neutral4 => "Neutral 4",
            ThemeSlot::Neutral5 => "Neutral 5",
            ThemeSlot::Neutral6 => "Neutral 6",
            ThemeSlot::Text0 => "Text 0",
            ThemeSlot::Text1 => "Text 1",
            ThemeSlot::TextDim0 => "Text Dim 0",
            ThemeSlot::TextDim1 => "Text Dim 1",
            ThemeSlot::Accent0 => "Accent 0",
            ThemeSlot::Accent1 => "Accent 1",
            ThemeSlot::Accent2 => "Accent 2",
            ThemeSlot::Accent3 => "Accent 3",
            ThemeSlot::Contrast => "Contrast",
            ThemeSlot::FocusRing => "Focus Ring",
            ThemeSlot::XAxis => "X Axis",
            ThemeSlot::YAxis => "Y Axis",
            ThemeSlot::ZAxis => "Z Axis",
            ThemeSlot::Transparent => "Transparent",
        }
    }
}

/// Fully-resolved colors: one [`Color`] per [`ThemeSlot`], built by
/// [`EditablePalette::resolve`] and read by [`build_theme`]. Indexed by slot.
#[derive(Clone, Debug)]
pub struct ResolvedPalette([Color; ThemeSlot::COUNT]);

impl core::ops::Index<ThemeSlot> for ResolvedPalette {
    type Output = Color;
    fn index(&self, slot: ThemeSlot) -> &Color {
        &self.0[slot as usize]
    }
}

/// Represents a set of [`Oklcha`] colors which have same hue and chroma but different lightnesses
#[derive(Clone, Debug)]
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

/// The theme's parametric palette
/// Call [`Self::resolve`] to bake it into a [`ResolvedPalette`].
#[derive(Clone, Debug)]
pub struct EditablePalette {
    /// Lightness of each neutral ramp stop; forms [`ThemeSlot::Neutral0`]..=[`ThemeSlot::Neutral6`].
    pub neutrals: OklchaArray<7>,

    /// Lightness of each accent stop; forms [`ThemeSlot::Accent0`]..=[`ThemeSlot::Accent3`]
    /// (and [`ThemeSlot::FocusRing`], derived from `accent[0]`).
    pub accent: OklchaArray<4>,

    /// Foreground on accent-filled components (white in most themes); forms [`ThemeSlot::Contrast`].
    pub contrast: Oklcha,

    /// Lightness of each text stop; forms [`ThemeSlot::Text0`]..=[`ThemeSlot::Text1`]
    /// (and [`ThemeSlot::TextDim0`]..=[`ThemeSlot::TextDim1`], derived).
    pub text: OklchaArray<2>,

    /// Alpha applied to `text` to derive [`ThemeSlot::TextDim0`]..=[`ThemeSlot::TextDim1`].
    pub dim_text_alpha_modifier: f32,

    /// RGB axis colors; form [`ThemeSlot::XAxis`], [`ThemeSlot::YAxis`], [`ThemeSlot::ZAxis`].
    pub axes: [Oklcha; 3],
}

impl EditablePalette {
    /// Bake the parametric palette into one resolved color per [`ThemeSlot`].
    ///
    /// The `copy_from_slice` blocks below rely on each ramp's variants being
    /// contiguous and in order within [`ThemeSlot`] (Neutral0..=Neutral6, etc.).
    pub fn resolve(&self) -> ResolvedPalette {
        let neutral = self.neutrals.to_array();
        let accent = self.accent.to_array();
        let text = self.text.to_array();
        let text_dim = text.map(|c| c.with_alpha(self.dim_text_alpha_modifier));
        let axes: [Color; 3] = self.axes.map(Into::into);

        let mut c = [Color::NONE; ThemeSlot::COUNT];
        c[ThemeSlot::Neutral0 as usize..=ThemeSlot::Neutral6 as usize].copy_from_slice(&neutral);
        c[ThemeSlot::Text0 as usize..=ThemeSlot::Text1 as usize].copy_from_slice(&text);
        c[ThemeSlot::TextDim0 as usize..=ThemeSlot::TextDim1 as usize].copy_from_slice(&text_dim);
        c[ThemeSlot::Accent0 as usize..=ThemeSlot::Accent3 as usize].copy_from_slice(&accent);
        c[ThemeSlot::Contrast as usize] = self.contrast.into();
        c[ThemeSlot::FocusRing as usize] = accent[0].with_alpha(0.5);
        c[ThemeSlot::XAxis as usize..=ThemeSlot::ZAxis as usize].copy_from_slice(&axes);
        // ThemeSlot::Transparent stays Color::NONE.
        ResolvedPalette(c)
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
    pub fn text_dim(&self, index: usize) -> Color {
        self.text(index).with_alpha(self.dim_text_alpha_modifier)
    }
    pub fn axis(&self, index: usize) -> Color {
        self.axes[index].into()
    }

    pub fn token(&self, token: &ThemeToken) -> Color {
        let resolved = self.resolve();
        let lookup: HashMap<ThemeToken, ThemeSlot> = DEFAULT_TOKEN_SLOTS.iter().cloned().collect();
        lookup
            .get(token)
            .map(|slot| resolved[*slot])
            .unwrap_or(Color::NONE)
            .clone()
    }
}

/// Build Plume theme properties by resolving every token's [`ThemeSlot`] against `p`.
pub fn build_theme(
    palette: &ResolvedPalette,
    token_slots: &[(ThemeToken, ThemeSlot)],
) -> ThemeProps {
    ThemeProps {
        color: token_slots
            .iter()
            .map(|(token, slot)| (token.clone(), palette[*slot]))
            .collect(),
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

static DEFAULT_TOKEN_SLOTS: &[(ThemeToken, ThemeSlot)] = &[
    (tokens::WINDOW_BG, ThemeSlot::Neutral0),
    (tokens::TEXT_MAIN, ThemeSlot::Text1),
    (tokens::TEXT_DIM, ThemeSlot::TextDim0),
    (tokens::BUTTON_BG, ThemeSlot::Neutral4),
    (tokens::BUTTON_BG_HOVER, ThemeSlot::Neutral5),
    (tokens::BUTTON_BG_PRESSED, ThemeSlot::Neutral6),
    (tokens::BUTTON_BG_DISABLED, ThemeSlot::Neutral3),
    (tokens::BUTTON_PRIMARY_BG, ThemeSlot::Accent0),
    (tokens::BUTTON_PRIMARY_BG_HOVER, ThemeSlot::Accent1),
    (tokens::BUTTON_PRIMARY_BG_PRESSED, ThemeSlot::Accent2),
    (tokens::BUTTON_PRIMARY_BG_DISABLED, ThemeSlot::Neutral3),
    (tokens::BUTTON_PLAIN_BG, ThemeSlot::Transparent),
    (tokens::BUTTON_PLAIN_BG_HOVER, ThemeSlot::Neutral5),
    (tokens::BUTTON_PLAIN_BG_PRESSED, ThemeSlot::Neutral6),
    (tokens::BUTTON_PLAIN_BG_DISABLED, ThemeSlot::Transparent),
    (tokens::BUTTON_TEXT, ThemeSlot::Text0),
    (tokens::BUTTON_TEXT_DISABLED, ThemeSlot::TextDim0),
    (tokens::BUTTON_PRIMARY_TEXT, ThemeSlot::Contrast),
    (tokens::BUTTON_PRIMARY_TEXT_DISABLED, ThemeSlot::TextDim0),
    (tokens::SLIDER_BG, ThemeSlot::Neutral3),
    (tokens::SLIDER_BG_HOVER, ThemeSlot::Neutral3),
    (tokens::SLIDER_BG_PRESSED, ThemeSlot::Neutral3),
    (tokens::SLIDER_BG_DISABLED, ThemeSlot::Neutral3),
    (tokens::SLIDER_BAR, ThemeSlot::Accent0),
    (tokens::SLIDER_BAR_HOVER, ThemeSlot::Accent1),
    (tokens::SLIDER_BAR_PRESSED, ThemeSlot::Accent2),
    (tokens::SLIDER_BAR_DISABLED, ThemeSlot::Neutral4),
    (tokens::SLIDER_THUMB, ThemeSlot::Accent0),
    (tokens::SLIDER_THUMB_HOVER, ThemeSlot::Accent1),
    (tokens::SLIDER_THUMB_PRESSED, ThemeSlot::Accent2),
    (tokens::SLIDER_THUMB_DISABLED, ThemeSlot::Neutral4),
    (tokens::SCROLLBAR_BG, ThemeSlot::Neutral3),
    (tokens::SCROLLBAR_THUMB, ThemeSlot::Accent0),
    (tokens::SCROLLBAR_THUMB_HOVER, ThemeSlot::Accent1),
    (tokens::SCROLLBAR_THUMB_PRESSED, ThemeSlot::Accent2),
    (tokens::CHECKBOX_BG, ThemeSlot::Transparent),
    (tokens::CHECKBOX_BG_DISABLED, ThemeSlot::Transparent),
    (tokens::CHECKBOX_BG_CHECKED, ThemeSlot::Accent0),
    (tokens::CHECKBOX_BG_CHECKED_DISABLED, ThemeSlot::Transparent),
    (tokens::CHECKBOX_BORDER, ThemeSlot::Neutral4),
    (tokens::CHECKBOX_BORDER_DISABLED, ThemeSlot::Neutral3), // was TextDim1
    (tokens::CHECKBOX_BORDER_CHECKED, ThemeSlot::Accent0),
    (
        tokens::CHECKBOX_BORDER_CHECKED_DISABLED,
        ThemeSlot::Neutral3,
    ), // was TextDim1
    (tokens::CHECKBOX_MARK, ThemeSlot::Contrast),
    (tokens::CHECKBOX_MARK_DISABLED, ThemeSlot::Neutral3), // was TextDim1
    (tokens::CHECKBOX_TEXT, ThemeSlot::Text1),
    (tokens::CHECKBOX_TEXT_DISABLED, ThemeSlot::TextDim1),
    (tokens::RADIO_BG, ThemeSlot::Transparent),
    (tokens::RADIO_BG_DISABLED, ThemeSlot::Transparent),
    (tokens::RADIO_BG_CHECKED, ThemeSlot::Accent0),
    (tokens::RADIO_BG_CHECKED_DISABLED, ThemeSlot::Transparent),
    (tokens::RADIO_BORDER, ThemeSlot::Neutral4),
    (tokens::RADIO_BORDER_DISABLED, ThemeSlot::Neutral3), // was TextDim1
    (tokens::RADIO_BORDER_CHECKED, ThemeSlot::Accent0),
    (tokens::RADIO_BORDER_CHECKED_DISABLED, ThemeSlot::Neutral3), // was TextDim1
    (tokens::RADIO_MARK, ThemeSlot::Contrast),
    (tokens::RADIO_MARK_DISABLED, ThemeSlot::Neutral3), // was TextDim1
    (tokens::RADIO_TEXT, ThemeSlot::Text1),
    (tokens::RADIO_TEXT_DISABLED, ThemeSlot::TextDim1),
    (tokens::SWITCH_BG, ThemeSlot::Neutral4),
    (tokens::SWITCH_BG_DISABLED, ThemeSlot::Transparent),
    (tokens::SWITCH_BG_CHECKED, ThemeSlot::Accent0),
    (tokens::SWITCH_BG_CHECKED_DISABLED, ThemeSlot::Transparent),
    (tokens::SWITCH_BORDER, ThemeSlot::Neutral4),
    (tokens::SWITCH_BORDER_DISABLED, ThemeSlot::Neutral3), // was TextDim1
    (tokens::SWITCH_BORDER_CHECKED, ThemeSlot::Accent0),
    (tokens::SWITCH_BORDER_CHECKED_DISABLED, ThemeSlot::Neutral3), // was TextDim1
    (tokens::SWITCH_SLIDE_BG, ThemeSlot::Text0),
    (tokens::SWITCH_SLIDE_BG_DISABLED, ThemeSlot::Neutral3), // was TextDim1
    (tokens::SWITCH_SLIDE_BG_CHECKED, ThemeSlot::Contrast),
    (
        tokens::SWITCH_SLIDE_BG_CHECKED_DISABLED,
        ThemeSlot::Neutral3,
    ), // was TextDim1
    (tokens::SWITCH_SLIDE_BORDER, ThemeSlot::Text0),
    (tokens::SWITCH_SLIDE_BORDER_DISABLED, ThemeSlot::Neutral3), // was TextDim1
    (tokens::SWITCH_SLIDE_BORDER_CHECKED, ThemeSlot::Contrast),
    (
        tokens::SWITCH_SLIDE_BORDER_CHECKED_DISABLED,
        ThemeSlot::Neutral3,
    ), // was TextDim1
    (tokens::MENU_BG, ThemeSlot::Neutral1),
    (tokens::MENU_BORDER, ThemeSlot::Neutral4),
    (tokens::TEXT_INPUT_BG, ThemeSlot::Neutral0),
    (tokens::TEXT_INPUT_BG_DISABLED, ThemeSlot::Neutral0),
    (tokens::TEXT_INPUT_TEXT, ThemeSlot::Text0),
    (tokens::TEXT_INPUT_TEXT_DISABLED, ThemeSlot::TextDim0),
    (tokens::TEXT_INPUT_CURSOR, ThemeSlot::Accent3),
    (tokens::TEXT_INPUT_SELECTION, ThemeSlot::Accent0),
    (
        tokens::TEXT_INPUT_SELECTION_UNFOCUSED,
        ThemeSlot::Transparent,
    ),
    (tokens::TEXT_INPUT_BORDER, ThemeSlot::Neutral4),
    (tokens::TEXT_INPUT_BORDER_DISABLED, ThemeSlot::Neutral3),
    (tokens::SUBPANE_HEADER_BG, ThemeSlot::Neutral2),
    (tokens::SUBPANE_HEADER_BORDER, ThemeSlot::Neutral4),
    (tokens::SUBPANE_HEADER_TEXT, ThemeSlot::Text0),
    (tokens::SUBPANE_BODY_BG, ThemeSlot::Neutral1),
    (tokens::SUBPANE_BODY_BORDER, ThemeSlot::Neutral4),
    (tokens::GROUP_BG, ThemeSlot::Neutral2),
    (tokens::GROUP_BORDER, ThemeSlot::Neutral2),
    (tokens::LISTROW_BG, ThemeSlot::Transparent),
    (tokens::LISTROW_BG_HOVER, ThemeSlot::Neutral3),
    (tokens::LISTROW_BG_SELECTED, ThemeSlot::Neutral4),
    (tokens::LISTROW_TEXT, ThemeSlot::Text0),
    (tokens::LISTROW_TEXT_DISABLED, ThemeSlot::TextDim0),
    (tokens::DIALOG_BG, ThemeSlot::Neutral1),
    (tokens::DIALOG_BORDER, ThemeSlot::Neutral4),
    (tokens::DIALOG_HEADER_BG, ThemeSlot::Neutral2),
    (tokens::DIALOG_TEXT, ThemeSlot::Text1),
    (tokens::DIALOG_HEADER_TEXT, ThemeSlot::Text0),
];

/// Default mapping from each token to a [`ThemeSlot`]
pub fn default_token_slots() -> &'static [(ThemeToken, ThemeSlot)] {
    &DEFAULT_TOKEN_SLOTS
}

pub mod dark_theme;
pub mod light_theme;
pub mod tokens;
