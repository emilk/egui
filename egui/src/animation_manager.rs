use std::collections::HashMap;

use crate::{
    math::{clamp, remap_clamp},
    Id, InputState,
};

#[derive(Clone, Default)]
pub(crate) struct AnimationManager {
    bools: HashMap<Id, BoolAnim>,
}

#[derive(Clone, Debug)]
struct BoolAnim {
    value: bool,
    /// when did `value` last toggle?
    toggle_time: f64,
}

impl AnimationManager {
    /// See `Context::animate_bool` for documentation
    pub fn animate_bool(
        &mut self,
        input: &InputState,
        animation_time: f32,
        id: Id,
        value: bool,
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

                ease_in_out_cubic(if value {
                    remap_clamp(time_since_toggle, 0.0..=animation_time, 0.0..=1.0)
                } else {
                    remap_clamp(time_since_toggle, 0.0..=animation_time, 1.0..=0.0)
                })
            }
        }
    }
}

/// Maps `x` in the range [0, 1] to the cubic easing function. Clamps outside of this range.
///
/// See: https://www.desmos.com/calculator/tdvd2vd4f1
fn ease_in_out_cubic(x: f32) -> f32 {
    clamp(
        if x < 0.5 {
            0.5 * (2.0 * x).powi(3)
        } else {
            0.5 * ((2.0 * x - 2.0).powi(3) + 2.0)
        },
        0.0..=1.0,
    )
}
