use crate::data::input::*;
use crate::{emath::*, util::History};
use std::collections::HashSet;

pub use crate::data::input::Key;

/// If the pointer moves more than this, it is no longer a click (but maybe a drag)
const MAX_CLICK_DIST: f32 = 6.0; // TODO: move to settings
/// The new pointer press must come within this many seconds from previous pointer release
const MAX_CLICK_DELAY: f64 = 0.3; // TODO: move to settings

/// Input state that egui updates each frame.
#[derive(Clone, Debug)]
pub struct InputState {
    /// The raw input we got this frame from the backend.
    pub raw: RawInput,

    /// State of the mouse or touch.
    pub pointer: PointerState,

    /// How many pixels the user scrolled.
    pub scroll_delta: Vec2,

    /// Position and size of the egui area.
    pub screen_rect: Rect,

    /// Also known as device pixel ratio, > 1 for HDPI screens.
    pub pixels_per_point: f32,

    /// Time in seconds. Relative to whatever. Used for animation.
    pub time: f64,

    /// Time since last frame, in seconds.
    ///
    /// This can be very unstable in reactive mode (when we don't paint each frame)
    /// so it can be smart ot use e.g. `unstable_dt.min(1.0 / 30.0)`.
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
        self.pointer.wants_repaint() || self.scroll_delta != Vec2::ZERO || !self.events.is_empty()
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

// ----------------------------------------------------------------------------

/// A pointer (mouse or touch) click.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Click {
    pub pos: Pos2,
    pub button: PointerButton,
    /// 1 or 2 (double-click)
    pub count: u32,
    /// Allows you to check for e.g. shift-click
    pub modifiers: Modifiers,
}

impl Click {
    pub fn is_double(&self) -> bool {
        self.count == 2
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PointerEvent {
    Moved(Pos2),
    Pressed(Pos2),
    Released(Option<Click>),
}

impl PointerEvent {
    pub fn is_press(&self) -> bool {
        matches!(self, PointerEvent::Pressed(_))
    }
    pub fn is_release(&self) -> bool {
        matches!(self, PointerEvent::Released(_))
    }
    pub fn is_click(&self) -> bool {
        matches!(self, PointerEvent::Released(Some(_click)))
    }
}

/// Mouse or touch state.
#[derive(Clone, Debug)]
pub struct PointerState {
    // Consider a finger tapping a touch screen.
    // What position should we report?
    // The location of the touch, or `None`, because the finger is gone?
    //
    // For some cases we want the first: e.g. to check for interaction.
    // For showing tooltips, we want the latter (no tooltips, since there are no fingers).
    /// Latest reported pointer position.
    /// When tapping a touch screen, this will be `None`.
    latest_pos: Option<Pos2>,

    /// Latest position of the mouse, but ignoring any [`Event::PointerGone`]
    /// if there were interactions this frame.
    /// When tapping a touch screen, this will be the location of the touch.
    interact_pos: Option<Pos2>,

    /// How much the pointer moved compared to last frame, in points.
    delta: Vec2,

    /// Current velocity of pointer.
    velocity: Vec2,

    /// Recent movement of the pointer.
    /// Used for calculating velocity of pointer.
    pos_history: History<Pos2>,

    down: [bool; NUM_POINTER_BUTTONS],

    /// Where did the current click/drag originate?
    /// `None` if no mouse button is down.
    press_origin: Option<Pos2>,

    /// If the pointer button is down, will it register as a click when released?
    /// Set to true on pointer button down, set to false when pointer button moves too much.
    could_be_click: bool,

    /// When did the pointer get click last?
    /// Used to check for double-clicks.
    last_click_time: f64,

    // /// All clicks that occurred this frame
    // clicks: Vec<Click>,
    /// All button events that occurred this frame
    pub(crate) pointer_events: Vec<PointerEvent>,
}

impl Default for PointerState {
    fn default() -> Self {
        Self {
            latest_pos: None,
            interact_pos: None,
            delta: Vec2::ZERO,
            velocity: Vec2::ZERO,
            pos_history: History::new(1000, 0.1),
            down: Default::default(),
            press_origin: None,
            could_be_click: false,
            last_click_time: std::f64::NEG_INFINITY,
            pointer_events: vec![],
        }
    }
}

impl PointerState {
    #[must_use]
    pub(crate) fn begin_frame(mut self, time: f64, new: &RawInput) -> PointerState {
        self.pointer_events.clear();

        let old_pos = self.latest_pos;
        self.interact_pos = self.latest_pos;

        for event in &new.events {
            match event {
                Event::PointerMoved(pos) => {
                    let pos = *pos;

                    self.latest_pos = Some(pos);
                    self.interact_pos = Some(pos);

                    if let Some(press_origin) = &mut self.press_origin {
                        self.could_be_click &= press_origin.distance(pos) < MAX_CLICK_DIST;
                    } else {
                        self.could_be_click = false;
                    }

                    self.pointer_events.push(PointerEvent::Moved(pos));
                }
                Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers,
                } => {
                    let pos = *pos;
                    let button = *button;
                    let pressed = *pressed;
                    let modifiers = *modifiers;

                    self.latest_pos = Some(pos);
                    self.interact_pos = Some(pos);

                    if pressed {
                        // Start of a drag: we want to track the velocity for during the drag
                        // and ignore any incoming movement
                        self.pos_history.clear();
                    }

                    if pressed {
                        self.press_origin = Some(pos);
                        self.could_be_click = true;
                        self.pointer_events.push(PointerEvent::Pressed(pos));
                    } else {
                        let clicked = self.could_be_click;

                        let click = if clicked {
                            let double_click = (time - self.last_click_time) < MAX_CLICK_DELAY;
                            let count = if double_click { 2 } else { 1 };

                            self.last_click_time = time;

                            Some(Click {
                                pos,
                                button,
                                count,
                                modifiers,
                            })
                        } else {
                            None
                        };

                        self.pointer_events.push(PointerEvent::Released(click));

                        self.press_origin = None;
                        self.could_be_click = false;
                    }

                    self.down[button as usize] = pressed;
                }
                Event::PointerGone => {
                    self.latest_pos = None;
                    // NOTE: we do NOT clear `self.interact_pos` here. It will be cleared next frame.
                }
                _ => {}
            }
        }

        self.delta = if let (Some(old_pos), Some(new_pos)) = (old_pos, self.latest_pos) {
            new_pos - old_pos
        } else {
            Vec2::ZERO
        };

        if let Some(pos) = self.latest_pos {
            self.pos_history.add(time, pos);
        } else {
            // we do not clear the `pos_history` here, because it is exactly when a finger has
            // released from the touch screen that we may want to assign a velocity to whatever
            // the user tried to throw.
        }

        self.pos_history.flush(time);

        self.velocity = if self.pos_history.len() >= 3 && self.pos_history.duration() > 0.01 {
            self.pos_history.velocity().unwrap_or_default()
        } else {
            Vec2::default()
        };

        self
    }

    fn wants_repaint(&self) -> bool {
        !self.pointer_events.is_empty() || self.delta != Vec2::ZERO
    }

    /// How much the pointer moved compared to last frame, in points.
    pub fn delta(&self) -> Vec2 {
        self.delta
    }

    /// Current velocity of pointer.
    pub fn velocity(&self) -> Vec2 {
        self.velocity
    }

    /// Where did the current click/drag originate?
    /// `None` if no mouse button is down.
    pub fn press_origin(&self) -> Option<Pos2> {
        self.press_origin
    }

    /// Latest reported pointer position.
    /// When tapping a touch screen, this will be `None`.
    pub(crate) fn latest_pos(&self) -> Option<Pos2> {
        self.latest_pos
    }

    /// If it is a good idea to show a tooltip, where is pointer?
    pub fn tooltip_pos(&self) -> Option<Pos2> {
        self.latest_pos
    }

    /// If you detect a click or drag and wants to know where it happened, use this.
    ///
    /// Latest position of the mouse, but ignoring any [`Event::PointerGone`]
    /// if there were interactions this frame.
    /// When tapping a touch screen, this will be the location of the touch.
    pub fn interact_pos(&self) -> Option<Pos2> {
        self.interact_pos
    }

    /// Do we have a pointer?
    ///
    /// `false` if the mouse is not over the egui area, or if no touches are down on touch screens.
    pub fn has_pointer(&self) -> bool {
        self.latest_pos.is_some()
    }

    /// Is the pointer currently moving?
    /// This is smoothed so a few frames of stillness is required before this returns `true`.
    pub fn is_moving(&self) -> bool {
        self.velocity != Vec2::ZERO
    }

    /// Was any pointer button pressed (`!down -> down`) this frame?
    /// This can sometimes return `true` even if `any_down() == false`
    /// because a press can be shorted than one frame.
    pub fn any_pressed(&self) -> bool {
        self.pointer_events.iter().any(|event| event.is_press())
    }

    /// Was any pointer button released (`down -> !down`) this frame?
    pub fn any_released(&self) -> bool {
        self.pointer_events.iter().any(|event| event.is_release())
    }

    /// Is any pointer button currently down?
    pub fn any_down(&self) -> bool {
        self.down.iter().any(|&down| down)
    }

    /// Were there any type of click this frame?
    pub fn any_click(&self) -> bool {
        self.pointer_events.iter().any(|event| event.is_click())
    }

    // /// Was this button pressed (`!down -> down`) this frame?
    // /// This can sometimes return `true` even if `any_down() == false`
    // /// because a press can be shorted than one frame.
    // pub fn button_pressed(&self, button: PointerButton) -> bool {
    //     self.pointer_events.iter().any(|event| event.is_press())
    // }

    // /// Was this button released (`down -> !down`) this frame?
    // pub fn button_released(&self, button: PointerButton) -> bool {
    //     self.pointer_events.iter().any(|event| event.is_release())
    // }

    /// Is this button currently down?
    pub fn button_down(&self, button: PointerButton) -> bool {
        self.down[button as usize]
    }

    pub(crate) fn could_any_button_be_click(&self) -> bool {
        self.could_be_click
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

        ui.style_mut().body_text_style = epaint::TextStyle::Monospace;
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
            latest_pos,
            interact_pos,
            delta,
            velocity,
            pos_history: _,
            down,
            press_origin,
            could_be_click,
            last_click_time,
            pointer_events,
        } = self;

        ui.label(format!("latest_pos: {:?}", latest_pos));
        ui.label(format!("interact_pos: {:?}", interact_pos));
        ui.label(format!("delta: {:?}", delta));
        ui.label(format!(
            "velocity: [{:3.0} {:3.0}] points/sec",
            velocity.x, velocity.y
        ));
        ui.label(format!("down: {:#?}", down));
        ui.label(format!("press_origin: {:?}", press_origin));
        ui.label(format!("could_be_click: {:#?}", could_be_click));
        ui.label(format!("last_click_time: {:#?}", last_click_time));
        ui.label(format!("pointer_events: {:?}", pointer_events));
    }
}
