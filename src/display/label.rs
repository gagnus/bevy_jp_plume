//! BSN scene function for displaying a plain text string in the correct font.
use bevy_app::PropagateOver;
use bevy_scene::{Scene, bsn};
use bevy_text::{FontFeatureTag, FontFeatures, FontSourceTemplate, FontWeight, TextFont};
use bevy_ui::widget::Text;

use crate::{
    constants::{fonts, size},
    theme::{ThemeTextColor, ThemedText},
    tokens,
};

/// A caption within, say, a button.
pub fn caption(text: impl Into<String>) -> impl Scene {
    bsn! {
        Text(text)
        ThemedText
    }
}

/// A caption rendered in small-caps via the font's OpenType features.
///
/// Enables both `smcp` (lowercase → small-caps) and `c2sc` (uppercase → small-
/// caps), so the text renders as small-caps regardless of the input casing.
///
/// Unlike [`caption`], this pins the font face and size (they cannot be
/// inherited alongside the features, since `TextFont` is set as a whole), so the
/// glyphs match the standard caption font. Text color is still themed.
pub fn caption_small_caps(text: impl Into<String>) -> impl Scene {
    bsn! {
        Text(text)
        ThemedText
        TextFont {
            font: FontSourceTemplate::Handle(fonts::REGULAR),
            font_size: size::MEDIUM_FONT,
            font_features: FontFeatures::from([
                FontFeatureTag::SMALL_CAPS,
                FontFeatureTag::CAPS_TO_SMALL_CAPS,
            ]),
        }
        PropagateOver<TextFont>
    }
}

/// A caption within, say, a button.
pub fn fa_icon_solid(icon_text: &'static str) -> impl Scene {
    bsn! {
        Text(icon_text)
        PropagateOver<TextFont>
        ThemedText
        TextFont {
            font: FontSourceTemplate::Handle(fonts::FA_SOLID),
            font_size: size::MEDIUM_FONT,
        }
    }
}

/// A caption within, say, a button.
pub fn fa_icon_regular(icon_text: &'static str) -> impl Scene {
    bsn! {
        Text(icon_text)
        PropagateOver<TextFont>
        ThemedText
        TextFont {
            font: FontSourceTemplate::Handle(fonts::FA_REGULAR),
            font_size: size::MEDIUM_FONT,
        }
    }
}

/// A text label.
pub fn label_bright(text: impl Into<String>) -> impl Scene {
    bsn! {
        Text(text)
        TextFont {
            font: FontSourceTemplate::Handle(fonts::REGULAR),
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
        PropagateOver<TextFont>
        ThemeTextColor(tokens::TEXT_MAIN)
    }
}

/// A text label with a dimmed color.
pub fn label_dim(text: impl Into<String>) -> impl Scene {
    bsn! {
        Text(text)
        TextFont {
            font: FontSourceTemplate::Handle(fonts::REGULAR),
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
        PropagateOver<TextFont>
        ThemeTextColor(tokens::TEXT_DIM)
    }
}
