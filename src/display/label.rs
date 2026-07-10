//! BSN scene function for displaying a plain text string in the correct font.
use bevy_app::PropagateOver;
use bevy_scene::{Scene, bsn};
use bevy_text::{FontSourceTemplate, FontWeight, TextFont};
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

/// A text label.
pub fn label(text: impl Into<String>) -> impl Scene {
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
