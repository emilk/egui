mod gestures;

use crate::{data::input::TouchDeviceId, Event, RawInput, TouchId, TouchPhase};
use epaint::emath::Pos2;
use gestures::{Gesture, Phase};

/// struct members are the same as in enum variant `Event::Touch`
#[derive(Clone, Copy, Debug)]
pub struct Touch {
    pos: Pos2,
    force: f32,
}

/// The current state (for a specific touch device) of touch events and gestures.  Uses a
/// collection of `Gesture` implementations which track their own state individually
#[derive(Clone, Debug)]
pub struct TouchState {
    device_id: TouchDeviceId,
    registered_gestures: Vec<RegisteredGesture>,
    active_touches: gestures::TouchMap,
}

#[derive(Debug)]
struct RegisteredGesture {
    /// The current processing state.  If it is `Rejected`, the gesture will not be considered
    /// any more until all touches end and a new touch sequence starts
    phase: Phase,
    gesture: Box<dyn Gesture>,
}

impl Clone for RegisteredGesture {
    fn clone(&self) -> Self {
        RegisteredGesture {
            phase: self.phase,
            gesture: self.gesture.boxed_clone(),
        }
    }
}

impl TouchState {
    pub fn new(device_id: TouchDeviceId) -> Self {
        Self {
            device_id,
            registered_gestures: Default::default(),
            active_touches: Default::default(),
        }
    }

    fn reset_gestures(&mut self) {
        self.registered_gestures = vec![RegisteredGesture {
            phase: gestures::Phase::Waiting,
            gesture: Box::new(gestures::TwoFingerPinchOrZoom::default()),
        }];
    }

    pub fn begin_frame(&mut self, time: f64, new: &RawInput) {
        let my_device_id = self.device_id;

        if self.active_touches.is_empty() {
            // no touches so far -> make sure all gestures are `Waiting`:
            self.reset_gestures();
        }

        let mut notified_gestures = false;
        new.events
            .iter()
            //
            // filter for Touch events belonging to my device_id:
            .filter_map(|event| {
                if let Event::Touch {
                    device_id,
                    id,
                    phase,
                    pos,
                    force,
                } = event
                {
                    Some((
                        device_id,
                        id,
                        phase,
                        Touch {
                            pos: *pos,
                            force: *force,
                        },
                    ))
                } else {
                    None
                }
            })
            .filter(|(&device_id, ..)| device_id == my_device_id)
            //
            // process matching Touch events:
            .for_each(|(_, &touch_id, phase, touch)| {
                notified_gestures = true;
                match phase {
                    TouchPhase::Start => {
                        self.active_touches.insert(touch_id, touch);
                        let ctx = gestures::Context {
                            time,
                            active_touches: &self.active_touches,
                            touch_id,
                        };
                        for reg in &mut self.registered_gestures {
                            reg.phase = reg.gesture.touch_started(&ctx);
                        }
                    }
                    TouchPhase::Move => {
                        self.active_touches.insert(touch_id, touch);
                        let ctx = gestures::Context {
                            time,
                            active_touches: &self.active_touches,
                            touch_id,
                        };
                        for reg in &mut self.registered_gestures {
                            reg.phase = reg.gesture.touch_changed(&ctx);
                        }
                    }
                    TouchPhase::End => {
                        if let Some(removed_touch) = self.active_touches.remove(&touch_id) {
                            let ctx = gestures::Context {
                                time,
                                active_touches: &self.active_touches,
                                touch_id,
                            };
                            for reg in &mut self.registered_gestures {
                                reg.phase = reg.gesture.touch_ended(&ctx, removed_touch);
                            }
                        }
                    }
                    TouchPhase::Cancel => {
                        self.active_touches.remove(&touch_id);
                        for reg in &mut self.registered_gestures {
                            reg.phase = Phase::Rejected;
                        }
                    }
                }
                self.registered_gestures
                    .retain(|g| g.phase != Phase::Rejected);
            });

        if !notified_gestures && !self.active_touches.is_empty() {
            for reg in &mut self.registered_gestures {
                if reg.phase == Phase::Checking {
                    reg.phase = reg.gesture.check(time, &self.active_touches);
                }
            }
            self.registered_gestures
                .retain(|g| g.phase != Phase::Rejected);
        }
    }

    pub fn zoom(&self) -> Option<f32> {
        self.registered_gestures
            .iter()
            .filter(|reg| reg.gesture.kind() == gestures::Kind::Zoom)
            .find_map(|reg| {
                if let Some(gestures::Details::Zoom { factor }) = reg.gesture.details() {
                    Some(factor)
                } else {
                    None
                }
            })
    }
}

impl TouchState {
    pub fn ui(&self, ui: &mut crate::Ui) {
        for gesture in &self.registered_gestures {
            ui.label(format!("{:?}", gesture));
        }
    }
}
