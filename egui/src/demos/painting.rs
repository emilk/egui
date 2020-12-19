use crate::{demos::*, *};

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Painting {
    lines: Vec<Vec<Vec2>>,
    stroke: Stroke,
}

impl Default for Painting {
    fn default() -> Self {
        Self {
            lines: Default::default(),
            stroke: Stroke::new(1.0, color::LIGHT_BLUE),
        }
    }
}

impl Painting {
    pub fn ui_control(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            self.stroke.ui(ui, "Stroke");
            ui.separator();
            if ui.button("Clear Painting").clicked {
                self.lines.clear();
            }
        });
    }

    pub fn ui_content(&mut self, ui: &mut Ui) {
        let painter = ui.allocate_painter(ui.available_size_before_wrap_finite());
        let rect = painter.clip_rect();
        let id = ui.make_position_id();
        let response = ui.interact(rect, id, Sense::drag());

        if self.lines.is_empty() {
            self.lines.push(vec![]);
        }

        let current_line = self.lines.last_mut().unwrap();

        if response.active {
            if let Some(mouse_pos) = ui.input().mouse.pos {
                let canvas_pos = mouse_pos - rect.min;
                if current_line.last() != Some(&canvas_pos) {
                    current_line.push(canvas_pos);
                }
            }
        } else if !current_line.is_empty() {
            self.lines.push(vec![]);
        }

        for line in &self.lines {
            if line.len() >= 2 {
                let points: Vec<Pos2> = line.iter().map(|p| rect.min + *p).collect();
                painter.add(PaintCmd::line(points, self.stroke));
            }
        }
    }
}

impl demos::Demo for Painting {
    fn name(&self) -> &str {
        "🖊 Painting"
    }

    fn show(&mut self, ctx: &CtxRef, open: &mut bool) {
        Window::new(self.name())
            .open(open)
            .default_size(vec2(512.0, 512.0))
            .scroll(false)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl demos::View for Painting {
    fn ui(&mut self, ui: &mut Ui) {
        ui.add(__egui_github_link_file!("(source code)"));
        self.ui_control(ui);
        ui.label("Paint with your mouse/touch!");
        Frame::dark_canvas(ui.style()).show(ui, |ui| {
            self.ui_content(ui);
        });
    }
}
