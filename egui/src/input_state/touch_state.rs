use std::{collections::BTreeMap, fmt::Debug};

use crate::{data::input::TouchDeviceId, Event, RawInput, TouchId, TouchPhase};
use epaint::emath::{pos2, Pos2, Vec2};

/// All you probably need to know about the current two-finger touch gesture.
pub struct TouchInfo {
    /// Point in time when the gesture started
    pub start_time: f64,
    /// Position where the gesture started (average of all individual touch positions)
    pub start_pos: Pos2,
    /// Current position of the gesture (average of all individual touch positions)
    pub current_pos: Pos2,
    /// Dynamic information about the touch gesture, relative to the start of the gesture.
    /// Refer to [`GestureInfo`].
    pub total: DynamicTouchInfo,
    /// Dynamic information about the touch gesture, relative to the previous frame.
    /// Refer to [`GestureInfo`].
    pub incremental: DynamicTouchInfo,
}

/// Information about the dynamic state of a gesture.  Note that there is no internal threshold
/// which needs to be reached before this information is updated.  If you want a threshold, you
/// have to manage this in your application code.
pub struct DynamicTouchInfo {
    /// Zoom factor (Pinch or Zoom).  Moving fingers closer together or further appart will change
    /// this value.
    pub zoom: f32,
    /// Rotation in radians.  Rotating the fingers, but also moving just one of them will change
    /// this value.
    pub rotation: f32,
    /// Movement (in points) of the average position of all touch points.
    pub translation: Vec2,
    /// Force of the touch (average of the forces of the individual fingers). This is a
    /// value in the interval `[0.0 .. =1.0]`.
    ///
    /// Note 1: A value of 0.0 either indicates a very light touch, or it means that the device
    /// is not capable of measuring the touch force at all.
    ///
    /// Note 2: Just increasing the physical pressure without actually moving the finger may not
    /// lead to a change of this value.
    pub force: f32,
}

/// The current state (for a specific touch device) of touch events and gestures.
#[derive(Clone)]
pub(crate) struct TouchState {
    /// Technical identifier of the touch device.  This is used to identify relevant touch events
    /// for this `TouchState` instance.
    device_id: TouchDeviceId,
    /// Active touches, if any.
    ///
    /// TouchId is the unique identifier of the touch.  It is valid as long as the finger/pen touches the surface.  The
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
    start: DynGestureState,
    previous: DynGestureState,
    current: DynGestureState,
}

/// Gesture data which can change over time
#[derive(Clone, Copy, Debug)]
struct DynGestureState {
    pos: Pos2,
    distance: f32,
    direction: f32,
    force: f32,
}

/// Describes an individual touch (finger or digitizer) on the touch surface.  Instances exist as
/// long as the finger/pen touches the surface.
#[derive(Clone, Copy, Debug)]
struct ActiveTouch {
    /// Screen position where this touch was when the gesture startet
    gesture_start_pos: Pos2,
    /// Current screen position of this touch
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

    pub fn begin_frame(&mut self, time: f64, new: &RawInput) {
        for event in &new.events {
            match *event {
                Event::Touch {
                    device_id,
                    id,
                    phase,
                    pos,
                    force,
                } if device_id == self.device_id => match phase {
                    TouchPhase::Start => self.touch_start(id, pos, force, time),
                    TouchPhase::Move => self.touch_move(id, pos, force),
                    TouchPhase::End | TouchPhase::Cancel => self.touch_end(id, time),
                },
                _ => (),
            }
        }
        // This needs to be called each frame, even if there are no new touch events.
        // Failing to do so may result in wrong information in `TouchInfo.incremental`
        self.update_gesture();
    }

    pub fn is_active(&self) -> bool {
        self.gesture_state.is_some()
    }

    pub fn info(&self) -> Option<TouchInfo> {
        self.gesture_state.as_ref().map(|state| TouchInfo {
            start_time: state.start_time,
            start_pos: state.start.pos,
            current_pos: state.current.pos,
            total: DynamicTouchInfo {
                zoom: state.current.distance / state.start.distance,
                rotation: state.current.direction - state.start.direction,
                translation: state.current.pos - state.start.pos,
                force: state.current.force,
            },
            incremental: DynamicTouchInfo {
                zoom: state.current.distance / state.previous.distance,
                rotation: state.current.direction - state.previous.direction,
                translation: state.current.pos - state.previous.pos,
                force: state.current.force - state.previous.force,
            },
        })
    }
}

// private methods
impl TouchState {
    fn touch_start(&mut self, id: TouchId, pos: Pos2, force: f32, time: f64) {
        self.active_touches.insert(
            id,
            ActiveTouch {
                gesture_start_pos: pos,
                pos,
                force,
            },
        );
        // for now we only support exactly two fingers:
        if self.active_touches.len() == 2 {
            self.start_gesture(time);
        } else {
            self.end_gesture()
        }
    }

    fn touch_move(&mut self, id: TouchId, pos: Pos2, force: f32) {
        if let Some(touch) = self.active_touches.get_mut(&id)
        // always true
        {
            touch.pos = pos;
            touch.force = force;
        }
    }

    fn touch_end(&mut self, id: TouchId, time: f64) {
        self.active_touches.remove(&id);
        // for now we only support exactly two fingers:
        if self.active_touches.len() == 2 {
            self.start_gesture(time);
        } else {
            self.end_gesture()
        }
    }

    fn start_gesture(&mut self, time: f64) {
        for mut touch in self.active_touches.values_mut() {
            touch.gesture_start_pos = touch.pos;
        }
        if let Some((touch1, touch2)) = self.both_touches()
        // always true
        {
            let start_dyn_state = DynGestureState {
                pos: self::center_pos(touch1.pos, touch2.pos),
                distance: self::distance(touch1.pos, touch2.pos),
                direction: self::direction(touch1.pos, touch2.pos),
                force: (touch1.force + touch2.force) * 0.5,
            };
            self.gesture_state = Some(GestureState {
                start_time: time,
                start: start_dyn_state,
                previous: start_dyn_state,
                current: start_dyn_state,
            });
        }
    }

    fn update_gesture(&mut self) {
        if let Some((touch1, touch2)) = self.both_touches() {
            let state_new = DynGestureState {
                pos: self::center_pos(touch1.pos, touch2.pos),
                distance: self::distance(touch1.pos, touch2.pos),
                direction: self::direction(touch1.pos, touch2.pos),
                force: (touch1.force + touch2.force) * 0.5,
            };
            if let Some(ref mut state) = &mut self.gesture_state
            // always true
            {
                state.previous = state.current;
                state.current = state_new;
            }
        }
    }

    fn end_gesture(&mut self) {
        self.gesture_state = None;
    }

    fn both_touches(&self) -> Option<(&ActiveTouch, &ActiveTouch)> {
        if self.active_touches.len() == 2 {
            let mut touches = self.active_touches.values();
            let touch1 = touches.next().unwrap();
            let touch2 = touches.next().unwrap();
            Some((touch1, touch2))
        } else {
            None
        }
    }
}

impl TouchState {
    pub fn ui(&self, ui: &mut crate::Ui) {
        ui.label(format!("{:?}", self));
    }
}

impl Debug for TouchState {
    // We could just use `#[derive(Debug)]`, but the implementation below produces a less cluttered
    // output:
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (id, touch) in self.active_touches.iter() {
            f.write_fmt(format_args!("#{:?}: {:#?}\n", id, touch))?;
        }
        f.write_fmt(format_args!("gesture: {:#?}\n", self.gesture_state))?;
        Ok(())
    }
}

fn center_pos(pos_1: Pos2, pos_2: Pos2) -> Pos2 {
    pos2((pos_1.x + pos_2.x) * 0.5, (pos_1.y + pos_2.y) * 0.5)
}

fn distance(pos_1: Pos2, pos_2: Pos2) -> f32 {
    (pos_2 - pos_1).length()
}

fn direction(pos_1: Pos2, pos_2: Pos2) -> f32 {
    let v = (pos_2 - pos_1).normalized();
    v.y.atan2(v.x)
}
