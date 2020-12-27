use {
    super::{fonts::TextStyle, Fonts, Galley, Srgba, Triangles},
    crate::{
        align::{anchor_rect, Align},
        math::{Pos2, Rect},
        *,
    },
};

/// A paint primitive such as a circle or a piece of text.
#[derive(Clone, Debug)]
pub enum PaintCmd {
    /// Paint nothing. This can be useful as a placeholder.
    Noop,
    Circle {
        center: Pos2,
        radius: f32,
        fill: Srgba,
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
        fill: Srgba,
        stroke: Stroke,
    },
    Rect {
        rect: Rect,
        /// How rounded the corners are. Use `0.0` for no rounding.
        corner_radius: f32,
        fill: Srgba,
        stroke: Stroke,
    },
    Text {
        /// Top left corner of the first character.
        pos: Pos2,
        /// The layed out text
        galley: Galley,
        text_style: TextStyle, // TODO: Font?
        color: Srgba,
    },
    Triangles(Triangles),
}

/// ## Constructors
impl PaintCmd {
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

    pub fn polygon(points: Vec<Pos2>, fill: impl Into<Srgba>, stroke: impl Into<Stroke>) -> Self {
        Self::Path {
            points,
            closed: true,
            fill: fill.into(),
            stroke: stroke.into(),
        }
    }

    pub fn circle_filled(center: Pos2, radius: f32, fill_color: impl Into<Srgba>) -> Self {
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

    pub fn rect_filled(rect: Rect, corner_radius: f32, fill_color: impl Into<Srgba>) -> Self {
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
        anchor: (Align, Align),
        text: impl Into<String>,
        text_style: TextStyle,
        color: Srgba,
    ) -> Self {
        let font = &fonts[text_style];
        let galley = font.layout_multiline(text.into(), f32::INFINITY);
        let rect = anchor_rect(Rect::from_min_size(pos, galley.size), anchor);
        Self::Text {
            pos: rect.min,
            galley,
            text_style,
            color,
        }
    }
}

/// ## Operations
impl PaintCmd {
    pub fn triangles(triangles: Triangles) -> Self {
        debug_assert!(triangles.is_valid());
        Self::Triangles(triangles)
    }

    pub fn texture_id(&self) -> super::TextureId {
        if let PaintCmd::Triangles(triangles) = self {
            triangles.texture_id
        } else {
            super::TextureId::Egui
        }
    }

    /// Translate location by this much, in-place
    pub fn translate(&mut self, delta: Vec2) {
        match self {
            PaintCmd::Noop => {}
            PaintCmd::Circle { center, .. } => {
                *center += delta;
            }
            PaintCmd::LineSegment { points, .. } => {
                for p in points {
                    *p += delta;
                }
            }
            PaintCmd::Path { points, .. } => {
                for p in points {
                    *p += delta;
                }
            }
            PaintCmd::Rect { rect, .. } => {
                *rect = rect.translate(delta);
            }
            PaintCmd::Text { pos, .. } => {
                *pos += delta;
            }
            PaintCmd::Triangles(triangles) => {
                triangles.translate(delta);
            }
        }
    }
}

/// Describes the width and color of a line.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Stroke {
    pub width: f32,
    pub color: Srgba,
}

impl Stroke {
    pub fn none() -> Self {
        Self::new(0.0, crate::color::TRANSPARENT)
    }

    pub fn new(width: impl Into<f32>, color: impl Into<Srgba>) -> Self {
        Self {
            width: width.into(),
            color: color.into(),
        }
    }
}

impl<Color> From<(f32, Color)> for Stroke
where
    Color: Into<Srgba>,
{
    fn from((width, color): (f32, Color)) -> Stroke {
        Stroke::new(width, color)
    }
}
