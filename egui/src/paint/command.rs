use {
    super::{font::Galley, fonts::TextStyle, Color, Path, Triangles},
    crate::math::{Pos2, Rect},
};

// TODO: rename, e.g. `paint::Cmd`?
#[derive(Clone, Debug)]
pub enum PaintCmd {
    /// Paint nothing. This can be useful as a placeholder.
    Noop,
    Circle {
        center: Pos2,
        radius: f32,
        fill: Option<Color>,
        outline: Option<LineStyle>,
    },
    LineSegment {
        points: [Pos2; 2],
        style: LineStyle,
    },
    Path {
        path: Path,
        closed: bool,
        fill: Option<Color>,
        outline: Option<LineStyle>,
    },
    Rect {
        rect: Rect,
        corner_radius: f32,
        fill: Option<Color>,
        outline: Option<LineStyle>,
    },
    Text {
        /// Top left corner of the first character.
        pos: Pos2,
        /// The layed out text
        galley: Galley,
        text_style: TextStyle, // TODO: Font?
        color: Color,
    },
    Triangles(Triangles),
}

impl PaintCmd {
    pub fn line_segment(points: [Pos2; 2], color: Color, width: f32) -> Self {
        Self::LineSegment {
            points,
            style: LineStyle::new(width, color),
        }
    }

    pub fn circle_filled(center: Pos2, radius: f32, fill_color: Color) -> Self {
        Self::Circle {
            center,
            radius,
            fill: Some(fill_color),
            outline: None,
        }
    }

    pub fn circle_outline(center: Pos2, radius: f32, outline: LineStyle) -> Self {
        Self::Circle {
            center,
            radius,
            fill: None,
            outline: Some(outline),
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct LineStyle {
    pub width: f32,
    pub color: Color,
}

impl LineStyle {
    pub fn new(width: impl Into<f32>, color: impl Into<Color>) -> Self {
        Self {
            width: width.into(),
            color: color.into(),
        }
    }
}
