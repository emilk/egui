use serde_derive::Deserialize;

use crate::{math::*, movement_tracker::MovementTracker};

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
}

/// What emigui maintains
#[derive(Clone, Debug, Default)]
pub struct InputState {
    /// The raw input we got this fraem
    pub raw: RawInput,

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
}

/// What emigui maintains
#[derive(Clone, Debug)]
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

    /// Recent movement of the mouse.
    /// Used for calculating velocity of mouse pointer.
    pub pos_tracker: MovementTracker<Pos2>,
}

impl Default for MouseInput {
    fn default() -> Self {
        Self {
            down: false,
            pressed: false,
            released: false,
            pos: None,
            delta: Vec2::zero(),
            velocity: Vec2::zero(),
            pos_tracker: MovementTracker::new(1000, 0.1),
        }
    }
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

impl InputState {
    #[must_use]
    pub fn begin_frame(self, new: RawInput) -> InputState {
        let mouse = self.mouse.begin_frame(&new);
        let dt = (new.time - self.raw.time) as f32;
        InputState {
            mouse,
            scroll_delta: new.scroll_delta,
            screen_size: new.screen_size,
            pixels_per_point: new.pixels_per_point.unwrap_or(1.0),
            time: new.time,
            dt,
            seconds_since_midnight: new.seconds_since_midnight,
            events: new.events.clone(), // TODO: remove clone() and use raw.events
            raw: new,
        }
    }
}

impl MouseInput {
    #[must_use]
    pub fn begin_frame(mut self, new: &RawInput) -> MouseInput {
        let delta = new
            .mouse_pos
            .and_then(|new| self.pos.map(|last| new - last))
            .unwrap_or_default();
        let pressed = !self.down && new.mouse_down;

        if let Some(mouse_pos) = new.mouse_pos {
            self.pos_tracker.add(new.time, mouse_pos);
        } else {
            // we do not clear the `mouse_tracker` here, because it is exactly when a finger has
            // released from the touch screen that we may want to assign a velocity to whatever
            // the user tried to throw
        }

        let velocity = self.pos_tracker.velocity_noew(new.time).unwrap_or_default();

        MouseInput {
            down: new.mouse_down && new.mouse_pos.is_some(),
            pressed,
            released: self.down && !new.mouse_down,
            pos: new.mouse_pos,
            delta,
            velocity,
            pos_tracker: self.pos_tracker,
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
        ui.add(label!("scroll_delta: {:?} points", self.scroll_delta));
        ui.add(label!("screen_size: {:?} points", self.screen_size));
        ui.add(label!("pixels_per_point: {:?}", self.pixels_per_point))
            .tooltip_text(
                "Also called hdpi factor.\nNumber of physical pixels per each logical pixel.",
            );
        ui.add(label!("time: {:.3} s", self.time));
        ui.add(label!(
            "seconds_since_midnight: {:?} s",
            self.seconds_since_midnight
        ));
        ui.add(label!("events: {:?}", self.events))
            .tooltip_text("key presses etc");
    }
}

impl InputState {
    pub fn ui(&self, ui: &mut crate::Ui) {
        use crate::label;

        ui.collapsing("Raw Input", |ui| self.raw.ui(ui));

        crate::containers::CollapsingHeader::new("mouse")
            .default_open(true)
            .show(ui, |ui| {
                self.mouse.ui(ui);
            });

        ui.add(label!("scroll_delta: {:?} points", self.scroll_delta));
        ui.add(label!("screen_size: {:?} points", self.screen_size));
        ui.add(label!(
            "{} points for each physical pixel (hdpi factor)",
            self.pixels_per_point
        ));
        ui.add(label!("time: {:.3} s", self.time));
        ui.add(label!("dt: {:.3} s", self.dt));
        ui.add(label!(
            "seconds_since_midnight: {:?} s",
            self.seconds_since_midnight
        ));
        ui.add(label!("events: {:?}", self.events))
            .tooltip_text("key presses etc");
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
