//! The standard Plume dark theme.
use crate::theme::{OklchaArray, ThemeEditablePalette, default_axis_colors};
use bevy_color::Oklcha;

/// Default plume dark palette editable inputs
pub fn default_dark_palette() -> ThemeEditablePalette {
    dark_palette(120.0)
}

/// Plume dark palette editable inputs with given hue
pub fn dark_palette(hue: f32) -> ThemeEditablePalette {
    ThemeEditablePalette {
        neutrals: OklchaArray {
            hue: hue,
            chroma: 0.015,
            l: [0.10, 0.26, 0.32, 0.38, 0.46, 0.48, 0.50],
        },
        accent: OklchaArray {
            hue: hue,
            chroma: 0.205,
            l: [0.54, 0.58, 0.60, 0.75],
        },
        text: OklchaArray {
            hue: hue,
            chroma: 0.030,
            l: [1.0, 0.7],
        },
        contrast: Oklcha::new(1.0, 0.0, 0.0, 1.0),
        axes: default_axis_colors(),
        disabled_text_alpha_modifier: 0.2,
    }
}
