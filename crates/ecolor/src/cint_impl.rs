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
