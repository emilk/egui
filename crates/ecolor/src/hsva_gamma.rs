use crate::{gamma_from_linear, linear_from_gamma, Color32, Hsva, Rgba};

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
