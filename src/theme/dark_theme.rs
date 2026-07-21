//! The standard Plume dark theme.
use crate::theme::{OklchaArray, ThemeEditablePalette, default_axis_colors};
use bevy_color::Oklcha;

/// Default plume dark palette editable inputs
pub fn default_dark_palette() -> ThemeEditablePalette {
    ThemeEditablePalette {
        neutrals: OklchaArray {
            hue: 240.0,
            chroma: 0.015,
            l: [0.10, 0.20, 0.275, 0.38, 0.42, 0.51, 0.58],
        },
        accent: OklchaArray {
            hue: 240.0,
            chroma: 0.205,
            l: [0.54, 0.58, 0.60, 0.75],
        },
        text: OklchaArray {
            hue: 240.0,
            chroma: 0.030,
            l: [1.0, 0.7],
        },
        contrast: Oklcha::new(1.0, 0.0, 0.0, 1.0),
        axes: default_axis_colors(),
        disabled_text_alpha_modifier: 0.2,
    }
}
