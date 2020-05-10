use serde_derive::{Deserialize, Serialize};

use crate::{
    color::Color,
    fonts::TextStyle,
    math::{Pos2, Rect},
    mesher::{Mesh, Path},
};

// ----------------------------------------------------------------------------

#[derive(Clone, Default, Serialize)]
pub struct Output {
    pub cursor_icon: CursorIcon,

    /// If set, open this url.
    pub open_url: Option<String>,

    /// Response to Event::Copy or Event::Cut. Ignore if empty.
    pub copied_text: String,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorIcon {
    Default,
    /// Pointing hand, used for e.g. web links
    PointingHand,
    ResizeNwSe,
    Text,
}

impl Default for CursorIcon {
    fn default() -> Self {
        Self::Default
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct InteractInfo {
    /// The mouse is hovering above this thing
    pub hovered: bool,

    /// The mouse pressed this thing ealier, and now released on this thing too.
    pub clicked: bool,

    /// The mouse is interacting with this thing (e.g. dragging it or holding it)
    pub active: bool,

    /// The region of the screen we are talking about
    pub rect: Rect,
}

// ----------------------------------------------------------------------------

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

#[derive(Clone, Debug)]
pub enum PaintCmd {
    Circle {
        center: Pos2,
        fill_color: Option<Color>,
        outline: Option<Outline>,
        radius: f32,
    },
    Line {
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
        color: Color,
        /// Top left corner of the first character.
        pos: Pos2,
        text: String,
        text_style: TextStyle, // TODO: Font
        /// Start each character in the text, as offset from pos.
        x_offsets: Vec<f32>,
        // TODO: font info
    },
    /// Low-level triangle mesh
    Mesh(Mesh),
}

impl PaintCmd {
    pub fn line_segment(seg: (Pos2, Pos2), color: Color, width: f32) -> Self {
        Self::Line {
            points: vec![seg.0, seg.1],
            color,
            width,
        }
    }
}
