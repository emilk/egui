use std::sync::Arc;

use egui::{
    Color32, Pos2, Rect, Sense, StrokeKind, Vec2,
    emath::{GuiRounding as _, TSTransform},
    epaint::{self, RectShape},
    vec2,
};

#[derive(Clone, Debug, PartialEq)]
pub struct TessellationTest {
    shape: RectShape,

    magnification_pixel_size: f32,
    tessellation_options: epaint::TessellationOptions,
    paint_edges: bool,
}

impl Default for TessellationTest {
    fn default() -> Self {
        let shape = Self::interesting_shapes()[0].1.clone();
        Self {
            shape,
            magnification_pixel_size: 12.0,
            tessellation_options: Default::default(),
            paint_edges: false,
        }
    }
}

impl TessellationTest {
    fn interesting_shapes() -> Vec<(&'static str, RectShape)> {
        fn sized(size: impl Into<Vec2>) -> Rect {
            Rect::from_center_size(Pos2::ZERO, size.into())
        }

        let baby_blue = Color32::from_rgb(0, 181, 255);

        let mut shapes = vec![
            (
                "Normal",
                RectShape::new(
                    sized([20.0, 16.0]),
                    2.0,
                    baby_blue,
                    (1.0, Color32::WHITE),
                    StrokeKind::Inside,
                ),
            ),
            (
                "Minimal rounding",
                RectShape::new(
                    sized([20.0, 16.0]),
                    1.0,
                    baby_blue,
                    (1.0, Color32::WHITE),
                    StrokeKind::Inside,
                ),
            ),
            (
                "Thin filled",
                RectShape::filled(sized([20.0, 0.5]), 2.0, baby_blue),
            ),
            (
                "Thin stroked",
                RectShape::new(
                    sized([20.0, 0.5]),
                    2.0,
                    baby_blue,
                    (0.5, Color32::WHITE),
                    StrokeKind::Inside,
                ),
            ),
            (
                "Blurred",
                RectShape::filled(sized([20.0, 16.0]), 2.0, baby_blue).with_blur_width(50.0),
            ),
            (
                "Thick stroke, minimal rounding",
                RectShape::new(
                    sized([20.0, 16.0]),
                    1.0,
                    baby_blue,
                    (3.0, Color32::WHITE),
                    StrokeKind::Inside,
                ),
            ),
            (
                "Blurred stroke",
                RectShape::new(
                    sized([20.0, 16.0]),
                    0.0,
                    baby_blue,
                    (5.0, Color32::WHITE),
                    StrokeKind::Inside,
                )
                .with_blur_width(5.0),
            ),
            (
                "Additive rectangle",
                RectShape::new(
                    sized([24.0, 12.0]),
                    0.0,
                    egui::Color32::LIGHT_RED.additive().linear_multiply(0.025),
                    (
                        1.0,
                        egui::Color32::LIGHT_BLUE.additive().linear_multiply(0.1),
                    ),
                    StrokeKind::Outside,
                ),
            ),
        ];

        for (_name, shape) in &mut shapes {
            shape.round_to_pixels = Some(true);
        }

        shapes
    }
}

impl crate::Demo for TessellationTest {
    fn name(&self) -> &'static str {
        "Tessellation Test"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .resizable(false)
            .open(open)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for TessellationTest {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.add(crate::egui_github_link_file!());
        egui::reset_button(ui, self, "Reset");

        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    rect_shape_ui(ui, &mut self.shape);
                });
            });

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.heading("Real size");
                    egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                        let (resp, painter) =
                            ui.allocate_painter(Vec2::splat(128.0), Sense::hover());
                        let canvas = resp.rect;

                        let pixels_per_point = ui.pixels_per_point();
                        let pixel_size = 1.0 / pixels_per_point;
                        let mut shape = self.shape.clone();
                        shape.rect = Rect::from_center_size(canvas.center(), shape.rect.size())
                            .round_to_pixel_center(pixels_per_point)
                            .translate(Vec2::new(pixel_size / 3.0, pixel_size / 5.0)); // Intentionally offset to test the effect of rounding
                        painter.add(shape);
                    });
                });
            });
        });

        ui.group(|ui| {
            ui.heading("Zoomed in");
            let magnification_pixel_size = &mut self.magnification_pixel_size;
            let tessellation_options = &mut self.tessellation_options;

            egui::Grid::new("TessellationOptions")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Magnification");
                    ui.add(
                        egui::DragValue::new(magnification_pixel_size)
                            .speed(0.5)
                            .range(1.0..=32.0),
                    );
                    ui.end_row();

                    ui.label("Feathering width");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut tessellation_options.feathering, "");
                        ui.add_enabled(
                            tessellation_options.feathering,
                            egui::DragValue::new(
                                &mut tessellation_options.feathering_size_in_pixels,
                            )
                            .speed(0.1)
                            .range(0.0..=4.0)
                            .suffix(" px"),
                        );
                    });
                    ui.end_row();

                    ui.label("Paint edges");
                    ui.checkbox(&mut self.paint_edges, "");
                    ui.end_row();
                });

            let magnification_pixel_size = *magnification_pixel_size;

            egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                let (resp, painter) = ui.allocate_painter(
                    magnification_pixel_size * (self.shape.rect.size() + Vec2::splat(8.0)),
                    Sense::hover(),
                );
                let canvas = resp.rect;

                let mut shape = self.shape.clone();
                shape.rect = shape.rect.translate(Vec2::new(1.0 / 3.0, 1.0 / 5.0)); // Intentionally offset to test the effect of rounding

                let mut mesh = epaint::Mesh::default();
                let mut tessellator = epaint::Tessellator::new(
                    1.0,
                    *tessellation_options,
                    ui.fonts(|f| f.font_image_size()),
                    vec![],
                );
                tessellator.tessellate_rect(&shape, &mut mesh);

                // Scale and position the mesh:
                mesh.transform(
                    TSTransform::from_translation(canvas.center().to_vec2())
                        * TSTransform::from_scaling(magnification_pixel_size),
                );
                let mesh = Arc::new(mesh);
                painter.add(epaint::Shape::mesh(Arc::clone(&mesh)));

                if self.paint_edges {
                    let stroke = epaint::Stroke::new(0.5, Color32::MAGENTA);
                    for triangle in mesh.triangles() {
                        let a = mesh.vertices[triangle[0] as usize];
                        let b = mesh.vertices[triangle[1] as usize];
                        let c = mesh.vertices[triangle[2] as usize];

                        painter.line_segment([a.pos, b.pos], stroke);
                        painter.line_segment([b.pos, c.pos], stroke);
                        painter.line_segment([c.pos, a.pos], stroke);
                    }
                }

                if 3.0 < magnification_pixel_size {
                    // Draw pixel centers:
                    let pixel_radius = 0.75;
                    let pixel_color = Color32::GRAY;
                    for yi in 0.. {
                        let y = (yi as f32 + 0.5) * magnification_pixel_size;
                        if y > canvas.height() / 2.0 {
                            break;
                        }
                        for xi in 0.. {
                            let x = (xi as f32 + 0.5) * magnification_pixel_size;
                            if x > canvas.width() / 2.0 {
                                break;
                            }
                            for offset in [vec2(x, y), vec2(x, -y), vec2(-x, y), vec2(-x, -y)] {
                                painter.circle_filled(
                                    canvas.center() + offset,
                                    pixel_radius,
                                    pixel_color,
                                );
                            }
                        }
                    }
                }
            });
        });
    }
}

fn rect_shape_ui(ui: &mut egui::Ui, shape: &mut RectShape) {
    egui::ComboBox::from_id_salt("prefabs")
        .selected_text("Prefabs")
        .show_ui(ui, |ui| {
            for (name, prefab) in TessellationTest::interesting_shapes() {
                ui.selectable_value(shape, prefab, name);
            }
        });

    ui.add_space(4.0);

    let RectShape {
        rect,
        corner_radius,
        fill,
        stroke,
        stroke_kind,
        blur_width,
        round_to_pixels,
        brush: _,
    } = shape;

    let round_to_pixels = round_to_pixels.get_or_insert(true);

    egui::Grid::new("RectShape")
        .num_columns(2)
        .spacing([12.0, 8.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Size");
            ui.horizontal(|ui| {
                let mut size = rect.size();
                ui.add(
                    egui::DragValue::new(&mut size.x)
                        .speed(0.2)
                        .range(0.0..=64.0),
                );
                ui.add(
                    egui::DragValue::new(&mut size.y)
                        .speed(0.2)
                        .range(0.0..=64.0),
                );
                *rect = Rect::from_center_size(Pos2::ZERO, size);
            });
            ui.end_row();

            ui.label("Corner radius");
            ui.add(corner_radius);
            ui.end_row();

            ui.label("Fill");
            ui.color_edit_button_srgba(fill);
            ui.end_row();

            ui.label("Stroke");
            ui.add(stroke);
            ui.end_row();

            ui.label("Stroke kind");
            ui.horizontal(|ui| {
                ui.selectable_value(stroke_kind, StrokeKind::Inside, "Inside");
                ui.selectable_value(stroke_kind, StrokeKind::Middle, "Middle");
                ui.selectable_value(stroke_kind, StrokeKind::Outside, "Outside");
            });
            ui.end_row();

            ui.label("Blur width");
            ui.add(
                egui::DragValue::new(blur_width)
                    .speed(0.5)
                    .range(0.0..=20.0),
            );
            ui.end_row();

            ui.label("Round to pixels");
            ui.checkbox(round_to_pixels, "");
            ui.end_row();
        });
}

#[cfg(test)]
mod tests {
    use crate::View as _;
    use egui_kittest::SnapshotResults;

    use super::*;

    #[test]
    fn snapshot_tessellation_test() {
        let mut results = SnapshotResults::new();
        for (name, shape) in TessellationTest::interesting_shapes() {
            let mut test = TessellationTest {
                shape,
                ..Default::default()
            };
            let mut harness = egui_kittest::Harness::new_ui(|ui| {
                test.ui(ui);
            });

            harness.fit_contents();
            harness.run();

            harness.snapshot(format!("tessellation_test/{name}"));
            results.extend_harness(&mut harness);
        }
    }
}
