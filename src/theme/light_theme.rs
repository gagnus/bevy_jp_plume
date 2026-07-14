//! The standard Plume light theme.
use crate::theme::{OklchaArray, ThemeEditablePalette, default_axis_colors};
use bevy_color::Oklcha;

/// Default plume light palette editable inputs
pub fn default_light_palette() -> ThemeEditablePalette {
    ThemeEditablePalette {
        neutrals: OklchaArray {
            hue: 266.0,
            chroma: 0.02,
            l: [0.99, 0.95, 0.80, 0.83, 0.76, 0.70, 0.66],
        },
        accent: OklchaArray {
            hue: 255.4,
            chroma: 0.1594,
            l: [0.61, 0.55, 0.50, 0.45],
        },
        text: OklchaArray {
            hue: 266.0,
            chroma: 0.0014,
            l: [0.07, 0.14],
        },
        contrast: Oklcha::new(1.0, 0.0, 0.0, 1.0),
        axes: default_axis_colors(),
        disabled_text_alpha_modifier: 0.4,
    }
}
