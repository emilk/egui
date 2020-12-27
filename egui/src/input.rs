//! The input needed by Egui.

use crate::{math::*, util::History};

/// If mouse moves more than this, it is no longer a click (but maybe a drag)
const MAX_CLICK_DIST: f32 = 6.0;
/// The new mouse press must come within this many seconds from previous mouse release
const MAX_CLICK_DELAY: f64 = 0.3;

/// What the integrations provides to Egui at the start of each frame.
///
/// Set the values that make sense, leave the rest at their `Default::default()`.
///
/// All coordinates are in points (logical pixels) with origin (0, 0) in the top left corner.
#[derive(Clone, Debug)]
pub struct RawInput {
    /// Is the button currently down?
    /// NOTE: Egui currently only supports the primary mouse button.
    pub mouse_down: bool,

    /// Current position of the mouse in points.
    pub mouse_pos: Option<Pos2>,

    /// How many points (logical pixels) the user scrolled
    pub scroll_delta: Vec2,

    #[deprecated = "Use screen_rect instead: `Some(Rect::from_pos_size(Default::default(), vec2(window_width, window_height)))`"]
    pub screen_size: Vec2,

    /// Position and size of the area that Egui should use.
    /// Usually you would set this to
    ///
    /// `Some(Rect::from_pos_size(Default::default(), vec2(window_width, window_height)))`.
    ///
    /// but you could also constrain Egui to some smaller portion of your window if you like.
    ///
    /// `None` will be treated as "same as last frame", with the default being a very big area.
    pub screen_rect: Option<Rect>,

    /// Also known as device pixel ratio, > 1 for HDPI screens.
    /// If text looks blurry on high resolution screens, you probably forgot to set this.
    pub pixels_per_point: Option<f32>,

    /// Monotonically increasing time, in seconds. Relative to whatever. Used for animations.
    /// If `None` is provided, Egui will assume a time delta of `predicted_dt` (default 1/60 seconds).
    pub time: Option<f64>,

    /// Should be set to the expected time between frames when painting at vsync speeds.
    /// The default for this is 1/60.
    /// Can safely be left at its default value.
    pub predicted_dt: f32,

    /// Which modifier keys are down at the start of the frame?
    pub modifiers: Modifiers,

    /// In-order events received this frame
    pub events: Vec<Event>,
}

impl Default for RawInput {
    fn default() -> Self {
        #![allow(deprecated)] // for screen_size
        Self {
            mouse_down: false,
            mouse_pos: None,
            scroll_delta: Vec2::zero(),
            screen_size: Default::default(),
            screen_rect: None,
            pixels_per_point: None,
            time: None,
            predicted_dt: 1.0 / 60.0,
            modifiers: Modifiers::default(),
            events: vec![],
        }
    }
}

impl RawInput {
    /// Helper: move volatile (deltas and events), clone the rest
    pub fn take(&mut self) -> RawInput {
        #![allow(deprecated)] // for screen_size
        RawInput {
            mouse_down: self.mouse_down,
            mouse_pos: self.mouse_pos,
            scroll_delta: std::mem::take(&mut self.scroll_delta),
            screen_size: self.screen_size,
            screen_rect: self.screen_rect,
            pixels_per_point: self.pixels_per_point,
            time: self.time,
            predicted_dt: self.predicted_dt,
            modifiers: self.modifiers,
            events: std::mem::take(&mut self.events),
        }
    }
}

/// Input state that Egui updates each frame.
#[derive(Clone, Debug)]
pub struct InputState {
    /// The raw input we got this frame
    pub raw: RawInput,

    pub mouse: MouseInput,

    /// How many pixels the user scrolled
    pub scroll_delta: Vec2,

    /// Position and size of the Egui area.
    pub screen_rect: Rect,

    /// Also known as device pixel ratio, > 1 for HDPI screens.
    pub pixels_per_point: f32,

    /// Time in seconds. Relative to whatever. Used for animation.
    pub time: f64,

    /// Time since last frame, in seconds.
    /// This can be very unstable in reactive mode (when we don't paint each frame).
    pub unstable_dt: f32,

    /// Used for animations to get instant feedback (avoid frame delay).
    /// Should be set to the expected time between frames when painting at vsync speeds.
    pub predicted_dt: f32,

    /// Which modifier keys are down at the start of the frame?
    pub modifiers: Modifiers,

    /// In-order events received this frame
    pub events: Vec<Event>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            raw: Default::default(),
            mouse: Default::default(),
            scroll_delta: Default::default(),
            screen_rect: Rect::from_min_size(Default::default(), vec2(10_000.0, 10_000.0)),
            pixels_per_point: 1.0,
            time: 0.0,
            unstable_dt: 1.0 / 6.0,
            predicted_dt: 1.0 / 6.0,
            modifiers: Default::default(),
            events: Default::default(),
        }
    }
}

/// Mouse (or touch) state.
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
    pos_history: History<Pos2>,
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
            pos_history: History::new(1000, 0.1),
        }
    }
}

/// An input event generated by the integration.
///
/// This only covers events that Egui cares about.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Event {
    /// The integration detected a "copy" event (e.g. Cmd+C).
    Copy,
    /// The integration detected a "cut" event (e.g. Cmd+X).
    Cut,
    /// Text input, e.g. via keyboard or paste action.
    ///
    /// When the user presses enter/return, do not send a `Text` (just [`Key::Enter`]).
    Text(String),
    Key {
        key: Key,
        pressed: bool,
        modifiers: Modifiers,
    },
}

/// State of the modifier keys. These must be fed to Egui.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Modifiers {
    /// Either of the alt keys are down (option ⌥ on Mac)
    pub alt: bool,
    /// Either of the control keys are down
    pub ctrl: bool,
    /// Either of the shift keys are down
    pub shift: bool,
    /// The Mac ⌘ Command key. Should always be set to `false` on other platforms.
    pub mac_cmd: bool,
    /// On Mac, this should be set whenever one of the ⌘ Command keys are down (same as `mac_cmd`).
    /// On Windows and Linux, set this to the same value as `ctrl`.
    /// This is so that Egui can, for instance, select all text by checking for `command + A`
    /// and it will work on both Mac and Windows.
    pub command: bool,
}

/// Keyboard key name. Only covers keys used by Egui (mostly for text editing).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Key {
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    Backspace,
    Delete,
    End,
    Enter,
    Space,
    Escape,
    Home,
    Insert,
    PageDown,
    PageUp,
    Tab,

    A, // Used for cmd+A (select All)
    K, // Used for ctrl+K (delete text after cursor)
    U, // Used for ctrl+U (delete text before cursor)
    W, // Used for ctrl+W (delete previous word)
    Z, // Used for cmd+Z (undo)
}

impl InputState {
    #[must_use]
    pub fn begin_frame(self, new: RawInput) -> InputState {
        #![allow(deprecated)] // for screen_size

        let time = new
            .time
            .unwrap_or_else(|| self.time + new.predicted_dt as f64);
        let unstable_dt = (time - self.time) as f32;
        let screen_rect = new.screen_rect.unwrap_or_else(|| {
            if new.screen_size != Default::default() {
                Rect::from_min_size(Default::default(), new.screen_size) // backwards compatability
            } else {
                self.screen_rect
            }
        });
        let mouse = self.mouse.begin_frame(time, &new);
        InputState {
            mouse,
            scroll_delta: new.scroll_delta,
            screen_rect,
            pixels_per_point: new.pixels_per_point.unwrap_or(self.pixels_per_point),
            time,
            unstable_dt,
            predicted_dt: new.predicted_dt,
            modifiers: new.modifiers,
            events: new.events.clone(), // TODO: remove clone() and use raw.events
            raw: new,
        }
    }

    pub fn screen_rect(&self) -> Rect {
        self.screen_rect
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
                    pressed: true,
                    ..
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
                    pressed: false,
                    ..
                } if *key == desired_key
            )
        })
    }

    /// Also known as device pixel ratio, > 1 for HDPI screens.
    pub fn pixels_per_point(&self) -> f32 {
        self.pixels_per_point
    }

    /// Size of a physical pixel in logical gui coordinates (points).
    pub fn physical_pixel_size(&self) -> f32 {
        1.0 / self.pixels_per_point()
    }

    /// How imprecise do we expect the mouse/touch input to be?
    /// Returns imprecision in points.
    pub fn aim_radius(&self) -> f32 {
        // TODO: multiply by ~3 for touch inputs because fingers are fat
        self.physical_pixel_size()
    }
}

impl MouseInput {
    #[must_use]
    pub fn begin_frame(mut self, time: f64, new: &RawInput) -> MouseInput {
        let delta = new
            .mouse_pos
            .and_then(|new| self.pos.map(|last| new - last))
            .unwrap_or_default();
        let pressed = !self.down && new.mouse_down;

        let released = self.down && !new.mouse_down;
        let click = released && self.could_be_click;
        let double_click = click && (time - self.last_click_time) < MAX_CLICK_DELAY;
        let mut press_origin = self.press_origin;
        let mut could_be_click = self.could_be_click;
        let mut last_click_time = self.last_click_time;
        if click {
            last_click_time = time
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

        if pressed {
            // Start of a drag: we want to track the velocity for during the drag
            // and ignore any incoming movement
            self.pos_history.clear();
        }

        if let Some(mouse_pos) = new.mouse_pos {
            self.pos_history.add(time, mouse_pos);
        } else {
            // we do not clear the `mouse_tracker` here, because it is exactly when a finger has
            // released from the touch screen that we may want to assign a velocity to whatever
            // the user tried to throw
        }

        self.pos_history.flush(time);
        let velocity = if self.pos_history.len() >= 3 && self.pos_history.duration() > 0.01 {
            self.pos_history.velocity().unwrap_or_default()
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
            pos_history: self.pos_history,
        }
    }
}

impl RawInput {
    pub fn ui(&self, ui: &mut crate::Ui) {
        #![allow(deprecated)] // for screen_size
        let Self {
            mouse_down,
            mouse_pos,
            scroll_delta,
            screen_size: _,
            screen_rect,
            pixels_per_point,
            time,
            predicted_dt,
            modifiers,
            events,
        } = self;

        // TODO: simpler way to show values, e.g. `ui.value("Mouse Pos:", self.mouse_pos);
        // TODO: `ui.style_mut().text_style = TextStyle::Monospace`;
        ui.label(format!("mouse_down: {}", mouse_down));
        ui.label(format!("mouse_pos: {:.1?}", mouse_pos));
        ui.label(format!("scroll_delta: {:?} points", scroll_delta));
        ui.label(format!("screen_rect: {:?} points", screen_rect));
        ui.label(format!("pixels_per_point: {:?}", pixels_per_point))
            .on_hover_text(
                "Also called HDPI factor.\nNumber of physical pixels per each logical pixel.",
            );
        if let Some(time) = time {
            ui.label(format!("time: {:.3} s", time));
        } else {
            ui.label("time: None");
        }
        ui.label(format!("predicted_dt: {:.1} ms", 1e3 * predicted_dt));
        ui.label(format!("modifiers: {:#?}", modifiers));
        ui.label(format!("events: {:?}", events))
            .on_hover_text("key presses etc");
    }
}

impl InputState {
    pub fn ui(&self, ui: &mut crate::Ui) {
        let Self {
            raw,
            mouse,
            scroll_delta,
            screen_rect,
            pixels_per_point,
            time,
            unstable_dt,
            predicted_dt,
            modifiers,
            events,
        } = self;

        ui.style_mut().body_text_style = crate::paint::TextStyle::Monospace;
        ui.collapsing("Raw Input", |ui| raw.ui(ui));

        crate::containers::CollapsingHeader::new("🖱 Mouse")
            .default_open(true)
            .show(ui, |ui| {
                mouse.ui(ui);
            });

        ui.label(format!("scroll_delta: {:?} points", scroll_delta));
        ui.label(format!("screen_rect: {:?} points", screen_rect));
        ui.label(format!(
            "{:?} physical pixels for each logical point",
            pixels_per_point
        ));
        ui.label(format!("time: {:.3} s", time));
        ui.label(format!(
            "time since previous frame: {:.1} ms",
            1e3 * unstable_dt
        ));
        ui.label(format!("predicted_dt: {:.1} ms", 1e3 * predicted_dt));
        ui.label(format!("modifiers: {:#?}", modifiers));
        ui.label(format!("events: {:?}", events))
            .on_hover_text("key presses etc");
    }
}

impl MouseInput {
    pub fn ui(&self, ui: &mut crate::Ui) {
        let Self {
            down,
            pressed,
            released,
            could_be_click,
            click,
            double_click,
            last_click_time,
            pos,
            press_origin,
            delta,
            velocity,
            pos_history: _,
        } = self;

        ui.label(format!("down: {}", down));
        ui.label(format!("pressed: {}", pressed));
        ui.label(format!("released: {}", released));
        ui.label(format!("could_be_click: {}", could_be_click));
        ui.label(format!("click: {}", click));
        ui.label(format!("double_click: {}", double_click));
        ui.label(format!("last_click_time: {:.3}", last_click_time));
        ui.label(format!("pos: {:?}", pos));
        ui.label(format!("press_origin: {:?}", press_origin));
        ui.label(format!("delta: {:?}", delta));
        ui.label(format!(
            "velocity: [{:3.0} {:3.0}] points/sec",
            velocity.x, velocity.y
        ));
    }
}
