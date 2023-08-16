use egui::{
    emath::{RectTransform, Rot2},
    vec2, Color32, Frame, Pos2, Rect, Sense, Stroke, Vec2,
};

pub struct MultiTouch {
    rotation: f32,
    translation: Vec2,
    zoom: f32,
    last_touch_time: f64,
}

impl Default for MultiTouch {
    fn default() -> Self {
        Self {
            rotation: 0.,
            translation: Vec2::ZERO,
            zoom: 1.,
            last_touch_time: 0.0,
        }
    }
}

impl super::Demo for MultiTouch {
    fn name(&self) -> &'static str {
        "👌 Multi Touch"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .default_size(vec2(512.0, 512.0))
            .resizable(true)
            .show(ctx, |ui| {
                use super::View as _;
                self.ui(ui);
            });
    }
}

impl super::View for MultiTouch {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
        ui.strong(
            "This demo only works on devices with multitouch support (e.g. mobiles and tablets).",
        );
        ui.separator();
        ui.label("Try touch gestures Pinch/Stretch, Rotation, and Pressure with 2+ fingers.");

        let num_touches = ui.input(|i| i.multi_touch().map_or(0, |mt| mt.num_touches));
        ui.label(format!("Current touches: {num_touches}"));

        let color = if ui.visuals().dark_mode {
            Color32::WHITE
        } else {
            Color32::BLACK
        };

        Frame::canvas(ui.style()).show(ui, |ui| {
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
                ui.allocate_painter(ui.available_size_before_wrap(), Sense::drag());

            // normalize painter coordinates to ±1 units in each direction with [0,0] in the center:
            let painter_proportions = response.rect.square_proportions();
            let to_screen = RectTransform::from_to(
                Rect::from_min_size(Pos2::ZERO - painter_proportions, 2. * painter_proportions),
                response.rect,
            );

            // check for touch input (or the lack thereof) and update zoom and scale factors, plus
            // color and width:
            let mut stroke_width = 1.;
            if let Some(multi_touch) = ui.ctx().multi_touch() {
                // This adjusts the current zoom factor and rotation angle according to the dynamic
                // change (for the current frame) of the touch gesture:
                self.zoom *= multi_touch.zoom_delta;
                self.rotation += multi_touch.rotation_delta;
                // the translation we get from `multi_touch` needs to be scaled down to the
                // normalized coordinates we use as the basis for painting:
                self.translation += to_screen.inverse().scale() * multi_touch.translation_delta;
                // touch pressure will make the arrow thicker (not all touch devices support this):
                stroke_width += 10. * multi_touch.force;

                self.last_touch_time = ui.input(|i| i.time);
            } else {
                self.slowly_reset(ui);
            }
            let zoom_and_rotate = self.zoom * Rot2::from_angle(self.rotation);
            let arrow_start_offset = self.translation + zoom_and_rotate * vec2(-0.5, 0.5);

            // Paints an arrow pointing from bottom-left (-0.5, 0.5) to top-right (0.5, -0.5), but
            // scaled, rotated, and translated according to the current touch gesture:
            let arrow_start = Pos2::ZERO + arrow_start_offset;
            let arrow_direction = zoom_and_rotate * vec2(1., -1.);
            painter.arrow(
                to_screen * arrow_start,
                to_screen.scale() * arrow_direction,
                Stroke::new(stroke_width, color),
            );
        });
    }
}

impl MultiTouch {
    fn slowly_reset(&mut self, ui: &egui::Ui) {
        // This has nothing to do with the touch gesture. It just smoothly brings the
        // painted arrow back into its original position, for a nice visual effect:

        let time_since_last_touch = (ui.input(|i| i.time) - self.last_touch_time) as f32;

        let delay = 0.5;
        if time_since_last_touch < delay {
            ui.ctx().request_repaint();
        } else {
            // seconds after which half the amount of zoom/rotation will be reverted:
            let half_life =
                egui::remap_clamp(time_since_last_touch, delay..=1.0, 1.0..=0.0).powf(4.0);

            if half_life <= 1e-3 {
                self.zoom = 1.0;
                self.rotation = 0.0;
                self.translation = Vec2::ZERO;
            } else {
                let dt = ui.input(|i| i.unstable_dt);
                let half_life_factor = (-(2_f32.ln()) / half_life * dt).exp();
                self.zoom = 1. + ((self.zoom - 1.) * half_life_factor);
                self.rotation *= half_life_factor;
                self.translation *= half_life_factor;
                ui.ctx().request_repaint();
            }
        }
    }
}
