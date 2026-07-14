//! The standard Plume dark theme.
use crate::theme::{
    EditablePalette, OklchaArray, ThemeProps, build_theme, default_axis_colors, default_token_slots,
};
use bevy_color::Oklcha;

/// Default plume dark palette
pub fn create_dark_theme() -> ThemeProps {
    build_theme(&default_dark_palette().resolve(), default_token_slots())
}

/// Default plume dark palette editable inputs
pub fn default_dark_palette() -> EditablePalette {
    EditablePalette {
        neutrals: OklchaArray {
            hue: 293.0,
            chroma: 0.008,
            //    l: [0.2414, 0.287, 0.3373, 0.35, 0.376, 0.399, 0.452],
            l: [0.16, 0.22, 0.25, 0.30, 0.35, 0.40, 0.45],
        },
        accent: OklchaArray {
            hue: 293.29,
            chroma: 0.2088,
            l: [0.542, 0.592, 0.642, 0.742],
        },
        text: OklchaArray {
            hue: 286.37,
            chroma: 0.0014,
            l: [1.0, 0.7607],
        },
        contrast: Oklcha::new(1.0, 0.0, 0.0, 1.0),
        axes: default_axis_colors(),
        disabled_text_alpha_modifier: 0.2,
    }
}
