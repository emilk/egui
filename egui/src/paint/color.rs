use crate::math::clamp;

/// 0-255 gamma space `sRGBA` color with premultiplied alpha.
/// Alpha channel is in linear space.
/// This format is used for space-efficient color representation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Srgba(pub [u8; 4]);

impl std::ops::Index<usize> for Srgba {
    type Output = u8;
    fn index(&self, index: usize) -> &u8 {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for Srgba {
    fn index_mut(&mut self, index: usize) -> &mut u8 {
        &mut self.0[index]
    }
}

pub const fn srgba(r: u8, g: u8, b: u8, a: u8) -> Srgba {
    Srgba([r, g, b, a])
}

impl Srgba {
    pub const fn gray(l: u8) -> Self {
        Self([l, l, l, 255])
    }

    pub const fn black_alpha(a: u8) -> Self {
        Self([0, 0, 0, a])
    }

    pub const fn additive_luminance(l: u8) -> Self {
        Self([l, l, l, 0])
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

/// 0-1 linear space `RGBA` color with premultiplied alpha.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Rgba(pub [f32; 4]);

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
    pub const TRANSPARENT: Rgba = Rgba::new(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Rgba = Rgba::new(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Rgba = Rgba::new(1.0, 1.0, 1.0, 1.0);
    pub const RED: Rgba = Rgba::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Rgba = Rgba::new(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Rgba = Rgba::new(0.0, 0.0, 1.0, 1.0);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self([r, g, b, a])
    }

    pub const fn gray(l: f32) -> Self {
        Self([l, l, l, 1.0])
    }

    pub fn luminance_alpha(l: f32, a: f32) -> Self {
        debug_assert!(0.0 <= l && l <= 1.0);
        debug_assert!(0.0 <= a && a <= 1.0);
        Self([l * a, l * a, l * a, a])
    }

    /// Transparent white
    pub fn white_alpha(a: f32) -> Self {
        debug_assert!(0.0 <= a && a <= 1.0);
        Self([a, a, a, a])
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

impl From<Srgba> for Rgba {
    fn from(srgba: Srgba) -> Rgba {
        Rgba([
            linear_from_srgb_byte(srgba[0]),
            linear_from_srgb_byte(srgba[1]),
            linear_from_srgb_byte(srgba[2]),
            srgba[3] as f32 / 255.0,
        ])
    }
}

impl From<Rgba> for Srgba {
    fn from(rgba: Rgba) -> Srgba {
        Srgba([
            srgb_byte_from_linear(rgba[0]),
            srgb_byte_from_linear(rgba[1]),
            srgb_byte_from_linear(rgba[2]),
            clamp(rgba[3] * 255.0, 0.0..=255.0).round() as u8,
        ])
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
