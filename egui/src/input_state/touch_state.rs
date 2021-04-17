use std::{collections::BTreeMap, fmt::Debug};

use crate::{data::input::TouchDeviceId, Event, RawInput, TouchId, TouchPhase};
use epaint::emath::Pos2;

/// The current state (for a specific touch device) of touch events and gestures.
#[derive(Clone)]
pub struct TouchState {
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
    /// Time when the current gesture started.  Currently, a new gesture is considered started
    /// whenever a finger starts or stops touching the surface.
    gesture_start_time: Option<f64>,
}

/// Describes an individual touch (finger or digitizer) on the touch surface.  Instances exist as
/// long as the finger/pen touches the surface.
#[derive(Clone, Copy, Debug)]
pub struct ActiveTouch {
    /// Screen position where this touch started
    start_pos: Pos2,
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
            gesture_start_time: None,
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
    }
}

// private methods
impl TouchState {
    fn touch_start(&mut self, id: TouchId, pos: Pos2, force: f32, time: f64) {
        self.active_touches.insert(
            id,
            ActiveTouch {
                start_pos: pos,
                pos,
                force,
            },
        );
        // adding a touch counts as the start of a new gesture:
        self.start_gesture(time);
    }

    fn touch_move(&mut self, id: TouchId, pos: Pos2, force: f32) {
        if let Some(touch) = self.active_touches.get_mut(&id) {
            touch.pos = pos;
            touch.force = force;
        }
    }

    fn touch_end(&mut self, id: TouchId, time: f64) {
        self.active_touches.remove(&id);
        // lifting a touch counts as the end of the gesture:
        if self.active_touches.is_empty() {
            self.end_gesture();
        } else {
            self.start_gesture(time);
        }
    }

    fn start_gesture(&mut self, time: f64) {
        self.end_gesture();
        self.gesture_start_time = Some(time);
    }

    fn end_gesture(&mut self) {
        self.gesture_start_time = None;
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
        f.write_fmt(format_args!(
            "gesture_start_time: {:?}\n",
            self.gesture_start_time
        ))?;
        for (id, touch) in self.active_touches.iter() {
            f.write_fmt(format_args!("#{:?}: {:#?}\n", id, touch))?;
        }
        Ok(())
    }
}
