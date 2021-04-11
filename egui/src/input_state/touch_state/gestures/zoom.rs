use super::{Details, Gesture, Kind, State, Touch, TouchId, TouchMap};

#[derive(Clone, Debug, Default)]
pub struct Zoom {
    state: State,
    previous_distance: Option<f32>,
    current_distance: Option<f32>,
}

impl Gesture for Zoom {
    fn boxed_clone(&self) -> Box<dyn Gesture> {
        Box::new(self.clone())
    }

    fn kind(&self) -> Kind {
        Kind::Zoom
    }

    fn state(&self) -> State {
        self.state
    }

    fn details(&self) -> Option<Details> {
        if let (Some(previous_distance), Some(current_distance)) =
            (self.previous_distance, self.current_distance)
        {
            Some(Details::Zoom {
                factor: current_distance / previous_distance,
            })
        } else {
            None
        }
    }

    fn start_position(&self) -> Option<epaint::emath::Pos2> {
        None
    }

    fn check(&mut self, _time: f64, _active_touches: &TouchMap) {}

    fn touch_started(&mut self, _touch_id: TouchId, _time: f64, active_touches: &TouchMap) {
        if active_touches.len() >= 2 {
            self.state = State::Active;
            self.update_details();
        } else {
            self.state = State::Checking;
        }
    }

    fn touch_changed(&mut self, _touch_id: TouchId, _time: f64, active_touches: &TouchMap) {
        if active_touches.len() >= 2 {
            self.state = State::Active;
            self.update_details();
        } else {
            self.state = State::Checking;
        }
    }

    fn touch_ended(&mut self, _touch: Touch, _time: f64, active_touches: &TouchMap) {
        if active_touches.len() < 2 {
            self.state = State::Rejected;
        } else {
            self.update_details();
        }
    }

    fn touch_cancelled(&mut self, _touch: Touch, _time: f64, _active_touches: &TouchMap) {
        self.state = State::Rejected;
    }
}

impl Zoom {
    fn update_details(&mut self) {
        // TODO
        // TODO
        // TODO
        // TODO
        // TODO
        self.previous_distance = Some(20.);
        self.current_distance = Some(25.);
    }
}
