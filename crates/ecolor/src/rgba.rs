use crate::{
    gamma_u8_from_linear_f32, linear_f32_from_gamma_u8, linear_f32_from_linear_u8,
    linear_u8_from_linear_f32,
};

/// 0-1 linear space `RGBA` color with premultiplied alpha.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Rgba(pub(crate) [f32; 4]);

impl std::ops::Index<usize> for Rgba {
    type Output = f32;

    #[inline]
    fn index(&self, index: usize) -> &f32 {
        &self.0[index]
    }
}

impl std::ops::IndexMut<usize> for Rgba {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut f32 {
        &mut self.0[index]
    }
}

/// Deterministically hash an `f32`, treating all NANs as equal, and ignoring the sign of zero.
#[inline]
pub(crate) fn f32_hash<H: std::hash::Hasher>(state: &mut H, f: f32) {
    if f == 0.0 {
        state.write_u8(0);
    } else if f.is_nan() {
        state.write_u8(1);
    } else {
        use std::hash::Hash;
        f.to_bits().hash(state);
    }
}

#[allow(clippy::derived_hash_with_manual_eq)]
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
    pub const TRANSPARENT: Self = Self::from_rgba_premultiplied(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Self = Self::from_rgb(0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::from_rgb(1.0, 1.0, 1.0);
    pub const RED: Self = Self::from_rgb(1.0, 0.0, 0.0);
    pub const GREEN: Self = Self::from_rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Self::from_rgb(0.0, 0.0, 1.0);

    #[inline]
    pub const fn from_rgba_premultiplied(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self([r, g, b, a])
    }

    #[inline]
    pub fn from_rgba_unmultiplied(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self([r * a, g * a, b * a, a])
    }

    #[inline]
    pub fn from_srgba_premultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        let r = linear_f32_from_gamma_u8(r);
        let g = linear_f32_from_gamma_u8(g);
        let b = linear_f32_from_gamma_u8(b);
        let a = linear_f32_from_linear_u8(a);
        Self::from_rgba_premultiplied(r, g, b, a)
    }

    #[inline]
    pub fn from_srgba_unmultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        let r = linear_f32_from_gamma_u8(r);
        let g = linear_f32_from_gamma_u8(g);
        let b = linear_f32_from_gamma_u8(b);
        let a = linear_f32_from_linear_u8(a);
        Self::from_rgba_premultiplied(r * a, g * a, b * a, a)
    }

    #[inline]
    pub const fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self([r, g, b, 1.0])
    }

    #[inline]
    pub const fn from_gray(l: f32) -> Self {
        Self([l, l, l, 1.0])
    }

    #[inline]
    pub fn from_luminance_alpha(l: f32, a: f32) -> Self {
        crate::ecolor_assert!(0.0 <= l && l <= 1.0);
        crate::ecolor_assert!(0.0 <= a && a <= 1.0);
        Self([l * a, l * a, l * a, a])
    }

    /// Transparent black
    #[inline]
    pub fn from_black_alpha(a: f32) -> Self {
        crate::ecolor_assert!(0.0 <= a && a <= 1.0);
        Self([0.0, 0.0, 0.0, a])
    }

    /// Transparent white
    #[inline]
    pub fn from_white_alpha(a: f32) -> Self {
        crate::ecolor_assert!(0.0 <= a && a <= 1.0, "a: {}", a);
        Self([a, a, a, a])
    }

    /// Return an additive version of this color (alpha = 0)
    #[inline]
    pub fn additive(self) -> Self {
        let [r, g, b, _] = self.0;
        Self([r, g, b, 0.0])
    }

    /// Is the alpha=0 ?
    #[inline]
    pub fn is_additive(self) -> bool {
        self.a() == 0.0
    }

    /// Multiply with e.g. 0.5 to make us half transparent
    #[inline]
    pub fn multiply(self, alpha: f32) -> Self {
        Self([
            alpha * self[0],
            alpha * self[1],
            alpha * self[2],
            alpha * self[3],
        ])
    }

    #[inline]
    pub fn r(&self) -> f32 {
        self.0[0]
    }

    #[inline]
    pub fn g(&self) -> f32 {
        self.0[1]
    }

    #[inline]
    pub fn b(&self) -> f32 {
        self.0[2]
    }

    #[inline]
    pub fn a(&self) -> f32 {
        self.0[3]
    }

    /// How perceptually intense (bright) is the color?
    #[inline]
    pub fn intensity(&self) -> f32 {
        0.3 * self.r() + 0.59 * self.g() + 0.11 * self.b()
    }

    /// Returns an opaque version of self
    #[inline]
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
    #[inline]
    pub fn to_array(&self) -> [f32; 4] {
        [self.r(), self.g(), self.b(), self.a()]
    }

    /// Premultiplied RGBA
    #[inline]
    pub fn to_tuple(&self) -> (f32, f32, f32, f32) {
        (self.r(), self.g(), self.b(), self.a())
    }

    /// unmultiply the alpha
    #[inline]
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
    #[inline]
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
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self([
            self[0] + rhs[0],
            self[1] + rhs[1],
            self[2] + rhs[2],
            self[3] + rhs[3],
        ])
    }
}

impl std::ops::Mul for Rgba {
    type Output = Self;

    #[inline]
    fn mul(self, other: Self) -> Self {
        Self([
            self[0] * other[0],
            self[1] * other[1],
            self[2] * other[2],
            self[3] * other[3],
        ])
    }
}

impl std::ops::Mul<f32> for Rgba {
    type Output = Self;

    #[inline]
    fn mul(self, factor: f32) -> Self {
        Self([
            self[0] * factor,
            self[1] * factor,
            self[2] * factor,
            self[3] * factor,
        ])
    }
}

impl std::ops::Mul<Rgba> for f32 {
    type Output = Rgba;

    #[inline]
    fn mul(self, rgba: Rgba) -> Rgba {
        Rgba([
            self * rgba[0],
            self * rgba[1],
            self * rgba[2],
            self * rgba[3],
        ])
    }
}
