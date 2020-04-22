use crate::{
    color::Color,
    fonts::TextStyle,
    math::{Pos2, Rect, Vec2},
    mesher::{Mesh, Path},
};

// ----------------------------------------------------------------------------

/// What the integration gives to the gui.
/// All coordinates in emigui is in point/logical coordinates.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default)]
pub struct RawInput {
    /// Is the button currently down?
    pub mouse_down: bool,

    /// Current position of the mouse in points.
    pub mouse_pos: Option<Pos2>,

    /// How many pixels the user scrolled
    pub scroll_delta: Vec2,

    /// Size of the screen in points.
    pub screen_size: Vec2,

    /// Also known as device pixel ratio, > 1 for HDPI screens.
    pub pixels_per_point: f32,

    /// Time in seconds. Relative to whatever. Used for animation.
    pub time: f64,
}

/// What the gui maintains
#[derive(Clone, Copy, Debug, Default)]
pub struct GuiInput {
    /// Is the button currently down?
    /// true the frame when it is pressed,
    /// false the frame it is released.
    pub mouse_down: bool,

    /// The mouse went from !down to down
    pub mouse_pressed: bool,

    /// The mouse went from down to !down
    pub mouse_released: bool,

    /// Current position of the mouse in points.
    pub mouse_pos: Option<Pos2>,

    /// How much the mouse moved compared to last frame, in points.
    pub mouse_move: Vec2,

    /// How many pixels the user scrolled
    pub scroll_delta: Vec2,

    /// Size of the screen in points.
    pub screen_size: Vec2,

    /// Also known as device pixel ratio, > 1 for HDPI screens.
    pub pixels_per_point: f32,

    /// Time in seconds. Relative to whatever. Used for animation.
    pub time: f64,
}

impl GuiInput {
    pub fn from_last_and_new(last: &RawInput, new: &RawInput) -> GuiInput {
        let mouse_move = new
            .mouse_pos
            .and_then(|new| last.mouse_pos.map(|last| new - last))
            .unwrap_or_default();
        GuiInput {
            mouse_down: new.mouse_down,
            mouse_pressed: !last.mouse_down && new.mouse_down,
            mouse_released: last.mouse_down && !new.mouse_down,
            mouse_pos: new.mouse_pos,
            mouse_move,
            scroll_delta: new.scroll_delta,
            screen_size: new.screen_size,
            pixels_per_point: new.pixels_per_point,
            time: new.time,
        }
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

#[derive(Clone, Debug, Serialize)]
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
        text_style: TextStyle,
        /// Start each character in the text, as offset from pos.
        x_offsets: Vec<f32>,
        // TODO: font info
    },
    /// Low-level triangle mesh
    Mesh(Mesh),
}
