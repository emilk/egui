//! Color conversions and types.
//!
//! This crate is built for the wants and needs of [`egui`](https://github.com/emilk/egui/).
//!
//! If you want an actual _good_ color crate, use [`color`](https://crates.io/crates/color) instead.
//!
//! If you want a compact color representation, use [`Color32`].
//! If you want to manipulate RGBA colors in linear space use [`Rgba`].
//! If you want to manipulate colors in a way closer to how humans think about colors, use [`HsvaGamma`].
//!
//! ## Conventions
//! The word "gamma" or "srgb" is used to refer to values in the non-linear space defined by
//! [the sRGB transfer function](https://en.wikipedia.org/wiki/SRGB).
//! We use `u8` for anything in the "gamma" space.
//!
//! We use `f32` in 0-1 range for anything in the linear space.
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]
//!

#![expect(clippy::wrong_self_convention)]

#[cfg(feature = "cint")]
mod cint_impl;

mod color32;
pub use color32::*;

mod hsva_gamma;
pub use hsva_gamma::*;

mod hsva;
pub use hsva::*;

#[cfg(feature = "color-hex")]
mod hex_color_macro;
#[cfg(feature = "color-hex")]
#[doc(hidden)]
pub use color_hex;

mod rgba;
pub use rgba::*;

mod hex_color_runtime;
pub use hex_color_runtime::*;

// ----------------------------------------------------------------------------
// Color conversion:

impl From<Color32> for Rgba {
    fn from(srgba: Color32) -> Self {
        let [r, g, b, a] = srgba.to_array();
        if a == 0 {
            // Additive, or completely transparent
            Self([
                linear_f32_from_gamma_u8(r),
                linear_f32_from_gamma_u8(g),
                linear_f32_from_gamma_u8(b),
                0.0,
            ])
        } else {
            let a = linear_f32_from_linear_u8(a);
            Self([
                linear_from_gamma(r as f32 / (255.0 * a)) * a,
                linear_from_gamma(g as f32 / (255.0 * a)) * a,
                linear_from_gamma(b as f32 / (255.0 * a)) * a,
                a,
            ])
        }
    }
}

impl From<Rgba> for Color32 {
    fn from(rgba: Rgba) -> Self {
        let [r, g, b, a] = rgba.to_array();
        if a == 0.0 {
            // Additive, or completely transparent
            Self([
                gamma_u8_from_linear_f32(r),
                gamma_u8_from_linear_f32(g),
                gamma_u8_from_linear_f32(b),
                0,
            ])
        } else {
            Self([
                fast_round(gamma_u8_from_linear_f32(r / a) as f32 * a),
                fast_round(gamma_u8_from_linear_f32(g / a) as f32 * a),
                fast_round(gamma_u8_from_linear_f32(b / a) as f32 * a),
                linear_u8_from_linear_f32(a),
            ])
        }
    }
}

/// gamma [0, 255] -> linear [0, 1].
pub fn linear_f32_from_gamma_u8(s: u8) -> f32 {
    if s <= 10 {
        s as f32 / 3294.6
    } else {
        ((s as f32 + 14.025) / 269.025).powf(2.4)
    }
}

/// linear [0, 255] -> linear [0, 1].
/// Useful for alpha-channel.
#[inline(always)]
pub const fn linear_f32_from_linear_u8(a: u8) -> f32 {
    a as f32 / 255.0
}

/// linear [0, 1] -> gamma [0, 255] (clamped).
/// Values outside this range will be clamped to the range.
pub fn gamma_u8_from_linear_f32(l: f32) -> u8 {
    if l <= 0.0 {
        0
    } else if l <= 0.0031308 {
        fast_round(3294.6 * l)
    } else if l <= 1.0 {
        fast_round(269.025 * l.powf(1.0 / 2.4) - 14.025)
    } else {
        255
    }
}

/// linear [0, 1] -> linear [0, 255] (clamped).
/// Useful for alpha-channel.
#[inline(always)]
pub fn linear_u8_from_linear_f32(a: f32) -> u8 {
    fast_round(a * 255.0)
}

const fn fast_round(r: f32) -> u8 {
    (r + 0.5) as _ // rust does a saturating cast since 1.45
}

#[test]
pub fn test_srgba_conversion() {
    for b in 0..=255 {
        let l = linear_f32_from_gamma_u8(b);
        assert!(0.0 <= l && l <= 1.0);
        assert_eq!(gamma_u8_from_linear_f32(l), b);
    }
}

/// gamma [0, 1] -> linear [0, 1] (not clamped).
/// Works for numbers outside this range (e.g. negative numbers).
pub fn linear_from_gamma(gamma: f32) -> f32 {
    if gamma < 0.0 {
        -linear_from_gamma(-gamma)
    } else if gamma <= 0.04045 {
        gamma / 12.92
    } else {
        ((gamma + 0.055) / 1.055).powf(2.4)
    }
}

/// linear [0, 1] -> gamma [0, 1] (not clamped).
/// Works for numbers outside this range (e.g. negative numbers).
pub fn gamma_from_linear(linear: f32) -> f32 {
    if linear < 0.0 {
        -gamma_from_linear(-linear)
    } else if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

// ----------------------------------------------------------------------------

/// Cheap and ugly.
/// Made for graying out disabled `Ui`s.
pub fn tint_color_towards(color: Color32, target: Color32) -> Color32 {
    let [mut r, mut g, mut b, mut a] = color.to_array();

    if a == 0 {
        r /= 2;
        g /= 2;
        b /= 2;
    } else if a < 170 {
        // Cheapish and looks ok.
        // Works for e.g. grid stripes.
        let div = (2 * 255 / a as i32) as u8;
        r = r / 2 + target.r() / div;
        g = g / 2 + target.g() / div;
        b = b / 2 + target.b() / div;
        a /= 2;
    } else {
        r = r / 2 + target.r() / 2;
        g = g / 2 + target.g() / 2;
        b = b / 2 + target.b() / 2;
    }
    Color32::from_rgba_premultiplied(r, g, b, a)
}
