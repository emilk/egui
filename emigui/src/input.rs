use serde_derive::Deserialize;

use crate::math::*;

/// What the integration gives to the gui.
/// All coordinates in emigui is in point/logical coordinates.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct RawInput {
    /// Is the button currently down?
    pub mouse_down: bool,

    /// Current position of the mouse in points.
    pub mouse_pos: Option<Pos2>,

    /// How many pixels the user scrolled
    pub scroll_delta: Vec2,

    /// Size of the screen in points.
    /// TODO: this should be screen_rect for easy sandboxing.
    pub screen_size: Vec2,

    /// Also known as device pixel ratio, > 1 for HDPI screens.
    pub pixels_per_point: Option<f32>,

    /// Time in seconds. Relative to whatever. Used for animation.
    pub time: f64,

    /// Local time. Only used for the clock in the example app.
    pub seconds_since_midnight: Option<f64>,

    /// In-order events received this frame
    pub events: Vec<Event>,

    /// Web-only input
    pub web: Option<Web>,
}

/// What emigui maintains
#[derive(Clone, Debug, Default)]
pub struct GuiInput {
    pub mouse: MouseInput,

    /// How many pixels the user scrolled
    pub scroll_delta: Vec2,

    /// Size of the screen in points.
    pub screen_size: Vec2,

    /// Also known as device pixel ratio, > 1 for HDPI screens.
    pub pixels_per_point: f32,

    /// Time in seconds. Relative to whatever. Used for animation.
    pub time: f64,

    /// Time since last frame, in seconds.
    pub dt: f32,

    /// Local time. Only used for the clock in the example app.
    pub seconds_since_midnight: Option<f64>,

    /// In-order events received this frame
    pub events: Vec<Event>,

    /// Web-only input
    pub web: Option<Web>,
}

/// What emigui maintains
#[derive(Clone, Debug, Default)]
pub struct MouseInput {
    /// Is the button currently down?
    /// true the frame when it is pressed,
    /// false the frame it is released.
    pub down: bool,

    /// The mouse went from !down to down
    pub pressed: bool,

    /// The mouse went from down to !down
    pub released: bool,

    /// Current position of the mouse in points.
    /// None for touch screens when finger is not down.
    pub pos: Option<Pos2>,

    /// How much the mouse moved compared to last frame, in points.
    pub delta: Vec2,

    /// Current velocity of mouse cursor.
    pub velocity: Vec2,
}

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
#[serde(default)]
pub struct Web {
    pub location: String,
    /// i.e. "#fragment"
    pub location_hash: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    Copy,
    Cut,
    /// Text input, e.g. via keyboard or paste action
    Text(String),
    Key {
        key: Key,
        pressed: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    Alt,
    Backspace,
    Control,
    Delete,
    Down,
    End,
    Escape,
    Home,
    Insert,
    Left,
    /// Windows key or Mac Command key
    Logo,
    PageDown,
    PageUp,
    Return,
    Right,
    Shift,
    // Space,
    Tab,
    Up,
}

impl GuiInput {
    pub fn from_last_and_new(last: &RawInput, new: &RawInput) -> GuiInput {
        let dt = (new.time - last.time) as f32;
        GuiInput {
            mouse: MouseInput::from_last_and_new(last, new),
            scroll_delta: new.scroll_delta,
            screen_size: new.screen_size,
            pixels_per_point: new.pixels_per_point.unwrap_or(1.0),
            time: new.time,
            dt,
            seconds_since_midnight: new.seconds_since_midnight,
            events: new.events.clone(),
            web: new.web.clone(),
        }
    }
}

impl MouseInput {
    pub fn from_last_and_new(last: &RawInput, new: &RawInput) -> MouseInput {
        let delta = new
            .mouse_pos
            .and_then(|new| last.mouse_pos.map(|last| new - last))
            .unwrap_or_default();
        let dt = (new.time - last.time) as f32;
        let mut velocity = delta / dt;
        if !velocity.is_finite() {
            velocity = Vec2::zero();
        }
        MouseInput {
            down: new.mouse_down && new.mouse_pos.is_some(),
            pressed: !last.mouse_down && new.mouse_down,
            released: last.mouse_down && !new.mouse_down,
            pos: new.mouse_pos,
            delta,
            velocity,
        }
    }
}

impl RawInput {
    pub fn ui(&self, ui: &mut crate::Ui) {
        use crate::label;
        // TODO: simpler way to show values, e.g. `ui.value("Mouse Pos:", self.mouse_pos);
        // TODO: easily change default font!
        ui.add(label!("mouse_down: {}", self.mouse_down));
        ui.add(label!("mouse_pos: {:.1?}", self.mouse_pos));
        ui.add(label!("scroll_delta: {:?}", self.scroll_delta));
        ui.add(label!("screen_size: {:?}", self.screen_size));
        ui.add(label!("pixels_per_point: {:?}", self.pixels_per_point));
        ui.add(label!("time: {:.3} s", self.time));
        ui.add(label!("events: {:?}", self.events));
        if let Some(web) = &self.web {
            web.ui(ui);
        }
    }
}

impl GuiInput {
    pub fn ui(&self, ui: &mut crate::Ui) {
        use crate::label;
        crate::containers::CollapsingHeader::new("mouse")
            .default_open(true)
            .show(ui, |ui| {
                self.mouse.ui(ui);
            });
        ui.add(label!("scroll_delta: {:?}", self.scroll_delta));
        ui.add(label!("screen_size: {:?}", self.screen_size));
        ui.add(label!("pixels_per_point: {}", self.pixels_per_point));
        ui.add(label!("time: {:.3} s", self.time));
        ui.add(label!("events: {:?}", self.events));
        if let Some(web) = &self.web {
            web.ui(ui);
        }
    }
}

impl MouseInput {
    pub fn ui(&self, ui: &mut crate::Ui) {
        use crate::label;
        ui.add(label!("down: {}", self.down));
        ui.add(label!("pressed: {}", self.pressed));
        ui.add(label!("released: {}", self.released));
        ui.add(label!("pos: {:?}", self.pos));
        ui.add(label!("delta: {:?}", self.delta));
        ui.add(label!(
            "velocity: [{:3.0} {:3.0}] points/sec",
            self.velocity.x,
            self.velocity.y
        ));
    }
}

impl Web {
    pub fn ui(&self, ui: &mut crate::Ui) {
        use crate::label;
        ui.add(label!("location: '{}'", self.location));
        ui.add(label!("location_hash: '{}'", self.location_hash));
    }
}
