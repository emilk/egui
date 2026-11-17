//! Easing functions for animations.
//!
//! Contains most easing functions from <https://easings.net/>.
//!
//! All functions take a value in `[0, 1]` and return a value in `[0, 1]`.
//!
//! Derived from <https://github.com/warrenm/AHEasing/blob/master/AHEasing/easing.c>.
use core::f32::consts::{FRAC_PI_2, PI};

use crate::fast_midpoint;

/// One of the easing curves in this module.
///
/// Useful when you want to store the choice of easing, e.g. in a style.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Easing {
    /// No easing, just `y = x`
    #[default]
    Linear,

    /// <https://easings.net/#easeInQuad>
    ///
    /// Modeled after the parabola `y = x^2`
    QuadraticIn,

    /// <https://easings.net/#easeOutQuad>
    ///
    /// Same as `1.0 - quadratic_in(1.0 - t)`.
    QuadraticOut,

    /// <https://easings.net/#easeInOutQuad>
    QuadraticInOut,

    /// <https://easings.net/#easeInCubic>
    ///
    /// Modeled after the parabola `y = x^3`
    CubicIn,

    /// <https://easings.net/#easeOutCubic>
    CubicOut,

    /// <https://easings.net/#easeInOutCubic>
    CubicInOut,

    /// <https://easings.net/#easeInSine>
    ///
    /// Modeled after quarter-cycle of sine wave
    SinIn,

    /// <https://easings.net/#easeOuSine>
    ///
    /// Modeled after quarter-cycle of sine wave (different phase)
    SinOut,

    /// <https://easings.net/#easeInOutSine>
    ///
    /// Modeled after half sine wave
    SinInOut,

    /// <https://easings.net/#easeInCirc>
    ///
    /// Modeled after shifted quadrant IV of unit circle
    CircularIn,

    /// <https://easings.net/#easeOutCirc>
    ///
    /// Modeled after shifted quadrant II of unit circle
    CircularOut,

    /// <https://easings.net/#easeInOutCirc>
    CircularInOut,

    /// <https://easings.net/#easeInExpo>
    ///
    /// There is a small discontinuity at 0.
    ExponentialIn,

    /// <https://easings.net/#easeOutExpo>
    ///
    /// There is a small discontinuity at 1.
    ExponentialOut,

    /// <https://easings.net/#easeInOutExpo>
    ///
    /// There is a small discontinuity at 0 and 1.
    ExponentialInOut,

    /// <https://easings.net/#easeInBack>
    BackIn,

    /// <https://easings.net/#easeOutBack>
    BackOut,

    /// <https://easings.net/#easeInOutBack>
    BackInOut,

    /// <https://easings.net/#easeInBounce>
    ///
    /// Each bounce is modelled as a parabola.
    BounceIn,

    /// <https://easings.net/#easeOutBounce>
    ///
    /// Each bounce is modelled as a parabola.
    BounceOut,

    /// <https://easings.net/#easeInOutBounce>
    ///
    /// Each bounce is modelled as a parabola.
    BounceInOut,
}

impl Easing {
    /// All easing curves, in the same order as the variants.
    pub const ALL: [Self; 22] = [
        Self::Linear,
        Self::QuadraticIn,
        Self::QuadraticOut,
        Self::QuadraticInOut,
        Self::CubicIn,
        Self::CubicOut,
        Self::CubicInOut,
        Self::SinIn,
        Self::SinOut,
        Self::SinInOut,
        Self::CircularIn,
        Self::CircularOut,
        Self::CircularInOut,
        Self::ExponentialIn,
        Self::ExponentialOut,
        Self::ExponentialInOut,
        Self::BackIn,
        Self::BackOut,
        Self::BackInOut,
        Self::BounceIn,
        Self::BounceOut,
        Self::BounceInOut,
    ];

    /// Map `t` in `[0, 1]` through this easing curve.
    #[inline]
    pub fn apply(self, t: f32) -> f32 {
        match self {
            Self::Linear => linear(t),
            Self::QuadraticIn => quadratic_in(t),
            Self::QuadraticOut => quadratic_out(t),
            Self::QuadraticInOut => quadratic_in_out(t),
            Self::CubicIn => cubic_in(t),
            Self::CubicOut => cubic_out(t),
            Self::CubicInOut => cubic_in_out(t),
            Self::SinIn => sin_in(t),
            Self::SinOut => sin_out(t),
            Self::SinInOut => sin_in_out(t),
            Self::CircularIn => circular_in(t),
            Self::CircularOut => circular_out(t),
            Self::CircularInOut => circular_in_out(t),
            Self::ExponentialIn => exponential_in(t),
            Self::ExponentialOut => exponential_out(t),
            Self::ExponentialInOut => exponential_in_out(t),
            Self::BackIn => back_in(t),
            Self::BackOut => back_out(t),
            Self::BackInOut => back_in_out(t),
            Self::BounceIn => bounce_in(t),
            Self::BounceOut => bounce_out(t),
            Self::BounceInOut => bounce_in_out(t),
        }
    }
}

#[inline]
fn powf(base: f32, exp: f32) -> f32 {
    base.powf(exp)
}

/// No easing, just `y = x`
#[inline]
pub fn linear(t: f32) -> f32 {
    t
}

/// <https://easings.net/#easeInQuad>
///
/// Modeled after the parabola `y = x^2`
#[inline]
pub fn quadratic_in(t: f32) -> f32 {
    t * t
}

/// <https://easings.net/#easeOutQuad>
///
/// Same as `1.0 - quadratic_in(1.0 - t)`.
#[inline]
pub fn quadratic_out(t: f32) -> f32 {
    -(t * (t - 2.))
}

/// <https://easings.net/#easeInOutQuad>
#[inline]
pub fn quadratic_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2. * t * t
    } else {
        (-2. * t * t) + (4. * t) - 1.
    }
}

/// <https://easings.net/#easeInCubic>
///
/// Modeled after the parabola `y = x^3`
#[inline]
pub fn cubic_in(t: f32) -> f32 {
    t * t * t
}

/// <https://easings.net/#easeOutCubic>
#[inline]
pub fn cubic_out(t: f32) -> f32 {
    let f = t - 1.;
    f * f * f + 1.
}

/// <https://easings.net/#easeInOutCubic>
#[inline]
pub fn cubic_in_out(t: f32) -> f32 {
    if t < 0.5 {
        4. * t * t * t
    } else {
        let f = (2. * t) - 2.;
        0.5 * f * f * f + 1.
    }
}

/// <https://easings.net/#easeInSine>
///
/// Modeled after quarter-cycle of sine wave
#[inline]
pub fn sin_in(t: f32) -> f32 {
    ((t - 1.) * FRAC_PI_2).sin() + 1.
}

/// <https://easings.net/#easeOuSine>
///
/// Modeled after quarter-cycle of sine wave (different phase)
#[inline]
pub fn sin_out(t: f32) -> f32 {
    (t * FRAC_PI_2).sin()
}

/// <https://easings.net/#easeInOutSine>
///
/// Modeled after half sine wave
#[inline]
pub fn sin_in_out(t: f32) -> f32 {
    0.5 * (1. - (t * PI).cos())
}

/// <https://easings.net/#easeInCirc>
///
/// Modeled after shifted quadrant IV of unit circle
#[inline]
pub fn circular_in(t: f32) -> f32 {
    1. - (1. - t * t).sqrt()
}

/// <https://easings.net/#easeOutCirc>
///
/// Modeled after shifted quadrant II of unit circle
#[inline]
pub fn circular_out(t: f32) -> f32 {
    (2. - t).sqrt() * t
}

/// <https://easings.net/#easeInOutCirc>
#[inline]
pub fn circular_in_out(t: f32) -> f32 {
    if t < 0.5 {
        0.5 * (1. - (1. - 4. * t * t).sqrt())
    } else {
        fast_midpoint((-(2. * t - 3.) * (2. * t - 1.)).sqrt(), 1.)
    }
}

/// <https://easings.net/#easeInExpo>
///
/// There is a small discontinuity at 0.
#[inline]
pub fn exponential_in(t: f32) -> f32 {
    if t == 0. {
        t
    } else {
        powf(2.0, 10. * (t - 1.))
    }
}

/// <https://easings.net/#easeOutExpo>
///
/// There is a small discontinuity at 1.
#[inline]
pub fn exponential_out(t: f32) -> f32 {
    if t == 1. { t } else { 1. - powf(2.0, -10. * t) }
}

/// <https://easings.net/#easeInOutExpo>
///
/// There is a small discontinuity at 0 and 1.
#[inline]
pub fn exponential_in_out(t: f32) -> f32 {
    if t == 0. || t == 1. {
        t
    } else if t < 0.5 {
        0.5 * powf(2.0, 20. * t - 10.)
    } else {
        0.5 * powf(2.0, -20. * t + 10.) + 1.
    }
}

/// <https://easings.net/#easeInBack>
#[inline]
pub fn back_in(t: f32) -> f32 {
    t * t * t - t * (t * PI).sin()
}

/// <https://easings.net/#easeOutBack>
#[inline]
pub fn back_out(t: f32) -> f32 {
    let f = 1. - t;
    1. - (f * f * f - f * (f * PI).sin())
}

/// <https://easings.net/#easeInOutBack>
#[inline]
pub fn back_in_out(t: f32) -> f32 {
    if t < 0.5 {
        let f = 2. * t;
        0.5 * (f * f * f - f * (f * PI).sin())
    } else {
        let f = 1. - (2. * t - 1.);
        0.5 * (1. - (f * f * f - f * (f * PI).sin())) + 0.5
    }
}

/// <https://easings.net/#easeInBounce>
///
/// Each bounce is modelled as a parabola.
#[inline]
pub fn bounce_in(t: f32) -> f32 {
    1. - bounce_out(1. - t)
}

/// <https://easings.net/#easeOutBounce>
///
/// Each bounce is modelled as a parabola.
#[inline]
pub fn bounce_out(t: f32) -> f32 {
    if t < 4. / 11. {
        const T2: f32 = 121. / 16.;
        T2 * t * t
    } else if t < 8. / 11. {
        const T2: f32 = 363. / 40.;
        const T1: f32 = -99. / 10.;
        const T0: f32 = 17. / 5.;
        T2 * t * t + T1 * t + T0
    } else if t < 9. / 10. {
        const T2: f32 = 4356. / 361.;
        const T1: f32 = -35442. / 1805.;
        const T0: f32 = 16061. / 1805.;
        T2 * t * t + T1 * t + T0
    } else {
        const T2: f32 = 54. / 5.;
        const T1: f32 = -513. / 25.;
        const T0: f32 = 268. / 25.;
        T2 * t * t + T1 * t + T0
    }
}

/// <https://easings.net/#easeInOutBounce>
///
/// Each bounce is modelled as a parabola.
#[inline]
pub fn bounce_in_out(t: f32) -> f32 {
    if t < 0.5 {
        0.5 * bounce_in(t * 2.)
    } else {
        0.5 * bounce_out(t * 2. - 1.) + 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::Easing;

    #[test]
    fn endpoints() {
        for easing in Easing::ALL {
            assert!(easing.apply(0.0).abs() < 1e-5, "{easing:?}");
            assert!((easing.apply(1.0) - 1.0).abs() < 1e-5, "{easing:?}");
        }
    }
}
