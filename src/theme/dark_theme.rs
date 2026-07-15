//! The standard Plume dark theme.
use crate::theme::{OklchaArray, ThemeEditablePalette, default_axis_colors};
use bevy_color::Oklcha;

/// Default plume dark palette editable inputs
pub fn default_dark_palette() -> ThemeEditablePalette {
    ThemeEditablePalette {
        neutrals: OklchaArray {
            hue: 251.0,
            chroma: 0.015,
            l: [0.10, 0.25, 0.31, 0.38, 0.42, 0.44, 0.46],
        },
        accent: OklchaArray {
            hue: 251.0,
            chroma: 0.18,
            l: [0.54, 0.58, 0.60, 0.75],
        },
        text: OklchaArray {
            hue: 0.0,
            chroma: 0.0,
            l: [1.0, 0.7],
        },
        contrast: Oklcha::new(1.0, 0.0, 0.0, 1.0),
        axes: default_axis_colors(),
        disabled_text_alpha_modifier: 0.2,
    }
}
