mod gestures;

use crate::RawInput;
use gestures::Gesture;

/// The current state of touch events and gestures.  Uses a collection of `Gesture` implementations
/// which track their own state individually
#[derive(Debug)]
pub struct TouchState {
    gestures: Vec<Box<dyn Gesture>>,
}

impl Clone for TouchState {
    fn clone(&self) -> Self {
        let gestures = self
            .gestures
            .iter()
            .map(|gesture| {
                // Cloning this way does not feel right.
                // Why do we have to implement `Clone` in the first place? – That's because
                // CtxRef::begin_frame() clones self.0.
                gesture.boxed_clone()
            })
            .collect();
        TouchState { gestures }
    }
}

impl Default for TouchState {
    fn default() -> Self {
        Self {
            gestures: vec![Box::new(gestures::Zoom::default())],
        }
    }
}

impl TouchState {
    #[must_use]
    pub fn begin_frame(self, time: f64, new: &RawInput) -> Self {
        self
    }

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
