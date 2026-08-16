//! Numeric abstraction letting the value controls bind to any builtin number
//! type. Adapted from egui's `emath::Numeric` (MIT/Apache-2.0).

/// Implemented for all builtin numeric types.
///
/// Values round-trip through `f32` because that is what bevy's [`SliderValue`]
/// holds, so `f64`, `i64`/`u64` and `usize` lose precision beyond 24 bits.
///
/// [`SliderValue`]: bevy::ui_widgets::SliderValue
pub trait Numeric: Clone + Copy + PartialEq + PartialOrd + 'static {
    /// Is this an integer type?
    const INTEGRAL: bool;

    /// Smallest finite value.
    const MIN: Self;

    /// Largest finite value.
    const MAX: Self;

    /// Widen to the `f32` the controls store their value in.
    fn to_f32(self) -> f32;

    /// Narrow back from the controls' `f32`, rounding and saturating.
    fn from_f32(num: f32) -> Self;
}

macro_rules! impl_numeric_float {
    ($t: ident) => {
        impl Numeric for $t {
            const INTEGRAL: bool = false;
            const MIN: Self = $t::MIN;
            const MAX: Self = $t::MAX;

            #[inline(always)]
            fn to_f32(self) -> f32 {
                #[allow(clippy::allow_attributes, trivial_numeric_casts)]
                {
                    self as f32
                }
            }

            #[inline(always)]
            fn from_f32(num: f32) -> Self {
                #[allow(clippy::allow_attributes, trivial_numeric_casts)]
                {
                    num as Self
                }
            }
        }
    };
}

macro_rules! impl_numeric_integer {
    ($t: ident) => {
        impl Numeric for $t {
            const INTEGRAL: bool = true;
            const MIN: Self = $t::MIN;
            const MAX: Self = $t::MAX;

            #[inline(always)]
            fn to_f32(self) -> f32 {
                self as f32
            }

            #[inline(always)]
            fn from_f32(num: f32) -> Self {
                // `round`, not a bare cast: truncation flips 2→3 at 3.0 rather
                // than 2.5, and is asymmetric either side of zero.
                num.round() as Self
            }
        }
    };
}

impl_numeric_float!(f32);
impl_numeric_float!(f64);
impl_numeric_integer!(i8);
impl_numeric_integer!(u8);
impl_numeric_integer!(i16);
impl_numeric_integer!(u16);
impl_numeric_integer!(i32);
impl_numeric_integer!(u32);
impl_numeric_integer!(i64);
impl_numeric_integer!(u64);
impl_numeric_integer!(isize);
impl_numeric_integer!(usize);
