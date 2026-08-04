//! The standard Plume light theme.
use bevy::color::Oklcha;

use crate::theme::{OklchaArray, ThemeEditablePalette, default_axis_colors};

/// Default plume light palette editable inputs
pub fn default_light_palette() -> ThemeEditablePalette {
    // Blue, well clear of the fixed danger hue: a red accent would make a destructive
    // action indistinguishable from the confirm button next to it.
    light_palette(250.0, false)
}

/// Plume light palette editable inputs with given hue
pub fn light_palette(hue: f32, complementary_neutral: bool) -> ThemeEditablePalette {
    let neutral_hue = if complementary_neutral {
        (hue + 180.0) % 360.0
    } else {
        hue
    };
    ThemeEditablePalette {
        neutrals: OklchaArray {
            hue: neutral_hue,
            chroma: 0.008,
            l: [0.99, 0.88, 0.79, 0.74, 0.69, 0.71, 0.73],
        },
        accent: OklchaArray {
            hue,
            chroma: 0.110,
            l: [0.62, 0.64, 0.66, 0.40],
        },
        text: OklchaArray {
            hue: neutral_hue,
            chroma: 0.044,
            l: [0.15, 0.40],
        },
        contrast: Oklcha::new(1.0, 0.0, 0.0, 1.0),
        axes: default_axis_colors(),
        disabled_text_alpha_modifier: 0.6,
    }
}
