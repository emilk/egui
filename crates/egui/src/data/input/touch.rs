/// this is a `u64` as values of this kind can always be obtained by hashing
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TouchDeviceId(pub u64);

/// Unique identification of a touch occurrence (finger or pen or …).
/// A Touch ID is valid until the finger is lifted.
/// A new ID is used for the next touch.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TouchId(pub u64);

/// In what phase a touch event is in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum TouchPhase {
    /// User just placed a touch point on the touch surface
    Start,

    /// User moves a touch point along the surface. This event is also sent when
    /// any attributes (position, force, …) of the touch point change.
    Move,

    /// User lifted the finger or pen from the surface, or slid off the edge of
    /// the surface
    End,

    /// Touch operation has been disrupted by something (various reasons are possible,
    /// maybe a pop-up alert or any other kind of interruption which may not have
    /// been intended by the user)
    Cancel,
}

impl From<u64> for TouchId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<i32> for TouchId {
    fn from(id: i32) -> Self {
        Self(id as u64)
    }
}

impl From<u32> for TouchId {
    fn from(id: u32) -> Self {
        Self(id as u64)
    }
}
