use {
    super::{font::Galley, fonts::TextStyle, Color, Path, Triangles},
    crate::math::{Pos2, Rect},
};

use serde_derive::{Deserialize, Serialize};

// TODO: rename, e.g. `paint::Cmd`?
#[derive(Clone, Debug)]
pub enum PaintCmd {
    Circle {
        center: Pos2,
        fill_color: Option<Color>,
        outline: Option<Outline>,
        radius: f32,
    },
    LineSegment {
        points: [Pos2; 2],
        color: Color,
        width: f32,
    },
    // TODO: remove. Just have Path.
    LinePath {
        points: Vec<Pos2>,
        color: Color,
        width: f32,
    },
    Path {
        path: Path,
        closed: bool,
        fill_color: Option<Color>,
        outline: Option<Outline>,
    },
    Rect {
        rect: Rect,
        corner_radius: f32,
        fill_color: Option<Color>,
        outline: Option<Outline>,
    },
    /// Paint a single line of text
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
            color,
            width,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Outline {
    pub width: f32,
    pub color: Color,
}

impl Outline {
    pub fn new(width: impl Into<f32>, color: impl Into<Color>) -> Self {
        Self {
            width: width.into(),
            color: color.into(),
        }
    }
}
