use serde_derive::{Deserialize, Serialize};

/// 0-255 `sRGBA`. TODO: rename `sRGBA` for clarity.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn transparent(self) -> Color {
        Color {
            r: self.r,
            g: self.g,
            b: self.b,
            a: 0,
        }
    }
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

pub const fn white(a: u8) -> Color {
    Color {
        r: 255,
        g: 255,
        b: 255,
        a,
    }
}

pub const BLACK: Color = srgba(0, 0, 0, 255);
pub const LIGHT_GRAY: Color = srgba(220, 220, 220, 255);
pub const WHITE: Color = srgba(255, 255, 255, 255);
pub const RED: Color = srgba(255, 0, 0, 255);
pub const GREEN: Color = srgba(0, 255, 0, 255);
pub const BLUE: Color = srgba(0, 0, 255, 255);
pub const YELLOW: Color = srgba(255, 255, 0, 255);
pub const LIGHT_BLUE: Color = srgba(140, 160, 255, 255);
