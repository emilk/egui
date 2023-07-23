use std::sync::{Arc, RwLock};

use egui::epaint::{CubicBezierShape, PathShape, QuadraticBezierShape};
use egui::*;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct PaintBezierData {
    /// Bézier curve degree, it can be 3, 4.
    degree: usize,

    /// The control points. The [`Self::degree`] first of them are used.
    control_points: [Pos2; 4],

    /// Stroke for Bézier curve.
    stroke: Stroke,

    /// Fill for Bézier curve.
    fill: Color32,

    /// Stroke for auxiliary lines.
    aux_stroke: Stroke,

    bounding_box_stroke: Stroke,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
#[derive(Default, Clone)]
pub struct PaintBezier {
    data: Arc<RwLock<PaintBezierData>>,
}

impl Default for PaintBezierData {
    fn default() -> Self {
        Self {
            degree: 4,
            control_points: [
                pos2(50.0, 50.0),
                pos2(60.0, 250.0),
                pos2(200.0, 200.0),
                pos2(250.0, 50.0),
            ],
            stroke: Stroke::new(1.0, Color32::from_rgb(25, 200, 100)),
            fill: Color32::from_rgb(50, 100, 150).linear_multiply(0.25),
            aux_stroke: Stroke::new(1.0, Color32::RED.linear_multiply(0.25)),
            bounding_box_stroke: Stroke::new(0.0, Color32::LIGHT_GREEN.linear_multiply(0.25)),
        }
    }
}

impl PaintBezier {
    pub fn ui_control(&mut self, ui: &mut egui::Ui) {
        let mut data = self.data.write().unwrap();
        ui.collapsing("Colors", |ui| {
            ui.horizontal(|ui| {
                ui.label("Fill color:");
                ui.color_edit_button_srgba(&mut data.fill);
            });
            egui::stroke_ui(ui, &mut data.stroke, "Curve Stroke");
            egui::stroke_ui(ui, &mut data.aux_stroke, "Auxiliary Stroke");
            egui::stroke_ui(ui, &mut data.bounding_box_stroke, "Bounding Box Stroke");
        });

        ui.collapsing("Global tessellation options", |ui| {
            let mut tessellation_options = ui.ctx().tessellation_options(|to| *to);
            tessellation_options.ui(ui);
            ui.ctx()
                .tessellation_options_mut(|to| *to = tessellation_options);
        });

        ui.radio_value(&mut data.degree, 3, "Quadratic Bézier");
        ui.radio_value(&mut data.degree, 4, "Cubic Bézier");
        ui.label("Move the points by dragging them.");
        ui.small("Only convex curves can be accurately filled.");
    }

    pub fn ui_content(&mut self, ui: &mut Ui) -> egui::Response {
        let (response, painter) =
            ui.allocate_painter(Vec2::new(ui.available_width(), 300.0), Sense::hover());

        let to_screen = emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, response.rect.size()),
            response.rect,
        );

        let control_point_radius = 8.0;
        let mut data = self.data.write().unwrap();
        let data = &mut *data;

        let control_point_shapes: Vec<Shape> = data
            .control_points
            .iter_mut()
            .enumerate()
            .take(data.degree)
            .map(|(i, point)| {
                let size = Vec2::splat(2.0 * control_point_radius);

                let point_in_screen = to_screen.transform_pos(*point);
                let point_rect = Rect::from_center_size(point_in_screen, size);
                let point_id = response.id.with(i);
                let point_response = ui.interact(point_rect, point_id, Sense::drag());

                *point += point_response.drag_delta();
                *point = to_screen.from().clamp(*point);

                let point_in_screen = to_screen.transform_pos(*point);
                let stroke = ui.style().interact(&point_response).fg_stroke;

                Shape::circle_stroke(point_in_screen, control_point_radius, stroke)
            })
            .collect();

        let points_in_screen: Vec<Pos2> = data
            .control_points
            .iter()
            .take(data.degree)
            .map(|p| to_screen * *p)
            .collect();

        match data.degree {
            3 => {
                let points = points_in_screen.clone().try_into().unwrap();
                let shape =
                    QuadraticBezierShape::from_points_stroke(points, true, data.fill, data.stroke);
                painter.add(epaint::RectShape::stroke(
                    shape.visual_bounding_rect(),
                    0.0,
                    data.bounding_box_stroke,
                ));
                painter.add(shape);
            }
            4 => {
                let points = points_in_screen.clone().try_into().unwrap();
                let shape =
                    CubicBezierShape::from_points_stroke(points, true, data.fill, data.stroke);
                painter.add(epaint::RectShape::stroke(
                    shape.visual_bounding_rect(),
                    0.0,
                    data.bounding_box_stroke,
                ));
                painter.add(shape);
            }
            _ => {
                unreachable!();
            }
        };

        painter.add(PathShape::line(points_in_screen, data.aux_stroke));
        painter.extend(control_point_shapes);

        response
    }
}

impl super::Demo for PaintBezier {
    fn name(&self) -> &'static str {
        "） Bézier Curve"
    }

    fn show(&mut self, ctx: &Context, open: &mut bool) {
        let clone = self.clone();
        use super::View as _;
        Window::new(self.name())
            .open(open)
            .vscroll(false)
            .resizable(false)
            .default_size([300.0, 350.0])
            .show(ctx, move |ui| clone.clone().ui(ui));
    }
}

impl super::View for PaintBezier {
    fn ui(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
        self.ui_control(ui);

        Frame::canvas(ui.style()).show(ui, |ui| {
            self.ui_content(ui);
        });
    }
}
