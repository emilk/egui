//! Vectors, positions, rectangles etc.

use std::ops::{Add, Mul, RangeInclusive};

// ----------------------------------------------------------------------------

mod movement_tracker;
mod pos2;
mod rect;
pub mod smart_aim;
mod vec2;

pub use {movement_tracker::*, pos2::*, rect::*, vec2::*};

// ----------------------------------------------------------------------------

/// Linear interpolation.
pub fn lerp<T>(range: RangeInclusive<T>, t: f32) -> T
where
    f32: Mul<T, Output = T>,
    T: Add<T, Output = T> + Copy,
{
    (1.0 - t) * *range.start() + t * *range.end()
}

/// Linearly remap a value from one range to another,
/// so that when `x == from.start()` returns `to.start()`
/// and when `x == from.end()` returns `to.end()`.
pub fn remap(x: f32, from: RangeInclusive<f32>, to: RangeInclusive<f32>) -> f32 {
    #![allow(clippy::float_cmp)]
    debug_assert!(from.start() != from.end());
    let t = (x - from.start()) / (from.end() - from.start());
    lerp(to, t)
}

/// Like `remap`, but also clamps the value so that the returned value is always in the `to` range.
pub fn remap_clamp(x: f32, from: RangeInclusive<f32>, to: RangeInclusive<f32>) -> f32 {
    #![allow(clippy::float_cmp)]
    if from.end() < from.start() {
        return remap_clamp(x, *from.end()..=*from.start(), *to.end()..=*to.start());
    }
    if x <= *from.start() {
        *to.start()
    } else if *from.end() <= x {
        *to.end()
    } else {
        debug_assert!(from.start() != from.end());
        let t = (x - from.start()) / (from.end() - from.start());
        // Ensure no numerical inaccuracies sneak in:
        if 1.0 <= t {
            *to.end()
        } else {
            lerp(to, t)
        }
    }
}

/// Returns `range.start()` if `x <= range.start()`,
/// returns `range.end()` if `x >= range.end()`
/// and returns `x` elsewhen.
pub fn clamp<T>(x: T, range: RangeInclusive<T>) -> T
where
    T: Copy + PartialOrd,
{
    debug_assert!(range.start() <= range.end());
    if x <= *range.start() {
        *range.start()
    } else if *range.end() <= x {
        *range.end()
    } else {
        x
    }
}

/// For t=[0,1], returns [0,1] with a derivate of zero at both ends
pub fn ease_in_ease_out(t: f32) -> f32 {
    3.0 * t * t - 2.0 * t * t * t
}

/// The circumference of a circle divided by its radius.
///
/// Represents one turn in radian angles. Equal to `2 * pi`.
///
/// See <https://tauday.com/>
pub const TAU: f32 = 2.0 * std::f32::consts::PI;

/// Round a value to the given number of decimal places.
pub fn round_to_precision_f32(value: f32, decimal_places: usize) -> f32 {
    // This is a stupid way of doing this, but stupid works.
    format!("{:.*}", decimal_places, value)
        .parse()
        .unwrap_or_else(|_| value)
}

/// Round a value to the given number of decimal places.
pub fn round_to_precision(value: f64, decimal_places: usize) -> f64 {
    // This is a stupid way of doing this, but stupid works.
    format!("{:.*}", decimal_places, value)
        .parse()
        .unwrap_or_else(|_| value)
}

pub fn format_with_minimum_precision(value: f32, precision: usize) -> String {
    let text = format!("{:.*}", precision, value);
    let epsilon = 16.0 * f32::EPSILON; // margin large enough to handle most peoples round-tripping needs
    if almost_equal(text.parse::<f32>().unwrap(), value, epsilon) {
        // Enough precision to show the value accurately - good!
        text
    } else {
        // The value has more precision than we expected.
        // Probably the value was set not by the slider, but from outside.
        // In any case: show the full value
        value.to_string()
    }
}

/// Should return true when arguments are the same within some rounding error.
/// For instance `almost_equal(x, x.to_degrees().to_radians(), f32::EPSILON)` should hold true for all x.
/// The `epsilon`  can be `f32::EPSILON` to handle simple transforms (like degrees -> radians)
/// but should be higher to handle more complex transformations.
pub fn almost_equal(a: f32, b: f32, epsilon: f32) -> bool {
    #![allow(clippy::float_cmp)]

    if a == b {
        true // handle infinites
    } else {
        let abs_max = a.abs().max(b.abs());
        abs_max <= epsilon || ((a - b).abs() / abs_max) <= epsilon
    }
}

#[test]
fn test_format() {
    assert_eq!(format_with_minimum_precision(1_234_567.0, 0), "1234567");
    assert_eq!(format_with_minimum_precision(1_234_567.0, 1), "1234567.0");
    assert_eq!(format_with_minimum_precision(3.14, 2), "3.14");
    assert_eq!(
        format_with_minimum_precision(std::f32::consts::PI, 2),
        "3.1415927"
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
