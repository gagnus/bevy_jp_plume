//! Mechanism for specifying which corners of a widget are rounded, used for segmented buttons
//! and control groups.
use bevy::ui::{BorderRadius, CornerRadius, Val};

/// Which corners of a control are rounded, all at the same radius. Only the
/// combinations that make sense for segmented buttons are offered.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum RoundedCorners {
    /// No corners are rounded.
    None,
    #[default]
    /// All corners are rounded.
    All,
    /// Top-left corner is rounded.
    TopLeft,
    /// Top-right corner is rounded.
    TopRight,
    /// Bottom-right corner is rounded.
    BottomRight,
    /// Bottom-left corner is rounded.
    BottomLeft,
    /// Top corners are rounded.
    Top,
    /// Right corners are rounded.
    Right,
    /// Bottom corners are rounded.
    Bottom,
    /// Left corners are rounded.
    Left,
}

impl RoundedCorners {
    /// Convert the `RoundedCorners` to a `BorderRadius` for use in a `Node`.
    pub fn to_border_radius<T: Into<Val>>(&self, radius: T) -> BorderRadius {
        let radius = CornerRadius::circular(radius.into());
        let zero = CornerRadius::ZERO;
        match self {
            RoundedCorners::None => BorderRadius::all(zero),
            RoundedCorners::All => BorderRadius::all(radius),
            RoundedCorners::TopLeft => BorderRadius {
                top_left: radius,
                top_right: zero,
                bottom_right: zero,
                bottom_left: zero,
            },
            RoundedCorners::TopRight => BorderRadius {
                top_left: zero,
                top_right: radius,
                bottom_right: zero,
                bottom_left: zero,
            },
            RoundedCorners::BottomRight => BorderRadius {
                top_left: zero,
                top_right: zero,
                bottom_right: radius,
                bottom_left: zero,
            },
            RoundedCorners::BottomLeft => BorderRadius {
                top_left: zero,
                top_right: zero,
                bottom_right: zero,
                bottom_left: radius,
            },
            RoundedCorners::Top => BorderRadius {
                top_left: radius,
                top_right: radius,
                bottom_right: zero,
                bottom_left: zero,
            },
            RoundedCorners::Right => BorderRadius {
                top_left: zero,
                top_right: radius,
                bottom_right: radius,
                bottom_left: zero,
            },
            RoundedCorners::Bottom => BorderRadius {
                top_left: zero,
                top_right: zero,
                bottom_right: radius,
                bottom_left: radius,
            },
            RoundedCorners::Left => BorderRadius {
                top_left: radius,
                top_right: zero,
                bottom_right: zero,
                bottom_left: radius,
            },
        }
    }
}
