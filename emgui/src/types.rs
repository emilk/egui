use crate::math::{Rect, Vec2};

// ----------------------------------------------------------------------------

/// What the integration gives to the gui.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct RawInput {
    /// Is the button currently down?
    pub mouse_down: bool,

    /// Current position of the mouse in points.
    pub mouse_pos: Vec2,

    /// Size of the screen in points.
    pub screen_size: Vec2,
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
    pub mouse_pos: Vec2,

    /// Size of the screen in points.
    pub screen_size: Vec2,
}

impl GuiInput {
    pub fn from_last_and_new(last: &RawInput, new: &RawInput) -> GuiInput {
        GuiInput {
            mouse_down: new.mouse_down,
            mouse_clicked: !last.mouse_down && new.mouse_down,
            mouse_released: last.mouse_down && !new.mouse_down,
            mouse_pos: new.mouse_pos,
            screen_size: new.screen_size,
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

pub fn srgba(r: u8, g: u8, b: u8, a: u8) -> Color {
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
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextStyle {
    Label,
}

#[derive(Clone, Debug, Serialize)]
pub enum GuiCmd {
    PaintCommands(Vec<PaintCmd>),
    /// The background for a button
    Button {
        interact: InteractInfo,
        rect: Rect,
    },
    Checkbox {
        checked: bool,
        interact: InteractInfo,
        rect: Rect,
    },
    /// The header button background for a foldable region
    FoldableHeader {
        interact: InteractInfo,
        open: bool,
        rect: Rect,
    },
    RadioButton {
        checked: bool,
        interact: InteractInfo,
        rect: Rect,
    },
    Slider {
        interact: InteractInfo,
        max: f32,
        min: f32,
        rect: Rect,
        value: f32,
    },
    /// Paint a single line of mono-space text.
    /// The text should start at the given position and flow to the right.
    /// The text should be vertically centered at the given position.
    Text {
        pos: Vec2,
        style: TextStyle,
        text: String,
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
    Clear {
        fill_color: Color,
    },
    Line {
        points: Vec<Vec2>,
        color: Color,
        width: f32,
    },
    Rect {
        corner_radius: f32,
        fill_color: Option<Color>,
        outline: Option<Outline>,
        pos: Vec2,
        size: Vec2,
    },
    /// Paint a single line of mono-space text.
    /// The text should start at the given position and flow to the right.
    /// The text should be vertically centered at the given position.
    Text {
        fill_color: Color,
        /// Name, e.g. Palatino
        font_name: String,
        /// Height in pixels, e.g. 12
        font_size: f32,
        pos: Vec2,
        text: String,
    },
}
