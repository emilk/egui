mod zoom;

pub use zoom::Zoom;

/// TODO: docu
/// ```
/// assert!( 1 == 0 )
/// ```
pub trait Gesture: std::fmt::Debug {
    fn boxed_clone(&self) -> Box<dyn Gesture>;
    fn state(&self) -> Status;
}

/// TODO: docu
/// ```
/// assert!( 1 == 0 )
/// ```
#[derive(Clone, Copy, Debug)]
pub enum Status {
    /// The `Gesture` is idle, and waiting for events
    Waiting,
    /// The `Gesture` has detected events, but the conditions for activating are not met (yet)
    Checking,
    /// The `Gesture` is active and can be asked for its `State`
    Active,
    /// The `Gesture` has decided that it does not match the current touch events.
    Rejected,
}

impl Default for Status {
    fn default() -> Self {
        Status::Waiting
    }
}
