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

/// Full-screen root surface for top-level immediate-mode content: a transparent,
/// padded column (like a dialog body sized to the viewport) that establishes the
/// standard font and text color, so bare `caption`/text works at root scope where
/// there is otherwise no [`InheritableFont`]/text-color ancestor.
///
/// Fills the viewport and stretches children to its width; content goes in
/// `Children`. It carries no background — it's an overlay over whatever renders
/// behind it — and is [`Pickable::IGNORE`] so empty areas don't swallow picks
/// meant for the scene below (children keep their own picking).
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
