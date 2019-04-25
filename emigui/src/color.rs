/// 0-255 sRGBA
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize)]
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

pub const WHITE: Color = srgba(255, 255, 255, 255);
