use std::{collections::BTreeMap, fmt::Debug};

use crate::{
    data::input::TouchDeviceId,
    emath::{normalized_angle, Pos2, Vec2},
    Event, RawInput, TouchId, TouchPhase,
};

/// All you probably need to know about a multi-touch gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MultiTouchInfo {
    /// Point in time when the gesture started.
    pub start_time: f64,

    /// Position of the pointer at the time the gesture started.
    pub start_pos: Pos2,

    /// Number of touches (fingers) on the surface. Value is ≥ 2 since for a single touch no
    /// [`MultiTouchInfo`] is created.
    pub num_touches: usize,

    /// Proportional zoom factor (pinch gesture).
    /// * `zoom = 1`: no change
    /// * `zoom < 1`: pinch together
    /// * `zoom > 1`: pinch spread
    pub zoom_delta: f32,

    /// 2D non-proportional zoom factor (pinch gesture).
    ///
    /// For horizontal pinches, this will return `[z, 1]`,
    /// for vertical pinches this will return `[1, z]`,
    /// and otherwise this will return `[z, z]`,
    /// where `z` is the zoom factor:
    /// * `zoom = 1`: no change
    /// * `zoom < 1`: pinch together
    /// * `zoom > 1`: pinch spread
    pub zoom_delta_2d: Vec2,

    /// Rotation in radians. Moving fingers around each other will change this value. This is a
    /// relative value, comparing the orientation of fingers in the current frame with the previous
    /// frame. If all fingers are resting, this value is `0.0`.
    pub rotation_delta: f32,

    /// Relative movement (comparing previous frame and current frame) of the average position of
    /// all touch points. Without movement this value is `Vec2::ZERO`.
    ///
    /// Note that this may not necessarily be measured in screen points (although it _will_ be for
    /// most mobile devices). In general (depending on the touch device), touch coordinates cannot
    /// be directly mapped to the screen. A touch always is considered to start at the position of
    /// the pointer, but touch movement is always measured in the units delivered by the device,
    /// and may depend on hardware and system settings.
    pub translation_delta: Vec2,

    /// Current force of the touch (average of the forces of the individual fingers). This is a
    /// value in the interval `[0.0 .. =1.0]`.
    ///
    /// Note 1: A value of `0.0` either indicates a very light touch, or it means that the device
    /// is not capable of measuring the touch force at all.
    ///
    /// Note 2: Just increasing the physical pressure without actually moving the finger may not
    /// necessarily lead to a change of this value.
    pub force: f32,
}

/// The current state (for a specific touch device) of touch events and gestures.
#[derive(Clone)]
pub(crate) struct TouchState {
    /// Technical identifier of the touch device. This is used to identify relevant touch events
    /// for this [`TouchState`] instance.
    device_id: TouchDeviceId,

    /// Active touches, if any.
    ///
    /// TouchId is the unique identifier of the touch. It is valid as long as the finger/pen touches the surface. The
    /// next touch will receive a new unique ID.
    ///
    /// Refer to [`ActiveTouch`].
    active_touches: BTreeMap<TouchId, ActiveTouch>,

    /// If a gesture has been recognized (i.e. when exactly two fingers touch the surface), this
    /// holds state information
    gesture_state: Option<GestureState>,
}

#[derive(Clone, Debug)]
struct GestureState {
    start_time: f64,
    start_pointer_pos: Pos2,
    pinch_type: PinchType,
    previous: Option<DynGestureState>,
    current: DynGestureState,
}

/// Gesture data that can change over time
#[derive(Clone, Copy, Debug)]
struct DynGestureState {
    /// used for proportional zooming
    avg_distance: f32,

    /// used for non-proportional zooming
    avg_abs_distance2: Vec2,

    avg_pos: Pos2,

    avg_force: f32,

    heading: f32,
}

/// Describes an individual touch (finger or digitizer) on the touch surface. Instances exist as
/// long as the finger/pen touches the surface.
#[derive(Clone, Copy, Debug)]
struct ActiveTouch {
    /// Current position of this touch, in device coordinates (not necessarily screen position)
    pos: Pos2,

    /// Current force of the touch. A value in the interval [0.0 .. 1.0]
    ///
    /// Note that a value of 0.0 either indicates a very light touch, or it means that the device
    /// is not capable of measuring the touch force.
    force: f32,
}

impl TouchState {
    pub fn new(device_id: TouchDeviceId) -> Self {
        Self {
            device_id,
            active_touches: Default::default(),
            gesture_state: None,
        }
    }

    pub fn begin_frame(&mut self, time: f64, new: &RawInput, pointer_pos: Option<Pos2>) {
        let mut added_or_removed_touches = false;
        for event in &new.events {
            match *event {
                Event::Touch {
                    device_id,
                    id,
                    phase,
                    pos,
                    force,
                } if device_id == self.device_id => match phase {
                    TouchPhase::Start => {
                        self.active_touches.insert(id, ActiveTouch { pos, force });
                        added_or_removed_touches = true;
                    }
                    TouchPhase::Move => {
                        if let Some(touch) = self.active_touches.get_mut(&id) {
                            touch.pos = pos;
                            touch.force = force;
                        }
                    }
                    TouchPhase::End | TouchPhase::Cancel => {
                        self.active_touches.remove(&id);
                        added_or_removed_touches = true;
                    }
                },
                _ => (),
            }
        }
        // This needs to be called each frame, even if there are no new touch events.
        // Otherwise, we would send the same old delta information multiple times:
        self.update_gesture(time, pointer_pos);

        if added_or_removed_touches {
            // Adding or removing fingers makes the average values "jump". We better forget
            // about the previous values, and don't create delta information for this frame:
            if let Some(ref mut state) = &mut self.gesture_state {
                state.previous = None;
            }
        }
    }

    pub fn is_active(&self) -> bool {
        self.gesture_state.is_some()
    }

    pub fn info(&self) -> Option<MultiTouchInfo> {
        self.gesture_state.as_ref().map(|state| {
            // state.previous can be `None` when the number of simultaneous touches has just
            // changed. In this case, we take `current` as `previous`, pretending that there
            // was no change for the current frame.
            let state_previous = state.previous.unwrap_or(state.current);

            let zoom_delta = state.current.avg_distance / state_previous.avg_distance;

            let zoom_delta2 = match state.pinch_type {
                PinchType::Horizontal => Vec2::new(
                    state.current.avg_abs_distance2.x / state_previous.avg_abs_distance2.x,
                    1.0,
                ),
                PinchType::Vertical => Vec2::new(
                    1.0,
                    state.current.avg_abs_distance2.y / state_previous.avg_abs_distance2.y,
                ),
                PinchType::Proportional => Vec2::splat(zoom_delta),
            };

            MultiTouchInfo {
                start_time: state.start_time,
                start_pos: state.start_pointer_pos,
                num_touches: self.active_touches.len(),
                zoom_delta,
                zoom_delta_2d: zoom_delta2,
                rotation_delta: normalized_angle(state.current.heading - state_previous.heading),
                translation_delta: state.current.avg_pos - state_previous.avg_pos,
                force: state.current.avg_force,
            }
        })
    }

    fn update_gesture(&mut self, time: f64, pointer_pos: Option<Pos2>) {
        if let Some(dyn_state) = self.calc_dynamic_state() {
            if let Some(ref mut state) = &mut self.gesture_state {
                // updating an ongoing gesture
                state.previous = Some(state.current);
                state.current = dyn_state;
            } else if let Some(pointer_pos) = pointer_pos {
                // starting a new gesture
                self.gesture_state = Some(GestureState {
                    start_time: time,
                    start_pointer_pos: pointer_pos,
                    pinch_type: PinchType::classify(&self.active_touches),
                    previous: None,
                    current: dyn_state,
                });
            }
        } else {
            // the end of a gesture (if there is any)
            self.gesture_state = None;
        }
    }

    /// `None` if less than two fingers
    fn calc_dynamic_state(&self) -> Option<DynGestureState> {
        let num_touches = self.active_touches.len();
        if num_touches < 2 {
            None
        } else {
            let mut state = DynGestureState {
                avg_distance: 0.0,
                avg_abs_distance2: Vec2::ZERO,
                avg_pos: Pos2::ZERO,
                avg_force: 0.0,
                heading: 0.0,
            };
            let num_touches_recip = 1. / num_touches as f32;

            // first pass: calculate force and center of touch positions:
            for touch in self.active_touches.values() {
                state.avg_force += touch.force;
                state.avg_pos.x += touch.pos.x;
                state.avg_pos.y += touch.pos.y;
            }
            state.avg_force *= num_touches_recip;
            state.avg_pos.x *= num_touches_recip;
            state.avg_pos.y *= num_touches_recip;

            // second pass: calculate distances from center:
            for touch in self.active_touches.values() {
                state.avg_distance += state.avg_pos.distance(touch.pos);
                state.avg_abs_distance2.x += (state.avg_pos.x - touch.pos.x).abs();
                state.avg_abs_distance2.y += (state.avg_pos.y - touch.pos.y).abs();
            }
            state.avg_distance *= num_touches_recip;
            state.avg_abs_distance2 *= num_touches_recip;

            // Calculate the direction from the first touch to the center position.
            // This is not the perfect way of calculating the direction if more than two fingers
            // are involved, but as long as all fingers rotate more or less at the same angular
            // velocity, the shortcomings of this method will not be noticed. One can see the
            // issues though, when touching with three or more fingers, and moving only one of them
            // (it takes two hands to do this in a controlled manner). A better technique would be
            // to store the current and previous directions (with reference to the center) for each
            // touch individually, and then calculate the average of all individual changes in
            // direction. But this approach cannot be implemented locally in this method, making
            // everything a bit more complicated.
            let first_touch = self.active_touches.values().next().unwrap();
            state.heading = (state.avg_pos - first_touch.pos).angle();

            Some(state)
        }
    }
}

impl TouchState {
    pub fn ui(&self, ui: &mut crate::Ui) {
        ui.label(format!("{:?}", self));
    }
}

impl Debug for TouchState {
    // This outputs less clutter than `#[derive(Debug)]`:
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (id, touch) in &self.active_touches {
            f.write_fmt(format_args!("#{:?}: {:#?}\n", id, touch))?;
        }
        f.write_fmt(format_args!("gesture: {:#?}\n", self.gesture_state))?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
enum PinchType {
    Horizontal,
    Vertical,
    Proportional,
}

impl PinchType {
    fn classify(touches: &BTreeMap<TouchId, ActiveTouch>) -> Self {
        // For non-proportional 2d zooming:
        // If the user is pinching with two fingers that have roughly the same Y coord,
        // then the Y zoom is unstable and should be 1.
        // Similarly, if the fingers are directly above/below each other,
        // we should only zoom on the Y axis.
        // If the fingers are roughly on a diagonal, we revert to the proportional zooming.

        if touches.len() == 2 {
            let mut touches = touches.values();
            let t0 = touches.next().unwrap().pos;
            let t1 = touches.next().unwrap().pos;

            let dx = (t0.x - t1.x).abs();
            let dy = (t0.y - t1.y).abs();

            if dx > 3.0 * dy {
                Self::Horizontal
            } else if dy > 3.0 * dx {
                Self::Vertical
            } else {
                Self::Proportional
            }
        } else {
            Self::Proportional
        }
    }
}
