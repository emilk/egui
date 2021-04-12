mod zoom;

use std::fmt::Debug;

pub use zoom::TwoFingerPinchOrZoom;

use super::{Touch, TouchId};

pub type TouchMap = std::collections::BTreeMap<TouchId, Touch>;

pub struct Context<'a> {
    /// Current time
    pub time: f64,
    /// Collection of active `Touch` instances
    pub active_touches: &'a TouchMap,
    /// Identifier of the added, changed, or removed touch
    pub touch_id: TouchId,
}

/// TODO: docu
/// ```
/// assert!( 1 == 0 )
/// ```
pub trait Gesture: Debug {
    /// Creates a clone in a `Box`.
    fn boxed_clone(&self) -> Box<dyn Gesture>;

    /// The `Kind` of the gesture. Used for filtering.
    fn kind(&self) -> Kind;

    /// Returns gesture specific detailed information.
    /// Returns `None` when `state()` is not `Active`.
    fn details(&self) -> Option<Details>;

    /// Returns the screen position at which the gesture was first detected.  
    /// Returns `None` when `state()` is not `Active`.
    fn start_position(&self) -> Option<epaint::emath::Pos2>;

    /// When the gesture's phase is `Phase::Checking`, this method is called, even if there is no
    /// event to process.  Thus, it is possible to activate gestures with a delay (e.g. activate a
    /// Single-Tap gesture after having waited for the Double-Tap timeout)
    #[must_use]
    fn check(&mut self, _time: f64, _active_touches: &TouchMap) -> Phase {
        Phase::Checking
    }

    /// indicates the start of an individual touch. `state` contains this touch and possibly other
    /// touches which have been notified earlier
    #[must_use]
    fn touch_started(&mut self, ctx: &Context<'_>) -> Phase;

    /// indicates that a known touch has changed in position or force
    #[must_use]
    fn touch_changed(&mut self, ctx: &Context<'_>) -> Phase;

    /// indicates that a known touch has ended. The touch is not contained in `state` any more.
    #[must_use]
    fn touch_ended(&mut self, _ctx: &Context<'_>, _removed_touch: Touch) -> Phase {
        Phase::Rejected
    }
}

/// TODO: docu
/// ```
/// assert!( 1 == 0 )
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// The `Gesture` is idle, and waiting for events.  This is the initial phase and should
    /// not be set by gesture implementations.
    Waiting,
    /// The `Gesture` has detected events, but the conditions for activating are not met (yet)
    Checking,
    /// The `Gesture` is active and can be asked for its `Context`
    Active,
    /// The `Gesture` has decided that it does not match the current touch events.
    Rejected,
}

impl Default for Phase {
    fn default() -> Self {
        Phase::Waiting
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
