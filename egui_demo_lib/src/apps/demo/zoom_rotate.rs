use egui::{
    emath::{RectTransform, Rot2},
    vec2, Color32, Frame, Pos2, Rect, Sense, Stroke,
};

pub struct ZoomRotate {
    last_time: Option<f64>,
    rotation: f32,
    zoom: f32,
}

impl Default for ZoomRotate {
    fn default() -> Self {
        Self {
            last_time: None,
            rotation: 0.,
            zoom: 1.,
        }
    }
}

impl super::Demo for ZoomRotate {
    fn name(&self) -> &'static str {
        "👌 Zoom/Rotate"
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
        self.last_time = Some(ctx.input().time);
    }
}

impl super::View for ZoomRotate {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::__egui_github_link_file!());
        });
        ui.colored_label(
            Color32::RED,
            "This only works on supported touch devices, like mobiles.",
        );
        ui.separator();
        ui.label("Pinch, Zoom, or Rotate the arrow with two fingers.");
        Frame::dark_canvas(ui.style()).show(ui, |ui| {
            // Note that we use `Sense::drag()` although we do not use any pointer events.  With
            // the current implementation, the fact that a touch event of two or more fingers is
            // recognized, does not mean that the pointer events are suppressed, which are always
            // generated for the first finger.  Therefore, if we do not explicitly consume pointer
            // events, the window will move around, not only when dragged with a single finger, but
            // also when a two-finger touch is active.  I guess this problem can only be cleanly
            // solved when the synthetic pointer events are created by egui, and not by the
            // backend.
            let (response, painter) =
                ui.allocate_painter(ui.available_size_before_wrap_finite(), Sense::drag());
            // normalize painter coordinates to ±1 units in each direction with [0,0] in the center:
            let painter_proportions = response.rect.square_proportions();
            let to_screen = RectTransform::from_to(
                Rect::from_min_size(Pos2::ZERO - painter_proportions, 2. * painter_proportions),
                response.rect,
            );

            if let Some(touches) = ui.input().touches() {
                // This adjusts the current zoom factor and rotation angle according to the dynamic
                // change (for the current frame) of the touch gesture:
                self.zoom *= touches.incremental.zoom;
                self.rotation += touches.incremental.rotation;
                // for a smooth touch experience (shouldn't this be done by egui automatically?):
                ui.ctx().request_repaint();
            } else if let Some(last_time) = self.last_time {
                // This has nothing to do with the touch gesture. It just smoothly brings the
                // painted arrow back into its original position, for a better visual effect:
                let dt = ui.input().time - last_time;
                const ZOOM_ROTATE_HALF_LIFE: f64 = 1.; // time[sec] after which half the amount of zoom/rotation will be reverted
                let half_life_factor = (-(2_f64.ln()) / ZOOM_ROTATE_HALF_LIFE * dt).exp() as f32;
                self.zoom = 1. + ((self.zoom - 1.) * half_life_factor);
                self.rotation *= half_life_factor;
                // this is an animation, so we want real-time UI updates:
                ui.ctx().request_repaint();
            }
            let zoom_and_rotate = self.zoom * Rot2::from_angle(self.rotation);

            // Paints an arrow pointing from bottom-left (-0.5, 0.5) to top-right (0.5, -0.5),
            // but scaled and rotated according to the current translation:
            let arrow_start = zoom_and_rotate * vec2(-0.5, 0.5);
            let arrow_direction = zoom_and_rotate * vec2(1., -1.);
            painter.arrow(
                to_screen * (Pos2::ZERO + arrow_start),
                to_screen.scale() * arrow_direction,
                Stroke::new(1.0, Color32::YELLOW),
            );
        });
    }
}
