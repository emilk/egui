#[derive(Deserialize, Serialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Deserialize, Serialize)]
pub struct Rect {
    pub pos: Vec2,
    pub size: Vec2,
}

impl Rect {
    pub fn contains(&self, p: &Vec2) -> bool {
        self.pos.x <= p.x
            && p.x <= self.pos.x + self.size.x
            && self.pos.y <= p.y
            && p.y <= self.pos.y + self.size.y
    }
}

#[derive(Deserialize)]
pub struct Input {
    pub screen_size: Vec2,
    pub mouse_pos: Vec2,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    Start,
    Center,
    End,
}

#[derive(Serialize)]
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
