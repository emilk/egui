// TODO: rename `Color` to `sRGBA` for clarity.
/// 0-255 `sRGBA`. Uses premultiplied alpha.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub const fn srgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color { r, g, b, a }
}

pub const fn gray(l: u8, a: u8) -> Color {
    Color {
        r: l,
        g: l,
        b: l,
        a,
    }
}

pub const fn black(a: u8) -> Color {
    Color {
        r: 0,
        g: 0,
        b: 0,
        a,
    }
}

pub const fn white(a: u8) -> Color {
    Color {
        r: a,
        g: a,
        b: a,
        a,
    }
}

pub const fn additive_gray(l: u8) -> Color {
    Color {
        r: l,
        g: l,
        b: l,
        a: 0,
    }
}

pub const TRANSPARENT: Color = srgba(0, 0, 0, 0);
pub const BLACK: Color = srgba(0, 0, 0, 255);
pub const LIGHT_GRAY: Color = srgba(220, 220, 220, 255);
pub const GRAY: Color = srgba(160, 160, 160, 255);
pub const WHITE: Color = srgba(255, 255, 255, 255);
pub const RED: Color = srgba(255, 0, 0, 255);
pub const GREEN: Color = srgba(0, 255, 0, 255);
pub const BLUE: Color = srgba(0, 0, 255, 255);
pub const YELLOW: Color = srgba(255, 255, 0, 255);
pub const LIGHT_BLUE: Color = srgba(140, 160, 255, 255);
