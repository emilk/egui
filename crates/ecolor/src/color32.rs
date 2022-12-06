use crate::{gamma_u8_from_linear_f32, linear_f32_from_gamma_u8, linear_f32_from_linear_u8, Rgba};

/// This format is used for space-efficient color representation (32 bits).
///
/// Instead of manipulating this directly it is often better
/// to first convert it to either [`Rgba`] or [`crate::Hsva`].
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
    pub const fn is_opaque(&self) -> bool {
        self.a() == 255
    }

    #[inline(always)]
    pub const fn r(&self) -> u8 {
        self.0[0]
    }

    #[inline(always)]
    pub const fn g(&self) -> u8 {
        self.0[1]
    }

    #[inline(always)]
    pub const fn b(&self) -> u8 {
        self.0[2]
    }

    #[inline(always)]
    pub const fn a(&self) -> u8 {
        self.0[3]
    }

    /// Returns an opaque version of self
    pub fn to_opaque(self) -> Self {
        Rgba::from(self).to_opaque().into()
    }

    /// Returns an additive version of self
    #[inline(always)]
    pub const fn additive(self) -> Self {
        let [r, g, b, _] = self.to_array();
        Self([r, g, b, 0])
    }

    /// Premultiplied RGBA
    #[inline(always)]
    pub const fn to_array(&self) -> [u8; 4] {
        [self.r(), self.g(), self.b(), self.a()]
    }

    /// Premultiplied RGBA
    #[inline(always)]
    pub const fn to_tuple(&self) -> (u8, u8, u8, u8) {
        (self.r(), self.g(), self.b(), self.a())
    }

    pub fn to_srgba_unmultiplied(&self) -> [u8; 4] {
        Rgba::from(*self).to_srgba_unmultiplied()
    }

    /// Multiply with 0.5 to make color half as opaque.
    pub fn linear_multiply(self, factor: f32) -> Color32 {
        crate::ecolor_assert!(0.0 <= factor && factor <= 1.0);
        // As an unfortunate side-effect of using premultiplied alpha
        // we need a somewhat expensive conversion to linear space and back.
        Rgba::from(self).multiply(factor).into()
    }
}
