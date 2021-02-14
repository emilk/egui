use egui::*;

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct Painting {
    /// in 0-1 normalized coordinates
    lines: Vec<Vec<Pos2>>,
    stroke: Stroke,
}

impl Default for Painting {
    fn default() -> Self {
        Self {
            lines: Default::default(),
            stroke: Stroke::new(1.0, Color32::LIGHT_BLUE),
        }
    }
}

impl Painting {
    pub fn ui_control(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            egui::stroke_ui(ui, &mut self.stroke, "Stroke");
            ui.separator();
            if ui.button("Clear Painting").clicked() {
                self.lines.clear();
            }
        });
    }

    pub fn ui_content(&mut self, ui: &mut Ui) {
        let (response, painter) =
            ui.allocate_painter(ui.available_size_before_wrap_finite(), Sense::drag());
        let rect = response.rect;

        let to_screen = emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, rect.square_proportions()),
            rect,
        );
        let from_screen = to_screen.inverse();

        if self.lines.is_empty() {
            self.lines.push(vec![]);
        }

        let current_line = self.lines.last_mut().unwrap();

        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let canvas_pos = from_screen * pointer_pos;
            if current_line.last() != Some(&canvas_pos) {
                current_line.push(canvas_pos);
            }
        } else if !current_line.is_empty() {
            self.lines.push(vec![]);
        }

        for line in &self.lines {
            if line.len() >= 2 {
                let points: Vec<Pos2> = line.iter().map(|p| to_screen * *p).collect();
                painter.add(Shape::line(points, self.stroke));
            }
        }
    }
}

impl super::Demo for Painting {
    fn name(&self) -> &str {
        "🖊 Painting"
    }

    fn show(&mut self, ctx: &CtxRef, open: &mut bool) {
        use super::View;
        Window::new(self.name())
            .open(open)
            .default_size(vec2(512.0, 512.0))
            .scroll(false)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for Painting {
    fn ui(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::__egui_github_link_file!());
        });
        self.ui_control(ui);
        ui.label("Paint with your mouse/touch!");
        Frame::dark_canvas(ui.style()).show(ui, |ui| {
            self.ui_content(ui);
        });
    }
}
