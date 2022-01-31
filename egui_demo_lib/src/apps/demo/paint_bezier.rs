use egui::emath::RectTransform;
use egui::epaint::{CircleShape, CubicBezierShape, QuadraticBezierShape};
use egui::*;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct PaintBezier {
    /// Current bezier curve degree, it can be 3, 4.
    bezier: usize,
    /// Track the bezier degree before change in order to clean the remaining points.
    degree_backup: usize,
    /// Points already clicked. once it reaches the 'bezier' degree, it will be pushed into the 'shapes'
    points: Vec<Pos2>,
    /// Track last points set in order to draw auxiliary lines.
    backup_points: Vec<Pos2>,
    /// Quadratic shapes already drawn.
    q_shapes: Vec<QuadraticBezierShape>,
    /// Cubic shapes already drawn.
    /// Since `Shape` can't be 'serialized', we can't use Shape as variable type.
    c_shapes: Vec<CubicBezierShape>,
    /// Stroke for auxiliary lines.
    aux_stroke: Stroke,
    /// Stroke for bezier curve.
    stroke: Stroke,
    /// Fill for bezier curve.
    fill: Color32,
    /// The curve should be closed or not.
    closed: bool,
    /// Display the bounding box or not.
    show_bounding_box: bool,
    /// Storke for the bounding box.
    bounding_box_stroke: Stroke,
}

impl Default for PaintBezier {
    fn default() -> Self {
        Self {
            bezier: 4, // default bezier degree, a cubic bezier curve
            degree_backup: 4,
            points: Default::default(),
            backup_points: Default::default(),
            q_shapes: Default::default(),
            c_shapes: Default::default(),
            aux_stroke: Stroke::new(1.0, Color32::RED),
            stroke: Stroke::new(1.0, Color32::LIGHT_BLUE),
            fill: Default::default(),
            closed: false,
            show_bounding_box: false,
            bounding_box_stroke: Stroke::new(1.0, Color32::LIGHT_GREEN),
        }
    }
}

impl PaintBezier {
    pub fn ui_control(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                egui::stroke_ui(ui, &mut self.stroke, "Curve Stroke");
                egui::stroke_ui(ui, &mut self.aux_stroke, "Auxiliary Stroke");
                ui.horizontal(|ui| {
                    ui.label("Fill Color:");
                    if ui.color_edit_button_srgba(&mut self.fill).changed()
                        && self.fill != Color32::TRANSPARENT
                    {
                        self.closed = true;
                    }
                    if ui.checkbox(&mut self.closed, "Closed").clicked() && !self.closed {
                        self.fill = Color32::TRANSPARENT;
                    }
                });
                egui::stroke_ui(ui, &mut self.bounding_box_stroke, "Bounding Box Stroke");
            });

            ui.separator();
            ui.vertical(|ui| {
                {
                    let mut tessellation_options = *(ui.ctx().tessellation_options());
                    let tessellation_options = &mut tessellation_options;
                    tessellation_options.ui(ui);
                    let mut new_tessellation_options = ui.ctx().tessellation_options();
                    *new_tessellation_options = *tessellation_options;
                }

                ui.checkbox(&mut self.show_bounding_box, "Bounding Box");
            });
            ui.separator();
            ui.vertical(|ui| {
                if ui.radio_value(&mut self.bezier, 3, "Quadratic").clicked()
                    && self.degree_backup != self.bezier
                {
                    self.points.clear();
                    self.degree_backup = self.bezier;
                };
                if ui.radio_value(&mut self.bezier, 4, "Cubic").clicked()
                    && self.degree_backup != self.bezier
                {
                    self.points.clear();
                    self.degree_backup = self.bezier;
                };
                // ui.radio_value(self.bezier, 5, "Quintic");
                ui.label("Click 3 or 4 points to build a bezier curve!");
                if ui.button("Clear Painting").clicked() {
                    self.points.clear();
                    self.backup_points.clear();
                    self.q_shapes.clear();
                    self.c_shapes.clear();
                }
            })
        })
        .response
    }

    pub fn ui_content(&mut self, ui: &mut Ui) -> egui::Response {
        let (mut response, painter) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::click());

        let to_screen = emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
            response.rect,
        );
        let from_screen = to_screen.inverse();

        if response.clicked() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let canvas_pos = from_screen * pointer_pos;
                self.points.push(canvas_pos);
                if self.points.len() >= self.bezier {
                    self.backup_points = self.points.clone();
                    let points = self.points.drain(..).collect::<Vec<_>>();
                    match points.len() {
                        3 => {
                            let quadratic = QuadraticBezierShape::from_points_stroke(
                                points,
                                self.closed,
                                self.fill,
                                self.stroke,
                            );
                            self.q_shapes.push(quadratic);
                        }
                        4 => {
                            let cubic = CubicBezierShape::from_points_stroke(
                                points,
                                self.closed,
                                self.fill,
                                self.stroke,
                            );
                            self.c_shapes.push(cubic);
                        }
                        _ => {
                            unreachable!();
                        }
                    }
                }

                response.mark_changed();
            }
        }
        let mut shapes = Vec::new();
        for shape in self.q_shapes.iter() {
            shapes.push(shape.to_screen(&to_screen).into());
            if self.show_bounding_box {
                shapes.push(self.build_bounding_box(shape.bounding_rect(), &to_screen));
            }
        }
        for shape in self.c_shapes.iter() {
            shapes.push(shape.to_screen(&to_screen).into());
            if self.show_bounding_box {
                shapes.push(self.build_bounding_box(shape.bounding_rect(), &to_screen));
            }
        }
        painter.extend(shapes);

        if !self.points.is_empty() {
            painter.extend(build_auxiliary_line(
                &self.points,
                &to_screen,
                &self.aux_stroke,
            ));
        } else if !self.backup_points.is_empty() {
            painter.extend(build_auxiliary_line(
                &self.backup_points,
                &to_screen,
                &self.aux_stroke,
            ));
        }

        response
    }

    pub fn build_bounding_box(&self, bbox: Rect, to_screen: &RectTransform) -> Shape {
        let bbox = Rect {
            min: to_screen * bbox.min,
            max: to_screen * bbox.max,
        };
        let bbox_shape = epaint::RectShape::stroke(bbox, 0.0, self.bounding_box_stroke);
        bbox_shape.into()
    }
}

/// An internal function to create auxiliary lines around the current bezier curve
/// or to auxiliary lines (points) before the points meet the bezier curve requirements.
fn build_auxiliary_line(
    points: &[Pos2],
    to_screen: &RectTransform,
    aux_stroke: &Stroke,
) -> Vec<Shape> {
    let mut shapes = Vec::new();
    if points.len() >= 2 {
        let points: Vec<Pos2> = points.iter().map(|p| to_screen * *p).collect();
        shapes.push(egui::Shape::line(points, *aux_stroke));
    }
    for point in points.iter() {
        let center = to_screen * *point;
        let radius = aux_stroke.width * 3.0;
        let circle = CircleShape {
            center,
            radius,
            fill: aux_stroke.color,
            stroke: *aux_stroke,
        };

        shapes.push(circle.into());
    }

    shapes
}

impl super::Demo for PaintBezier {
    fn name(&self) -> &'static str {
        "✔ Bezier Curve"
    }

    fn show(&mut self, ctx: &Context, open: &mut bool) {
        use super::View as _;
        Window::new(self.name())
            .open(open)
            .default_size(vec2(512.0, 512.0))
            .vscroll(false)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for PaintBezier {
    fn ui(&mut self, ui: &mut Ui) {
        // ui.vertical_centered(|ui| {
        //     ui.add(crate::__egui_github_link_file!());
        // });
        self.ui_control(ui);

        Frame::dark_canvas(ui.style()).show(ui, |ui| {
            self.ui_content(ui);
        });
    }
}
