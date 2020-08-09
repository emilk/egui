use crate::{math::*, movement_tracker::MovementTracker};

/// If mouse moves more than this, it is no longer a click (but maybe a drag)
const MAX_CLICK_DIST: f32 = 6.0;
/// The new mouse press must come within this many seconds from previous mouse release
const MAX_CLICK_DELAY: f64 = 0.3;

/// What the integration gives to the gui.
/// All coordinates in egui is in point/logical coordinates.
#[derive(Clone, Debug, Default)]
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

    /// Local time. Only used for the clock in the demo app.
    pub seconds_since_midnight: Option<f64>,

    /// In-order events received this frame
    pub events: Vec<Event>,
}

impl RawInput {
    /// Helper: move volatile (deltas and events), clone the rest
    pub fn take(&mut self) -> RawInput {
        RawInput {
            mouse_down: self.mouse_down,
            mouse_pos: self.mouse_pos,
            scroll_delta: std::mem::take(&mut self.scroll_delta),
            screen_size: self.screen_size,
            pixels_per_point: self.pixels_per_point,
            time: self.time,
            seconds_since_midnight: self.seconds_since_midnight,
            events: std::mem::take(&mut self.events),
        }
    }
}

/// What egui maintains
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
    /// This can be very unstable in reactive mode (when we don't paint each frame).
    pub unstable_dt: f32,

    /// Can be used to fast-forward to next frame for instance feedback. hacky.
    pub predicted_dt: f32,

    /// Local time. Only used for the clock in the demo app.
    pub seconds_since_midnight: Option<f64>,

    /// In-order events received this frame
    pub events: Vec<Event>,
}

/// What egui maintains
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

    /// If the mouse is down, will it register as a click when released?
    /// Set to true on mouse down, set to false when mouse moves too much.
    pub could_be_click: bool,

    /// Was there a click?
    /// Did a mouse button get released this frame closely after going down?
    pub click: bool,

    /// Was there a double-click?
    pub double_click: bool,

    /// When did the mouse get click last?
    /// Used to check for double-clicks.
    pub last_click_time: f64,

    /// Current position of the mouse in points.
    /// None for touch screens when finger is not down.
    pub pos: Option<Pos2>,

    /// Where did the current click/drag originate?
    pub press_origin: Option<Pos2>,

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
            could_be_click: false,
            click: false,
            double_click: false,
            last_click_time: std::f64::NEG_INFINITY,
            pos: None,
            press_origin: None,
            delta: Vec2::zero(),
            velocity: Vec2::zero(),
            pos_tracker: MovementTracker::new(1000, 0.1),
        }
    }
}

/// An input event. Only covers events used by Egui.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Event {
    Copy,
    Cut,
    /// Text input, e.g. via keyboard or paste action.
    /// Do not pass '\n', '\r' here, but send `Key::Enter` instead.
    Text(String),
    Key {
        key: Key,
        pressed: bool,
    },
}

/// Keyboard key name. Only covers keys used by Egui.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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
    /// Enter/Return key
    Enter,
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
        let unstable_dt = (new.time - self.raw.time) as f32;
        InputState {
            mouse,
            scroll_delta: new.scroll_delta,
            screen_size: new.screen_size,
            pixels_per_point: new.pixels_per_point.unwrap_or(1.0),
            time: new.time,
            unstable_dt,
            predicted_dt: 1.0 / 60.0, // TODO: remove this hack
            seconds_since_midnight: new.seconds_since_midnight,
            events: new.events.clone(), // TODO: remove clone() and use raw.events
            raw: new,
        }
    }

    pub fn wants_repaint(&self) -> bool {
        self.mouse.pressed
            || self.mouse.released
            || self.mouse.delta != Vec2::zero()
            || self.scroll_delta != Vec2::zero()
            || !self.events.is_empty()
    }

    /// Was the given key pressed this frame?
    pub fn key_pressed(&self, desired_key: Key) -> bool {
        self.events.iter().any(|event| {
            matches!(
                event,
                Event::Key {
                    key,
                    pressed: true
                } if *key == desired_key
            )
        })
    }

    /// Was the given key released this frame?
    pub fn key_released(&self, desired_key: Key) -> bool {
        self.events.iter().any(|event| {
            matches!(
                event,
                Event::Key {
                    key,
                    pressed: false
                } if *key == desired_key
            )
        })
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

        let released = self.down && !new.mouse_down;
        let click = released && self.could_be_click;
        let double_click = click && (new.time - self.last_click_time) < MAX_CLICK_DELAY;
        let mut press_origin = self.press_origin;
        let mut could_be_click = self.could_be_click;
        let mut last_click_time = self.last_click_time;
        if click {
            last_click_time = new.time
        }

        if pressed {
            press_origin = new.mouse_pos;
            could_be_click = true;
        } else if !self.down || self.pos.is_none() {
            press_origin = None;
        }

        if let (Some(press_origin), Some(mouse_pos)) = (new.mouse_pos, press_origin) {
            could_be_click &= press_origin.distance(mouse_pos) < MAX_CLICK_DIST;
        } else {
            could_be_click = false;
        }

        if self.pressed {
            // Start of a drag: we want to track the velocity for during the drag
            // and ignore any incoming movement
            self.pos_tracker.clear();
        }

        if let Some(mouse_pos) = new.mouse_pos {
            self.pos_tracker.add(new.time, mouse_pos);
        } else {
            // we do not clear the `mouse_tracker` here, because it is exactly when a finger has
            // released from the touch screen that we may want to assign a velocity to whatever
            // the user tried to throw
        }

        self.pos_tracker.flush(new.time);
        let velocity = if self.pos_tracker.len() >= 3 && self.pos_tracker.dt() > 0.01 {
            self.pos_tracker.velocity().unwrap_or_default()
        } else {
            Vec2::default()
        };

        MouseInput {
            down: new.mouse_down && new.mouse_pos.is_some(),
            pressed,
            released,
            could_be_click,
            click,
            double_click,
            last_click_time,
            pos: new.mouse_pos,
            press_origin,
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
        ui.add(label!("dt: {:.1} ms", 1e3 * self.unstable_dt));
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
        ui.add(label!("could_be_click: {}", self.could_be_click));
        ui.add(label!("click: {}", self.click));
        ui.add(label!("double_click: {}", self.double_click));
        ui.add(label!("last_click_time: {:.3}", self.last_click_time));
        ui.add(label!("pos: {:?}", self.pos));
        ui.add(label!("press_origin: {:?}", self.press_origin));
        ui.add(label!("delta: {:?}", self.delta));
        ui.add(label!(
            "velocity: [{:3.0} {:3.0}] points/sec",
            self.velocity.x,
            self.velocity.y
        ));
    }
}
