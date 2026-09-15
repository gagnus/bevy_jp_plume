//! The standard Plume light theme.
use bevy::color::Oklcha;

use crate::theme::{OklchaArray, ThemeEditablePalette, default_axis_colors};

/// Default light hue, see `default_light_palette`
pub const DEFAULT_LIGHT_HUE: f32 = 250.0;

/// Default light palette does not have a complementary (ie 180 degrees hue shifted) neutral color.
pub const DEFAULT_LIGHT_COMPLEMENTARY_NEUTRAL: bool = false;

/// Default plume light palette editable inputs.
pub fn default_light_palette() -> ThemeEditablePalette {
    // Blue, well clear of the fixed danger hue: a red accent would make a destructive
    // action indistinguishable from the confirm button next to it.
    light_palette(
        DEFAULT_LIGHT_HUE,
        DEFAULT_LIGHT_COMPLEMENTARY_NEUTRAL,
        false,
    )
}

/// Plume light palette editable inputs with given hue.
pub fn light_palette(
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
            chroma: if boosted_neutral_chroma { 0.04 } else { 0.02 },
            l: [0.92, 0.88, 0.82, 0.77, 0.74, 0.71, 0.69],
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
        disabled: OklchaArray {
            hue: neutral_hue,
            chroma: 0.008,
            l: [0.63, 0.78],
        },
        contrast: Oklcha::new(1.0, 0.0, 0.0, 1.0),
        axes: default_axis_colors(),
    }
}
