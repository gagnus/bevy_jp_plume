//! The standard Plume dark theme.
use crate::theme::{OklchaArray, ThemeEditablePalette, default_axis_colors};
use bevy_color::Oklcha;

/// Default plume dark palette editable inputs
pub fn default_dark_palette() -> ThemeEditablePalette {
    ThemeEditablePalette {
        neutrals: OklchaArray {
            hue: 251.0,
            chroma: 0.025,
            //    l: [0.2414, 0.287, 0.3373, 0.35, 0.376, 0.399, 0.452],
            l: [0.10, 0.15, 0.20, 0.30, 0.40, 0.425, 0.45],
        },
        accent: OklchaArray {
            hue: 251.0,
            chroma: 0.18,
            l: [0.542, 0.592, 0.642, 0.742],
        },
        text: OklchaArray {
            hue: 0.0,
            chroma: 0.0,
            l: [1.0, 0.75],
        },
        contrast: Oklcha::new(1.0, 0.0, 0.0, 1.0),
        axes: default_axis_colors(),
        disabled_text_alpha_modifier: 0.2,
    }
}
