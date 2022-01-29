//! Color conversions and types.
//!
//! If you want a compact color representation, use [`Color32`].
//! If you want to manipulate RGBA colors use [`Rgba`].
//! If you want to manipulate colors in a way closer to how humans think about colors, use [`HsvaGamma`].

#![allow(clippy::wrong_self_convention)]

/// This format is used for space-efficient color representation (32 bits).
///
/// Instead of manipulating this directly it is often better
/// to first convert it to either [`Rgba`] or [`Hsva`].
///
/// Internally this uses 0-255 gamma space `sRGBA` color with premultiplied alpha.
/// Alpha channel is in linear space.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Color32(pub(crate) [u8; 4]);

impl std::ops::Index<usize> for Color32 {
    type Output = u8;

    #[inline(always)]
    fn index(&self, index: usize) -> &u8 {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for Color32 {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut u8 {
        &mut self.0[index]
    }
}

impl Color32 {
    // Mostly follows CSS names:

    pub const TRANSPARENT: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 0);
    pub const BLACK: Color32 = Color32::from_rgb(0, 0, 0);
    pub const DARK_GRAY: Color32 = Color32::from_rgb(96, 96, 96);
    pub const GRAY: Color32 = Color32::from_rgb(160, 160, 160);
    pub const LIGHT_GRAY: Color32 = Color32::from_rgb(220, 220, 220);
    pub const WHITE: Color32 = Color32::from_rgb(255, 255, 255);

    pub const BROWN: Color32 = Color32::from_rgb(165, 42, 42);
    pub const DARK_RED: Color32 = Color32::from_rgb(0x8B, 0, 0);
    pub const RED: Color32 = Color32::from_rgb(255, 0, 0);
    pub const LIGHT_RED: Color32 = Color32::from_rgb(255, 128, 128);

    pub const YELLOW: Color32 = Color32::from_rgb(255, 255, 0);
    pub const LIGHT_YELLOW: Color32 = Color32::from_rgb(255, 255, 0xE0);
    pub const KHAKI: Color32 = Color32::from_rgb(240, 230, 140);

    pub const DARK_GREEN: Color32 = Color32::from_rgb(0, 0x64, 0);
    pub const GREEN: Color32 = Color32::from_rgb(0, 255, 0);
    pub const LIGHT_GREEN: Color32 = Color32::from_rgb(0x90, 0xEE, 0x90);

    pub const DARK_BLUE: Color32 = Color32::from_rgb(0, 0, 0x8B);
    pub const BLUE: Color32 = Color32::from_rgb(0, 0, 255);
    pub const LIGHT_BLUE: Color32 = Color32::from_rgb(0xAD, 0xD8, 0xE6);

    pub const GOLD: Color32 = Color32::from_rgb(255, 215, 0);

    pub const DEBUG_COLOR: Color32 = Color32::from_rgba_premultiplied(0, 200, 0, 128);

    /// An ugly color that is planned to be replaced before making it to the screen.
    pub const TEMPORARY_COLOR: Color32 = Color32::from_rgb(64, 254, 0);

    #[inline(always)]
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b, 255])
    }

    #[inline(always)]
    pub const fn from_rgb_additive(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b, 0])
    }

    /// From `sRGBA` with premultiplied alpha.
    #[inline(always)]
    pub const fn from_rgba_premultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([r, g, b, a])
    }

    /// From `sRGBA` WITHOUT premultiplied alpha.
    pub fn from_rgba_unmultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        if a == 255 {
            Self::from_rgb(r, g, b) // common-case optimization
        } else if a == 0 {
            Self::TRANSPARENT // common-case optimization
        } else {
            let r_lin = linear_f32_from_gamma_u8(r);
            let g_lin = linear_f32_from_gamma_u8(g);
            let b_lin = linear_f32_from_gamma_u8(b);
            let a_lin = linear_f32_from_linear_u8(a);

            let r = gamma_u8_from_linear_f32(r_lin * a_lin);
            let g = gamma_u8_from_linear_f32(g_lin * a_lin);
            let b = gamma_u8_from_linear_f32(b_lin * a_lin);

            Self::from_rgba_premultiplied(r, g, b, a)
        }
    }

    #[inline(always)]
    pub const fn from_gray(l: u8) -> Self {
        Self([l, l, l, 255])
    }

    #[inline(always)]
    pub const fn from_black_alpha(a: u8) -> Self {
        Self([0, 0, 0, a])
    }

    pub fn from_white_alpha(a: u8) -> Self {
        Rgba::from_white_alpha(linear_f32_from_linear_u8(a)).into()
    }

    #[inline(always)]
    pub const fn from_additive_luminance(l: u8) -> Self {
        Self([l, l, l, 0])
    }

    #[inline(always)]
    pub fn is_opaque(&self) -> bool {
        self.a() == 255
    }

    #[inline(always)]
    pub fn r(&self) -> u8 {
        self.0[0]
    }

    #[inline(always)]
    pub fn g(&self) -> u8 {
        self.0[1]
    }

    #[inline(always)]
    pub fn b(&self) -> u8 {
        self.0[2]
    }

    #[inline(always)]
    pub fn a(&self) -> u8 {
        self.0[3]
    }

    /// Returns an opaque version of self
    pub fn to_opaque(self) -> Self {
        Rgba::from(self).to_opaque().into()
    }

    /// Returns an additive version of self
    #[inline(always)]
    pub fn additive(self) -> Self {
        let [r, g, b, _] = self.to_array();
        Self([r, g, b, 0])
    }

    /// Premultiplied RGBA
    #[inline(always)]
    pub fn to_array(&self) -> [u8; 4] {
        [self.r(), self.g(), self.b(), self.a()]
    }

    /// Premultiplied RGBA
    #[inline(always)]
    pub fn to_tuple(&self) -> (u8, u8, u8, u8) {
        (self.r(), self.g(), self.b(), self.a())
    }

    pub fn to_srgba_unmultiplied(&self) -> [u8; 4] {
        Rgba::from(*self).to_srgba_unmultiplied()
    }

    /// Multiply with 0.5 to make color half as opaque.
    pub fn linear_multiply(self, factor: f32) -> Color32 {
        crate::epaint_assert!(0.0 <= factor && factor <= 1.0);
        // As an unfortunate side-effect of using premultiplied alpha
        // we need a somewhat expensive conversion to linear space and back.
        Rgba::from(self).multiply(factor).into()
    }
}

// ----------------------------------------------------------------------------

/// 0-1 linear space `RGBA` color with premultiplied alpha.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Rgba(pub(crate) [f32; 4]);

impl std::ops::Index<usize> for Rgba {
    type Output = f32;

    #[inline(always)]
    fn index(&self, index: usize) -> &f32 {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for Rgba {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut f32 {
        &mut self.0[index]
    }
}

#[allow(clippy::derive_hash_xor_eq)]
impl std::hash::Hash for Rgba {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        crate::f32_hash(state, self.0[0]);
        crate::f32_hash(state, self.0[1]);
        crate::f32_hash(state, self.0[2]);
        crate::f32_hash(state, self.0[3]);
    }
}

impl Rgba {
    pub const TRANSPARENT: Rgba = Rgba::from_rgba_premultiplied(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Rgba = Rgba::from_rgb(0.0, 0.0, 0.0);
    pub const WHITE: Rgba = Rgba::from_rgb(1.0, 1.0, 1.0);
    pub const RED: Rgba = Rgba::from_rgb(1.0, 0.0, 0.0);
    pub const GREEN: Rgba = Rgba::from_rgb(0.0, 1.0, 0.0);
    pub const BLUE: Rgba = Rgba::from_rgb(0.0, 0.0, 1.0);

    #[inline(always)]
    pub const fn from_rgba_premultiplied(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self([r, g, b, a])
    }

    #[inline(always)]
    pub fn from_rgba_unmultiplied(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self([r * a, g * a, b * a, a])
    }

    #[inline(always)]
    pub fn from_srgba_premultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        let r = linear_f32_from_gamma_u8(r);
        let g = linear_f32_from_gamma_u8(g);
        let b = linear_f32_from_gamma_u8(b);
        let a = linear_f32_from_linear_u8(a);
        Self::from_rgba_premultiplied(r, g, b, a)
    }

    #[inline(always)]
    pub fn from_srgba_unmultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        let r = linear_f32_from_gamma_u8(r);
        let g = linear_f32_from_gamma_u8(g);
        let b = linear_f32_from_gamma_u8(b);
        let a = linear_f32_from_linear_u8(a);
        Self::from_rgba_premultiplied(r * a, g * a, b * a, a)
    }

    #[inline(always)]
    pub const fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self([r, g, b, 1.0])
    }

    #[inline(always)]
    pub const fn from_gray(l: f32) -> Self {
        Self([l, l, l, 1.0])
    }

    pub fn from_luminance_alpha(l: f32, a: f32) -> Self {
        crate::epaint_assert!(0.0 <= l && l <= 1.0);
        crate::epaint_assert!(0.0 <= a && a <= 1.0);
        Self([l * a, l * a, l * a, a])
    }

    /// Transparent black
    #[inline(always)]
    pub fn from_black_alpha(a: f32) -> Self {
        crate::epaint_assert!(0.0 <= a && a <= 1.0);
        Self([0.0, 0.0, 0.0, a])
    }

    /// Transparent white
    #[inline(always)]
    pub fn from_white_alpha(a: f32) -> Self {
        crate::epaint_assert!(0.0 <= a && a <= 1.0);
        Self([a, a, a, a])
    }

    /// Return an additive version of this color (alpha = 0)
    #[inline(always)]
    pub fn additive(self) -> Self {
        let [r, g, b, _] = self.0;
        Self([r, g, b, 0.0])
    }

    /// Multiply with e.g. 0.5 to make us half transparent
    #[inline(always)]
    pub fn multiply(self, alpha: f32) -> Self {
        Self([
            alpha * self[0],
            alpha * self[1],
            alpha * self[2],
            alpha * self[3],
        ])
    }

    #[inline(always)]
    pub fn r(&self) -> f32 {
        self.0[0]
    }

    #[inline(always)]
    pub fn g(&self) -> f32 {
        self.0[1]
    }

    #[inline(always)]
    pub fn b(&self) -> f32 {
        self.0[2]
    }

    #[inline(always)]
    pub fn a(&self) -> f32 {
        self.0[3]
    }

    /// How perceptually intense (bright) is the color?
    #[inline]
    pub fn intensity(&self) -> f32 {
        0.3 * self.r() + 0.59 * self.g() + 0.11 * self.b()
    }

    /// Returns an opaque version of self
    pub fn to_opaque(&self) -> Self {
        if self.a() == 0.0 {
            // Additive or fully transparent black.
            Self::from_rgb(self.r(), self.g(), self.b())
        } else {
            // un-multiply alpha:
            Self::from_rgb(
                self.r() / self.a(),
                self.g() / self.a(),
                self.b() / self.a(),
            )
        }
    }

    /// Premultiplied RGBA
    #[inline(always)]
    pub fn to_array(&self) -> [f32; 4] {
        [self.r(), self.g(), self.b(), self.a()]
    }

    /// Premultiplied RGBA
    #[inline(always)]
    pub fn to_tuple(&self) -> (f32, f32, f32, f32) {
        (self.r(), self.g(), self.b(), self.a())
    }

    /// unmultiply the alpha
    pub fn to_rgba_unmultiplied(&self) -> [f32; 4] {
        let a = self.a();
        if a == 0.0 {
            // Additive, let's assume we are black
            self.0
        } else {
            [self.r() / a, self.g() / a, self.b() / a, a]
        }
    }

    /// unmultiply the alpha
    pub fn to_srgba_unmultiplied(&self) -> [u8; 4] {
        let [r, g, b, a] = self.to_rgba_unmultiplied();
        [
            gamma_u8_from_linear_f32(r),
            gamma_u8_from_linear_f32(g),
            gamma_u8_from_linear_f32(b),
            linear_u8_from_linear_f32(a.abs()),
        ]
    }
}

impl std::ops::Add for Rgba {
    type Output = Rgba;

    #[inline(always)]
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

    #[inline(always)]
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

    #[inline(always)]
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

    #[inline(always)]
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
            linear_f32_from_gamma_u8(srgba.0[0]),
            linear_f32_from_gamma_u8(srgba.0[1]),
            linear_f32_from_gamma_u8(srgba.0[2]),
            linear_f32_from_linear_u8(srgba.0[3]),
        ])
    }
}

impl From<Rgba> for Color32 {
    fn from(rgba: Rgba) -> Color32 {
        Color32([
            gamma_u8_from_linear_f32(rgba.0[0]),
            gamma_u8_from_linear_f32(rgba.0[1]),
            gamma_u8_from_linear_f32(rgba.0[2]),
            linear_u8_from_linear_f32(rgba.0[3]),
        ])
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
pub fn linear_f32_from_linear_u8(a: u8) -> f32 {
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

fn fast_round(r: f32) -> u8 {
    (r + 0.5).floor() as _ // rust does a saturating cast since 1.45
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

    /// From `sRGBA` with premultiplied alpha
    pub fn from_srgba_premultiplied(srgba: [u8; 4]) -> Self {
        Self::from_rgba_premultiplied(
            linear_f32_from_gamma_u8(srgba[0]),
            linear_f32_from_gamma_u8(srgba[1]),
            linear_f32_from_gamma_u8(srgba[2]),
            linear_f32_from_linear_u8(srgba[3]),
        )
    }

    /// From `sRGBA` without premultiplied alpha
    pub fn from_srgba_unmultiplied(srgba: [u8; 4]) -> Self {
        Self::from_rgba_unmultiplied(
            linear_f32_from_gamma_u8(srgba[0]),
            linear_f32_from_gamma_u8(srgba[1]),
            linear_f32_from_gamma_u8(srgba[2]),
            linear_f32_from_linear_u8(srgba[3]),
        )
    }

    /// From linear RGBA with premultiplied alpha
    pub fn from_rgba_premultiplied(r: f32, g: f32, b: f32, a: f32) -> Self {
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
    pub fn from_rgba_unmultiplied(r: f32, g: f32, b: f32, a: f32) -> Self {
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

    pub fn from_rgb(rgb: [f32; 3]) -> Self {
        let (h, s, v) = hsv_from_rgb(rgb);
        Hsva { h, s, v, a: 1.0 }
    }

    pub fn from_srgb([r, g, b]: [u8; 3]) -> Self {
        Self::from_rgb([
            linear_f32_from_gamma_u8(r),
            linear_f32_from_gamma_u8(g),
            linear_f32_from_gamma_u8(b),
        ])
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
            gamma_u8_from_linear_f32(r),
            gamma_u8_from_linear_f32(g),
            gamma_u8_from_linear_f32(b),
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
            gamma_u8_from_linear_f32(r),
            gamma_u8_from_linear_f32(g),
            gamma_u8_from_linear_f32(b),
            linear_u8_from_linear_f32(a),
        ]
    }

    pub fn to_srgba_unmultiplied(&self) -> [u8; 4] {
        let [r, g, b, a] = self.to_rgba_unmultiplied();
        [
            gamma_u8_from_linear_f32(r),
            gamma_u8_from_linear_f32(g),
            gamma_u8_from_linear_f32(b),
            linear_u8_from_linear_f32(a.abs()),
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
        Self::from_rgba_premultiplied(rgba.0[0], rgba.0[1], rgba.0[2], rgba.0[3])
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
    let s = s.clamp(0.0, 1.0);

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

/// Like Hsva but with the `v` value (brightness) being gamma corrected
/// so that it is somewhat perceptually even.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HsvaGamma {
    /// hue 0-1
    pub h: f32,
    /// saturation 0-1
    pub s: f32,
    /// value 0-1, in gamma-space (~perceptually even)
    pub v: f32,
    /// alpha 0-1. A negative value signifies an additive color (and alpha is ignored).
    pub a: f32,
}

impl From<HsvaGamma> for Rgba {
    fn from(hsvag: HsvaGamma) -> Rgba {
        Hsva::from(hsvag).into()
    }
}

impl From<HsvaGamma> for Color32 {
    fn from(hsvag: HsvaGamma) -> Color32 {
        Rgba::from(hsvag).into()
    }
}

impl From<HsvaGamma> for Hsva {
    fn from(hsvag: HsvaGamma) -> Hsva {
        let HsvaGamma { h, s, v, a } = hsvag;
        Hsva {
            h,
            s,
            v: linear_from_gamma(v),
            a,
        }
    }
}

impl From<Rgba> for HsvaGamma {
    fn from(rgba: Rgba) -> HsvaGamma {
        Hsva::from(rgba).into()
    }
}

impl From<Color32> for HsvaGamma {
    fn from(srgba: Color32) -> HsvaGamma {
        Hsva::from(srgba).into()
    }
}

impl From<Hsva> for HsvaGamma {
    fn from(hsva: Hsva) -> HsvaGamma {
        let Hsva { h, s, v, a } = hsva;
        HsvaGamma {
            h,
            s,
            v: gamma_from_linear(v),
            a,
        }
    }
}

// ----------------------------------------------------------------------------

/// Cheap and ugly.
/// Made for graying out disabled `Ui`:s.
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

#[cfg(feature = "cint")]
mod impl_cint {
    use super::*;
    use cint::{Alpha, ColorInterop, EncodedSrgb, Hsv, LinearSrgb, PremultipliedAlpha};

    // ---- Color32 ----

    impl From<Alpha<EncodedSrgb<u8>>> for Color32 {
        fn from(srgba: Alpha<EncodedSrgb<u8>>) -> Self {
            let Alpha {
                color: EncodedSrgb { r, g, b },
                alpha: a,
            } = srgba;

            Color32::from_rgba_unmultiplied(r, g, b, a)
        }
    }

    // No From<Color32> for Alpha<_> because Color32 is premultiplied

    impl From<PremultipliedAlpha<EncodedSrgb<u8>>> for Color32 {
        fn from(srgba: PremultipliedAlpha<EncodedSrgb<u8>>) -> Self {
            let PremultipliedAlpha {
                color: EncodedSrgb { r, g, b },
                alpha: a,
            } = srgba;

            Color32::from_rgba_premultiplied(r, g, b, a)
        }
    }

    impl From<Color32> for PremultipliedAlpha<EncodedSrgb<u8>> {
        fn from(col: Color32) -> Self {
            let (r, g, b, a) = col.to_tuple();

            PremultipliedAlpha {
                color: EncodedSrgb { r, g, b },
                alpha: a,
            }
        }
    }

    impl From<PremultipliedAlpha<EncodedSrgb<f32>>> for Color32 {
        fn from(srgba: PremultipliedAlpha<EncodedSrgb<f32>>) -> Self {
            let PremultipliedAlpha {
                color: EncodedSrgb { r, g, b },
                alpha: a,
            } = srgba;

            // This is a bit of an abuse of the function name but it does what we want.
            let r = linear_u8_from_linear_f32(r);
            let g = linear_u8_from_linear_f32(g);
            let b = linear_u8_from_linear_f32(b);
            let a = linear_u8_from_linear_f32(a);

            Color32::from_rgba_premultiplied(r, g, b, a)
        }
    }

    impl From<Color32> for PremultipliedAlpha<EncodedSrgb<f32>> {
        fn from(col: Color32) -> Self {
            let (r, g, b, a) = col.to_tuple();

            // This is a bit of an abuse of the function name but it does what we want.
            let r = linear_f32_from_linear_u8(r);
            let g = linear_f32_from_linear_u8(g);
            let b = linear_f32_from_linear_u8(b);
            let a = linear_f32_from_linear_u8(a);

            PremultipliedAlpha {
                color: EncodedSrgb { r, g, b },
                alpha: a,
            }
        }
    }

    impl ColorInterop for Color32 {
        type CintTy = PremultipliedAlpha<EncodedSrgb<u8>>;
    }

    // ---- Rgba ----

    impl From<PremultipliedAlpha<LinearSrgb<f32>>> for Rgba {
        fn from(srgba: PremultipliedAlpha<LinearSrgb<f32>>) -> Self {
            let PremultipliedAlpha {
                color: LinearSrgb { r, g, b },
                alpha: a,
            } = srgba;

            Rgba([r, g, b, a])
        }
    }

    impl From<Rgba> for PremultipliedAlpha<LinearSrgb<f32>> {
        fn from(col: Rgba) -> Self {
            let (r, g, b, a) = col.to_tuple();

            PremultipliedAlpha {
                color: LinearSrgb { r, g, b },
                alpha: a,
            }
        }
    }

    impl ColorInterop for Rgba {
        type CintTy = PremultipliedAlpha<LinearSrgb<f32>>;
    }

    // ---- Hsva ----

    impl From<Alpha<Hsv<f32>>> for Hsva {
        fn from(srgba: Alpha<Hsv<f32>>) -> Self {
            let Alpha {
                color: Hsv { h, s, v },
                alpha: a,
            } = srgba;

            Hsva::new(h, s, v, a)
        }
    }

    impl From<Hsva> for Alpha<Hsv<f32>> {
        fn from(col: Hsva) -> Self {
            let Hsva { h, s, v, a } = col;

            Alpha {
                color: Hsv { h, s, v },
                alpha: a,
            }
        }
    }

    impl ColorInterop for Hsva {
        type CintTy = Alpha<Hsv<f32>>;
    }

    // ---- HsvaGamma ----

    impl ColorInterop for HsvaGamma {
        type CintTy = Alpha<Hsv<f32>>;
    }

    impl From<Alpha<Hsv<f32>>> for HsvaGamma {
        fn from(srgba: Alpha<Hsv<f32>>) -> Self {
            let Alpha {
                color: Hsv { h, s, v },
                alpha: a,
            } = srgba;

            Hsva::new(h, s, v, a).into()
        }
    }

    impl From<HsvaGamma> for Alpha<Hsv<f32>> {
        fn from(col: HsvaGamma) -> Self {
            let Hsva { h, s, v, a } = col.into();

            Alpha {
                color: Hsv { h, s, v },
                alpha: a,
            }
        }
    }
}
