use crate::SizedAtomicKind;
use emath::Vec2;

/// A [`Atomic`] which has been sized.
#[derive(Clone, Debug)]
pub struct SizedAtomic<'a> {
    pub grow: bool,
    pub size: Vec2,
    pub preferred_size: Vec2,
    pub kind: SizedAtomicKind<'a>,
}
