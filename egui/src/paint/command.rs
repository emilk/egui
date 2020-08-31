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
        fill: Srgba,
        outline: LineStyle,
    },
    LineSegment {
        points: [Pos2; 2],
        style: LineStyle,
    },
    Path {
        path: Path,
        closed: bool,
        fill: Srgba,
        outline: LineStyle,
    },
    Rect {
        rect: Rect,
        corner_radius: f32,
        fill: Srgba,
        outline: LineStyle,
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
pub struct LineStyle {
    pub width: f32,
    pub color: Srgba,
}

impl LineStyle {
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

impl<Color> From<(f32, Color)> for LineStyle
where
    Color: Into<Srgba>,
{
    fn from((width, color): (f32, Color)) -> LineStyle {
        LineStyle::new(width, color)
    }
}
