//! BSN scene function for displaying a plain text string in the correct font.
use bevy_app::PropagateOver;
use bevy_scene::{Scene, bsn};
use bevy_text::{FontFeatureTag, FontFeatures, FontSourceTemplate, FontWeight, TextFont};
use bevy_ui::widget::Text;

use crate::{
    constants::{FaIcon, fonts, size},
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

/// A caption in small-caps, whatever the input casing.
///
/// Pins the font face and size, since `TextFont` is set as a whole and the
/// features cannot be inherited alongside them. Text color is still themed.
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

/// A FontAwesome icon, drawn in the face its glyph belongs to.
pub fn fa_icon(icon: FaIcon) -> impl Scene {
    let glyph = icon.glyph();
    let font_path = icon.face().font_path();
    bsn! {
        Text(glyph)
        PropagateOver<TextFont>
        ThemedText
        TextFont {
            font: FontSourceTemplate::Handle(font_path),
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
