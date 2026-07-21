//! The standard Plume light theme.
use crate::theme::{OklchaArray, ThemeEditablePalette, default_axis_colors};
use bevy_color::Oklcha;

/// Default plume light palette editable inputs
pub fn default_light_palette() -> ThemeEditablePalette {
    ThemeEditablePalette {
        neutrals: OklchaArray {
            hue: 240.0,
            chroma: 0.017,
            l: [0.99, 0.92, 0.83, 0.77, 0.73, 0.75, 0.77],
        },
        accent: OklchaArray {
            hue: 240.0,
            chroma: 0.110,
            l: [0.62, 0.72, 0.82, 0.40],
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
