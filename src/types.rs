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

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct InteractInfo {
    pub hovered: bool,

    /// The mouse went got pressed on this thing this frame
    pub clicked: bool,

    /// The mouse is interacting with this thing (e.g. dragging it)
    pub active: bool,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    Start, // Test with arabic text
    Center,
    End,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextStyle {
    Label,
}

#[derive(Clone, Debug, Serialize)]
pub enum GuiCmd {
    PaintCommands(Vec<PaintCmd>),
    Button {
        interact: InteractInfo,
        rect: Rect,
        text: String,
    },
    Checkbox {
        checked: bool,
        interact: InteractInfo,
        rect: Rect,
        text: String,
    },
    RadioButton {
        checked: bool,
        interact: InteractInfo,
        rect: Rect,
        text: String,
    },
    Slider {
        interact: InteractInfo,
        label: String,
        max: f32,
        min: f32,
        rect: Rect,
        value: f32,
    },
    Text {
        pos: Vec2,
        style: TextStyle,
        text: String,
        text_align: TextAlign,
    },
}

// ----------------------------------------------------------------------------

pub type Style = String;

#[derive(Clone, Debug, Serialize)]
pub struct Outline {
    pub width: f32,
    pub style: Style,
}

#[derive(Clone, Debug, Serialize)] // TODO: copy
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PaintCmd {
    Circle {
        center: Vec2,
        fill_style: Option<Style>,
        outline: Option<Outline>,
        radius: f32,
    },
    Clear {
        fill_style: Style,
    },
    Line {
        points: Vec<Vec2>,
        style: Style,
        width: f32,
    },
    Rect {
        corner_radius: f32,
        fill_style: Option<Style>,
        outline: Option<Outline>,
        pos: Vec2,
        size: Vec2,
    },
    Text {
        fill_style: Style,
        font: String,
        pos: Vec2,
        text: String,
        text_align: TextAlign,
    },
}
