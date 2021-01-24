use crate::{math::*, util::History};
use std::collections::HashSet;

use crate::data::input::*;

pub use crate::data::input::Key;

/// If the pointer moves more than this, it is no longer a click (but maybe a drag)
const MAX_CLICK_DIST: f32 = 6.0; // TODO: move to settings
/// The new pointer press must come within this many seconds from previous pointer release
const MAX_CLICK_DELAY: f64 = 0.3; // TODO: move to settings

/// Input state that egui updates each frame.
#[derive(Clone, Debug)]
pub struct InputState {
    /// The raw input we got this frame
    pub raw: RawInput,

    pub pointer: PointerState,

    /// How many pixels the user scrolled
    pub scroll_delta: Vec2,

    /// Position and size of the egui area.
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

    // The keys that are currently being held down.
    pub keys_down: HashSet<Key>,

    /// In-order events received this frame
    pub events: Vec<Event>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            raw: Default::default(),
            pointer: Default::default(),
            scroll_delta: Default::default(),
            screen_rect: Rect::from_min_size(Default::default(), vec2(10_000.0, 10_000.0)),
            pixels_per_point: 1.0,
            time: 0.0,
            unstable_dt: 1.0 / 6.0,
            predicted_dt: 1.0 / 6.0,
            modifiers: Default::default(),
            keys_down: Default::default(),
            events: Default::default(),
        }
    }
}

/// Mouse or touch state.
#[derive(Clone, Debug)]
pub struct PointerState {
    /// Is the button currently down?
    /// true the frame when it is pressed,
    /// false the frame it is released.
    pub down: bool,

    /// The pointer button went from !down to down
    pub pressed: bool,

    /// The pointer button went from down to !down
    pub released: bool,

    /// If the pointer button is down, will it register as a click when released?
    /// Set to true on pointer button down, set to false when pointer moves too much.
    pub could_be_click: bool,

    /// Was there a click?
    /// Did a pointer button get released this frame closely after going down?
    pub click: bool,

    /// Was there a double-click?
    pub double_click: bool,

    /// When did the pointer get click last?
    /// Used to check for double-clicks.
    pub last_click_time: f64,

    /// Current position of the pointer in points.
    /// None for touch screens when finger is not down.
    pub pos: Option<Pos2>,

    /// Where did the current click/drag originate?
    pub press_origin: Option<Pos2>,

    /// How much the pointer moved compared to last frame, in points.
    pub delta: Vec2,

    /// Current velocity of pointer.
    pub velocity: Vec2,

    /// Recent movement of the pointer.
    /// Used for calculating velocity of pointer.
    pos_history: History<Pos2>,
}

impl Default for PointerState {
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
        let pointer = self.pointer.begin_frame(time, &new);
        let mut keys_down = self.keys_down;
        for event in &new.events {
            if let Event::Key { key, pressed, .. } = event {
                if *pressed {
                    keys_down.insert(*key);
                } else {
                    keys_down.remove(key);
                }
            }
        }
        InputState {
            pointer,
            scroll_delta: new.scroll_delta,
            screen_rect,
            pixels_per_point: new.pixels_per_point.unwrap_or(self.pixels_per_point),
            time,
            unstable_dt,
            predicted_dt: new.predicted_dt,
            modifiers: new.modifiers,
            keys_down,
            events: new.events.clone(), // TODO: remove clone() and use raw.events
            raw: new,
        }
    }

    pub fn screen_rect(&self) -> Rect {
        self.screen_rect
    }

    pub fn wants_repaint(&self) -> bool {
        self.pointer.pressed
            || self.pointer.released
            || self.pointer.delta != Vec2::zero()
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

    /// Is the given key currently held down?
    pub fn key_down(&self, desired_key: Key) -> bool {
        self.keys_down.contains(&desired_key)
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

impl PointerState {
    #[must_use]
    pub fn begin_frame(mut self, time: f64, new: &RawInput) -> PointerState {
        let delta = new
            .pointer_pos
            .and_then(|new| self.pos.map(|last| new - last))
            .unwrap_or_default();
        let pressed = !self.down && new.pointer_button_down;

        let released = self.down && !new.pointer_button_down;
        let click = released && self.could_be_click;
        let double_click = click && (time - self.last_click_time) < MAX_CLICK_DELAY;
        let mut press_origin = self.press_origin;
        let mut could_be_click = self.could_be_click;
        let mut last_click_time = self.last_click_time;
        if click {
            last_click_time = time
        }

        if pressed {
            press_origin = new.pointer_pos;
            could_be_click = true;
        } else if !self.down || self.pos.is_none() {
            press_origin = None;
        }

        if let (Some(press_origin), Some(pointer_pos)) = (new.pointer_pos, press_origin) {
            could_be_click &= press_origin.distance(pointer_pos) < MAX_CLICK_DIST;
        } else {
            could_be_click = false;
        }

        if pressed {
            // Start of a drag: we want to track the velocity for during the drag
            // and ignore any incoming movement
            self.pos_history.clear();
        }

        if let Some(pointer_pos) = new.pointer_pos {
            self.pos_history.add(time, pointer_pos);
        } else {
            // we do not clear the `pos_history` here, because it is exactly when a finger has
            // released from the touch screen that we may want to assign a velocity to whatever
            // the user tried to throw
        }

        self.pos_history.flush(time);
        let velocity = if self.pos_history.len() >= 3 && self.pos_history.duration() > 0.01 {
            self.pos_history.velocity().unwrap_or_default()
        } else {
            Vec2::default()
        };

        PointerState {
            down: new.pointer_button_down && new.pointer_pos.is_some(),
            pressed,
            released,
            could_be_click,
            click,
            double_click,
            last_click_time,
            pos: new.pointer_pos,
            press_origin,
            delta,
            velocity,
            pos_history: self.pos_history,
        }
    }

    pub fn is_moving(&self) -> bool {
        self.velocity != Vec2::zero()
    }
}

impl InputState {
    pub fn ui(&self, ui: &mut crate::Ui) {
        let Self {
            raw,
            pointer,
            scroll_delta,
            screen_rect,
            pixels_per_point,
            time,
            unstable_dt,
            predicted_dt,
            modifiers,
            keys_down,
            events,
        } = self;

        ui.style_mut().body_text_style = crate::paint::TextStyle::Monospace;
        ui.collapsing("Raw Input", |ui| raw.ui(ui));

        crate::containers::CollapsingHeader::new("🖱 Pointer")
            .default_open(true)
            .show(ui, |ui| {
                pointer.ui(ui);
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
        ui.label(format!("keys_down: {:?}", keys_down));
        ui.label(format!("events: {:?}", events))
            .on_hover_text("key presses etc");
    }
}

impl PointerState {
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
