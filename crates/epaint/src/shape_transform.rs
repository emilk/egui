use std::sync::Arc;

use crate::{
    CircleShape, Color32, ColorMode, CubicBezierShape, EllipseShape, Mesh, PathShape,
    QuadraticBezierShape, RectShape, Shape, TextShape, color,
};

/// Remember to handle [`Color32::PLACEHOLDER`] specially!
pub fn adjust_colors(
    shape: &mut Shape,
    adjust_color: impl Fn(&mut Color32) + Send + Sync + Copy + 'static,
) {
    #![expect(clippy::match_same_arms)]
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

        Shape::Path(PathShape {
            points: _,
            closed: _,
            fill,
            stroke,
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
            adjust_color_mode(&mut stroke.color, adjust_color);
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
        | Shape::Rect(RectShape {
            rect: _,
            corner_radius: _,
            fill,
            stroke,
            stroke_kind: _,
            round_to_pixels: _,
            blur_width: _,
            brush: _,
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
                let galley = Arc::make_mut(galley);
                for placed_row in &mut galley.rows {
                    let row = Arc::make_mut(&mut placed_row.row);
                    for vertex in &mut row.visuals.mesh.vertices {
                        adjust_color(&mut vertex.color);
                    }
                }
            }
        }

        Shape::Mesh(mesh) => {
            let Mesh {
                indices: _,
                vertices,
                texture_id: _,
            } = Arc::make_mut(mesh);

            for v in vertices {
                adjust_color(&mut v.color);
            }
        }

        Shape::Callback(_) => {
            // Can't tint user callback code
        }
    }
}

fn adjust_color_mode(
    color_mode: &mut ColorMode,
    adjust_color: impl Fn(&mut Color32) + Send + Sync + Copy + 'static,
) {
    match color_mode {
        color::ColorMode::Solid(color) => adjust_color(color),
        color::ColorMode::UV(callback) => {
            let callback = Arc::clone(callback);
            *color_mode = color::ColorMode::UV(Arc::new(Box::new(move |rect, pos| {
                let mut color = callback(rect, pos);
                adjust_color(&mut color);
                color
            })));
        }
    }
}
