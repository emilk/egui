use crate::{
    text::{Fonts, Galley, TextStyle},
    Color32, Stroke, Triangles,
};
use emath::*;

/// A paint primitive such as a circle or a piece of text.
/// Coordinates are all screen space points (not physical pixels).
#[must_use = "Add a Shape to a Painter"]
#[derive(Clone, Debug)]
pub enum Shape {
    /// Paint nothing. This can be useful as a placeholder.
    Noop,
    /// Recursively nest more shapes - sometimes a convenience to be able to do.
    /// For performance reasons it is better to avoid it.
    Vec(Vec<Shape>),
    Circle {
        center: Pos2,
        radius: f32,
        fill: Color32,
        stroke: Stroke,
    },
    LineSegment {
        points: [Pos2; 2],
        stroke: Stroke,
    },
    Path {
        points: Vec<Pos2>,
        /// If true, connect the first and last of the points together.
        /// This is required if `fill != TRANSPARENT`.
        closed: bool,
        fill: Color32,
        stroke: Stroke,
    },
    Rect {
        rect: Rect,
        /// How rounded the corners are. Use `0.0` for no rounding.
        corner_radius: f32,
        fill: Color32,
        stroke: Stroke,
    },
    Text {
        /// Top left corner of the first character.
        pos: Pos2,
        /// The layed out text
        galley: Galley,
        text_style: TextStyle, // TODO: Font?
        color: Color32,
    },
    Triangles(Triangles),
}

/// ## Constructors
impl Shape {
    pub fn line_segment(points: [Pos2; 2], stroke: impl Into<Stroke>) -> Self {
        Self::LineSegment {
            points,
            stroke: stroke.into(),
        }
    }

    pub fn line(points: Vec<Pos2>, stroke: impl Into<Stroke>) -> Self {
        Self::Path {
            points,
            closed: false,
            fill: Default::default(),
            stroke: stroke.into(),
        }
    }

    pub fn closed_line(points: Vec<Pos2>, stroke: impl Into<Stroke>) -> Self {
        Self::Path {
            points,
            closed: true,
            fill: Default::default(),
            stroke: stroke.into(),
        }
    }

    pub fn polygon(points: Vec<Pos2>, fill: impl Into<Color32>, stroke: impl Into<Stroke>) -> Self {
        Self::Path {
            points,
            closed: true,
            fill: fill.into(),
            stroke: stroke.into(),
        }
    }

    pub fn circle_filled(center: Pos2, radius: f32, fill_color: impl Into<Color32>) -> Self {
        Self::Circle {
            center,
            radius,
            fill: fill_color.into(),
            stroke: Default::default(),
        }
    }

    pub fn circle_stroke(center: Pos2, radius: f32, stroke: impl Into<Stroke>) -> Self {
        Self::Circle {
            center,
            radius,
            fill: Default::default(),
            stroke: stroke.into(),
        }
    }

    pub fn rect_filled(rect: Rect, corner_radius: f32, fill_color: impl Into<Color32>) -> Self {
        Self::Rect {
            rect,
            corner_radius,
            fill: fill_color.into(),
            stroke: Default::default(),
        }
    }

    pub fn rect_stroke(rect: Rect, corner_radius: f32, stroke: impl Into<Stroke>) -> Self {
        Self::Rect {
            rect,
            corner_radius,
            fill: Default::default(),
            stroke: stroke.into(),
        }
    }

    pub fn text(
        fonts: &Fonts,
        pos: Pos2,
        anchor: Align2,
        text: impl Into<String>,
        text_style: TextStyle,
        color: Color32,
    ) -> Self {
        let font = &fonts[text_style];
        let galley = font.layout_multiline(text.into(), f32::INFINITY);
        let rect = anchor.anchor_rect(Rect::from_min_size(pos, galley.size));
        Self::Text {
            pos: rect.min,
            galley,
            text_style,
            color,
        }
    }
}

/// ## Operations
impl Shape {
    pub fn triangles(triangles: Triangles) -> Self {
        debug_assert!(triangles.is_valid());
        Self::Triangles(triangles)
    }

    pub fn texture_id(&self) -> super::TextureId {
        if let Shape::Triangles(triangles) = self {
            triangles.texture_id
        } else {
            super::TextureId::Egui
        }
    }

    /// Translate location by this much, in-place
    pub fn translate(&mut self, delta: Vec2) {
        match self {
            Shape::Noop => {}
            Shape::Vec(shapes) => {
                for shape in shapes {
                    shape.translate(delta);
                }
            }
            Shape::Circle { center, .. } => {
                *center += delta;
            }
            Shape::LineSegment { points, .. } => {
                for p in points {
                    *p += delta;
                }
            }
            Shape::Path { points, .. } => {
                for p in points {
                    *p += delta;
                }
            }
            Shape::Rect { rect, .. } => {
                *rect = rect.translate(delta);
            }
            Shape::Text { pos, .. } => {
                *pos += delta;
            }
            Shape::Triangles(triangles) => {
                triangles.translate(delta);
            }
        }
    }
}
