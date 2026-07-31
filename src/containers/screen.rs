//! BSN scene function for the full-screen root surface.
use bevy::ecs::name::Name;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::picking::Pickable;
use bevy::scene::{Scene, bsn};
use bevy::ui::{
    AlignItems, Display, FlexDirection, LayoutConfig, Node, PositionType, Val, percent,
};

use crate::{
    constants::size, font_styles::InheritableFont, theme::InheritableThemeTextToken, tokens,
};

/// Transparent, full-bleed column filling the viewport, establishing the standard
/// [`InheritableFont`] and text color so bare text works at root scope.
///
/// Unpadded: the viewport root is a canvas, so edge-anchored content sits flush.
/// Add `.padding(size::PAD)` for a surface that wants an inset.
///
/// Carries a [`TabGroup`], so every control inside is Tab-reachable without the
/// app adding one — the same scope [`PlumeDialog`](crate::retained::PlumeDialog)
/// provides.
///
/// [`Pickable::IGNORE`], so empty areas don't swallow picks meant for the scene
/// behind it; children keep their own picking.
pub fn screen() -> impl Scene {
    bsn! {
        Name("PlumeScreen")
        TabGroup::new(0)
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Stretch,
            position_type: PositionType::Absolute,
            left: Val::ZERO,
            top: Val::ZERO,
            width: percent(100),
            height: percent(100),
            row_gap: size::GAP,
        }
        Pickable::IGNORE
        InheritableThemeTextToken(tokens::TEXT_DIM)
        // All-inherit: resolves to the standard face and size at root scope.
        InheritableFont
        LayoutConfig {
            use_rounding: false,
        }
    }
}
