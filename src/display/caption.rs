//! BSN scene function for displaying a plain text string in the correct font.
use bevy::app::PropagateOver;
use bevy::color::Color;
use bevy::scene::{Scene, bsn, template_value};
use bevy::text::{FontFeatureTag, FontFeatures, FontSourceTemplate, TextColor};
use bevy::ui::widget::Text;

use crate::{
    constants::FaIcon,
    font_styles::InheritableFont,
    theme::{ThemeSlot, ThemeTextSlot, ThemedText},
};

/// A caption within, say, a button using inherited color.
pub fn caption(text: impl Into<String>) -> impl Scene {
    bsn! {
        Text(text)
        ThemedText
    }
}

/// A caption in a fixed raw color, for one-offs outside the theme.
pub fn caption_color(text: impl Into<String>, color: Color) -> impl Scene {
    bsn! {
        Text(text)
        ThemedText
        template_value(TextColor(color))
        // Keeps the inherited themed color from overwriting it.
        PropagateOver<TextColor>
    }
}

/// A caption colored from a theme slot instead of the inherited color.
pub fn caption_slot(text: impl Into<String>, slot: ThemeSlot) -> impl Scene {
    bsn! {
        Text(text)
        ThemedText
        template_value(ThemeTextSlot(slot))
    }
}

/// A caption in small-caps, whatever the input casing; face and size inherit.
pub fn caption_small_caps(text: impl Into<String>) -> impl Scene {
    bsn! {
        Text(text)
        ThemedText
        InheritableFont {
            font_features: FontFeatures::from([
                FontFeatureTag::SMALL_CAPS,
                FontFeatureTag::CAPS_TO_SMALL_CAPS,
            ]),
        }
    }
}

/// A FontAwesome icon, drawn in the face its glyph belongs to; size inherits,
/// so icons track the surrounding text.
pub fn fa_icon(icon: FaIcon) -> impl Scene {
    let glyph = icon.glyph();
    let font_path = icon.face().font_path();
    bsn! {
        Text(glyph)
        ThemedText
        InheritableFont {
            font: FontSourceTemplate::Handle(font_path),
        }
    }
}
