use super::{Details, Gesture, Kind, State};

#[derive(Clone, Debug, Default)]
pub struct Zoom {
    state: State,
}

impl Gesture for Zoom {
    fn boxed_clone(&self) -> Box<dyn Gesture> {
        Box::new(self.clone())
    }

    fn state(&self) -> State {
        self.state
    }

    fn kind(&self) -> Kind {
        Kind::Zoom
    }

    fn details(&self) -> Option<Details> {
        None
    }

    fn start_position(&self) -> Option<epaint::emath::Pos2> {
        todo!()
    }
}
