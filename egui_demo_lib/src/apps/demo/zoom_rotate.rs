use egui::{
    emath::{RectTransform, Rot2},
    vec2, Color32, Frame, Pos2, Rect, Sense, Stroke,
};

#[derive(Default)]
pub struct ZoomRotate {}

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
    }
}

impl super::View for ZoomRotate {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::__egui_github_link_file!());
        });
        ui.colored_label(Color32::RED, "This only works on mobile devices or with other touch devices supported by the backend.");
        ui.separator();
        ui.label("Pinch, Zoom, or Rotate the arrow with two fingers.");
        Frame::dark_canvas(ui.style()).show(ui, |ui| {
            let (response, painter) =
                ui.allocate_painter(ui.available_size_before_wrap_finite(), Sense::hover());
            let painter_proportions = response.rect.square_proportions();
            // scale painter to ±1 units in each direction with [0,0] in the center:
            let to_screen = RectTransform::from_to(
                Rect::from_min_size(Pos2::ZERO - painter_proportions, 2. * painter_proportions),
                response.rect,
            );
            let stroke = Stroke::new(1.0, Color32::YELLOW);

            let (zoom_factor, rotation);
            if let Some(touches) = ui.input().touches() {
                zoom_factor = touches.total.zoom;
                rotation = touches.total.rotation;
            } else {
                zoom_factor = 1.;
                rotation = 0.;
            }
            let scaled_rotation = zoom_factor * Rot2::from_angle(rotation);

            painter.arrow(
                to_screen * (Pos2::ZERO + scaled_rotation * vec2(-0.5, 0.5)),
                to_screen.scale() * (scaled_rotation * vec2(1., -1.)),
                stroke,
            );
        });
    }
}
