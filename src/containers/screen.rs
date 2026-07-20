//! BSN scene function for the full-screen root surface.
use bevy_picking::Pickable;
use bevy_scene::{Scene, bsn};
use bevy_text::FontWeight;
use bevy_ui::{AlignItems, Display, FlexDirection, Node, PositionType, percent, px};

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
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            row_gap: size::GAP,
            padding: size::PAD,
        }
        Pickable::IGNORE
        InheritableThemeTextColor(tokens::TEXT_MAIN)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
    }
}
