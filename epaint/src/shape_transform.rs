use crate::*;

pub fn adjust_colors(shape: &mut Shape, adjust_color: &impl Fn(&mut Color32)) {
    #![allow(clippy::match_same_arms)]
    match shape {
        Shape::Noop => {}
        Shape::Vec(shapes) => {
            for shape in shapes {
                adjust_colors(shape, adjust_color)
            }
        }
        Shape::Circle { fill, stroke, .. } => {
            adjust_color(fill);
            adjust_color(&mut stroke.color);
        }
        Shape::LineSegment { stroke, .. } => {
            adjust_color(&mut stroke.color);
        }
        Shape::Path { fill, stroke, .. } => {
            adjust_color(fill);
            adjust_color(&mut stroke.color);
        }
        Shape::Rect { fill, stroke, .. } => {
            adjust_color(fill);
            adjust_color(&mut stroke.color);
        }
        Shape::Text {
            galley,
            override_text_color,
            ..
        } => {
            if let Some(override_text_color) = override_text_color {
                adjust_color(override_text_color);
            }

            if !galley.is_empty() {
                let galley = std::sync::Arc::make_mut(galley);
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
    }
}
