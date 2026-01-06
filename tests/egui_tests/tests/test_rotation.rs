use egui::epaint::{EllipseShape, RectShape, StrokeKind};
use egui::{Color32, Grid, Pos2, Rect, Shape, Stroke, Vec2};
use egui_kittest::Harness;

const SHAPE_COLOR: Color32 = Color32::from_rgb(255, 165, 0);
const GHOST_COLOR: Color32 = Color32::from_rgb(0, 255, 255);
const PIVOT_COLOR: Color32 = Color32::from_rgb(255, 0, 255);
const CELL_SIZE: Vec2 = Vec2::new(180.0, 180.0);

#[test]
fn rotated_rect() {
    let shape_stroke = Stroke::new(2.0, Color32::BLACK);
    let ghost_stroke = Stroke::new(1.0, GHOST_COLOR);

    let mut harness = Harness::new_ui(|ui| {
        ui.ctx().set_pixels_per_point(1.0);

        let rect_size = Vec2::new(100.0, 60.0);
        let cell_center = Pos2::new(90.0, 90.0);
        let cell_rect = Rect::from_center_size(cell_center, rect_size);

        Grid::new("rotated_rect_grid")
            .spacing(Vec2::new(30.0, 30.0))
            .show(ui, |ui| {
                for (label, angle, pivot) in [
                    ("0°", 0.0, None),
                    ("Center 45°", 45.0f32.to_radians(), None),
                    (
                        "Top-Left 45°",
                        45.0f32.to_radians(),
                        Some(cell_rect.left_top()),
                    ),
                ] {
                    paint_case(ui, label, |offset| {
                        let rect = cell_rect.translate(offset);
                        let pivot = pivot.map(|p| p + offset);
                        let pivot_pos = pivot.unwrap_or_else(|| rect.center());

                        let ghost = RectShape::stroke(rect, 0.0, ghost_stroke, StrokeKind::Outside);
                        let shape = RectShape::new(
                            rect,
                            0.0,
                            SHAPE_COLOR,
                            shape_stroke,
                            StrokeKind::Outside,
                        )
                        .with_angle_and_pivot(angle, pivot_pos);

                        (ghost.into(), shape.into(), pivot_pos)
                    });
                }
            });
    });

    harness.fit_contents();
    harness.try_snapshot("rotated_rect").unwrap();
}

#[test]
fn rotated_ellipse() {
    let shape_stroke = Stroke::new(2.0, Color32::BLACK);
    let ghost_stroke = Stroke::new(1.0, GHOST_COLOR);

    let mut harness = Harness::new_ui(|ui| {
        ui.ctx().set_pixels_per_point(1.0);

        let rect_size = Vec2::new(100.0, 60.0);
        let cell_center = Pos2::new(90.0, 90.0);
        let radius = rect_size / 2.0;

        Grid::new("rotated_ellipse_grid")
            .spacing(Vec2::new(30.0, 30.0))
            .show(ui, |ui| {
                for (label, angle, pivot) in [
                    ("0°", 0.0, None),
                    ("Center 45°", 45.0f32.to_radians(), None),
                    (
                        "Top-Left 45°",
                        45.0f32.to_radians(),
                        Some(cell_center - radius),
                    ),
                ] {
                    paint_case(ui, label, |offset| {
                        let center = cell_center + offset;
                        let pivot = pivot.map(|p| p + offset);
                        let pivot_pos = pivot.unwrap_or_else(|| center);

                        let ghost = EllipseShape::stroke(center, radius, ghost_stroke);
                        let mut shape = EllipseShape::filled(center, radius, SHAPE_COLOR);
                        shape.stroke = shape_stroke;
                        let shape = shape.with_angle_and_pivot(angle, pivot_pos);

                        (ghost.into(), shape.into(), pivot_pos)
                    });
                }
            });
    });

    harness.fit_contents();
    harness.try_snapshot("rotated_ellipse").unwrap();
}

fn paint_case<F>(ui: &mut egui::Ui, label: &str, make_shapes: F)
where
    F: FnOnce(Vec2) -> (Shape, Shape, Pos2),
{
    ui.vertical(|ui| {
        ui.label(label);
        let (response, painter) = ui.allocate_painter(CELL_SIZE, egui::Sense::hover());
        let offset = response.rect.min.to_vec2();

        let (ghost, shape, pivot) = make_shapes(offset);
        painter.add(ghost);
        painter.add(shape);
        painter.circle_filled(pivot, 3.0, PIVOT_COLOR);
    });
}
