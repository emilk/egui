use super::{Gesture, Status};

#[derive(Clone, Debug, Default)]
pub struct Zoom {
    state: Status,
}

impl Gesture for Zoom {
    fn boxed_clone(&self) -> Box<dyn Gesture> {
        Box::new(self.clone())
    }

    fn state(&self) -> Status {
        self.state
    }
}
