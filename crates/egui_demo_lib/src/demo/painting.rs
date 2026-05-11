use egui::{Color32, Frame, Pos2, Rect, Sense, Shape, Stroke, Ui, Window, emath, vec2};

/// A point in a stroke, optionally carrying pressure information.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct LinePoint {
    pos: Pos2,
    /// Pressure in the range [0.0, 1.0], or `None` for constant-width strokes.
    pressure: Option<f32>,
}

/// A completed or in-progress stroke.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct Line {
    points: Vec<LinePoint>,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Painting {
    lines: Vec<Line>,
    stroke: Stroke,
    use_pressure: bool,
}

impl Default for Painting {
    fn default() -> Self {
        Self {
            lines: Default::default(),
            stroke: Stroke::new(1.0, Color32::from_rgb(25, 200, 100)),
            use_pressure: false,
        }
    }
}

impl Painting {
    pub fn ui_control(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal(|ui| {
            ui.label("Stroke:");
            ui.add(&mut self.stroke);
            ui.separator();
            if ui.button("Clear Painting").clicked() {
                self.lines.clear();
            }
        })
        .response
    }

    pub fn ui_content(&mut self, ui: &mut Ui) -> egui::Response {
        let (mut response, painter) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::drag());

        let to_screen = emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
            response.rect,
        );
        let from_screen = to_screen.inverse();

        let pressure = ui.ctx().input(|i| i.pointer.pressure());
        let has_pressure = pressure.is_some() && self.use_pressure;

        self.paint_strokes(&mut response, &painter, &to_screen, &from_screen, if has_pressure { pressure } else { None });

        response
    }

    fn paint_strokes(
        &mut self,
        response: &mut egui::Response,
        painter: &egui::Painter,
        to_screen: &emath::RectTransform,
        from_screen: &emath::RectTransform,
        pressure: Option<f32>,
    ) {
        if self.lines.is_empty() {
            self.lines.push(Line { points: Vec::new() });
        }

        let current_line = self.lines.last_mut().unwrap();

        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let canvas_pos = from_screen * pointer_pos;
            if current_line.points.last().map_or(true, |p| p.pos != canvas_pos) {
                current_line.points.push(LinePoint { pos: canvas_pos, pressure });
                response.mark_changed();
            }
        } else if current_line.points.last().is_some() {
            self.lines.push(Line { points: Vec::new() });
            response.mark_changed();
        }

        for line in &self.lines {
            if line.points.len() < 2 {
                continue;
            }

            let is_pressure = line.points.iter().any(|p| p.pressure.is_some());
            if is_pressure {
                self.paint_pressure_line(painter, line, to_screen);
            } else {
                self.paint_constant_line(painter, line, to_screen);
            }
        }
    }

    fn paint_constant_line(
        &self,
        painter: &egui::Painter,
        line: &Line,
        to_screen: &emath::RectTransform,
    ) {
        let points: Vec<Pos2> = line.points.iter().map(|p| to_screen * p.pos).collect();
        painter.add(Shape::line(points, self.stroke));
    }

    fn paint_pressure_line(
        &self,
        painter: &egui::Painter,
        line: &Line,
        to_screen: &emath::RectTransform,
    ) {
        for i in 1..line.points.len() {
            let from = to_screen * line.points[i - 1].pos;
            let to = to_screen * line.points[i].pos;

            let delta = to - from;
            let len_sq = delta.length_sq();
            if len_sq < 0.001 {
                continue;
            }

            let pressure_from = line.points[i - 1].pressure.unwrap_or(0.5);
            let pressure_to = line.points[i].pressure.unwrap_or(0.5);

            let width_from = self.stroke.width * pressure_from * 2.0;
            let width_to = self.stroke.width * pressure_to * 2.0;

            let dir = delta.normalized();
            let perp = egui::vec2(-dir.y, dir.x);

            let p0 = from + perp * width_from / 2.0;
            let p1 = from - perp * width_from / 2.0;
            let p2 = to - perp * width_to / 2.0;
            let p3 = to + perp * width_to / 2.0;

            painter.add(Shape::convex_polygon(
                vec![p0, p1, p2, p3],
                self.stroke.color,
                Stroke::NONE,
            ));
        }

        for point in &line.points {
            let pos = to_screen * point.pos;
            let pressure = point.pressure.unwrap_or(0.5);
            let radius = self.stroke.width * pressure;
            painter.circle_filled(pos, radius, self.stroke.color);
        }
    }
}

impl crate::Demo for Painting {
    fn name(&self) -> &'static str {
        "🖊 Painting"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        use crate::View as _;
        Window::new(self.name())
            .open(open)
            .default_size(vec2(512.0, 512.0))
            .vscroll(false)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| self.ui(ui));
    }
}

impl crate::View for Painting {
    fn ui(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
        self.ui_control(ui);

        let pressure_info = ui.ctx().input(|i| {
            if let Some(pressure) = i.pointer.pressure() {
                format!("Pressure: {:.2}", pressure)
            } else {
                "Pressure: not available".to_owned()
            }
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.use_pressure, "Use pressure sensitivity");
            if self.use_pressure {
                ui.separator();
                ui.label(&pressure_info);
            }
        });

        ui.label("Paint with your mouse/touch/stylus!");
        Frame::canvas(ui.style()).show(ui, |ui| {
            self.ui_content(ui);
        });
    }
}
