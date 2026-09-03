//! The standard Plume dark theme.
use bevy::color::Oklcha;

use crate::theme::{OklchaArray, ThemeEditablePalette, default_axis_colors};

/// Default dark hue, see `default_dark_palette`
pub const DEFAULT_DARK_HUE: f32 = 120.0;

/// Default dark palette has a complementary (ie 180 degrees hue shifted) neutral color.
pub const DEFAULT_DARK_COMPLEMENTARY_NEUTRAL: bool = true;

/// Default plume dark palette editable inputs.
pub fn default_dark_palette() -> ThemeEditablePalette {
    dark_palette(DEFAULT_DARK_HUE, DEFAULT_DARK_COMPLEMENTARY_NEUTRAL, false)
}

/// Plume dark palette editable inputs with given hue.
pub fn dark_palette(
    hue: f32,
    complementary_neutral: bool,
    boosted_neutral_chroma: bool,
) -> ThemeEditablePalette {
    let neutral_hue = if complementary_neutral {
        (hue + 180.0) % 360.0
    } else {
        hue
    };
    ThemeEditablePalette {
        neutrals: OklchaArray {
            hue: neutral_hue,
            chroma: if boosted_neutral_chroma { 0.035 } else { 0.015 },
            l: [0.24, 0.28, 0.32, 0.37, 0.42, 0.45, 0.48],
        },
        accent: OklchaArray {
            hue,
            chroma: 0.205,
            l: [0.54, 0.57, 0.60, 0.75],
        },
        text: OklchaArray {
            hue: neutral_hue,
            chroma: 0.030,
            l: [0.85, 0.70],
        },
        disabled: OklchaArray {
            hue: neutral_hue,
            chroma: 0.015,
            l: [0.53, 0.38],
        },
        contrast: Oklcha::new(1.0, 0.0, 0.0, 1.0),
        axes: default_axis_colors(),
    }
}
