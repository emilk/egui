//! Opinionated 2D math library for building GUIs.
//!
//! Includes vectors, positions, rectangles etc.
//!
//! Conventions (unless otherwise specified):
//!
//! * All angles are in radians
//! * X+ is right and Y+ is down.
//! * (0,0) is left top.
//! * Dimension order is always `x y`
//!
//! ## Integrating with other math libraries.
//! `emath` does not strive to become a general purpose or all-powerful math library.
//!
//! For that, use something else ([`glam`](https://docs.rs/glam), [`nalgebra`](https://docs.rs/nalgebra), …)
//! and enable the `mint` feature flag in `emath` to enable implicit conversion to/from `emath`.
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]
//!

#![allow(clippy::float_cmp)]

use std::ops::{Add, Div, Mul, RangeInclusive, Sub};

// ----------------------------------------------------------------------------

pub mod align;
mod history;
mod numeric;
mod pos2;
mod rect;
mod rect_transform;
mod rot2;
pub mod smart_aim;
mod vec2;

pub use {
    align::{Align, Align2},
    history::History,
    numeric::*,
    pos2::*,
    rect::*,
    rect_transform::*,
    rot2::*,
    vec2::*,
};

// ----------------------------------------------------------------------------

/// Helper trait to implement [`lerp`] and [`remap`].
pub trait One {
    fn one() -> Self;
}

impl One for f32 {
    #[inline(always)]
    fn one() -> Self {
        1.0
    }
}

impl One for f64 {
    #[inline(always)]
    fn one() -> Self {
        1.0
    }
}

/// Helper trait to implement [`lerp`] and [`remap`].
pub trait Real:
    Copy
    + PartialEq
    + PartialOrd
    + One
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + Mul<Self, Output = Self>
    + Div<Self, Output = Self>
{
}

impl Real for f32 {}

impl Real for f64 {}

// ----------------------------------------------------------------------------

/// Linear interpolation.
///
/// ```
/// # use emath::lerp;
/// assert_eq!(lerp(1.0..=5.0, 0.0), 1.0);
/// assert_eq!(lerp(1.0..=5.0, 0.5), 3.0);
/// assert_eq!(lerp(1.0..=5.0, 1.0), 5.0);
/// assert_eq!(lerp(1.0..=5.0, 2.0), 9.0);
/// ```
#[inline(always)]
pub fn lerp<R, T>(range: RangeInclusive<R>, t: T) -> R
where
    T: Real + Mul<R, Output = R>,
    R: Copy + Add<R, Output = R>,
{
    (T::one() - t) * *range.start() + t * *range.end()
}

/// Where in the range is this value? Returns 0-1 if within the range.
///
/// Returns <0 if before and >1 if after.
///
/// Returns `None` if the input range is zero-width.
///
/// ```
/// # use emath::inverse_lerp;
/// assert_eq!(inverse_lerp(1.0..=5.0, 1.0), Some(0.0));
/// assert_eq!(inverse_lerp(1.0..=5.0, 3.0), Some(0.5));
/// assert_eq!(inverse_lerp(1.0..=5.0, 5.0), Some(1.0));
/// assert_eq!(inverse_lerp(1.0..=5.0, 9.0), Some(2.0));
/// assert_eq!(inverse_lerp(1.0..=1.0, 3.0), None);
/// ```
#[inline]
pub fn inverse_lerp<R>(range: RangeInclusive<R>, value: R) -> Option<R>
where
    R: Copy + PartialEq + Sub<R, Output = R> + Div<R, Output = R>,
{
    let min = *range.start();
    let max = *range.end();
    if min == max {
        None
    } else {
        Some((value - min) / (max - min))
    }
}

/// Linearly remap a value from one range to another,
/// so that when `x == from.start()` returns `to.start()`
/// and when `x == from.end()` returns `to.end()`.
pub fn remap<T>(x: T, from: RangeInclusive<T>, to: RangeInclusive<T>) -> T
where
    T: Real,
{
    crate::emath_assert!(from.start() != from.end());
    let t = (x - *from.start()) / (*from.end() - *from.start());
    lerp(to, t)
}

/// Like [`remap`], but also clamps the value so that the returned value is always in the `to` range.
pub fn remap_clamp<T>(x: T, from: RangeInclusive<T>, to: RangeInclusive<T>) -> T
where
    T: Real,
{
    if from.end() < from.start() {
        return remap_clamp(x, *from.end()..=*from.start(), *to.end()..=*to.start());
    }
    if x <= *from.start() {
        *to.start()
    } else if *from.end() <= x {
        *to.end()
    } else {
        crate::emath_assert!(from.start() != from.end());
        let t = (x - *from.start()) / (*from.end() - *from.start());
        // Ensure no numerical inaccuracies sneak in:
        if T::one() <= t {
            *to.end()
        } else {
            lerp(to, t)
        }
    }
}

/// Round a value to the given number of decimal places.
pub fn round_to_decimals(value: f64, decimal_places: usize) -> f64 {
    // This is a stupid way of doing this, but stupid works.
    format!("{:.*}", decimal_places, value)
        .parse()
        .unwrap_or(value)
}

pub fn format_with_minimum_decimals(value: f64, decimals: usize) -> String {
    format_with_decimals_in_range(value, decimals..=6)
}

pub fn format_with_decimals_in_range(value: f64, decimal_range: RangeInclusive<usize>) -> String {
    let min_decimals = *decimal_range.start();
    let max_decimals = *decimal_range.end();
    crate::emath_assert!(min_decimals <= max_decimals);
    crate::emath_assert!(max_decimals < 100);
    let max_decimals = max_decimals.min(16);
    let min_decimals = min_decimals.min(max_decimals);

    if min_decimals != max_decimals {
        // Ugly/slow way of doing this. TODO(emilk): clean up precision.
        for decimals in min_decimals..max_decimals {
            let text = format!("{:.*}", decimals, value);
            let epsilon = 16.0 * f32::EPSILON; // margin large enough to handle most peoples round-tripping needs
            if almost_equal(text.parse::<f32>().unwrap(), value as f32, epsilon) {
                // Enough precision to show the value accurately - good!
                return text;
            }
        }
        // The value has more precision than we expected.
        // Probably the value was set not by the slider, but from outside.
        // In any case: show the full value
    }
    format!("{:.*}", max_decimals, value)
}

/// Return true when arguments are the same within some rounding error.
///
/// For instance `almost_equal(x, x.to_degrees().to_radians(), f32::EPSILON)` should hold true for all x.
/// The `epsilon`  can be `f32::EPSILON` to handle simple transforms (like degrees -> radians)
/// but should be higher to handle more complex transformations.
pub fn almost_equal(a: f32, b: f32, epsilon: f32) -> bool {
    if a == b {
        true // handle infinites
    } else {
        let abs_max = a.abs().max(b.abs());
        abs_max <= epsilon || ((a - b).abs() / abs_max) <= epsilon
    }
}

#[allow(clippy::approx_constant)]
#[test]
fn test_format() {
    assert_eq!(format_with_minimum_decimals(1_234_567.0, 0), "1234567");
    assert_eq!(format_with_minimum_decimals(1_234_567.0, 1), "1234567.0");
    assert_eq!(format_with_minimum_decimals(3.14, 2), "3.14");
    assert_eq!(format_with_minimum_decimals(3.14, 3), "3.140");
    assert_eq!(
        format_with_minimum_decimals(std::f64::consts::PI, 2),
        "3.14159"
    );
}

#[test]
fn test_almost_equal() {
    for &x in &[
        0.0_f32,
        f32::MIN_POSITIVE,
        1e-20,
        1e-10,
        f32::EPSILON,
        0.1,
        0.99,
        1.0,
        1.001,
        1e10,
        f32::MAX / 100.0,
        // f32::MAX, // overflows in rad<->deg test
        f32::INFINITY,
    ] {
        for &x in &[-x, x] {
            for roundtrip in &[
                |x: f32| x.to_degrees().to_radians(),
                |x: f32| x.to_radians().to_degrees(),
            ] {
                let epsilon = f32::EPSILON;
                assert!(
                    almost_equal(x, roundtrip(x), epsilon),
                    "{} vs {}",
                    x,
                    roundtrip(x)
                );
            }
        }
    }
}

#[test]
fn test_remap() {
    assert_eq!(remap_clamp(1.0, 0.0..=1.0, 0.0..=16.0), 16.0);
    assert_eq!(remap_clamp(1.0, 1.0..=0.0, 16.0..=0.0), 16.0);
    assert_eq!(remap_clamp(0.5, 1.0..=0.0, 16.0..=0.0), 8.0);
}

// ----------------------------------------------------------------------------

/// Extends `f32`, [`Vec2`] etc with `at_least` and `at_most` as aliases for `max` and `min`.
pub trait NumExt {
    /// More readable version of `self.max(lower_limit)`
    #[must_use]
    fn at_least(self, lower_limit: Self) -> Self;

    /// More readable version of `self.min(upper_limit)`
    #[must_use]
    fn at_most(self, upper_limit: Self) -> Self;
}

macro_rules! impl_num_ext {
    ($t: ty) => {
        impl NumExt for $t {
            #[inline(always)]
            fn at_least(self, lower_limit: Self) -> Self {
                self.max(lower_limit)
            }

            #[inline(always)]
            fn at_most(self, upper_limit: Self) -> Self {
                self.min(upper_limit)
            }
        }
    };
}

impl_num_ext!(u8);
impl_num_ext!(u16);
impl_num_ext!(u32);
impl_num_ext!(u64);
impl_num_ext!(u128);
impl_num_ext!(usize);
impl_num_ext!(i8);
impl_num_ext!(i16);
impl_num_ext!(i32);
impl_num_ext!(i64);
impl_num_ext!(i128);
impl_num_ext!(isize);
impl_num_ext!(f32);
impl_num_ext!(f64);
impl_num_ext!(Vec2);
impl_num_ext!(Pos2);

// ----------------------------------------------------------------------------

/// Wrap angle to `[-PI, PI]` range.
pub fn normalized_angle(mut angle: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    angle %= TAU;
    if angle > PI {
        angle -= TAU;
    } else if angle < -PI {
        angle += TAU;
    }
    angle
}

#[test]
fn test_normalized_angle() {
    macro_rules! almost_eq {
        ($left: expr, $right: expr) => {
            let left = $left;
            let right = $right;
            assert!((left - right).abs() < 1e-6, "{} != {}", left, right);
        };
    }

    use std::f32::consts::TAU;
    almost_eq!(normalized_angle(-3.0 * TAU), 0.0);
    almost_eq!(normalized_angle(-2.3 * TAU), -0.3 * TAU);
    almost_eq!(normalized_angle(-TAU), 0.0);
    almost_eq!(normalized_angle(0.0), 0.0);
    almost_eq!(normalized_angle(TAU), 0.0);
    almost_eq!(normalized_angle(2.7 * TAU), -0.3 * TAU);
}

// ----------------------------------------------------------------------------

/// Calculate a lerp-factor for exponential smoothing using a time step.
///
/// * `exponential_smooth_factor(0.90, 1.0, dt)`: reach 90% in 1.0 seconds
/// * `exponential_smooth_factor(0.50, 0.2, dt)`: reach 50% in 0.2 seconds
///
/// Example:
/// ```
/// # use emath::{lerp, exponential_smooth_factor};
/// # let (mut smoothed_value, target_value, dt) = (0.0_f32, 1.0_f32, 0.01_f32);
/// let t = exponential_smooth_factor(0.90, 0.2, dt); // reach 90% in 0.2 seconds
/// smoothed_value = lerp(smoothed_value..=target_value, t);
/// ```
pub fn exponential_smooth_factor(
    reach_this_fraction: f32,
    in_this_many_seconds: f32,
    dt: f32,
) -> f32 {
    1.0 - (1.0 - reach_this_fraction).powf(dt / in_this_many_seconds)
}

// ----------------------------------------------------------------------------

/// An assert that is only active when `emath` is compiled with the `extra_asserts` feature
/// or with the `extra_debug_asserts` feature in debug builds.
#[macro_export]
macro_rules! emath_assert {
    ($($arg: tt)*) => {
        if cfg!(any(
            feature = "extra_asserts",
            all(feature = "extra_debug_asserts", debug_assertions),
        )) {
            assert!($($arg)*);
        }
    }
}
