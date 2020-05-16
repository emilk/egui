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
    pub pixels_per_point: f32,

    /// Time in seconds. Relative to whatever. Used for animation.
    pub time: f64,

    /// Local time. Only used for the clock in the example app.
    pub seconds_since_midnight: Option<f64>,

    /// Files has been dropped into the window.
    pub dropped_files: Vec<std::path::PathBuf>,

    /// Someone is threatening to drop these on us.
    pub hovered_files: Vec<std::path::PathBuf>,

    /// In-order events received this frame
    pub events: Vec<Event>,

    /// Web-only input
    pub web: Option<Web>,
}

/// What emigui maintains
#[derive(Clone, Debug, Default)]
pub struct GuiInput {
    // TODO: mouse: Mouse as separate
    //
    /// Is the button currently down?
    /// true the frame when it is pressed,
    /// false the frame it is released.
    pub mouse_down: bool,

    /// The mouse went from !down to down
    pub mouse_pressed: bool,

    /// The mouse went from down to !down
    pub mouse_released: bool,

    /// Current position of the mouse in points.
    /// None for touch screens when finger is not down.
    pub mouse_pos: Option<Pos2>,

    /// How much the mouse moved compared to last frame, in points.
    pub mouse_move: Vec2,

    /// Current velocity of mouse cursor, if any.
    pub mouse_velocity: Vec2,

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

    /// Files has been dropped into the window.
    pub dropped_files: Vec<std::path::PathBuf>,

    /// Someone is threatening to drop these on us.
    pub hovered_files: Vec<std::path::PathBuf>,

    /// In-order events received this frame
    pub events: Vec<Event>,

    /// Web-only input
    pub web: Option<Web>,
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
        let mouse_move = new
            .mouse_pos
            .and_then(|new| last.mouse_pos.map(|last| new - last))
            .unwrap_or_default();
        let dt = (new.time - last.time) as f32;
        let mut mouse_velocity = mouse_move / dt;
        if !mouse_velocity.is_finite() {
            mouse_velocity = Vec2::zero();
        }
        GuiInput {
            mouse_down: new.mouse_down && new.mouse_pos.is_some(),
            mouse_pressed: !last.mouse_down && new.mouse_down,
            mouse_released: last.mouse_down && !new.mouse_down,
            mouse_pos: new.mouse_pos,
            mouse_move,
            mouse_velocity,
            scroll_delta: new.scroll_delta,
            screen_size: new.screen_size,
            pixels_per_point: new.pixels_per_point,
            time: new.time,
            dt,
            seconds_since_midnight: new.seconds_since_midnight,
            dropped_files: new.dropped_files.clone(),
            hovered_files: new.hovered_files.clone(),
            events: new.events.clone(),
            web: new.web.clone(),
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
        ui.add(label!("pixels_per_point: {}", self.pixels_per_point));
        ui.add(label!("time: {:.3} s", self.time));
        ui.add(label!("events: {:?}", self.events));
        ui.add(label!("dropped_files: {:?}", self.dropped_files));
        ui.add(label!("hovered_files: {:?}", self.hovered_files));
        if let Some(web) = &self.web {
            web.ui(ui);
        }
    }
}

impl GuiInput {
    pub fn ui(&self, ui: &mut crate::Ui) {
        use crate::label;
        ui.add(label!("mouse_down: {}", self.mouse_down));
        ui.add(label!("mouse_pressed: {}", self.mouse_pressed));
        ui.add(label!("mouse_released: {}", self.mouse_released));
        ui.add(label!("mouse_pos: {:?}", self.mouse_pos));
        ui.add(label!("mouse_move: {:?}", self.mouse_move));
        ui.add(label!(
            "mouse_velocity: [{:3.0} {:3.0}] points/sec",
            self.mouse_velocity.x,
            self.mouse_velocity.y
        ));
        ui.add(label!("scroll_delta: {:?}", self.scroll_delta));
        ui.add(label!("screen_size: {:?}", self.screen_size));
        ui.add(label!("pixels_per_point: {}", self.pixels_per_point));
        ui.add(label!("time: {:.3} s", self.time));
        ui.add(label!("events: {:?}", self.events));
        ui.add(label!("dropped_files: {:?}", self.dropped_files));
        ui.add(label!("hovered_files: {:?}", self.hovered_files));
        if let Some(web) = &self.web {
            web.ui(ui);
        }
    }
}

impl Web {
    pub fn ui(&self, ui: &mut crate::Ui) {
        use crate::label;
        ui.add(label!("location: '{}'", self.location));
        ui.add(label!("location_hash: '{}'", self.location_hash));
    }
}
