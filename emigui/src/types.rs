use crate::{
    fonts::TextStyle,
    math::{Rect, Vec2},
    mesher::Mesh,
};

// ----------------------------------------------------------------------------

/// What the integration gives to the gui.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct RawInput {
    /// Is the button currently down?
    pub mouse_down: bool,

    /// Current position of the mouse in points.
    pub mouse_pos: Option<Vec2>,

    /// Size of the screen in points.
    pub screen_size: Vec2,

    /// Also known as device pixel ratio, > 1 for HDPI screens.
    pub pixels_per_point: f32,
}

/// What the gui maintains
#[derive(Clone, Copy, Debug, Default)]
pub struct GuiInput {
    /// Is the button currently down?
    pub mouse_down: bool,

    /// The mouse went from !down to down
    pub mouse_clicked: bool,

    /// The mouse went from down to !down
    pub mouse_released: bool,

    /// Current position of the mouse in points.
    pub mouse_pos: Option<Vec2>,

    /// Size of the screen in points.
    pub screen_size: Vec2,

    /// Also known as device pixel ratio, > 1 for HDPI screens.
    pub pixels_per_point: f32,
}

impl GuiInput {
    pub fn from_last_and_new(last: &RawInput, new: &RawInput) -> GuiInput {
        GuiInput {
            mouse_down: new.mouse_down,
            mouse_clicked: !last.mouse_down && new.mouse_down,
            mouse_released: last.mouse_down && !new.mouse_down,
            mouse_pos: new.mouse_pos,
            screen_size: new.screen_size,
            pixels_per_point: new.pixels_per_point,
        }
    }
}

// ----------------------------------------------------------------------------

/// 0-255 sRGBA
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const WHITE: Color = srgba(255, 255, 255, 255);

    pub fn transparent(self) -> Color {
        Color {
            r: self.r,
            g: self.g,
            b: self.b,
            a: 0,
        }
    }
}

pub const fn srgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color { r, g, b, a }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct InteractInfo {
    /// The mouse is hovering above this
    pub hovered: bool,

    /// The mouse went got pressed on this thing this frame
    pub clicked: bool,

    /// The mouse is interacting with this thing (e.g. dragging it)
    pub active: bool,

    /// The region of the screen we are talking about
    pub rect: Rect,
}

#[derive(Clone, Debug, Serialize)]
pub enum GuiCmd {
    PaintCommands(Vec<PaintCmd>),
    /// The background for a button
    Button {
        interact: InteractInfo,
    },
    Checkbox {
        checked: bool,
        interact: InteractInfo,
    },
    /// The header button background for a foldable region
    FoldableHeader {
        interact: InteractInfo,
        open: bool,
    },
    RadioButton {
        checked: bool,
        interact: InteractInfo,
    },
    Slider {
        interact: InteractInfo,
        max: f32,
        min: f32,
        value: f32,
    },
    /// A string of text with a position for each character.
    Text {
        color: Option<Color>,
        pos: Vec2,
        text: String,
        text_style: TextStyle,
        /// Start each character in the text, as offset from pos.
        x_offsets: Vec<f32>,
    },
    /// Background of e.g. a popup
    Window {
        rect: Rect,
    },
}

// ----------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct Outline {
    pub width: f32,
    pub color: Color,
}

#[derive(Clone, Debug, Serialize)] // TODO: copy
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PaintCmd {
    Circle {
        center: Vec2,
        fill_color: Option<Color>,
        outline: Option<Outline>,
        radius: f32,
    },
    Mesh(Mesh),
    Line {
        points: Vec<Vec2>,
        color: Color,
        width: f32,
    },
    Rect {
        corner_radius: f32,
        fill_color: Option<Color>,
        outline: Option<Outline>,
        rect: Rect,
    },
    /// Paint a single line of text
    Text {
        color: Color,
        /// Top left corner of the first character.
        pos: Vec2,
        text: String,
        text_style: TextStyle,
        /// Start each character in the text, as offset from pos.
        x_offsets: Vec<f32>,
        // TODO: font info
    },
}
