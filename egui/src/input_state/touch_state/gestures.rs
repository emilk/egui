mod zoom;

use std::fmt::Debug;

pub use zoom::Zoom;

/// TODO: docu
/// ```
/// assert!( 1 == 0 )
/// ```
pub trait Gesture: Debug {
    /// Creates a clone in a `Box`.
    fn boxed_clone(&self) -> Box<dyn Gesture>;
    /// The `Kind` of the gesture. Used for filtering.
    fn kind(&self) -> Kind;
    /// The current processing state.
    fn state(&self) -> State;
    /// Returns gesture specific detailed information.
    /// Returns `None` when `state()` is not `Active`.
    fn details(&self) -> Option<Details>;
    /// Returns the screen position at which the gesture was first detected.  
    /// Returns `None` when `state()` is not `Active`.
    fn start_position(&self) -> Option<epaint::emath::Pos2>;
}

/// TODO: docu
/// ```
/// assert!( 1 == 0 )
/// ```
#[derive(Clone, Copy, Debug)]
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

pub enum Kind {
    Zoom,
    // more to come...
    // Tap,
    // Rotate,
    // Swipe,
}

pub enum Details {
    Zoom { factor: f32 },
    // Rotate { angle: f32 },
    // Swipe { direction: Vec2, velocity: Vec2 }
}
