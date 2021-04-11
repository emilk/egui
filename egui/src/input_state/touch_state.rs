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
            .map(|gesture| gesture.boxed_clone())
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

    fn gestures(&self) -> &Vec<Box<dyn Gesture>> {
        &self.gestures
    }
}

impl TouchState {
    pub fn ui(&self, ui: &mut crate::Ui) {
        for gesture in self.gestures() {
            ui.label(format!("{:?}", gesture));
        }
    }
}
