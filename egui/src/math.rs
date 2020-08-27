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
    let t = (x - from.start()) / (from.end() - from.start());
    lerp(to, t)
}

/// Like `remap`, but also clamps the value so that the returned value is always in the `to` range.
pub fn remap_clamp(x: f32, from: RangeInclusive<f32>, to: RangeInclusive<f32>) -> f32 {
    if x <= *from.start() {
        *to.start()
    } else if *from.end() <= x {
        *to.end()
    } else {
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
pub fn round_to_precision(value: f32, decimal_places: usize) -> f32 {
    // This is a stupid way of doing this, but stupid works.
    format!("{:.*}", decimal_places, value)
        .parse()
        .unwrap_or_else(|_| value)
}
