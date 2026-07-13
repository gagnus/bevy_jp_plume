use bevy_scene::{Scene, bsn};
use bevy_text::FontWeight;
use bevy_ui::{Display, FlexDirection, Node, UiRect, px};

use crate::{
    constants::{fonts, size},
    font_styles::InheritableFont,
    rounded_corners::RoundedCorners,
    theme::{ThemeBackgroundColor, ThemeBorderColor},
    tokens,
};

/// A bordered box for visually grouping related controls; content goes in `Children`.
pub fn group() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            border: UiRect::all(px(1)),
            row_gap: px(4),
            padding: px(6),
            border_radius: {RoundedCorners::All.to_border_radius(4.0)}
        }
        ThemeBackgroundColor(tokens::GROUP_BG)
        ThemeBorderColor(tokens::GROUP_BORDER)
        InheritableFont {
            font: fonts::REGULAR,
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
    }
}
