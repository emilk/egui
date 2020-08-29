use {
    super::{font::Galley, fonts::TextStyle, Path, Srgba, Triangles},
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
        fill: Option<Srgba>,
        outline: Option<LineStyle>,
    },
    LineSegment {
        points: [Pos2; 2],
        style: LineStyle,
    },
    Path {
        path: Path,
        closed: bool,
        fill: Option<Srgba>,
        outline: Option<LineStyle>,
    },
    Rect {
        rect: Rect,
        corner_radius: f32,
        fill: Option<Srgba>,
        outline: Option<LineStyle>,
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

impl PaintCmd {
    pub fn line_segment(points: [Pos2; 2], width: f32, color: impl Into<Srgba>) -> Self {
        Self::LineSegment {
            points,
            style: LineStyle::new(width, color),
        }
    }

    pub fn circle_filled(center: Pos2, radius: f32, fill_color: impl Into<Srgba>) -> Self {
        Self::Circle {
            center,
            radius,
            fill: Some(fill_color.into()),
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
    pub color: Srgba,
}

impl LineStyle {
    pub fn new(width: impl Into<f32>, color: impl Into<Srgba>) -> Self {
        Self {
            width: width.into(),
            color: color.into(),
        }
    }
}
