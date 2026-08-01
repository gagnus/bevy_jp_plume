//! Theme-driven styling components: the markers a retained scene puts on an
//! entity to have `ThemePlugin` color it.
use bevy::app::{Inherited, Propagate, PropagateOver, PropagateStop};
use bevy::color::{Alpha, Color, Luminance, Srgba};
use bevy::ecs::{
    component::Component,
    hierarchy::ChildOf,
    lifecycle::Insert,
    observer::On,
    query::{Changed, Without},
    reflect::ReflectComponent,
    system::{Commands, Query, Res},
};
use bevy::reflect::{Reflect, prelude::ReflectDefault};
use bevy::text::TextColor;
use bevy::ui::{
    BackgroundColor, BackgroundGradient, BorderColor, BoxShadow, ColorStop, Gradient,
    InterpolationColorSpace, LinearGradient, Val, percent,
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

// The standard luminance adjust (+-) for an active control's [`ThemeBackgroundGradient`].
pub(crate) const GRADIENT_AMOUNT: f32 = 0.05;

/// Opt-in marker: the entity's themed fills render flat (gradient amount 0).
/// Honored by the gradient-drawn elements (button, checkbox, radio, toggle,
/// slider thumb, section header); everything else ignores it.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct Flat;

/// Opt-in marker: the entity does not repond to hover and pressed. Currently only
/// respected by `PlumeButton`.
#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct Inert;

// Fills an entity's background with a gentle top-to-bottom gradient derived from a
// theme token — plume-internal (the gradient look belongs to plume's own controls).
#[derive(Component, Clone, Default)]
#[require(BackgroundGradient)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub(crate) struct ThemeBackgroundGradient(pub ThemeToken, pub f32);

// Build the vertical gradient a `ThemeBackgroundGradient` resolves to.
pub(crate) fn theme_background_gradient(base: Color, amount: f32) -> BackgroundGradient {
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
        Srgba::BLACK.with_alpha(0.4).into(),
        size::GAP / 8.0,
        size::GAP / 4.0,
        Val::ZERO,
        size::GAP / 2.0,
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
// TODO: This is necessary because an entity with Propagate doesn't update itself, only its
// descendants.
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

pub(crate) fn on_changed_background_token(
    insert: On<Insert, ThemeBackgroundToken>,
    mut q_background: Query<
        (&mut BackgroundColor, &ThemeBackgroundToken),
        Changed<ThemeBackgroundToken>,
    >,
    theme: Res<UiTheme>,
) {
    if let Ok((mut bg, theme_bg)) = q_background.get_mut(insert.entity) {
        bg.0 = theme.color(&theme_bg.0);
    }
}

pub(crate) fn on_changed_background_slot(
    insert: On<Insert, ThemeBackgroundSlot>,
    mut q_background: Query<
        (&mut BackgroundColor, &ThemeBackgroundSlot),
        Changed<ThemeBackgroundSlot>,
    >,
    theme: Res<UiTheme>,
) {
    if let Ok((mut bg, theme_bg)) = q_background.get_mut(insert.entity) {
        bg.0 = theme.palette(theme_bg.0);
    }
}

pub(crate) fn on_changed_gradient(
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
    bevy::ecs::query::Or<(
        bevy::ecs::query::With<bevy::ui::widget::Text>,
        bevy::ecs::query::With<bevy::text::EditableText>,
    )>,
    Without<ThemeTextSlot>,
    Without<ThemeTextToken>,
);

pub(crate) fn apply_inheritable_color(
    commands: &mut Commands,
    entity: bevy::ecs::entity::Entity,
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
