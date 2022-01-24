use egui::*;
use egui::emath::RectTransform;
use egui::{epaint::{QuadraticBezierShape, CubicBezierShape,CircleShape}};

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct PaintBezier {
    bezier: usize, // current bezier curve degree, it can be 3,4,
    bezier_backup: usize, //track the bezier degree before change in order to clean the remaining points.
    points: Vec<Pos2>, //points already clicked. once it reaches the 'bezier' degree, it will be pushed into the 'shapes'
    backup_points:Vec<Pos2>, //track last points set in order to draw auxiliary lines.
    quadratic_shapes: Vec<QuadraticBezierShape>, //shapes already drawn. once it reaches the 'bezier' degree, it will be pushed into the 'shapes'
    cubic_shapes: Vec<CubicBezierShape>,
    aux_stroke: Stroke,
    stroke: Stroke,
}

impl Default for PaintBezier {
    fn default() -> Self {
        Self {
            bezier: 4, // default bezier degree, a cubic bezier curve
            bezier_backup: 4,
            points: Default::default(),
            backup_points: Default::default(),
            quadratic_shapes: Default::default(),
            cubic_shapes: Default::default(),
            aux_stroke: Stroke::new(1.0, Color32::RED),
            stroke: Stroke::new(1.0, Color32::LIGHT_BLUE),
        }
    }
}

impl PaintBezier {
    pub fn ui_control(&mut self, ui: &mut egui::Ui) -> egui::Response {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                egui::stroke_ui(ui, &mut self.stroke, "Curve Stroke");
                egui::stroke_ui(ui, &mut self.aux_stroke, "Auxiliary Stroke");
            });
            
            ui.separator();
            ui.vertical(|ui|{
                if ui.radio_value(&mut self.bezier, 3, "Quadratic").clicked(){
                    if self.bezier_backup != self.bezier{
                        self.points.clear();
                        self.bezier_backup = self.bezier;
                    }
                };
                if ui.radio_value(&mut self.bezier, 4, "Cubic").clicked(){
                    if self.bezier_backup != self.bezier{
                        self.points.clear();
                        self.bezier_backup = self.bezier;
                    }
                };
                
                // ui.radio_value(self.bezier, 5, "Quintic");
            });
            ui.separator();
            ui.vertical(|ui|{
                if ui.button("Clear Painting").clicked() {
                    self.points.clear();
                    self.backup_points.clear();
                    self.quadratic_shapes.clear();
                    self.cubic_shapes.clear();
                }
                ui.label("Click 3 or 4 points to build a bezier curve!");
            })
            
        })
        .response
    }

    // an internal function to create auxiliary lines around the current bezier curve
    // or to auxiliary lines (points) before the points meet the bezier curve requirements.
    fn build_auxiliary_line(&self,points:&[Pos2],to_screen:&RectTransform,aux_stroke:&Stroke)->Vec<Shape>{
        let mut shapes = Vec::new();
        if points.len()>=2 {
            let points:Vec<Pos2> = points.iter().map(|p| to_screen * *p).collect();
            shapes.push(egui::Shape::line(points, aux_stroke.clone()));
        }
        for point in points.iter(){
            let center = to_screen * *point;
            let radius = aux_stroke.width * 3.0;
            let circle = CircleShape {
                center,
                radius,
                fill: aux_stroke.color,
                stroke: aux_stroke.clone(),
            };
            
            shapes.push(circle.into());
        }
        
        shapes
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
                    match points.len(){
                        3 => {
                            let points: Vec<Pos2> = points.iter().map(|p| to_screen * *p).collect();
                            let stroke = self.stroke.clone();
                            let quadratic = QuadraticBezierShape::from_points_stroke(points, stroke);
                            self.quadratic_shapes.push(quadratic);
                        },
                        4 => {
                            let points: Vec<Pos2> = points.iter().map(|p| to_screen * *p).collect();
                            let stroke = self.stroke.clone();
                            let cubic = CubicBezierShape::from_points_stroke(points, stroke);
                            self.cubic_shapes.push(cubic);
                        },
                        _ => {
                            todo!();
                        }
                    }
                    
                    // self.points.clear();
                }
                
                response.mark_changed();
            } 
        }
        let mut shapes = Vec::new();
        for shape in self.quadratic_shapes.iter() {
            shapes.push(shape.clone().into());
        }
        for shape in self.cubic_shapes.iter() {
            shapes.push(shape.clone().into());
        }
        painter.extend(shapes);
        if self.points.len()>0{
            painter.extend(self.build_auxiliary_line(&self.points,&to_screen,&self.aux_stroke));
        }else if self.backup_points.len()>0{
            painter.extend(self.build_auxiliary_line(&self.backup_points,&to_screen,&self.aux_stroke));
        }

        response
    }
}

impl super::Demo for PaintBezier {
    fn name(&self) -> &'static str {
        "🖊 Bezier Curve"
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
