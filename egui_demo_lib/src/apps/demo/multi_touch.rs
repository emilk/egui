use egui::{
    emath::{RectTransform, Rot2},
    vec2, Color32, Frame, Pos2, Rect, Sense, Stroke, Vec2,
};

pub struct MultiTouch {
    previous_arrow_start_offset: Vec2,
    rotation: f32,
    smoothed_velocity: Vec2,
    translation: Vec2,
    zoom: f32,
}

impl Default for MultiTouch {
    fn default() -> Self {
        Self {
            previous_arrow_start_offset: Vec2::ZERO,
            rotation: 0.,
            smoothed_velocity: Vec2::ZERO,
            translation: Vec2::ZERO,
            zoom: 1.,
        }
    }
}

impl super::Demo for MultiTouch {
    fn name(&self) -> &'static str {
        "👌 Multi Touch"
    }

    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .default_size(vec2(512.0, 512.0))
            .resizable(true)
            .show(ctx, |ui| {
                use super::View;
                self.ui(ui);
            });
    }
}

impl super::View for MultiTouch {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::__egui_github_link_file!());
        });
        ui.colored_label(
            Color32::RED,
            "This only works on devices which send native touch events (mostly mobiles).",
        );
        ui.separator();
        ui.label("Try touch gestures Pinch/Stretch, Rotation, and Pressure with 2+ fingers.");
        Frame::dark_canvas(ui.style()).show(ui, |ui| {
            // Note that we use `Sense::drag()` although we do not use any pointer events. With
            // the current implementation, the fact that a touch event of two or more fingers is
            // recognized, does not mean that the pointer events are suppressed, which are always
            // generated for the first finger. Therefore, if we do not explicitly consume pointer
            // events, the window will move around, not only when dragged with a single finger, but
            // also when a two-finger touch is active. I guess this problem can only be cleanly
            // solved when the synthetic pointer events are created by egui, and not by the
            // backend.

            // set up the drawing canvas with normalized coordinates:
            let (response, painter) =
                ui.allocate_painter(ui.available_size_before_wrap_finite(), Sense::drag());
            // normalize painter coordinates to ±1 units in each direction with [0,0] in the center:
            let painter_proportions = response.rect.square_proportions();
            let to_screen = RectTransform::from_to(
                Rect::from_min_size(Pos2::ZERO - painter_proportions, 2. * painter_proportions),
                response.rect,
            );
            let dt = ui.input().unstable_dt;

            // check for touch input (or the lack thereof) and update zoom and scale factors, plus
            // color and width:
            let mut stroke_width = 1.;
            let mut color = Color32::GRAY;
            if let Some(multi_touch) = ui.input().multi_touch() {
                // This adjusts the current zoom factor and rotation angle according to the dynamic
                // change (for the current frame) of the touch gesture:
                self.zoom *= multi_touch.zoom_delta;
                self.rotation += multi_touch.rotation_delta;
                // the translation we get from `multi_touch` needs to be scaled down to the
                // normalized coordinates we use as the basis for painting:
                self.translation += to_screen.inverse().scale() * multi_touch.translation_delta;
                // touch pressure shall make the arrow thicker (not all touch devices support this):
                stroke_width += 10. * multi_touch.force;
                // the drawing color depends on the number of touches:
                color = match multi_touch.num_touches {
                    2 => Color32::GREEN,
                    3 => Color32::BLUE,
                    4 => Color32::YELLOW,
                    _ => Color32::RED,
                };
            } else {
                // This has nothing to do with the touch gesture. It just smoothly brings the
                // painted arrow back into its original position, for a nice visual effect:
                const ZOOM_ROTATE_HALF_LIFE: f32 = 1.; // time[sec] after which half the amount of zoom/rotation will be reverted
                let half_life_factor = (-(2_f32.ln()) / ZOOM_ROTATE_HALF_LIFE * dt).exp();
                self.zoom = 1. + ((self.zoom - 1.) * half_life_factor);
                self.rotation *= half_life_factor;
                self.translation *= half_life_factor;
            }
            let zoom_and_rotate = self.zoom * Rot2::from_angle(self.rotation);
            let arrow_start_offset = self.translation + zoom_and_rotate * vec2(-0.5, 0.5);
            let current_velocity = (arrow_start_offset - self.previous_arrow_start_offset) / dt;
            self.previous_arrow_start_offset = arrow_start_offset;

            // aggregate the average velocity of the arrow's start position from latest samples:
            const NUM_SMOOTHING_SAMPLES: f32 = 10.;
            self.smoothed_velocity = ((NUM_SMOOTHING_SAMPLES - 1.) * self.smoothed_velocity
                + current_velocity)
                / NUM_SMOOTHING_SAMPLES;

            // Paints an arrow pointing from bottom-left (-0.5, 0.5) to top-right (0.5, -0.5), but
            // scaled, rotated, and translated according to the current touch gesture:
            let arrow_start = Pos2::ZERO + arrow_start_offset;
            let arrow_direction = zoom_and_rotate * vec2(1., -1.);
            painter.arrow(
                to_screen * arrow_start,
                to_screen.scale() * arrow_direction,
                Stroke::new(stroke_width, color),
            );
            // Paints a circle at the origin of the arrow. The size and opacity of the circle
            // depend on the current velocity, and the circle is translated in the opposite
            // direction of the movement, so it follows the origin's movement. Constant factors
            // have been determined by trial and error.
            let speed = self.smoothed_velocity.length();
            painter.circle_filled(
                to_screen * (arrow_start - 0.2 * self.smoothed_velocity),
                2. + to_screen.scale().length() * 0.1 * speed,
                Color32::RED.linear_multiply(1. / (1. + (5. * speed).powi(2))),
            );

            // we want continuous UI updates, so the circle can smoothly follow the arrow's origin:
            ui.ctx().request_repaint();
        });
    }
}
