mod gestures;

use crate::{data::input::TouchDeviceId, RawInput};
use gestures::Gesture;

/// The current state of touch events and gestures.  Uses a collection of `Gesture` implementations
/// which track their own state individually
#[derive(Debug)]
pub struct TouchState {
    device_id: TouchDeviceId,
    gestures: Vec<Box<dyn Gesture>>,
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
        }
    }
}

impl TouchState {
    pub fn new(device_id: TouchDeviceId) -> Self {
        Self {
            device_id,
            gestures: vec![Box::new(gestures::Zoom::default())],
        }
    }

    pub fn begin_frame(&mut self, time: f64, new: &RawInput) {}

    pub fn zoom(&self) -> Option<f32> {
        self.gestures
            .iter()
            .filter(|gesture| matches!(gesture.kind(), gestures::Kind::Zoom))
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
