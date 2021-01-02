//! Color conversions and types.

use emath::clamp;

/// This format is used for space-efficient color representation (32 bits).
///
/// Instead of manipulating this directly it is often better
/// to first convert it to either [`Rgba`] or [`Hsva`].
///
/// Internally this uses 0-255 gamma space `sRGBA` color with premultiplied alpha.
/// Alpha channel is in linear space.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct Color32(pub(crate) [u8; 4]);

impl std::ops::Index<usize> for Color32 {
    type Output = u8;
    fn index(&self, index: usize) -> &u8 {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for Color32 {
    fn index_mut(&mut self, index: usize) -> &mut u8 {
        &mut self.0[index]
    }
}

#[deprecated = "Replaced by Color32::from_rgb… family of functions."]
pub const fn srgba(r: u8, g: u8, b: u8, a: u8) -> Color32 {
    Color32::from_rgba_premultiplied(r, g, b, a)
}

impl Color32 {
    pub const TRANSPARENT: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 0);
    pub const BLACK: Color32 = Color32::from_rgb(0, 0, 0);
    pub const LIGHT_GRAY: Color32 = Color32::from_rgb(220, 220, 220);
    pub const GRAY: Color32 = Color32::from_rgb(160, 160, 160);
    pub const WHITE: Color32 = Color32::from_rgb(255, 255, 255);
    pub const RED: Color32 = Color32::from_rgb(255, 0, 0);
    pub const YELLOW: Color32 = Color32::from_rgb(255, 255, 0);
    pub const GREEN: Color32 = Color32::from_rgb(0, 255, 0);
    pub const BLUE: Color32 = Color32::from_rgb(0, 0, 255);
    pub const LIGHT_BLUE: Color32 = Color32::from_rgb(140, 160, 255);
    pub const GOLD: Color32 = Color32::from_rgb(255, 215, 0);

    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b, 255])
    }

    pub const fn from_rgb_additive(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b, 0])
    }

    /// From `sRGBA` with premultiplied alpha.
    pub const fn from_rgba_premultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([r, g, b, a])
    }

    /// From `sRGBA` WITHOUT premultiplied alpha.
    pub fn from_rgba_unmultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        if a == 255 {
            Self::from_rgba_premultiplied(r, g, b, 255) // common-case optimization
        } else if a == 0 {
            Self::TRANSPARENT // common-case optimization
        } else {
            let r_lin = linear_from_gamma_byte(r);
            let g_lin = linear_from_gamma_byte(g);
            let b_lin = linear_from_gamma_byte(b);
            let a_lin = linear_from_alpha_byte(a);

            let r = gamma_byte_from_linear(r_lin * a_lin);
            let g = gamma_byte_from_linear(g_lin * a_lin);
            let b = gamma_byte_from_linear(b_lin * a_lin);

            Self::from_rgba_premultiplied(r, g, b, a)
        }
    }

    #[deprecated = "Use from_rgb(..), from_rgba_premultiplied(..) or from_srgba_unmultiplied(..)"]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([r, g, b, a])
    }

    pub const fn from_gray(l: u8) -> Self {
        Self([l, l, l, 255])
    }

    pub const fn from_black_alpha(a: u8) -> Self {
        Self([0, 0, 0, a])
    }

    pub fn from_white_alpha(a: u8) -> Self {
        Rgba::from_white_alpha(linear_from_alpha_byte(a)).into()
    }

    pub const fn from_additive_luminance(l: u8) -> Self {
        Self([l, l, l, 0])
    }

    pub fn is_opaque(&self) -> bool {
        self.a() == 255
    }

    pub fn r(&self) -> u8 {
        self.0[0]
    }
    pub fn g(&self) -> u8 {
        self.0[1]
    }
    pub fn b(&self) -> u8 {
        self.0[2]
    }
    pub fn a(&self) -> u8 {
        self.0[3]
    }

    /// Returns an opaque version of self
    pub fn to_opaque(self) -> Self {
        Rgba::from(self).to_opaque().into()
    }

    /// Premultiplied RGBA
    pub fn to_array(&self) -> [u8; 4] {
        [self.r(), self.g(), self.b(), self.a()]
    }

    /// Premultiplied RGBA
    pub fn to_tuple(&self) -> (u8, u8, u8, u8) {
        (self.r(), self.g(), self.b(), self.a())
    }

    /// Multiply with 0.5 to make color half as opaque.
    pub fn linear_multiply(self, factor: f32) -> Color32 {
        debug_assert!(0.0 <= factor && factor <= 1.0);
        // As an unfortunate side-effect of using premultiplied alpha
        // we need a somewhat expensive conversion to linear space and back.
        Rgba::from(self).multiply(factor).into()
    }
}

// ----------------------------------------------------------------------------

/// 0-1 linear space `RGBA` color with premultiplied alpha.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct Rgba(pub(crate) [f32; 4]);

impl std::ops::Index<usize> for Rgba {
    type Output = f32;
    fn index(&self, index: usize) -> &f32 {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for Rgba {
    fn index_mut(&mut self, index: usize) -> &mut f32 {
        &mut self.0[index]
    }
}

impl Rgba {
    pub const TRANSPARENT: Rgba = Rgba::from_rgba_premultiplied(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Rgba = Rgba::from_rgb(0.0, 0.0, 0.0);
    pub const WHITE: Rgba = Rgba::from_rgb(1.0, 1.0, 1.0);
    pub const RED: Rgba = Rgba::from_rgb(1.0, 0.0, 0.0);
    pub const GREEN: Rgba = Rgba::from_rgb(0.0, 1.0, 0.0);
    pub const BLUE: Rgba = Rgba::from_rgb(0.0, 0.0, 1.0);

    pub const fn from_rgba_premultiplied(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self([r, g, b, a])
    }

    pub const fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self([r, g, b, 1.0])
    }

    pub const fn from_gray(l: f32) -> Self {
        Self([l, l, l, 1.0])
    }

    pub fn from_luminance_alpha(l: f32, a: f32) -> Self {
        debug_assert!(0.0 <= l && l <= 1.0);
        debug_assert!(0.0 <= a && a <= 1.0);
        Self([l * a, l * a, l * a, a])
    }

    /// Transparent black
    pub fn from_black_alpha(a: f32) -> Self {
        debug_assert!(0.0 <= a && a <= 1.0);
        Self([0.0, 0.0, 0.0, a])
    }

    /// Transparent white
    pub fn from_white_alpha(a: f32) -> Self {
        debug_assert!(0.0 <= a && a <= 1.0);
        Self([a, a, a, a])
    }

    /// Return an additive version of this color (alpha = 0)
    pub fn additive(self) -> Self {
        let [r, g, b, _] = self.0;
        Self([r, g, b, 0.0])
    }

    /// Multiply with e.g. 0.5 to make us half transparent
    pub fn multiply(self, alpha: f32) -> Self {
        Self([
            alpha * self[0],
            alpha * self[1],
            alpha * self[2],
            alpha * self[3],
        ])
    }

    pub fn r(&self) -> f32 {
        self.0[0]
    }
    pub fn g(&self) -> f32 {
        self.0[1]
    }
    pub fn b(&self) -> f32 {
        self.0[2]
    }
    pub fn a(&self) -> f32 {
        self.0[3]
    }

    /// How perceptually intense (bright) is the color?
    pub fn intensity(&self) -> f32 {
        // 0.3 * self.r() + 0.59 * self.g() + 0.11 * self.b()
        Lcha::from_rgb([self.r(), self.g(), self.b()]).l
    }

    /// Returns an opaque version of self
    pub fn to_opaque(&self) -> Self {
        if self.a() == 0.0 {
            // Additive or fully transparent black.
            Self::from_rgba_premultiplied(self.r(), self.g(), self.b(), 1.0)
        } else {
            // un-multiply alpha:
            Self::from_rgba_premultiplied(
                self.r() / self.a(),
                self.g() / self.a(),
                self.b() / self.a(),
                1.0,
            )
        }
    }
}

impl std::ops::Add for Rgba {
    type Output = Rgba;
    fn add(self, rhs: Rgba) -> Rgba {
        Rgba([
            self[0] + rhs[0],
            self[1] + rhs[1],
            self[2] + rhs[2],
            self[3] + rhs[3],
        ])
    }
}

impl std::ops::Mul<Rgba> for Rgba {
    type Output = Rgba;
    fn mul(self, other: Rgba) -> Rgba {
        Rgba([
            self[0] * other[0],
            self[1] * other[1],
            self[2] * other[2],
            self[3] * other[3],
        ])
    }
}

impl std::ops::Mul<f32> for Rgba {
    type Output = Rgba;
    fn mul(self, factor: f32) -> Rgba {
        Rgba([
            self[0] * factor,
            self[1] * factor,
            self[2] * factor,
            self[3] * factor,
        ])
    }
}

impl std::ops::Mul<Rgba> for f32 {
    type Output = Rgba;
    fn mul(self, rgba: Rgba) -> Rgba {
        Rgba([
            self * rgba[0],
            self * rgba[1],
            self * rgba[2],
            self * rgba[3],
        ])
    }
}

// ----------------------------------------------------------------------------
// Color conversion:

impl From<Color32> for Rgba {
    fn from(srgba: Color32) -> Rgba {
        Rgba([
            linear_from_gamma_byte(srgba[0]),
            linear_from_gamma_byte(srgba[1]),
            linear_from_gamma_byte(srgba[2]),
            linear_from_alpha_byte(srgba[3]),
        ])
    }
}

impl From<Rgba> for Color32 {
    fn from(rgba: Rgba) -> Color32 {
        Color32([
            gamma_byte_from_linear(rgba[0]),
            gamma_byte_from_linear(rgba[1]),
            gamma_byte_from_linear(rgba[2]),
            alpha_byte_from_linear(rgba[3]),
        ])
    }
}

/// [0, 255] -> [0, 1]
fn linear_from_gamma_byte(s: u8) -> f32 {
    if s <= 10 {
        s as f32 / 3294.6
    } else {
        ((s as f32 + 14.025) / 269.025).powf(2.4)
    }
}

fn linear_from_alpha_byte(a: u8) -> f32 {
    a as f32 / 255.0
}

/// [0, 1] -> [0, 255]
fn gamma_byte_from_linear(l: f32) -> u8 {
    if l <= 0.0 {
        0
    } else if l <= 0.0031308 {
        (3294.6 * l).round() as u8
    } else if l <= 1.0 {
        (269.025 * l.powf(1.0 / 2.4) - 14.025).round() as u8
    } else {
        255
    }
}

fn alpha_byte_from_linear(a: f32) -> u8 {
    clamp(a * 255.0, 0.0..=255.0).round() as u8
}

#[test]
fn test_srgba_conversion() {
    #![allow(clippy::float_cmp)]
    for b in 0..=255 {
        let l = linear_from_gamma_byte(b);
        assert!(0.0 <= l && l <= 1.0);
        assert_eq!(gamma_byte_from_linear(l), b);
    }
}

// ----------------------------------------------------------------------------

/// Hue, saturation, value, alpha. All in the range [0, 1].
/// No premultiplied alpha.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hsva {
    /// hue 0-1
    pub h: f32,
    /// saturation 0-1
    pub s: f32,
    /// value 0-1
    pub v: f32,
    /// alpha 0-1. A negative value signifies an additive color (and alpha is ignored).
    pub a: f32,
}

impl Hsva {
    pub fn new(h: f32, s: f32, v: f32, a: f32) -> Self {
        Self { h, s, v, a }
    }

    pub fn from_rgb(rgb: [f32; 3]) -> Self {
        let (h, s, v) = hsv_from_rgb(rgb);
        Hsva { h, s, v, a: 1.0 }
    }

    pub fn from_srgb([r, g, b]: [u8; 3]) -> Self {
        Self::from_rgb([
            linear_from_gamma_byte(r),
            linear_from_gamma_byte(g),
            linear_from_gamma_byte(b),
        ])
    }

    /// From `sRGBA` with premultiplied alpha
    pub fn from_srgba_premultiplied(srgba: [u8; 4]) -> Self {
        Self::from_rgba_premultiplied([
            linear_from_gamma_byte(srgba[0]),
            linear_from_gamma_byte(srgba[1]),
            linear_from_gamma_byte(srgba[2]),
            linear_from_alpha_byte(srgba[3]),
        ])
    }

    /// From `sRGBA` without premultiplied alpha
    pub fn from_srgba_unmultiplied(srgba: [u8; 4]) -> Self {
        Self::from_rgba_unmultiplied([
            linear_from_gamma_byte(srgba[0]),
            linear_from_gamma_byte(srgba[1]),
            linear_from_gamma_byte(srgba[2]),
            linear_from_alpha_byte(srgba[3]),
        ])
    }

    /// From linear RGBA with premultiplied alpha
    pub fn from_rgba_premultiplied([r, g, b, a]: [f32; 4]) -> Self {
        #![allow(clippy::many_single_char_names)]
        if a == 0.0 {
            if r == 0.0 && b == 0.0 && a == 0.0 {
                Hsva::default()
            } else {
                Hsva::from_additive_rgb([r, g, b])
            }
        } else {
            let (h, s, v) = hsv_from_rgb([r / a, g / a, b / a]);
            Hsva { h, s, v, a }
        }
    }

    /// From linear RGBA without premultiplied alpha
    pub fn from_rgba_unmultiplied([r, g, b, a]: [f32; 4]) -> Self {
        #![allow(clippy::many_single_char_names)]
        let (h, s, v) = hsv_from_rgb([r, g, b]);
        Hsva { h, s, v, a }
    }

    pub fn from_additive_rgb(rgb: [f32; 3]) -> Self {
        let (h, s, v) = hsv_from_rgb(rgb);
        Hsva {
            h,
            s,
            v,
            a: -0.5, // anything negative is treated as additive
        }
    }

    // ------------------------------------------------------------------------

    pub fn to_opaque(self) -> Self {
        Self { a: 1.0, ..self }
    }

    pub fn to_rgb(&self) -> [f32; 3] {
        rgb_from_hsv((self.h, self.s, self.v))
    }

    pub fn to_srgb(&self) -> [u8; 3] {
        let [r, g, b] = self.to_rgb();
        [
            gamma_byte_from_linear(r),
            gamma_byte_from_linear(g),
            gamma_byte_from_linear(b),
        ]
    }

    pub fn to_rgba_premultiplied(&self) -> [f32; 4] {
        let [r, g, b, a] = self.to_rgba_unmultiplied();
        let additive = a < 0.0;
        if additive {
            [r, g, b, 0.0]
        } else {
            [a * r, a * g, a * b, a]
        }
    }

    /// Represents additive colors using a negative alpha.
    pub fn to_rgba_unmultiplied(&self) -> [f32; 4] {
        let Hsva { h, s, v, a } = *self;
        let [r, g, b] = rgb_from_hsv((h, s, v));
        [r, g, b, a]
    }

    pub fn to_srgba_premultiplied(&self) -> [u8; 4] {
        let [r, g, b, a] = self.to_rgba_premultiplied();
        [
            gamma_byte_from_linear(r),
            gamma_byte_from_linear(g),
            gamma_byte_from_linear(b),
            alpha_byte_from_linear(a),
        ]
    }

    pub fn to_srgba_unmultiplied(&self) -> [u8; 4] {
        let [r, g, b, a] = self.to_rgba_unmultiplied();
        [
            gamma_byte_from_linear(r),
            gamma_byte_from_linear(g),
            gamma_byte_from_linear(b),
            alpha_byte_from_linear(a.abs()),
        ]
    }
}

impl From<Hsva> for Rgba {
    fn from(hsva: Hsva) -> Rgba {
        Rgba(hsva.to_rgba_premultiplied())
    }
}
impl From<Rgba> for Hsva {
    fn from(rgba: Rgba) -> Hsva {
        Self::from_rgba_premultiplied(rgba.0)
    }
}

impl From<Hsva> for Color32 {
    fn from(hsva: Hsva) -> Color32 {
        Color32::from(Rgba::from(hsva))
    }
}
impl From<Color32> for Hsva {
    fn from(srgba: Color32) -> Hsva {
        Hsva::from(Rgba::from(srgba))
    }
}

/// All ranges in 0-1, rgb is linear.
pub fn hsv_from_rgb([r, g, b]: [f32; 3]) -> (f32, f32, f32) {
    #![allow(clippy::float_cmp)]
    #![allow(clippy::many_single_char_names)]
    let min = r.min(g.min(b));
    let max = r.max(g.max(b)); // value

    let range = max - min;

    let h = if max == min {
        0.0 // hue is undefined
    } else if max == r {
        (g - b) / (6.0 * range)
    } else if max == g {
        (b - r) / (6.0 * range) + 1.0 / 3.0
    } else {
        // max == b
        (r - g) / (6.0 * range) + 2.0 / 3.0
    };
    let h = (h + 1.0).fract(); // wrap
    let s = if max == 0.0 { 0.0 } else { 1.0 - min / max };
    (h, s, max)
}

/// All ranges in 0-1, rgb is linear.
pub fn rgb_from_hsv((h, s, v): (f32, f32, f32)) -> [f32; 3] {
    #![allow(clippy::many_single_char_names)]
    let h = (h.fract() + 1.0).fract(); // wrap
    let s = clamp(s, 0.0..=1.0);

    let f = h * 6.0 - (h * 6.0).floor();
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);

    match (h * 6.0).floor() as i32 % 6 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        5 => [v, p, q],
        _ => unreachable!(),
    }
}

#[test]
#[ignore] // a bit expensive
fn test_hsv_roundtrip() {
    for r in 0..=255 {
        for g in 0..=255 {
            for b in 0..=255 {
                let srgba = Color32::from_rgb(r, g, b);
                let hsva = Hsva::from(srgba);
                assert_eq!(srgba, Color32::from(hsva));
            }
        }
    }
}

// ----------------------------------------------------------------------------

// /// A simple perceptual color space.
// ///
// /// https://bottosson.github.io/posts/oklab/
// #[derive(Clone, Copy, Debug, Default, PartialEq)]
// struct Oklab {
//     /// Perceived lightness (0-1)
//     pub l: f32,
//     /// How green/red the color is ([-1, 1])
//     pub a: f32,
//     /// How blue/yellow the color is ([-1, 1])
//     pub b: f32,
// }

// impl Oklab {
//     pub fn from_linear_rgb(r: f32, g: f32, b: f32) -> Oklab {
//         let (l, a, b) = lab_from_rgb([r, g, b]);
//         Oklab { l, a, b }
//     }

//     pub fn to_linear_rgb(self) -> [f32; 3] {
//         rgb_from_lab((self.l, self.a, self.b))
//     }
// }

// /// Polar form of [`Oklab`], all coordinated in 0-1 range.
// #[derive(Clone, Copy, Debug, Default, PartialEq)]
// struct Oklch {
//     /// Perceived lightness in [0, 1] range.
//     pub l: f32,
//     /// Chroma in [0, 1] range.
//     pub c: f32,
//     /// Hue in [0, 1] range.
//     pub h: f32,
// }

// impl From<Oklab> for Oklch {
//     fn from(i: Oklab) -> Oklch {
//         use std::f32::consts::TAU;
//         Oklch {
//             l: i.l,
//             c: i.a.hypot(i.b),
//             h: (i.b.atan2(i.a) + TAU) % TAU / TAU,
//         }
//     }
// }

// impl From<Oklch> for Oklab {
//     fn from(i: Oklch) -> Oklab {
//         use std::f32::consts::TAU;
//         let (sin_h, cos_h) = (i.h * TAU).sin_cos();
//         Oklab {
//             l: i.l,
//             a: i.c * cos_h,
//             b: i.c * sin_h,
//         }
//     }
// }

// impl From<Oklab> for Color32 {
//     fn from(i: Oklab) -> Color32 {
//         let [r, g, b] = i.to_linear_rgb();
//         Rgba::from_rgb(r, g, b).into()
//     }
// }

// impl From<Oklch> for Color32 {
//     fn from(i: Oklch) -> Color32 {
//         Oklab::from(i).into()
//     }
// }

// #[test]
// // #[ignore] // a bit expensive
// fn test_oklab_roundtrip() {
//     for r in 0..=255 {
//         for g in 0..=255 {
//             for b in 0..=255 {
//                 let srgba = Color32::from_rgb(r, g, b);
//                 let rgba = Rgba::from(srgba);
//                 let oklab = Oklab::from_linear_rgb(rgba.r(), rgba.g(), rgba.b());
//                 assert_eq!(srgba, Color32::from(oklab));
//                 let oklch = Oklch::from(oklab);
//                 assert_eq!(srgba, Color32::from(oklch),);
//             }
//         }
//     }
// }

// ----------------------------------------------------------------------------

/// oklab from linear rgb
fn lab_from_rgb([r, g, b]: [f32; 3]) -> (f32, f32, f32) {
    let x = 0.4121656120 * r + 0.5362752080 * g + 0.0514575653 * b;
    let y = 0.2118591070 * r + 0.6807189584 * g + 0.1074065790 * b;
    let z = 0.0883097947 * r + 0.2818474174 * g + 0.6302613616 * b;

    let x = x.cbrt();
    let y = y.cbrt();
    let z = z.cbrt();

    (
        0.2104542553 * x + 0.7936177850 * y - 0.0040720468 * z,
        1.9779984951 * x - 2.4285922050 * y + 0.4505937099 * z,
        0.0259040371 * x + 0.7827717662 * y - 0.8086757660 * z,
    )
}

/// linear rgb from oklab
pub fn rgb_from_lab((l, a, b): (f32, f32, f32)) -> [f32; 3] {
    let x = l + 0.3963377774 * a + 0.2158037573 * b;
    let y = l - 0.1055613458 * a - 0.0638541728 * b;
    let z = l - 0.0894841775 * a - 1.2914855480 * b;

    let x = x.powi(3);
    let y = y.powi(3);
    let z = z.powi(3);

    [
        4.0767245293 * x - 3.3072168827 * y + 0.2307590544 * z,
        -1.2681437731 * x + 2.6093323231 * y - 0.3411344290 * z,
        -0.0041119885 * x - 0.7034763098 * y + 1.7068625689 * z,
    ]
}

/// 0-1 normalized lch from oklab.
fn lch_from_lab((l, a, b): (f32, f32, f32)) -> (f32, f32, f32) {
    use std::f32::consts::TAU;
    let c = a.hypot(b);
    let h = (b.atan2(a) + TAU) % TAU / TAU;
    (l, c, h)
}

/// Oklab from 0-1 normalized lch.
fn lab_from_lch((l, c, h): (f32, f32, f32)) -> (f32, f32, f32) {
    use std::f32::consts::TAU;
    let (sin_h, cos_h) = (h * TAU).sin_cos();
    let a = c * cos_h;
    let b = c * sin_h;
    (l, a, b)
}

/// 0-1 normalized lch from linear rgb
fn lch_from_rgb(rgb: [f32; 3]) -> (f32, f32, f32) {
    lch_from_lab(lab_from_rgb(rgb))
}
/// linear rgb from 0-1 normalized lch
fn rgb_from_lch(lch: (f32, f32, f32)) -> [f32; 3] {
    rgb_from_lab(lab_from_lch(lch))
}

/// Lightness, chroma, hue and alpha. All in the range [0, 1].
/// No premultiplied alpha.
/// Based on the the perceptual color space Oklab (https://bottosson.github.io/posts/oklab/).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Lcha {
    /// Perceived lightness in [0, 1] range.
    pub l: f32,
    /// Chroma in [0, 1] range.
    pub c: f32,
    /// Hue in [0, 1] range.
    pub h: f32,
    /// Alpha in [0, 1] range. A negative value signifies an additive color (and alpha is ignored).
    pub a: f32,
}

impl Lcha {
    pub fn new(l: f32, c: f32, h: f32, a: f32) -> Self {
        Self { l, c, h, a }
    }

    /// From linear RGB.
    pub fn from_rgb(rgb: [f32; 3]) -> Self {
        let (l, c, h) = lch_from_rgb(rgb);
        Lcha { l, c, h, a: 1.0 }
    }

    /// From `sRGBA` with premultiplied alpha
    pub fn from_srgba_premultiplied(srgba: [u8; 4]) -> Self {
        Self::from_rgba_premultiplied([
            linear_from_gamma_byte(srgba[0]),
            linear_from_gamma_byte(srgba[1]),
            linear_from_gamma_byte(srgba[2]),
            linear_from_alpha_byte(srgba[3]),
        ])
    }

    /// From `sRGBA` without premultiplied alpha
    pub fn from_srgba_unmultiplied(srgba: [u8; 4]) -> Self {
        Self::from_rgba_unmultiplied([
            linear_from_gamma_byte(srgba[0]),
            linear_from_gamma_byte(srgba[1]),
            linear_from_gamma_byte(srgba[2]),
            linear_from_alpha_byte(srgba[3]),
        ])
    }

    /// From linear RGBA with premultiplied alpha
    pub fn from_rgba_premultiplied(rgba: [f32; 4]) -> Self {
        #![allow(clippy::many_single_char_names)]
        let [r, g, b, a] = rgba;
        if a == 0.0 {
            if r == 0.0 && b == 0.0 && a == 0.0 {
                Lcha::default()
            } else {
                Lcha::from_additive_rgb([r, g, b])
            }
        } else {
            let (l, c, h) = lch_from_rgb([r / a, g / a, b / a]);
            Lcha { l, c, h, a }
        }
    }

    /// From linear RGBA without premultiplied alpha
    pub fn from_rgba_unmultiplied(rgba: [f32; 4]) -> Self {
        #![allow(clippy::many_single_char_names)]
        let [r, g, b, a] = rgba;
        let (l, c, h) = lch_from_rgb([r, g, b]);
        Lcha { l, c, h, a }
    }

    pub fn from_additive_rgb(rgb: [f32; 3]) -> Self {
        let (l, c, h) = lch_from_rgb(rgb);
        Lcha {
            l,
            c,
            h,
            a: -0.5, // anything negative is treated as additive
        }
    }
    // ------------------------------------------------------------------------

    pub fn to_rgb(&self) -> [f32; 3] {
        rgb_from_lch((self.l, self.c, self.h))
    }

    pub fn to_rgba_premultiplied(&self) -> [f32; 4] {
        let [r, g, b, a] = self.to_rgba_unmultiplied();
        let additive = a < 0.0;
        if additive {
            [r, g, b, 0.0]
        } else {
            [a * r, a * g, a * b, a]
        }
    }

    pub fn to_rgba_unmultiplied(&self) -> [f32; 4] {
        let Lcha { l, c, h, a } = *self;
        let [r, g, b] = rgb_from_lch((l, c, h));
        [r, g, b, a]
    }

    pub fn to_srgba_premultiplied(&self) -> [u8; 4] {
        let [r, g, b, a] = self.to_rgba_premultiplied();
        [
            gamma_byte_from_linear(r),
            gamma_byte_from_linear(g),
            gamma_byte_from_linear(b),
            alpha_byte_from_linear(a),
        ]
    }

    pub fn to_srgba_unmultiplied(&self) -> [u8; 4] {
        let [r, g, b, a] = self.to_rgba_unmultiplied();
        [
            gamma_byte_from_linear(r),
            gamma_byte_from_linear(g),
            gamma_byte_from_linear(b),
            alpha_byte_from_linear(a.abs()),
        ]
    }
}

impl From<Lcha> for Rgba {
    fn from(hsva: Lcha) -> Rgba {
        Rgba(hsva.to_rgba_premultiplied())
    }
}
impl From<Rgba> for Lcha {
    fn from(rgba: Rgba) -> Lcha {
        Self::from_rgba_premultiplied(rgba.0)
    }
}

impl From<Lcha> for Color32 {
    fn from(hsva: Lcha) -> Color32 {
        Color32::from(Rgba::from(hsva))
    }
}
impl From<Color32> for Lcha {
    fn from(srgba: Color32) -> Lcha {
        Lcha::from(Rgba::from(srgba))
    }
}

#[test]
// #[ignore] // a bit expensive
fn test_lcha_roundtrip() {
    for r in 0..=255 {
        for g in 0..=255 {
            for b in 0..=255 {
                let srgba = Color32::from_rgb(r, g, b);
                let lcha = Lcha::from(srgba);
                assert_eq!(srgba, Color32::from(lcha),);
            }
        }
    }
}
