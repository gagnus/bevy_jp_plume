//! BSN scene function for the full-screen root surface.
use bevy::picking::Pickable;
use bevy::scene::{Scene, bsn};
use bevy::text::FontWeight;
use bevy::ui::{
    AlignItems, Display, FlexDirection, LayoutConfig, Node, PositionType, Val, percent,
};

use crate::{
    constants::{fonts, size},
    font_styles::InheritableFont,
    theme::InheritableThemeTextColor,
    tokens,
};

/// Transparent, padded column filling the viewport, establishing the standard
/// [`InheritableFont`] and text color so bare text works at root scope.
///
/// [`Pickable::IGNORE`], so empty areas don't swallow picks meant for the scene
/// behind it; children keep their own picking.
pub fn screen() -> impl Scene {
    bsn! {
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
            padding: size::PAD,
        }
        Pickable::IGNORE
        InheritableThemeTextColor(tokens::TEXT_DIM)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
        LayoutConfig {
            use_rounding: false,
        }
    }
}
