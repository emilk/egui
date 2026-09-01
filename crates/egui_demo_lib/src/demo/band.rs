use egui::{Color32, Sense, Stroke, StrokeKind, Vec2, emath::TSTransform, epaint};

use crate::View as _;

/// Demonstrates a varying-width band with an animated waveform.
pub struct BandDemo {
    phase: f32,
    speed: f32,
    angle: f32,
    fill_opacity: f32,
    stroke_opacity: f32,
    stroke_width: f32,
    stroke_kind: StrokeKind,
}

impl Default for BandDemo {
    fn default() -> Self {
        Self {
            phase: 0.0,
            speed: 0.04,
            angle: 0.0,
            fill_opacity: 0.38,
            stroke_opacity: 1.0,
            stroke_width: 1.5,
            stroke_kind: StrokeKind::Inside,
        }
    }
}

impl crate::Demo for BandDemo {
    fn name(&self) -> &'static str {
        "Band"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .resizable(false)
            .open(open)
            .show(ui.ctx(), |ui| self.ui(ui));
    }
}

impl crate::View for BandDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.add(crate::egui_github_link_file!());
        ui.label("An animated waveform rendered as a rotated band.");

        egui::Grid::new("band_controls")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("Angle");
                ui.add(
                    egui::DragValue::new(&mut self.angle)
                        .speed(0.01)
                        .suffix(" rad"),
                );
                ui.end_row();

                ui.label("Animation speed");
                ui.add(
                    egui::DragValue::new(&mut self.speed)
                        .speed(0.001)
                        .range(0.0..=1.0),
                );
                ui.end_row();

                ui.label("Fill opacity");
                ui.add(
                    egui::DragValue::new(&mut self.fill_opacity)
                        .speed(0.01)
                        .range(0.0..=1.0),
                );
                ui.end_row();

                ui.label("Stroke opacity");
                ui.add(
                    egui::DragValue::new(&mut self.stroke_opacity)
                        .speed(0.01)
                        .range(0.0..=1.0),
                );
                ui.end_row();

                ui.label("Stroke width");
                ui.add(
                    egui::DragValue::new(&mut self.stroke_width)
                        .speed(0.1)
                        .range(0.0..=10.0),
                );
                ui.end_row();

                ui.label("Stroke kind");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.stroke_kind, StrokeKind::Inside, "Inside");
                    ui.selectable_value(&mut self.stroke_kind, StrokeKind::Middle, "Middle");
                    ui.selectable_value(&mut self.stroke_kind, StrokeKind::Outside, "Outside");
                });
                ui.end_row();
            });

        egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
            let (response, painter) = ui.allocate_painter(Vec2::new(480.0, 240.0), Sense::hover());
            let center = response.rect.center();
            let width = response.rect.width() - 80.0;
            let phase = self.phase;
            let points = (0..=96)
                .map(|index| {
                    let t = index as f32 / 96.0;
                    let x = (t - 0.5) * width;
                    let y = 12.0 * (x * 0.025 + phase).sin();
                    let radius = 14.0 + 20.0 * (x * 0.05 - phase).sin().abs();
                    epaint::BandPoint::new(x, (y - radius)..=(y + radius))
                })
                .collect();
            let mut band = epaint::BandShape::new(
                points,
                Color32::from_rgba_unmultiplied(80, 190, 255, (255.0 * self.fill_opacity) as u8),
                Stroke::new(
                    self.stroke_width,
                    Color32::from_white_alpha((255.0 * self.stroke_opacity) as u8),
                ),
            )
            .with_stroke_kind(self.stroke_kind);
            band.transform(TSTransform::from_translation(center.to_vec2()));
            painter.add(band.with_angle_and_pivot(self.angle, center));
        });

        self.phase += self.speed;
        if 0.0 < self.speed.abs() {
            ui.ctx().request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_band() {
        let mut demo = BandDemo {
            speed: 0.0,
            ..Default::default()
        };
        let mut harness = egui_kittest::Harness::new_ui(|ui| demo.ui(ui));
        harness.fit_contents();
        harness.run();
        harness.snapshot("band");
    }
}
