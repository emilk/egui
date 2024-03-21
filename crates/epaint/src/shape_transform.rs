use crate::*;

/// Remember to handle [`Color32::PLACEHOLDER`] specially!
pub fn adjust_colors(shape: &mut Shape, adjust_color: &impl Fn(&mut Color32)) {
    #![allow(clippy::match_same_arms)]
    match shape {
        Shape::Noop => {}
        Shape::Vec(shapes) => {
            for shape in shapes {
                adjust_colors(shape, adjust_color);
            }
        }
        Shape::LineSegment { stroke, points: _ } => {
            adjust_color(&mut stroke.color);
        }

        Shape::Circle(CircleShape {
            center: _,
            radius: _,
            fill,
            stroke,
        })
        | Shape::Ellipse(EllipseShape {
            center: _,
            radius: _,
            fill,
            stroke,
        })
        | Shape::Path(PathShape {
            points: _,
            closed: _,
            fill,
            stroke,
        })
        | Shape::Rect(RectShape {
            rect: _,
            rounding: _,
            fill,
            stroke,
            fill_texture_id: _,
            uv: _,
        })
        | Shape::QuadraticBezier(QuadraticBezierShape {
            points: _,
            closed: _,
            fill,
            stroke,
        })
        | Shape::CubicBezier(CubicBezierShape {
            points: _,
            closed: _,
            fill,
            stroke,
        }) => {
            adjust_color(fill);
            adjust_color(&mut stroke.color);
        }

        Shape::Text(TextShape {
            pos: _,
            galley,
            underline,
            fallback_color,
            override_text_color,
            opacity_factor: _,
            angle: _,
        }) => {
            adjust_color(&mut underline.color);
            adjust_color(fallback_color);
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

        Shape::Mesh(Mesh {
            indices: _,
            vertices,
            texture_id: _,
        }) => {
            for v in vertices {
                adjust_color(&mut v.color);
            }
        }

        Shape::Callback(_) => {
            // Can't tint user callback code
        }
    }
}
