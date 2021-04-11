mod gestures;

use std::collections::BTreeMap;

use crate::{data::input::TouchDeviceId, Event, RawInput, TouchId, TouchPhase};
use epaint::emath::Pos2;
use gestures::Gesture;

/// struct members are the same as in enum variant `Event::Touch`
#[derive(Clone, Copy, Debug)]
pub struct Touch {
    pos: Pos2,
    force: f32,
}

pub type TouchMap = BTreeMap<TouchId, Touch>;

/// The current state of touch events and gestures.  Uses a collection of `Gesture` implementations
/// which track their own state individually
#[derive(Debug)]
pub struct TouchState {
    device_id: TouchDeviceId,
    gestures: Vec<Box<dyn Gesture>>,
    active_touches: TouchMap,
}

impl Clone for TouchState {
    fn clone(&self) -> Self {
        TouchState {
            device_id: self.device_id,
            gestures: self
                .gestures
                .iter()
                .map(|gesture| {
                    // Cloning this way does not feel right.
                    // Why do we have to implement `Clone` in the first place? – That's because
                    // CtxRef::begin_frame() clones self.0.
                    gesture.boxed_clone()
                })
                .collect(),
            active_touches: self.active_touches.clone(),
        }
    }
}

impl TouchState {
    pub fn new(device_id: TouchDeviceId) -> Self {
        let mut result = Self {
            device_id,
            gestures: Vec::new(),
            active_touches: Default::default(),
        };
        result.reset_gestures();
        result
    }

    fn reset_gestures(&mut self) {
        self.gestures = vec![Box::new(gestures::Zoom::default())];
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
            .for_each(|(_, touch_id, phase, touch)| {
                notified_gestures = true;
                match phase {
                    TouchPhase::Start => {
                        self.active_touches.insert(*touch_id, touch);
                        for gesture in &mut self.gestures {
                            gesture.touch_started(*touch_id, time, &self.active_touches);
                        }
                    }
                    TouchPhase::Move => {
                        self.active_touches.insert(*touch_id, touch);
                        for gesture in &mut self.gestures {
                            gesture.touch_changed(*touch_id, time, &self.active_touches);
                        }
                    }
                    TouchPhase::End => {
                        if let Some(removed_touch) = self.active_touches.remove(touch_id) {
                            for gesture in &mut self.gestures {
                                gesture.touch_ended(removed_touch, time, &self.active_touches);
                            }
                        }
                    }
                    TouchPhase::Cancel => {
                        if let Some(removed_touch) = self.active_touches.remove(touch_id) {
                            for gesture in &mut self.gestures {
                                gesture.touch_cancelled(removed_touch, time, &self.active_touches);
                            }
                        }
                    }
                }
                self.remove_rejected_gestures();
            });

        if !notified_gestures && !self.active_touches.is_empty() {
            for gesture in &mut self.gestures {
                gesture.check(time, &self.active_touches);
            }
            self.remove_rejected_gestures();
        }
    }

    fn remove_rejected_gestures(&mut self) {
        for i in (0..self.gestures.len()).rev() {
            if self.gestures.get(i).unwrap().state() == gestures::State::Rejected {
                self.gestures.remove(i);
            }
        }
    }

    pub fn zoom(&self) -> Option<f32> {
        self.gestures
            .iter()
            .filter(|gesture| gesture.kind() == gestures::Kind::Zoom)
            .find_map(|gesture| {
                if let Some(gestures::Details::Zoom { factor }) = gesture.details() {
                    Some(factor)
                } else {
                    None
                }
            })
    }
}

impl TouchState {
    pub fn ui(&self, ui: &mut crate::Ui) {
        for gesture in &self.gestures {
            ui.label(format!("{:?}", gesture));
        }
    }
}
