//! Easing functions for animations.
//!
//! Contains most easing functions from <https://easings.net/>.
//!
//! All functions take a value in `[0, 1]` and map 0 to 0 and 1 to 1.
//! The output is also in `[0, 1]`, except for the `back_*` functions, which overshoot.
//!
//! "Ease in" means the motion starts slow and speeds up.
//! "Ease out" means the motion starts fast and slows down.
//! Use [`Easing`] to store the choice of curve as a value.
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
    /// Constant speed: no easing in or out.
    ///
    /// `y = x`
    #[default]
    Linear,

    /// Ease in: starts slow, ends fast. Gentle acceleration.
    ///
    /// Modeled after the parabola `y = x^2`.
    ///
    /// <https://easings.net/#easeInQuad>
    QuadraticIn,

    /// Ease out: starts fast, ends slow. Gentle deceleration.
    ///
    /// Same as `1.0 - quadratic_in(1.0 - t)`.
    ///
    /// <https://easings.net/#easeOutQuad>
    QuadraticOut,

    /// Ease in and out: slow at both ends, fast in the middle. Gentle.
    ///
    /// <https://easings.net/#easeInOutQuad>
    QuadraticInOut,

    /// Ease in: starts slow, ends fast. Stronger than [`Self::QuadraticIn`].
    ///
    /// Modeled after the curve `y = x^3`.
    ///
    /// <https://easings.net/#easeInCubic>
    CubicIn,

    /// Ease out: starts fast, ends slow. Stronger than [`Self::QuadraticOut`].
    ///
    /// Feels responsive, since most of the motion happens early.
    ///
    /// <https://easings.net/#easeOutCubic>
    CubicOut,

    /// Ease in and out: slow at both ends, fast in the middle. Stronger than
    /// [`Self::QuadraticInOut`].
    ///
    /// <https://easings.net/#easeInOutCubic>
    CubicInOut,

    /// Ease in: starts slow, ends fast. Very gentle, softer than [`Self::QuadraticIn`].
    ///
    /// Modeled after a quarter-cycle of a sine wave.
    ///
    /// <https://easings.net/#easeInSine>
    SinIn,

    /// Ease out: starts fast, ends slow. Very gentle, softer than [`Self::QuadraticOut`].
    ///
    /// Modeled after a quarter-cycle of a sine wave.
    ///
    /// <https://easings.net/#easeOutSine>
    SinOut,

    /// Ease in and out: slow at both ends, fast in the middle. Very gentle, softer than
    /// [`Self::QuadraticInOut`].
    ///
    /// Modeled after half a cycle of a cosine wave.
    ///
    /// <https://easings.net/#easeInOutSine>
    SinInOut,

    /// Ease in: starts very slow, ends very fast, with a vertical slope at the end.
    ///
    /// Modeled after shifted quadrant IV of the unit circle.
    ///
    /// <https://easings.net/#easeInCirc>
    CircularIn,

    /// Ease out: starts very fast, with a vertical slope, and ends very slow.
    ///
    /// Modeled after shifted quadrant II of the unit circle.
    ///
    /// <https://easings.net/#easeOutCirc>
    CircularOut,

    /// Ease in and out: very slow at both ends, with a steep jump in the middle.
    ///
    /// <https://easings.net/#easeInOutCirc>
    CircularInOut,

    /// Ease in: almost no motion at first, then very fast at the end. Strongest of the power
    /// curves.
    ///
    /// There is a small discontinuity at 0.
    ///
    /// <https://easings.net/#easeInExpo>
    ExponentialIn,

    /// Ease out: very fast at first, then almost no motion at the end. Strongest of the power
    /// curves.
    ///
    /// There is a small discontinuity at 1.
    ///
    /// <https://easings.net/#easeOutExpo>
    ExponentialOut,

    /// Ease in and out: almost still at both ends, with a sharp jump in the middle.
    ///
    /// There is a small discontinuity at 0 and 1.
    ///
    /// <https://easings.net/#easeInOutExpo>
    ExponentialInOut,

    /// Ease in with anticipation: first pulls back below 0, then accelerates to 1.
    ///
    /// The output goes outside `[0, 1]`.
    ///
    /// <https://easings.net/#easeInBack>
    BackIn,

    /// Ease out with overshoot: goes past 1, then settles back to 1.
    ///
    /// The output goes outside `[0, 1]`.
    ///
    /// <https://easings.net/#easeOutBack>
    BackOut,

    /// Ease in and out with anticipation and overshoot: dips below 0 at the start and goes past 1
    /// at the end.
    ///
    /// The output goes outside `[0, 1]`.
    ///
    /// <https://easings.net/#easeInOutBack>
    BackInOut,

    /// Bounces in: several small bounces off 0 that grow, before reaching 1.
    ///
    /// Each bounce is modeled as a parabola.
    ///
    /// <https://easings.net/#easeInBounce>
    BounceIn,

    /// Bounces out: reaches 1 quickly, then bounces off 1 with shrinking bounces, like a dropped
    /// ball.
    ///
    /// Each bounce is modeled as a parabola.
    ///
    /// <https://easings.net/#easeOutBounce>
    BounceOut,

    /// Bounces in and out: [`Self::BounceIn`] for the first half, [`Self::BounceOut`] for the
    /// second.
    ///
    /// Each bounce is modeled as a parabola.
    ///
    /// <https://easings.net/#easeInOutBounce>
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

/// Constant speed: no easing in or out.
///
/// `y = x`
#[inline]
pub fn linear(t: f32) -> f32 {
    t
}

/// Ease in: starts slow, ends fast. Gentle acceleration.
///
/// Modeled after the parabola `y = x^2`.
///
/// <https://easings.net/#easeInQuad>
#[inline]
pub fn quadratic_in(t: f32) -> f32 {
    t * t
}

/// Ease out: starts fast, ends slow. Gentle deceleration.
///
/// Same as `1.0 - quadratic_in(1.0 - t)`.
///
/// <https://easings.net/#easeOutQuad>
#[inline]
pub fn quadratic_out(t: f32) -> f32 {
    -(t * (t - 2.))
}

/// Ease in and out: slow at both ends, fast in the middle. Gentle.
///
/// <https://easings.net/#easeInOutQuad>
#[inline]
pub fn quadratic_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2. * t * t
    } else {
        (-2. * t * t) + (4. * t) - 1.
    }
}

/// Ease in: starts slow, ends fast. Stronger than [`quadratic_in`].
///
/// Modeled after the curve `y = x^3`.
///
/// <https://easings.net/#easeInCubic>
#[inline]
pub fn cubic_in(t: f32) -> f32 {
    t * t * t
}

/// Ease out: starts fast, ends slow. Stronger than [`quadratic_out`].
///
/// Feels responsive, since most of the motion happens early.
///
/// <https://easings.net/#easeOutCubic>
#[inline]
pub fn cubic_out(t: f32) -> f32 {
    let f = t - 1.;
    f * f * f + 1.
}

/// Ease in and out: slow at both ends, fast in the middle. Stronger than [`quadratic_in_out`].
///
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

/// Ease in: starts slow, ends fast. Very gentle, softer than [`quadratic_in`].
///
/// Modeled after a quarter-cycle of a sine wave.
///
/// <https://easings.net/#easeInSine>
#[inline]
pub fn sin_in(t: f32) -> f32 {
    ((t - 1.) * FRAC_PI_2).sin() + 1.
}

/// Ease out: starts fast, ends slow. Very gentle, softer than [`quadratic_out`].
///
/// Modeled after a quarter-cycle of a sine wave.
///
/// <https://easings.net/#easeOutSine>
#[inline]
pub fn sin_out(t: f32) -> f32 {
    (t * FRAC_PI_2).sin()
}

/// Ease in and out: slow at both ends, fast in the middle. Very gentle, softer than
/// [`quadratic_in_out`].
///
/// Modeled after half a cycle of a cosine wave.
///
/// <https://easings.net/#easeInOutSine>
#[inline]
pub fn sin_in_out(t: f32) -> f32 {
    0.5 * (1. - (t * PI).cos())
}

/// Ease in: starts very slow, ends very fast, with a vertical slope at the end.
///
/// Modeled after shifted quadrant IV of the unit circle.
///
/// <https://easings.net/#easeInCirc>
#[inline]
pub fn circular_in(t: f32) -> f32 {
    1. - (1. - t * t).sqrt()
}

/// Ease out: starts very fast, with a vertical slope, and ends very slow.
///
/// Modeled after shifted quadrant II of the unit circle.
///
/// <https://easings.net/#easeOutCirc>
#[inline]
pub fn circular_out(t: f32) -> f32 {
    (2. - t).sqrt() * t
}

/// Ease in and out: very slow at both ends, with a steep jump in the middle.
///
/// <https://easings.net/#easeInOutCirc>
#[inline]
pub fn circular_in_out(t: f32) -> f32 {
    if t < 0.5 {
        0.5 * (1. - (1. - 4. * t * t).sqrt())
    } else {
        fast_midpoint((-(2. * t - 3.) * (2. * t - 1.)).sqrt(), 1.)
    }
}

/// Ease in: almost no motion at first, then very fast at the end.
///
/// There is a small discontinuity at 0.
///
/// <https://easings.net/#easeInExpo>
#[inline]
pub fn exponential_in(t: f32) -> f32 {
    if t == 0. {
        t
    } else {
        powf(2.0, 10. * (t - 1.))
    }
}

/// Ease out: very fast at first, then almost no motion at the end.
///
/// There is a small discontinuity at 1.
///
/// <https://easings.net/#easeOutExpo>
#[inline]
pub fn exponential_out(t: f32) -> f32 {
    if t == 1. { t } else { 1. - powf(2.0, -10. * t) }
}

/// Ease in and out: almost still at both ends, with a sharp jump in the middle.
///
/// There is a small discontinuity at 0 and 1.
///
/// <https://easings.net/#easeInOutExpo>
#[inline]
pub fn exponential_in_out(t: f32) -> f32 {
    if t == 0. || t == 1. {
        t
    } else if t < 0.5 {
        0.5 * powf(2.0, 20. * t - 10.)
    } else {
        1. - 0.5 * powf(2.0, -20. * t + 10.)
    }
}

/// Ease in with anticipation: first pulls back below 0, then accelerates to 1.
///
/// The output goes outside `[0, 1]`.
///
/// <https://easings.net/#easeInBack>
#[inline]
pub fn back_in(t: f32) -> f32 {
    t * t * t - t * (t * PI).sin()
}

/// Ease out with overshoot: goes past 1, then settles back to 1.
///
/// The output goes outside `[0, 1]`.
///
/// <https://easings.net/#easeOutBack>
#[inline]
pub fn back_out(t: f32) -> f32 {
    let f = 1. - t;
    1. - (f * f * f - f * (f * PI).sin())
}

/// Ease in and out with anticipation and overshoot: dips below 0 at the start and goes past 1 at
/// the end.
///
/// The output goes outside `[0, 1]`.
///
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

/// Bounces in: several small bounces off 0 that grow, before reaching 1.
///
/// Each bounce is modeled as a parabola.
///
/// <https://easings.net/#easeInBounce>
#[inline]
pub fn bounce_in(t: f32) -> f32 {
    1. - bounce_out(1. - t)
}

/// Bounces out: reaches 1 quickly, then bounces off 1 with shrinking bounces, like a dropped ball.
///
/// Each bounce is modeled as a parabola.
///
/// <https://easings.net/#easeOutBounce>
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

/// Bounces in and out: [`bounce_in`] for the first half, [`bounce_out`] for the second.
///
/// Each bounce is modeled as a parabola.
///
/// <https://easings.net/#easeInOutBounce>
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

    #[test]
    fn stays_in_unit_range() {
        let overshooting = [Easing::BackIn, Easing::BackOut, Easing::BackInOut];
        for easing in Easing::ALL {
            if overshooting.contains(&easing) {
                continue;
            }
            for i in 0..=100 {
                let t = i as f32 / 100.0;
                let y = easing.apply(t);
                assert!((-1e-5..=1.0 + 1e-5).contains(&y), "{easing:?}({t}) = {y}");
            }
        }
    }
}
