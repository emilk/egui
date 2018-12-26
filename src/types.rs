#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct Rect {
    pub pos: Vec2,
    pub size: Vec2,
}

impl Rect {
    pub fn contains(&self, p: Vec2) -> bool {
        self.pos.x <= p.x
            && p.x <= self.pos.x + self.size.x
            && self.pos.y <= p.y
            && p.y <= self.pos.y + self.size.y
    }

    pub fn center(&self) -> Vec2 {
        Vec2 {
            x: self.pos.x + self.size.x / 2.0,
            y: self.pos.y + self.size.y / 2.0,
        }
    }
}

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
#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    Start, // Test with arabic text
    Center,
    End,
}

#[derive(Clone, Debug, Serialize)] // TODOcopy
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum PaintCmd {
    Clear {
        fill_style: String,
    },
    RoundedRect {
        fill_style: String,
        pos: Vec2,
        size: Vec2,
        corner_radius: f32,
    },
    Text {
        fill_style: String,
        font: String,
        pos: Vec2,
        text: String,
        text_align: TextAlign,
    },
}
