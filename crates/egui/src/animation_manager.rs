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

                if value {
                    remap_clamp(time_since_toggle, 0.0..=animation_time, 0.0..=1.0)
                } else {
                    remap_clamp(time_since_toggle, 0.0..=animation_time, 1.0..=0.0)
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
                let current_value = remap_clamp(
                    time_since_toggle,
                    0.0..=animation_time,
                    anim.from_value..=anim.to_value,
                );
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
