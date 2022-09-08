use crate::*;

pub fn adjust_colors(shape: &mut Shape, adjust_color: &impl Fn(&mut Color32)) {
    #![allow(clippy::match_same_arms)]
    match shape {
        Shape::Noop => {}
        Shape::Vec(shapes) => {
            for shape in shapes {
                adjust_colors(shape, adjust_color);
            }
        }
        Shape::Circle(circle_shape) => {
            adjust_color(&mut circle_shape.fill);
            adjust_color(&mut circle_shape.stroke.color);
        }
        Shape::LineSegment { stroke, .. } => {
            adjust_color(&mut stroke.color);
        }
        Shape::Path(path_shape) => {
            adjust_color(&mut path_shape.fill);
            adjust_color(&mut path_shape.stroke.color);
        }
        Shape::Rect(rect_shape) => {
            adjust_color(&mut rect_shape.fill);
            adjust_color(&mut rect_shape.stroke.color);
        }
        Shape::Text(text_shape) => {
            if let Some(override_text_color) = &mut text_shape.override_text_color {
                adjust_color(override_text_color);
            }

            if !text_shape.galley.is_empty() {
                let galley = std::sync::Arc::make_mut(&mut text_shape.galley);
                for row in &mut galley.rows {
                    for vertex in &mut row.visuals.mesh.vertices {
                        adjust_color(&mut vertex.color);
                    }
                }
            }
        }
        Shape::Mesh(mesh) => {
            for v in &mut mesh.vertices {
                adjust_color(&mut v.color);
            }
        }
        Shape::QuadraticBezier(quatratic) => {
            adjust_color(&mut quatratic.fill);
            adjust_color(&mut quatratic.stroke.color);
        }
        Shape::CubicBezier(bezier) => {
            adjust_color(&mut bezier.fill);
            adjust_color(&mut bezier.stroke.color);
        }
        Shape::Callback(_) => {
            // Can't tint user callback code
        }
    }
}
