mod zoom;

use std::fmt::Debug;

pub use zoom::Zoom;

use super::{Touch, TouchId, TouchMap};

/// TODO: docu
/// ```
/// assert!( 1 == 0 )
/// ```
pub trait Gesture: Debug {
    /// Creates a clone in a `Box`.
    fn boxed_clone(&self) -> Box<dyn Gesture>;

    /// The `Kind` of the gesture. Used for filtering.
    fn kind(&self) -> Kind;

    /// The current processing state.  If it is `Rejected`, the gesture will not be considered
    /// until all touches end and a new touch sequence starts
    fn state(&self) -> State;

    /// Returns gesture specific detailed information.
    /// Returns `None` when `state()` is not `Active`.
    fn details(&self) -> Option<Details>;

    /// Returns the screen position at which the gesture was first detected.  
    /// Returns `None` when `state()` is not `Active`.
    fn start_position(&self) -> Option<epaint::emath::Pos2>;

    /// This method is called, even if there is no event to process.  Thus, it is possible to
    /// activate gestures with a delay (e.g. a Single Tap gesture, after having waited for the
    /// Double-Tap timeout)
    fn check(&mut self, time: f64, active_touches: &TouchMap);

    /// indicates the start of an individual touch. `state` contains this touch and possibly other
    /// touches which have been notified earlier
    fn touch_started(&mut self, touch_id: TouchId, time: f64, active_touches: &TouchMap);

    /// indicates that a known touch has changed in position or force
    fn touch_changed(&mut self, touch_id: TouchId, time: f64, active_touches: &TouchMap);

    /// indicates that a known touch has ended. The touch is not contained in `state` any more.
    fn touch_ended(&mut self, touch: Touch, time: f64, active_touches: &TouchMap);

    /// indicates that a known touch has ended unexpectedly (e.g. by an interrupted error pop up or
    /// other circumstances). The touch is not contained in `state` any more.
    fn touch_cancelled(&mut self, touch: Touch, time: f64, active_touches: &TouchMap);
}

/// TODO: docu
/// ```
/// assert!( 1 == 0 )
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The `Gesture` is idle, and waiting for events
    Waiting,
    /// The `Gesture` has detected events, but the conditions for activating are not met (yet)
    Checking,
    /// The `Gesture` is active and can be asked for its `Context`
    Active,
    /// The `Gesture` has decided that it does not match the current touch events.
    Rejected,
}

impl Default for State {
    fn default() -> Self {
        State::Waiting
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Zoom,
    // more to come...
    // Tap,
    // Rotate,
    // Swipe,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Details {
    Zoom { factor: f32 },
    // Rotate { angle: f32 },
    // Swipe { direction: Vec2, velocity: Vec2 }
}
