use crate::SizedAtomicKind;
use emath::Vec2;

/// A [`Atomic`] which has been sized.
#[derive(Clone, Debug)]
pub struct SizedAtomic<'a> {
    grow: bool,
    pub size: Vec2,
    pub preferred_size: Vec2,
    pub kind: SizedAtomicKind<'a>,
}

impl SizedAtomic<'_> {
    /// Was this [`Atomic`] marked as `grow`?
    fn is_grow(&self) -> bool {
        self.grow
    }
}
