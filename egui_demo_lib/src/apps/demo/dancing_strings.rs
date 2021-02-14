use egui::{containers::*, *};

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct DancingStrings {}

impl Default for DancingStrings {
    fn default() -> Self {
        Self {}
    }
}

impl super::Demo for DancingStrings {
    fn name(&self) -> &str {
        "♫ Dancing Strings"
    }

    fn show(&mut self, ctx: &CtxRef, open: &mut bool) {
        use super::View;
        Window::new(self.name())
            .open(open)
            .default_size(vec2(512.0, 256.0))
            .scroll(false)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for DancingStrings {
    fn ui(&mut self, ui: &mut Ui) {
        Frame::dark_canvas(ui.style()).show(ui, |ui| {
            ui.ctx().request_repaint();
            let time = ui.input().time;

            let desired_size = ui.available_width() * vec2(1.0, 0.35);
            let (_id, rect) = ui.allocate_space(desired_size);

            let to_screen =
                emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0), rect);

            let mut shapes = vec![];

            for &mode in &[2, 3, 5] {
                let mode = mode as f32;
                let n = 120;
                let speed = 1.5;

                let points: Vec<Pos2> = (0..=n)
                    .map(|i| {
                        let t = i as f32 / (n as f32);
                        let amp = (time as f32 * speed * mode).sin() / mode;
                        let y = amp * (t * std::f32::consts::TAU / 2.0 * mode).sin();
                        to_screen * pos2(t, y)
                    })
                    .collect();

                let thickness = 10.0 / mode;
                shapes.push(epaint::Shape::line(
                    points,
                    Stroke::new(thickness, Color32::from_additive_luminance(196)),
                ));
            }

            ui.painter().extend(shapes);
        });
        ui.vertical_centered(|ui| {
            ui.add(crate::__egui_github_link_file!());
        });
    }
}
