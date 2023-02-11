use crate::{emath::remap_clamp, Id, IdMap, InputState};

#[derive(Clone, Default)]
pub(crate) struct AnimationManager {
    bools: IdMap<BoolAnim>,
    values: IdMap<ValueAnim>,
}

#[derive(Clone, Debug)]
struct BoolAnim {
    value: bool,

    /// when did `value` last toggle?
    toggle_time: f64,
}

#[derive(Clone, Debug)]
struct ValueAnim {
    from_value: f32,

    to_value: f32,

    /// when did `value` last toggle?
    toggle_time: f64,
}

#[derive(Clone, Copy, Debug)]
pub enum Ease {
    /// Simple linear easing.
    Linear,
    /// Cubic bezier curve, corresponding to the control points `P1.x`, `P1.y`, `P2.x`, `P2.y`.
    /// Extremely versatile for animation.
    CubicBezier(f32, f32, f32, f32),
    /// User defined shaping function. Given a `time` within `0..=1`, this function should remap to
    /// a new value, usually - but not necessarily -  within the same range.
    Custom(fn(f32) -> f32),
}

impl Ease {
    /// Maximum allowable error for iterative bezier solve
    const EPSILON: f32 = 0.0000001;

    /// Maximum number of iterations during bezier solve
    const MAX_ITERS: u8 = 8;

    /// Given a `time` within `0..=1`, remaps this using a shaping function to a new value. The new
    /// value may be outside of this range.
    pub fn remap(&self, time: f32) -> f32 {
        debug_assert!((0.0..=1.0).contains(&time));
        match *self {
            Ease::Linear => time,
            Ease::CubicBezier(p1x, p1y, p2x, p2y) => {
                let t = Self::find_t(time, p1x, p2x);
                Self::bezier_position(t, p1y, p2y)
            }
            Ease::Custom(f) => f(time),
        }
    }

    /// Compute the bezier position at the given time `t` using De Casteljau's method.
    pub(crate) fn bezier_position(t: f32, p1x: f32, p2x: f32) -> f32 {
        let p0x = 0.0;
        let p3x = 1.0;
        p0x * (1. - t).powi(3)
            + p1x * t * (3. * (1. - t).powi(2))
            + p2x * 3. * (1. - t) * t.powi(2)
            + p3x * t.powi(3)
    }

    /// Compute the slope of a cubic bezier at the given parametric value `t`.
    pub(crate) fn bezier_slope(t: f32, p1x: f32, p2x: f32) -> f32 {
        let p0x = 0.0;
        let p3x = 1.0;
        3. * (1. - t).powi(2) * (p1x - p0x)
            + 6. * (1. - t) * t * (p2x - p1x)
            + 3. * t.powi(2) * (p3x - p2x)
    }

    /// Searches for the parametric value `t` that produces the desired output value of `x`, along
    /// the bezier curve with control points `p1x` and `p2x`, given `p0x = 0` and `p3x = 1`.
    pub(crate) fn find_t(x: f32, p1x: f32, p2x: f32) -> f32 {
        // We will use the desired value x as our initial guess for t. This is a good estimate, as
        // cubic bezier curves for animation are usually near the line where x = t.
        (0..Self::MAX_ITERS).fold(x, |guess, _| {
            if (x - guess).abs() < Self::EPSILON {
                guess
            } else {
                let position = Self::bezier_position(guess, p1x, p2x);
                let slope = Self::bezier_slope(guess, p1x, p2x);
                guess - position / slope
            }
        })
    }

    /// Material Design "Standard curve".
    pub const fn standard() -> Self {
        Self::CubicBezier(0.4, 0.0, 0.2, 1.0)
    }

    /// Material Design "Deceleration curve" useful for objects leaving the screen.
    pub const fn ease_out() -> Self {
        Self::CubicBezier(0.0, 0.0, 0.2, 1.0)
    }

    /// Material Design "Acceleration curve" useful for objects entering the screen.
    pub const fn ease_in() -> Self {
        Self::CubicBezier(0.4, 0.0, 1.0, 1.0)
    }

    /// Material Design "Sharp curve" useful for objects exiting the screen that may return
    /// onscreen.
    pub const fn ease_in_out() -> Self {
        Self::CubicBezier(0.4, 0.0, 0.6, 1.0)
    }
}

impl AnimationManager {
    /// See `Context::animate_bool` for documentation
    pub fn animate_bool(
        &mut self,
        input: &InputState,
        animation_time: f32,
        id: Id,
        value: bool,
        easing: Ease,
    ) -> f32 {
        match self.bools.get_mut(&id) {
            None => {
                self.bools.insert(
                    id,
                    BoolAnim {
                        value,
                        toggle_time: -f64::INFINITY, // long time ago
                    },
                );
                if value {
                    1.0
                } else {
                    0.0
                }
            }
            Some(anim) => {
                if anim.value != value {
                    anim.value = value;
                    anim.toggle_time = input.time;
                }

                let time_since_toggle = (input.time - anim.toggle_time) as f32;

                // On the frame we toggle we don't want to return the old value,
                // so we extrapolate forwards:
                let time_since_toggle = time_since_toggle + input.predicted_dt;

                let eased_value = easing.remap(time_since_toggle / animation_time);

                if value {
                    // Don't need to use remap_clamp because the range is already 0..=1.
                    f32::clamp(eased_value, 0.0, 1.0)
                } else {
                    remap_clamp(eased_value, 0.0..=1.0, 1.0..=0.0)
                }
            }
        }
    }

    pub fn animate_value(
        &mut self,
        input: &InputState,
        animation_time: f32,
        id: Id,
        value: f32,
        easing: Ease,
    ) -> f32 {
        match self.values.get_mut(&id) {
            None => {
                self.values.insert(
                    id,
                    ValueAnim {
                        from_value: value,
                        to_value: value,
                        toggle_time: -f64::INFINITY, // long time ago
                    },
                );
                value
            }
            Some(anim) => {
                let time_since_toggle = (input.time - anim.toggle_time) as f32;
                // On the frame we toggle we don't want to return the old value,
                // so we extrapolate forwards:
                let time_since_toggle = time_since_toggle + input.predicted_dt;

                let eased_value = easing.remap(time_since_toggle / animation_time);

                let current_value =
                    remap_clamp(eased_value, 0.0..=1.0, anim.from_value..=anim.to_value);
                if anim.to_value != value {
                    anim.from_value = current_value; //start new animation from current position of playing animation
                    anim.to_value = value;
                    anim.toggle_time = input.time;
                }
                if animation_time == 0.0 {
                    anim.from_value = value;
                    anim.to_value = value;
                }
                current_value
            }
        }
    }
}
