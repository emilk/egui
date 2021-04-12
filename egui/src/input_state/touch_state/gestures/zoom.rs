use super::{Context, Details, Gesture, Kind, Phase};
use epaint::emath::Pos2;

#[derive(Clone, Debug, Default)]
pub struct TwoFingerPinchOrZoom {
    previous_distance: Option<f32>,
    current_distance: Option<f32>,
    start_position: Option<Pos2>,
}

impl Gesture for TwoFingerPinchOrZoom {
    fn boxed_clone(&self) -> Box<dyn Gesture> {
        Box::new(self.clone())
    }

    fn kind(&self) -> Kind {
        Kind::TwoFingerPinchOrZoom
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

    fn start_position(&self) -> Option<Pos2> {
        self.start_position
    }

    fn touch_started(&mut self, ctx: &Context<'_>) -> Phase {
        match ctx.active_touches.len() {
            1 => Phase::Checking,
            2 => {
                self.update(ctx);
                Phase::Checking // received the second touch, now awaiting first movement
            }
            _ => Phase::Rejected,
        }
    }

    fn touch_changed(&mut self, ctx: &Context<'_>) -> Phase {
        match ctx.active_touches.len() {
            1 => Phase::Checking,
            2 => {
                self.update(ctx);
                Phase::Active
            }
            _ => Phase::Rejected,
        }
    }
}

impl TwoFingerPinchOrZoom {
    /// Updates current and previous distance of touch points.
    ///
    /// # Panics
    ///
    /// Panics if `ctx.active_touches` does not contain two touches.
    fn update(&mut self, ctx: &Context<'_>) {
        let first_activation = self.previous_distance.is_none();
        self.previous_distance = self.current_distance;

        let mut touch_points = ctx.active_touches.values();
        let v1 = touch_points.next().unwrap().pos.to_vec2();
        let v2 = touch_points.next().unwrap().pos.to_vec2();

        self.current_distance = Some((v1 - v2).length());
        if first_activation {
            let v_mid = (v1 + v2) * 0.5;
            self.start_position = Some(Pos2::new(v_mid.x, v_mid.y));
        }
    }
}
