mod touch_state;

use crate::data::input::*;
use crate::{emath::*, util::History};
use std::collections::{BTreeMap, HashSet};

pub use crate::data::input::Key;
pub use touch_state::MultiTouchInfo;
use touch_state::TouchState;

/// If the pointer moves more than this, it won't become a click (but it is still a drag)
const MAX_CLICK_DIST: f32 = 6.0; // TODO(emilk): move to settings

/// If the pointer is down for longer than this, it won't become a click (but it is still a drag)
const MAX_CLICK_DURATION: f64 = 0.6; // TODO(emilk): move to settings

/// The new pointer press must come within this many seconds from previous pointer release
const MAX_DOUBLE_CLICK_DELAY: f64 = 0.3; // TODO(emilk): move to settings

/// Input state that egui updates each frame.
///
/// You can check if `egui` is using the inputs using
/// [`crate::Context::wants_pointer_input`] and [`crate::Context::wants_keyboard_input`].
#[derive(Clone, Debug)]
pub struct InputState {
    /// The raw input we got this frame from the backend.
    pub raw: RawInput,

    /// State of the mouse or simple touch gestures which can be mapped to mouse operations.
    pub pointer: PointerState,

    /// State of touches, except those covered by PointerState (like clicks and drags).
    /// (We keep a separate [`TouchState`] for each encountered touch device.)
    touch_states: BTreeMap<TouchDeviceId, TouchState>,

    /// How many points the user scrolled.
    ///
    /// The delta dictates how the _content_ should move.
    ///
    /// A positive X-value indicates the content is being moved right,
    /// as when swiping right on a touch-screen or track-pad with natural scrolling.
    ///
    /// A positive Y-value indicates the content is being moved down,
    /// as when swiping down on a touch-screen or track-pad with natural scrolling.
    pub scroll_delta: Vec2,

    /// Zoom scale factor this frame (e.g. from ctrl-scroll or pinch gesture).
    ///
    /// * `zoom = 1`: no change.
    /// * `zoom < 1`: pinch together
    /// * `zoom > 1`: pinch spread
    zoom_factor_delta: f32,

    /// Position and size of the egui area.
    pub screen_rect: Rect,

    /// Also known as device pixel ratio, > 1 for high resolution screens.
    pub pixels_per_point: f32,

    /// Maximum size of one side of a texture.
    ///
    /// This depends on the backend.
    pub max_texture_side: usize,

    /// Time in seconds. Relative to whatever. Used for animation.
    pub time: f64,

    /// Time since last frame, in seconds.
    ///
    /// This can be very unstable in reactive mode (when we don't paint each frame).
    /// For animations it is therefore better to use [`Self::stable_dt`].
    pub unstable_dt: f32,

    /// Estimated time until next frame (provided we repaint right away).
    ///
    /// Used for animations to get instant feedback (avoid frame delay).
    /// Should be set to the expected time between frames when painting at vsync speeds.
    ///
    /// On most integrations this has a fixed value of `1.0 / 60.0`, so it is not a very accurate estimate.
    pub predicted_dt: f32,

    /// Time since last frame (in seconds), but gracefully handles the first frame after sleeping in reactive mode.
    ///
    /// In reactive mode (available in e.g. `eframe`), `egui` only updates when there is new input
    /// or something is animating.
    /// This can lead to large gaps of time (sleep), leading to large [`Self::unstable_dt`].
    ///
    /// If `egui` requested a repaint the previous frame, then `egui` will use
    /// `stable_dt = unstable_dt;`, but if `egui` did not not request a repaint last frame,
    /// then `egui` will assume `unstable_dt` is too large, and will use
    /// `stable_dt = predicted_dt;`.
    ///
    /// This means that for the first frame after a sleep,
    /// `stable_dt` will be a prediction of the delta-time until the next frame,
    /// and in all other situations this will be an accurate measurement of time passed
    /// since the previous frame.
    ///
    /// Note that a frame can still stall for various reasons, so `stable_dt` can
    /// still be unusually large in some situations.
    ///
    /// When animating something, it is recommended that you use something like
    /// `stable_dt.min(0.1)` - this will give you smooth animations when the framerate is good
    /// (even in reactive mode), but will avoid large jumps when framerate is bad,
    /// and will effectively slow down the animation when FPS drops below 10.
    pub stable_dt: f32,

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
            touch_states: Default::default(),
            scroll_delta: Vec2::ZERO,
            zoom_factor_delta: 1.0,
            screen_rect: Rect::from_min_size(Default::default(), vec2(10_000.0, 10_000.0)),
            pixels_per_point: 1.0,
            max_texture_side: 2048,
            time: 0.0,
            unstable_dt: 1.0 / 60.0,
            predicted_dt: 1.0 / 60.0,
            stable_dt: 1.0 / 60.0,
            modifiers: Default::default(),
            keys_down: Default::default(),
            events: Default::default(),
        }
    }
}

impl InputState {
    #[must_use]
    pub fn begin_frame(
        mut self,
        mut new: RawInput,
        requested_repaint_last_frame: bool,
    ) -> InputState {
        let time = new.time.unwrap_or(self.time + new.predicted_dt as f64);
        let unstable_dt = (time - self.time) as f32;

        let stable_dt = if requested_repaint_last_frame {
            // we should have had a repaint straight away,
            // so this should be trustable.
            unstable_dt
        } else {
            new.predicted_dt
        };

        let screen_rect = new.screen_rect.unwrap_or(self.screen_rect);
        self.create_touch_states_for_new_devices(&new.events);
        for touch_state in self.touch_states.values_mut() {
            touch_state.begin_frame(time, &new, self.pointer.interact_pos);
        }
        let pointer = self.pointer.begin_frame(time, &new);

        let mut keys_down = self.keys_down;
        let mut scroll_delta = Vec2::ZERO;
        let mut zoom_factor_delta = 1.0;
        for event in &mut new.events {
            match event {
                Event::Key {
                    key,
                    pressed,
                    repeat,
                    ..
                } => {
                    if *pressed {
                        let first_press = keys_down.insert(*key);
                        *repeat = !first_press;
                    } else {
                        keys_down.remove(key);
                    }
                }
                Event::Scroll(delta) => {
                    scroll_delta += *delta;
                }
                Event::Zoom(factor) => {
                    zoom_factor_delta *= *factor;
                }
                _ => {}
            }
        }

        InputState {
            pointer,
            touch_states: self.touch_states,
            scroll_delta,
            zoom_factor_delta,
            screen_rect,
            pixels_per_point: new.pixels_per_point.unwrap_or(self.pixels_per_point),
            max_texture_side: new.max_texture_side.unwrap_or(self.max_texture_side),
            time,
            unstable_dt,
            predicted_dt: new.predicted_dt,
            stable_dt,
            modifiers: new.modifiers,
            keys_down,
            events: new.events.clone(), // TODO(emilk): remove clone() and use raw.events
            raw: new,
        }
    }

    #[inline(always)]
    pub fn screen_rect(&self) -> Rect {
        self.screen_rect
    }

    /// Zoom scale factor this frame (e.g. from ctrl-scroll or pinch gesture).
    /// * `zoom = 1`: no change
    /// * `zoom < 1`: pinch together
    /// * `zoom > 1`: pinch spread
    #[inline(always)]
    pub fn zoom_delta(&self) -> f32 {
        // If a multi touch gesture is detected, it measures the exact and linear proportions of
        // the distances of the finger tips. It is therefore potentially more accurate than
        // `zoom_factor_delta` which is based on the `ctrl-scroll` event which, in turn, may be
        // synthesized from an original touch gesture.
        self.multi_touch()
            .map_or(self.zoom_factor_delta, |touch| touch.zoom_delta)
    }

    /// 2D non-proportional zoom scale factor this frame (e.g. from ctrl-scroll or pinch gesture).
    ///
    /// For multitouch devices the user can do a horizontal or vertical pinch gesture.
    /// In these cases a non-proportional zoom factor is a available.
    /// In other cases, this reverts to `Vec2::splat(self.zoom_delta())`.
    ///
    /// For horizontal pinches, this will return `[z, 1]`,
    /// for vertical pinches this will return `[1, z]`,
    /// and otherwise this will return `[z, z]`,
    /// where `z` is the zoom factor:
    /// * `zoom = 1`: no change
    /// * `zoom < 1`: pinch together
    /// * `zoom > 1`: pinch spread
    #[inline(always)]
    pub fn zoom_delta_2d(&self) -> Vec2 {
        // If a multi touch gesture is detected, it measures the exact and linear proportions of
        // the distances of the finger tips.  It is therefore potentially more accurate than
        // `zoom_factor_delta` which is based on the `ctrl-scroll` event which, in turn, may be
        // synthesized from an original touch gesture.
        self.multi_touch().map_or_else(
            || Vec2::splat(self.zoom_factor_delta),
            |touch| touch.zoom_delta_2d,
        )
    }

    pub fn wants_repaint(&self) -> bool {
        self.pointer.wants_repaint() || self.scroll_delta != Vec2::ZERO || !self.events.is_empty()
    }

    /// Count presses of a key. If non-zero, the presses are consumed, so that this will only return non-zero once.
    ///
    /// Includes key-repeat events.
    pub fn count_and_consume_key(&mut self, modifiers: Modifiers, key: Key) -> usize {
        let mut count = 0usize;

        self.events.retain(|event| {
            let is_match = matches!(
                event,
                Event::Key {
                    key: ev_key,
                    modifiers: ev_mods,
                    pressed: true,
                    ..
                } if *ev_key == key && ev_mods.matches(modifiers)
            );

            count += is_match as usize;

            !is_match
        });

        count
    }

    /// Check for a key press. If found, `true` is returned and the key pressed is consumed, so that this will only return `true` once.
    ///
    /// Includes key-repeat events.
    pub fn consume_key(&mut self, modifiers: Modifiers, key: Key) -> bool {
        self.count_and_consume_key(modifiers, key) > 0
    }

    /// Check if the given shortcut has been pressed.
    ///
    /// If so, `true` is returned and the key pressed is consumed, so that this will only return `true` once.
    ///
    /// Includes key-repeat events.
    pub fn consume_shortcut(&mut self, shortcut: &KeyboardShortcut) -> bool {
        let KeyboardShortcut { modifiers, key } = *shortcut;
        self.consume_key(modifiers, key)
    }

    /// Was the given key pressed this frame?
    ///
    /// Includes key-repeat events.
    pub fn key_pressed(&self, desired_key: Key) -> bool {
        self.num_presses(desired_key) > 0
    }

    /// How many times was the given key pressed this frame?
    ///
    /// Includes key-repeat events.
    pub fn num_presses(&self, desired_key: Key) -> usize {
        self.events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    Event::Key { key, pressed: true, .. }
                    if *key == desired_key
                )
            })
            .count()
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

    /// Also known as device pixel ratio, > 1 for high resolution screens.
    #[inline(always)]
    pub fn pixels_per_point(&self) -> f32 {
        self.pixels_per_point
    }

    /// Size of a physical pixel in logical gui coordinates (points).
    #[inline(always)]
    pub fn physical_pixel_size(&self) -> f32 {
        1.0 / self.pixels_per_point()
    }

    /// How imprecise do we expect the mouse/touch input to be?
    /// Returns imprecision in points.
    #[inline(always)]
    pub fn aim_radius(&self) -> f32 {
        // TODO(emilk): multiply by ~3 for touch inputs because fingers are fat
        self.physical_pixel_size()
    }

    /// Returns details about the currently ongoing multi-touch gesture, if any. Note that this
    /// method returns `None` for single-touch gestures (click, drag, …).
    ///
    /// ```
    /// # use egui::emath::Rot2;
    /// # egui::__run_test_ui(|ui| {
    /// let mut zoom = 1.0; // no zoom
    /// let mut rotation = 0.0; // no rotation
    /// let multi_touch = ui.input(|i| i.multi_touch());
    /// if let Some(multi_touch) = multi_touch {
    ///     zoom *= multi_touch.zoom_delta;
    ///     rotation += multi_touch.rotation_delta;
    /// }
    /// let transform = zoom * Rot2::from_angle(rotation);
    /// # });
    /// ```
    ///
    /// By far not all touch devices are supported, and the details depend on the `egui`
    /// integration backend you are using. `eframe` web supports multi touch for most mobile
    /// devices, but not for a `Trackpad` on `MacOS`, for example. The backend has to be able to
    /// capture native touch events, but many browsers seem to pass such events only for touch
    /// _screens_, but not touch _pads._
    ///
    /// Refer to [`MultiTouchInfo`] for details about the touch information available.
    ///
    /// Consider using `zoom_delta()` instead of `MultiTouchInfo::zoom_delta` as the former
    /// delivers a synthetic zoom factor based on ctrl-scroll events, as a fallback.
    pub fn multi_touch(&self) -> Option<MultiTouchInfo> {
        // In case of multiple touch devices simply pick the touch_state of the first active device
        if let Some(touch_state) = self.touch_states.values().find(|t| t.is_active()) {
            touch_state.info()
        } else {
            None
        }
    }

    /// True if there currently are any fingers touching egui.
    pub fn any_touches(&self) -> bool {
        !self.touch_states.is_empty()
    }

    /// Scans `events` for device IDs of touch devices we have not seen before,
    /// and creates a new [`TouchState`] for each such device.
    fn create_touch_states_for_new_devices(&mut self, events: &[Event]) {
        for event in events {
            if let Event::Touch { device_id, .. } = event {
                self.touch_states
                    .entry(*device_id)
                    .or_insert_with(|| TouchState::new(*device_id));
            }
        }
    }

    #[cfg(feature = "accesskit")]
    pub fn accesskit_action_requests(
        &self,
        id: crate::Id,
        action: accesskit::Action,
    ) -> impl Iterator<Item = &accesskit::ActionRequest> {
        let accesskit_id = id.accesskit_id();
        self.events.iter().filter_map(move |event| {
            if let Event::AccessKitActionRequest(request) = event {
                if request.target == accesskit_id && request.action == action {
                    return Some(request);
                }
            }
            None
        })
    }

    #[cfg(feature = "accesskit")]
    pub fn has_accesskit_action_request(&self, id: crate::Id, action: accesskit::Action) -> bool {
        self.accesskit_action_requests(id, action).next().is_some()
    }

    #[cfg(feature = "accesskit")]
    pub fn num_accesskit_action_requests(&self, id: crate::Id, action: accesskit::Action) -> usize {
        self.accesskit_action_requests(id, action).count()
    }
}

// ----------------------------------------------------------------------------

/// A pointer (mouse or touch) click.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Click {
    pub pos: Pos2,

    /// 1 or 2 (double-click) or 3 (triple-click)
    pub count: u32,

    /// Allows you to check for e.g. shift-click
    pub modifiers: Modifiers,
}

impl Click {
    pub fn is_double(&self) -> bool {
        self.count == 2
    }

    pub fn is_triple(&self) -> bool {
        self.count == 3
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PointerEvent {
    Moved(Pos2),
    Pressed {
        position: Pos2,
        button: PointerButton,
    },
    Released {
        click: Option<Click>,
        button: PointerButton,
    },
}

impl PointerEvent {
    pub fn is_press(&self) -> bool {
        matches!(self, PointerEvent::Pressed { .. })
    }

    pub fn is_release(&self) -> bool {
        matches!(self, PointerEvent::Released { .. })
    }

    pub fn is_click(&self) -> bool {
        matches!(self, PointerEvent::Released { click: Some(_), .. })
    }
}

/// Mouse or touch state.
#[derive(Clone, Debug)]
pub struct PointerState {
    /// Latest known time
    time: f64,

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

    /// When did the current click/drag originate?
    /// `None` if no mouse button is down.
    press_start_time: Option<f64>,

    /// Set to `true` if the pointer has moved too much (since being pressed)
    /// for it to be registered as a click.
    pub(crate) has_moved_too_much_for_a_click: bool,

    /// When did the pointer get click last?
    /// Used to check for double-clicks.
    last_click_time: f64,

    /// When did the pointer get click two clicks ago?
    /// Used to check for triple-clicks.
    last_last_click_time: f64,

    /// All button events that occurred this frame
    pub(crate) pointer_events: Vec<PointerEvent>,
}

impl Default for PointerState {
    fn default() -> Self {
        Self {
            time: -f64::INFINITY,
            latest_pos: None,
            interact_pos: None,
            delta: Vec2::ZERO,
            velocity: Vec2::ZERO,
            pos_history: History::new(0..1000, 0.1),
            down: Default::default(),
            press_origin: None,
            press_start_time: None,
            has_moved_too_much_for_a_click: false,
            last_click_time: std::f64::NEG_INFINITY,
            last_last_click_time: std::f64::NEG_INFINITY,
            pointer_events: vec![],
        }
    }
}

impl PointerState {
    #[must_use]
    pub(crate) fn begin_frame(mut self, time: f64, new: &RawInput) -> PointerState {
        self.time = time;

        self.pointer_events.clear();

        let old_pos = self.latest_pos;
        self.interact_pos = self.latest_pos;

        for event in &new.events {
            match event {
                Event::PointerMoved(pos) => {
                    let pos = *pos;

                    self.latest_pos = Some(pos);
                    self.interact_pos = Some(pos);

                    if let Some(press_origin) = self.press_origin {
                        self.has_moved_too_much_for_a_click |=
                            press_origin.distance(pos) > MAX_CLICK_DIST;
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
                        self.press_start_time = Some(time);
                        self.has_moved_too_much_for_a_click = false;
                        self.pointer_events.push(PointerEvent::Pressed {
                            position: pos,
                            button,
                        });
                    } else {
                        let clicked = self.could_any_button_be_click();

                        let click = if clicked {
                            let double_click =
                                (time - self.last_click_time) < MAX_DOUBLE_CLICK_DELAY;
                            let triple_click =
                                (time - self.last_last_click_time) < (MAX_DOUBLE_CLICK_DELAY * 2.0);
                            let count = if triple_click {
                                3
                            } else if double_click {
                                2
                            } else {
                                1
                            };

                            self.last_last_click_time = self.last_click_time;
                            self.last_click_time = time;

                            Some(Click {
                                pos,
                                count,
                                modifiers,
                            })
                        } else {
                            None
                        };

                        self.pointer_events
                            .push(PointerEvent::Released { click, button });

                        self.press_origin = None;
                        self.press_start_time = None;
                    }

                    self.down[button as usize] = pressed; // must be done after the above call to `could_any_button_be_click`
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
    #[inline(always)]
    pub fn delta(&self) -> Vec2 {
        self.delta
    }

    /// Current velocity of pointer.
    #[inline(always)]
    pub fn velocity(&self) -> Vec2 {
        self.velocity
    }

    /// Where did the current click/drag originate?
    /// `None` if no mouse button is down.
    #[inline(always)]
    pub fn press_origin(&self) -> Option<Pos2> {
        self.press_origin
    }

    /// When did the current click/drag originate?
    /// `None` if no mouse button is down.
    #[inline(always)]
    pub fn press_start_time(&self) -> Option<f64> {
        self.press_start_time
    }

    /// Latest reported pointer position.
    /// When tapping a touch screen, this will be `None`.
    #[inline(always)]
    pub(crate) fn latest_pos(&self) -> Option<Pos2> {
        self.latest_pos
    }

    /// If it is a good idea to show a tooltip, where is pointer?
    #[inline(always)]
    pub fn hover_pos(&self) -> Option<Pos2> {
        self.latest_pos
    }

    /// If you detect a click or drag and wants to know where it happened, use this.
    ///
    /// Latest position of the mouse, but ignoring any [`Event::PointerGone`]
    /// if there were interactions this frame.
    /// When tapping a touch screen, this will be the location of the touch.
    #[inline(always)]
    pub fn interact_pos(&self) -> Option<Pos2> {
        self.interact_pos
    }

    /// Do we have a pointer?
    ///
    /// `false` if the mouse is not over the egui area, or if no touches are down on touch screens.
    #[inline(always)]
    pub fn has_pointer(&self) -> bool {
        self.latest_pos.is_some()
    }

    /// Is the pointer currently still?
    /// This is smoothed so a few frames of stillness is required before this returns `true`.
    #[inline(always)]
    pub fn is_still(&self) -> bool {
        self.velocity == Vec2::ZERO
    }

    /// Is the pointer currently moving?
    /// This is smoothed so a few frames of stillness is required before this returns `false`.
    #[inline]
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

    /// Was the button given pressed this frame?
    pub fn button_pressed(&self, button: PointerButton) -> bool {
        self.pointer_events
            .iter()
            .any(|event| matches!(event, &PointerEvent::Pressed{button: b, ..} if button == b))
    }

    /// Was the button given released this frame?
    pub fn button_released(&self, button: PointerButton) -> bool {
        self.pointer_events
            .iter()
            .any(|event| matches!(event, &PointerEvent::Released{button: b, ..} if button == b))
    }

    /// Was the primary button pressed this frame?
    pub fn primary_pressed(&self) -> bool {
        self.button_pressed(PointerButton::Primary)
    }

    /// Was the secondary button pressed this frame?
    pub fn secondary_pressed(&self) -> bool {
        self.button_pressed(PointerButton::Secondary)
    }

    /// Was the primary button released this frame?
    pub fn primary_released(&self) -> bool {
        self.button_released(PointerButton::Primary)
    }

    /// Was the secondary button released this frame?
    pub fn secondary_released(&self) -> bool {
        self.button_released(PointerButton::Secondary)
    }

    /// Is any pointer button currently down?
    pub fn any_down(&self) -> bool {
        self.down.iter().any(|&down| down)
    }

    /// Were there any type of click this frame?
    pub fn any_click(&self) -> bool {
        self.pointer_events.iter().any(|event| event.is_click())
    }

    /// Was the button given clicked this frame?
    pub fn button_clicked(&self, button: PointerButton) -> bool {
        self.pointer_events
            .iter()
            .any(|event| matches!(event, &PointerEvent::Pressed { button: b, .. } if button == b))
    }

    /// Was the button given double clicked this frame?
    pub fn button_double_clicked(&self, button: PointerButton) -> bool {
        self.pointer_events.iter().any(|event| {
            matches!(
                &event,
                PointerEvent::Released {
                    click: Some(click),
                    button: b,
                } if *b == button && click.is_double()
            )
        })
    }

    /// Was the button given triple clicked this frame?
    pub fn button_triple_clicked(&self, button: PointerButton) -> bool {
        self.pointer_events.iter().any(|event| {
            matches!(
                &event,
                PointerEvent::Released {
                    click: Some(click),
                    button: b,
                } if *b == button && click.is_triple()
            )
        })
    }

    /// Was the primary button clicked this frame?
    pub fn primary_clicked(&self) -> bool {
        self.button_clicked(PointerButton::Primary)
    }

    /// Was the secondary button clicked this frame?
    pub fn secondary_clicked(&self) -> bool {
        self.button_clicked(PointerButton::Secondary)
    }

    /// Is this button currently down?
    #[inline(always)]
    pub fn button_down(&self, button: PointerButton) -> bool {
        self.down[button as usize]
    }

    /// If the pointer button is down, will it register as a click when released?
    #[inline(always)]
    pub(crate) fn could_any_button_be_click(&self) -> bool {
        if !self.any_down() {
            return false;
        }

        if self.has_moved_too_much_for_a_click {
            return false;
        }

        if let Some(press_start_time) = self.press_start_time {
            if self.time - press_start_time > MAX_CLICK_DURATION {
                return false;
            }
        }

        true
    }

    /// Is the primary button currently down?
    #[inline(always)]
    pub fn primary_down(&self) -> bool {
        self.button_down(PointerButton::Primary)
    }

    /// Is the secondary button currently down?
    #[inline(always)]
    pub fn secondary_down(&self) -> bool {
        self.button_down(PointerButton::Secondary)
    }

    /// Is the middle button currently down?
    #[inline(always)]
    pub fn middle_down(&self) -> bool {
        self.button_down(PointerButton::Middle)
    }
}

impl InputState {
    pub fn ui(&self, ui: &mut crate::Ui) {
        let Self {
            raw,
            pointer,
            touch_states,
            scroll_delta,
            zoom_factor_delta,
            screen_rect,
            pixels_per_point,
            max_texture_side,
            time,
            unstable_dt,
            predicted_dt,
            stable_dt,
            modifiers,
            keys_down,
            events,
        } = self;

        ui.style_mut()
            .text_styles
            .get_mut(&crate::TextStyle::Body)
            .unwrap()
            .family = crate::FontFamily::Monospace;

        ui.collapsing("Raw Input", |ui| raw.ui(ui));

        crate::containers::CollapsingHeader::new("🖱 Pointer")
            .default_open(true)
            .show(ui, |ui| {
                pointer.ui(ui);
            });

        for (device_id, touch_state) in touch_states {
            ui.collapsing(format!("Touch State [device {}]", device_id.0), |ui| {
                touch_state.ui(ui);
            });
        }

        ui.label(format!("scroll_delta: {:?} points", scroll_delta));
        ui.label(format!("zoom_factor_delta: {:4.2}x", zoom_factor_delta));
        ui.label(format!("screen_rect: {:?} points", screen_rect));
        ui.label(format!(
            "{} physical pixels for each logical point",
            pixels_per_point
        ));
        ui.label(format!(
            "max texture size (on each side): {}",
            max_texture_side
        ));
        ui.label(format!("time: {:.3} s", time));
        ui.label(format!(
            "time since previous frame: {:.1} ms",
            1e3 * unstable_dt
        ));
        ui.label(format!("predicted_dt: {:.1} ms", 1e3 * predicted_dt));
        ui.label(format!("stable_dt:    {:.1} ms", 1e3 * stable_dt));
        ui.label(format!("modifiers: {:#?}", modifiers));
        ui.label(format!("keys_down: {:?}", keys_down));
        ui.scope(|ui| {
            ui.set_min_height(150.0);
            ui.label(format!("events: {:#?}", events))
                .on_hover_text("key presses etc");
        });
    }
}

impl PointerState {
    pub fn ui(&self, ui: &mut crate::Ui) {
        let Self {
            time: _,
            latest_pos,
            interact_pos,
            delta,
            velocity,
            pos_history: _,
            down,
            press_origin,
            press_start_time,
            has_moved_too_much_for_a_click,
            last_click_time,
            last_last_click_time,
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
        ui.label(format!("press_start_time: {:?} s", press_start_time));
        ui.label(format!(
            "has_moved_too_much_for_a_click: {}",
            has_moved_too_much_for_a_click
        ));
        ui.label(format!("last_click_time: {:#?}", last_click_time));
        ui.label(format!("last_last_click_time: {:#?}", last_last_click_time));
        ui.label(format!("pointer_events: {:?}", pointer_events));
    }
}
