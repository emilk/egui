use super::{Context, Details, Gesture, Kind, Phase};

#[derive(Clone, Debug, Default)]
pub struct TwoFingerPinchOrZoom {
    previous_distance: Option<f32>,
    current_distance: Option<f32>,
}

impl Gesture for TwoFingerPinchOrZoom {
    fn boxed_clone(&self) -> Box<dyn Gesture> {
        Box::new(self.clone())
    }

    fn kind(&self) -> Kind {
        Kind::Zoom
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

    fn touch_started(&mut self, ctx: &Context<'_>) -> Phase {
        match ctx.active_touches.len() {
            1 => Phase::Checking,
            2 => {
                self.update_details();
                Phase::Checking
            }
            _ => Phase::Rejected,
        }
    }

    fn touch_changed(&mut self, ctx: &Context<'_>) -> Phase {
        match ctx.active_touches.len() {
            1 => Phase::Checking,
            2 => {
                self.update_details();
                Phase::Active
            }
            _ => Phase::Rejected,
        }
    }
}

impl TwoFingerPinchOrZoom {
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
