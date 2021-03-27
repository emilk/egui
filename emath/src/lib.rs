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

#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::explicit_into_iter_loop,
    clippy::filter_map_next,
    clippy::fn_params_excessive_bools,
    clippy::if_let_mutex,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_pass_by_value,
    clippy::option_option,
    clippy::pub_enum_variant_names,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::string_add_assign,
    clippy::string_add,
    clippy::string_to_string,
    clippy::todo,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::verbose_file_reads,
    future_incompatible,
    missing_crate_level_docs,
    nonstandard_style,
    rust_2018_idioms
)]
#![allow(clippy::manual_range_contains)]

use std::ops::{Add, Div, Mul, RangeInclusive, Sub};

// ----------------------------------------------------------------------------

pub mod align;
mod numeric;
mod pos2;
mod rect;
mod rect_transform;
mod rot2;
pub mod smart_aim;
mod vec2;

pub use {
    align::{Align, Align2},
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
    fn one() -> Self {
        1.0
    }
}
impl One for f64 {
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
pub fn lerp<R, T>(range: RangeInclusive<R>, t: T) -> R
where
    T: Real + Mul<R, Output = R>,
    R: Copy + Add<R, Output = R>,
{
    (T::one() - t) * *range.start() + t * *range.end()
}

/// Linearly remap a value from one range to another,
/// so that when `x == from.start()` returns `to.start()`
/// and when `x == from.end()` returns `to.end()`.
pub fn remap<T>(x: T, from: RangeInclusive<T>, to: RangeInclusive<T>) -> T
where
    T: Real,
{
    #![allow(clippy::float_cmp)]
    debug_assert!(from.start() != from.end());
    let t = (x - *from.start()) / (*from.end() - *from.start());
    lerp(to, t)
}

/// Like [`remap`], but also clamps the value so that the returned value is always in the `to` range.
pub fn remap_clamp<T>(x: T, from: RangeInclusive<T>, to: RangeInclusive<T>) -> T
where
    T: Real,
{
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
        let t = (x - *from.start()) / (*from.end() - *from.start());
        // Ensure no numerical inaccuracies sneak in:
        if T::one() <= t {
            *to.end()
        } else {
            lerp(to, t)
        }
    }
}

/// Returns `range.start()` if `x <= range.start()`,
/// returns `range.end()` if `x >= range.end()`
/// and returns `x` elsewhen.
#[deprecated = "Use f32::clamp instead"]
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
    debug_assert!(min_decimals <= max_decimals);
    debug_assert!(max_decimals < 100);
    let max_decimals = max_decimals.min(16);
    let min_decimals = min_decimals.min(max_decimals);

    if min_decimals == max_decimals {
        format!("{:.*}", max_decimals, value)
    } else {
        // Ugly/slow way of doing this. TODO: clean up precision.
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
        format!("{:.*}", max_decimals, value)
    }
}

/// Return true when arguments are the same within some rounding error.
///
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

#[allow(clippy::float_cmp)]
#[test]
fn test_remap() {
    assert_eq!(remap_clamp(1.0, 0.0..=1.0, 0.0..=16.0), 16.0);
    assert_eq!(remap_clamp(1.0, 1.0..=0.0, 16.0..=0.0), 16.0);
    assert_eq!(remap_clamp(0.5, 1.0..=0.0, 16.0..=0.0), 8.0);
}

// ----------------------------------------------------------------------------

/// Extends `f32`, `Vec2` etc with `at_least` and `at_most` as aliases for `max` and `min`.
pub trait NumExt {
    /// More readable version of `self.max(lower_limit)`
    fn at_least(self, lower_limit: Self) -> Self;

    /// More readable version of `self.min(upper_limit)`
    fn at_most(self, upper_limit: Self) -> Self;
}

macro_rules! impl_num_ext {
    ($t: ty) => {
        impl NumExt for $t {
            fn at_least(self, lower_limit: Self) -> Self {
                self.max(lower_limit)
            }
            fn at_most(self, upper_limit: Self) -> Self {
                self.min(upper_limit)
            }
        }
    };
}

impl_num_ext!(f32);
impl_num_ext!(f64);
impl_num_ext!(usize);
impl_num_ext!(Vec2);
impl_num_ext!(Pos2);
