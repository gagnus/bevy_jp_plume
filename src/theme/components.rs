//! Theme-driven styling components: the markers a retained scene puts on an
//! entity to have `ThemePlugin` color it.
use bevy::app::{Inherited, Propagate, PropagateOver, PropagateStop};
use bevy::color::{Alpha, Color, Luminance, Srgba};
use bevy::ecs::change_detection::DetectChanges;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::hierarchy::ChildOf;
use bevy::ecs::lifecycle::{Insert, RemovedComponents};
use bevy::ecs::observer::On;
use bevy::ecs::query::{Added, Changed, Has, Or, With, Without};
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::system::{Commands, Query, Res};
use bevy::reflect::Reflect;
use bevy::reflect::prelude::ReflectDefault;
use bevy::text::TextColor;
use bevy::ui::{
    BackgroundColor, BackgroundGradient, BorderColor, BoxShadow, ColorStop, Gradient,
    InterpolationColorSpace, LinearGradient, percent,
};

use super::UiTheme;
use crate::constants::size;
use crate::theme::slots::ThemeSlot;
use crate::theme::tokens::ThemeToken;

// Background color by theme token — plume-internal; apps use [`ThemeBackgroundSlot`].
#[derive(Component, Clone, Default)]
#[require(BackgroundColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub(crate) struct ThemeBackgroundToken(pub ThemeToken);

/// Component which sets the background color of an entity from a theme slot.
/// Takes priority over the internal token form.
#[derive(Component, Clone, Default)]
#[require(BackgroundColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBackgroundSlot(pub ThemeSlot);

// The standard luminance adjust (+-) for an active control's [`GradientAmount`].
pub(crate) const GRADIENT_AMOUNT: f32 = 0.05;

/// Shades an entity's themed background into a vertical gradient: the fill is
/// lightened by this much at the top and darkened by it at the bottom. Zero — or
/// no component at all — paints a flat fill instead.
///
/// Orthogonal to the source of the color, so it shades a [`ThemeBackgroundSlot`]
/// as readily as one of plume's internal tokens. [`Flat`] overrides it to zero.
#[derive(Component, Clone, Copy, Default, PartialEq)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone, Default)]
pub struct GradientAmount(pub f32);

impl GradientAmount {
    /// The shading plume's own raised controls carry.
    pub const STANDARD: Self = Self(GRADIENT_AMOUNT);
}

/// Opt-in marker: the entity's themed fill renders flat, whatever
/// [`GradientAmount`] it carries. Honored wherever a themed background is
/// painted, so it works on plume's controls and on an app's own surfaces alike.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct Flat;

/// Opt-in marker: the entity does not repond to hover and pressed. Currently only
/// respected by `PlumeButton`.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct Inert;

// Build the gentle top-to-bottom gradient a non-zero `GradientAmount` resolves to.
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

/// The standard drop shadow under a raised control.
pub fn control_box_shadow() -> BoxShadow {
    BoxShadow::new(
        Srgba::BLACK.with_alpha(0.5).into(),
        size::GAP / 4.0,
        size::GAP / 4.0,
        size::GAP / 8.0,
        size::GAP / 4.0,
    )
}

// Border color by theme token — plume-internal; apps use [`ThemeBorderSlot`].
// Only supports setting all borders to the same color.
#[derive(Component, Clone, Default)]
#[require(BorderColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub(crate) struct ThemeBorderToken(pub ThemeToken);

/// Component which sets the border color of an entity from a theme slot.
/// Takes priority over the internal token form. Only supports setting all
/// borders to the same color.
#[derive(Component, Clone, Default)]
#[require(BorderColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBorderSlot(pub ThemeSlot);

// Inherited text color by theme token — plume-internal; apps use
// [`InheritableThemeTextSlot`].
#[derive(Component, Clone, Default)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
#[require(ThemedText, PropagateOver::<TextColor>)]
pub(crate) struct InheritableThemeTextToken(pub ThemeToken);

/// Propagates a theme-slot text color to descendant themed text, and to the
/// carrier itself when it is text (a direct [`ThemeTextSlot`] still wins).
#[derive(Component, Clone, Default)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
#[require(ThemedText, PropagateOver::<TextColor>)]
pub struct InheritableThemeTextSlot(pub ThemeSlot);

/// Propagates a one-off raw text color to descendant themed text, and to the
/// carrier itself when it is text.
#[derive(Component, Clone, Default)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
#[require(ThemedText, PropagateOver::<TextColor>)]
pub struct InheritableTextColor(pub Color);

// Text color of the span itself by theme token — plume-internal; apps use
// [`ThemeTextSlot`]. Unlike the inheritable forms this works set directly on the
// text entity, and is not inherited.
#[derive(Component, Clone, Default)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
#[require(ThemedText, PropagateOver::<TextColor>)]
pub(crate) struct ThemeTextToken(pub ThemeToken);

/// Component which sets the color of the text span it is on from a theme slot
/// (the inheritable forms only reach descendants). Takes priority over the
/// internal token form.
#[derive(Component, Clone, Default)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
#[require(ThemedText, PropagateOver::<TextColor>)]
pub struct ThemeTextSlot(pub ThemeSlot);

/// A marker component that is used to indicate that the text entity wants to opt-in to using
/// inherited text styles.
#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
pub struct ThemedText;

// Finish propagating `C` to a child that gained [`ThemedText`] after it was
// parented.
//
// Propagation copies `Inherited<C>` onto a new child when its `ChildOf` lands, but
// only onto children already matching the `With<ThemedText>` filter. The imm
// reconciler parents an entity before its scene applies the marker, so that copy is
// missed and the text keeps the engine default font/color; this repeats it from the
// other side. Entities whose parent's `Inherited<C>` is itself new are covered by
// the ordinary downward pass.
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

// Everything that decides what a themed background paints: where the color comes
// from, and how hard it is shaded.
type BackgroundSource<'w> = (
    Entity,
    Option<&'w ThemeBackgroundToken>,
    Option<&'w ThemeBackgroundSlot>,
    Option<&'w GradientAmount>,
    Has<Flat>,
);

// Carries a themed background at all — the slot form wins where both are present,
// which is how an app overrides a plume control's internal token.
type HasBackground = Or<(With<ThemeBackgroundToken>, With<ThemeBackgroundSlot>)>;

// Paint one themed background as either a gradient or a flat fill, and blank
// whichever of the two is not in use: bevy draws `BackgroundGradient` over
// `BackgroundColor`, so leaving both populated stacks two fills.
fn paint_background(commands: &mut Commands, entity: Entity, color: Color, amount: f32) {
    let mut entity = commands.entity(entity);
    if amount == 0.0 {
        entity
            .insert(BackgroundColor(color))
            .remove::<BackgroundGradient>();
    } else {
        entity.insert((
            BackgroundColor(Color::NONE),
            theme_background_gradient(color, amount),
        ));
    }
}

// Resolve one entity's source components against the theme and paint it.
fn resolve_background(
    commands: &mut Commands,
    theme: &UiTheme,
    (entity, token, slot, amount, flat): (
        Entity,
        Option<&ThemeBackgroundToken>,
        Option<&ThemeBackgroundSlot>,
        Option<&GradientAmount>,
        bool,
    ),
) {
    let color = match (slot, token) {
        (Some(slot), _) => theme.palette(slot.0),
        (None, Some(token)) => theme.color(&token.0),
        (None, None) => return,
    };
    let amount = if flat {
        0.0
    } else {
        amount.map_or(0.0, |a| a.0)
    };
    paint_background(commands, entity, color, amount);
}

// Repaints themed backgrounds whose source, shading or `Flat` marker moved, and
// every one of them when the palette itself changes.
//
// A system rather than insert observers, because the inputs arrive on an entity
// separately and in no fixed order — a scene supplies the token and `Flat` from
// two different layers, and the control style systems write the token and the
// amount as independent commands. Running once, late, after all of them have
// landed is what makes the result order-independent.
pub(crate) fn resolve_backgrounds(
    q_backgrounds: Query<BackgroundSource, HasBackground>,
    q_dirty: Query<
        Entity,
        (
            HasBackground,
            Or<(
                Changed<ThemeBackgroundToken>,
                Changed<ThemeBackgroundSlot>,
                Changed<GradientAmount>,
                Added<Flat>,
            )>,
        ),
    >,
    mut removed_flat: RemovedComponents<Flat>,
    mut removed_amount: RemovedComponents<GradientAmount>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    if theme.is_changed() {
        // A palette swap repaints everything, so the dirty lists are moot — but
        // they still have to be drained or they fire again next frame.
        removed_flat.clear();
        removed_amount.clear();
        for source in &q_backgrounds {
            resolve_background(&mut commands, &theme, source);
        }
        return;
    }

    for entity in q_dirty
        .iter()
        .chain(removed_flat.read())
        .chain(removed_amount.read())
    {
        if let Ok(source) = q_backgrounds.get(entity) {
            resolve_background(&mut commands, &theme, source);
        }
    }
}

pub(crate) fn on_changed_border_token(
    insert: On<Insert, ThemeBorderToken>,
    mut q_border: Query<(&mut BorderColor, &ThemeBorderToken), Changed<ThemeBorderToken>>,
    theme: Res<UiTheme>,
) {
    if let Ok((mut border, theme_border)) = q_border.get_mut(insert.entity) {
        border.set_all(theme.color(&theme_border.0));
    }
}

pub(crate) fn on_changed_border_slot(
    insert: On<Insert, ThemeBorderSlot>,
    mut q_border: Query<(&mut BorderColor, &ThemeBorderSlot), Changed<ThemeBorderSlot>>,
    theme: Res<UiTheme>,
) {
    if let Ok((mut border, theme_border)) = q_border.get_mut(insert.entity) {
        border.set_all(theme.palette(theme_border.0));
    }
}

pub(crate) fn on_changed_text_token(
    insert: On<Insert, ThemeTextToken>,
    mut q_span: Query<(&mut TextColor, &ThemeTextToken), Changed<ThemeTextToken>>,
    theme: Res<UiTheme>,
) {
    if let Ok((mut text_color, theme_text_color)) = q_span.get_mut(insert.entity) {
        text_color.0 = theme.color(&theme_text_color.0);
    }
}

pub(crate) fn on_changed_text_slot(
    insert: On<Insert, ThemeTextSlot>,
    mut q_span: Query<(&mut TextColor, &ThemeTextSlot), Changed<ThemeTextSlot>>,
    theme: Res<UiTheme>,
) {
    if let Ok((mut text_color, theme_text_slot)) = q_span.get_mut(insert.entity) {
        text_color.0 = theme.palette(theme_text_slot.0);
    }
}

// Apply a resolved inheritable color: the `Propagate` source, plus the plain
// `TextColor` when the carrier is itself text (`PropagateOver` blocks the
// output write there); direct `ThemeTextSlot`/`ThemeTextToken` keep precedence.
pub(crate) type SelfColorFilter = (
    Or<(With<bevy::ui::widget::Text>, With<bevy::text::EditableText>)>,
    Without<ThemeTextSlot>,
    Without<ThemeTextToken>,
);

pub(crate) fn apply_inheritable_color(
    commands: &mut Commands,
    entity: Entity,
    color: Color,
    styles_own_text: bool,
) {
    if styles_own_text {
        commands
            .entity(entity)
            .insert((Propagate(TextColor(color)), TextColor(color)));
    } else {
        commands.entity(entity).insert(Propagate(TextColor(color)));
    }
}

// Propagates the resolved text color down to every participating text entity.
pub(crate) fn on_changed_inheritable_text_token(
    insert: On<Insert, InheritableThemeTextToken>,
    font_color: Query<&InheritableThemeTextToken>,
    q_self: Query<(), SelfColorFilter>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    if let Ok(token) = font_color.get(insert.entity) {
        let color = theme.color(&token.0);
        apply_inheritable_color(
            &mut commands,
            insert.entity,
            color,
            q_self.contains(insert.entity),
        );
    }
}

// Slot counterpart of `on_changed_inheritable_text_token`.
pub(crate) fn on_changed_inheritable_text_slot(
    insert: On<Insert, InheritableThemeTextSlot>,
    q_slot: Query<&InheritableThemeTextSlot>,
    q_self: Query<(), SelfColorFilter>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    if let Ok(slot) = q_slot.get(insert.entity) {
        let color = theme.palette(slot.0);
        apply_inheritable_color(
            &mut commands,
            insert.entity,
            color,
            q_self.contains(insert.entity),
        );
    }
}

// Raw counterpart: no theme lookup, the color propagates as given.
pub(crate) fn on_changed_inheritable_text_color(
    insert: On<Insert, InheritableTextColor>,
    q_color: Query<&InheritableTextColor>,
    q_self: Query<(), SelfColorFilter>,
    mut commands: Commands,
) {
    if let Ok(color) = q_color.get(insert.entity) {
        apply_inheritable_color(
            &mut commands,
            insert.entity,
            color.0,
            q_self.contains(insert.entity),
        );
    }
}
