use {
    super::{font::Galley, fonts::TextStyle, Srgba, Triangles},
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

#[derive(Clone, Copy, Debug, Default)]
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
