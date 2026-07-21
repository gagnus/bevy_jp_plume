//! The standard Plume light theme.
use crate::theme::{OklchaArray, ThemeEditablePalette, default_axis_colors};
use bevy_color::Oklcha;

/// Default plume light palette editable inputs
pub fn default_light_palette() -> ThemeEditablePalette {
    ThemeEditablePalette {
        neutrals: OklchaArray {
            hue: 240.0,
            chroma: 0.017,
            l: [0.99, 0.84, 0.79, 0.74, 0.69, 0.71, 0.73],
        },
        accent: OklchaArray {
            hue: 240.0,
            chroma: 0.110,
            l: [0.62, 0.64, 0.66, 0.40],
        },
        text: OklchaArray {
            hue: 240.0,
            chroma: 0.044,
            l: [0.1, 0.4],
        },
        contrast: Oklcha::new(1.0, 0.0, 0.0, 1.0),
        axes: default_axis_colors(),
        disabled_text_alpha_modifier: 0.6,
    }
}
