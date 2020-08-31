use crate::math::clamp;

/// 0-255 gamma space `sRGBA` color with premultiplied alpha.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Srgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    /// Alpha is in linear space (not subject to sRGBA gamma conversion)
    pub a: u8,
}
/// 0-1 linear space `RGBA` color with premultiplied alpha.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

// ----------------------------------------------------------------------------
// Color conversion:

impl From<Srgba> for Rgba {
    fn from(srgba: Srgba) -> Rgba {
        Rgba {
            r: linear_from_srgb_byte(srgba.r),
            g: linear_from_srgb_byte(srgba.g),
            b: linear_from_srgb_byte(srgba.b),
            a: srgba.a as f32 / 255.0,
        }
    }
}

impl From<Rgba> for Srgba {
    fn from(rgba: Rgba) -> Srgba {
        Srgba {
            r: srgb_byte_from_linear(rgba.r),
            g: srgb_byte_from_linear(rgba.g),
            b: srgb_byte_from_linear(rgba.b),
            a: clamp(rgba.a * 255.0, 0.0..=255.0).round() as u8,
        }
    }
}

fn linear_from_srgb_byte(s: u8) -> f32 {
    if s <= 10 {
        s as f32 / 3294.6
    } else {
        ((s as f32 + 14.025) / 269.025).powf(2.4)
    }
}

fn srgb_byte_from_linear(l: f32) -> u8 {
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

#[test]
fn test_srgba_conversion() {
    #![allow(clippy::float_cmp)]
    for b in 0..=255 {
        let l = linear_from_srgb_byte(b);
        assert!(0.0 <= l && l <= 1.0);
        assert_eq!(srgb_byte_from_linear(l), b);
    }
}

// ----------------------------------------------------------------------------

pub const fn srgba(r: u8, g: u8, b: u8, a: u8) -> Srgba {
    Srgba { r, g, b, a }
}

impl Srgba {
    pub const fn gray(l: u8) -> Self {
        Self {
            r: l,
            g: l,
            b: l,
            a: 255,
        }
    }

    pub const fn black_alpha(a: u8) -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a,
        }
    }

    pub const fn additive_luminance(l: u8) -> Self {
        Self {
            r: l,
            g: l,
            b: l,
            a: 0,
        }
    }
}

// ----------------------------------------------------------------------------

pub const TRANSPARENT: Srgba = srgba(0, 0, 0, 0);
pub const BLACK: Srgba = srgba(0, 0, 0, 255);
pub const LIGHT_GRAY: Srgba = srgba(220, 220, 220, 255);
pub const GRAY: Srgba = srgba(160, 160, 160, 255);
pub const WHITE: Srgba = srgba(255, 255, 255, 255);
pub const RED: Srgba = srgba(255, 0, 0, 255);
pub const GREEN: Srgba = srgba(0, 255, 0, 255);
pub const BLUE: Srgba = srgba(0, 0, 255, 255);
pub const YELLOW: Srgba = srgba(255, 255, 0, 255);
pub const LIGHT_BLUE: Srgba = srgba(140, 160, 255, 255);

// ----------------------------------------------------------------------------

impl Rgba {
    pub const TRANSPARENT: Rgba = Rgba::new(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Rgba = Rgba::new(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Rgba = Rgba::new(1.0, 1.0, 1.0, 1.0);
    pub const RED: Rgba = Rgba::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Rgba = Rgba::new(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Rgba = Rgba::new(0.0, 0.0, 1.0, 1.0);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn gray(l: f32) -> Self {
        Self {
            r: l,
            g: l,
            b: l,
            a: 1.0,
        }
    }

    pub fn luminance_alpha(l: f32, a: f32) -> Self {
        debug_assert!(0.0 <= l && l <= 1.0);
        debug_assert!(0.0 <= a && a <= 1.0);
        Self {
            r: l * a,
            g: l * a,
            b: l * a,
            a,
        }
    }

    /// Transparent white
    pub fn white_alpha(a: f32) -> Self {
        debug_assert!(0.0 <= a && a <= 1.0);
        Self {
            r: a,
            g: a,
            b: a,
            a,
        }
    }

    /// Multiply with e.g. 0.5 to make us half transparent
    pub fn multiply(self, alpha: f32) -> Self {
        Self {
            r: alpha * self.r,
            g: alpha * self.g,
            b: alpha * self.b,
            a: alpha * self.a,
        }
    }
}

impl std::ops::Add for Rgba {
    type Output = Rgba;
    fn add(self, rhs: Rgba) -> Rgba {
        Rgba {
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
            a: self.a + rhs.a,
        }
    }
}

impl std::ops::Mul<f32> for Rgba {
    type Output = Rgba;
    fn mul(self, factor: f32) -> Rgba {
        Rgba {
            r: self.r * factor,
            g: self.g * factor,
            b: self.b * factor,
            a: self.a * factor,
        }
    }
}

impl std::ops::Mul<Rgba> for f32 {
    type Output = Rgba;
    fn mul(self, rgba: Rgba) -> Rgba {
        Rgba {
            r: self * rgba.r,
            g: self * rgba.g,
            b: self * rgba.b,
            a: self * rgba.a,
        }
    }
}
