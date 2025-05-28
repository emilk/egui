use crate::SizedAtomicKind;
use emath::Vec2;

/// A [`crate::Atomic`] which has been sized.
#[derive(Clone, Debug)]
pub struct SizedAtomic<'a> {
    pub(crate) grow: bool,

    /// The size of the atomic.
    ///
    /// Used for placing this atomic in [`crate::AtomicLayout`], the cursor will advance by
    /// size.x + gap.
    pub size: Vec2,

    /// Preferred size of the atomic. This is used to calculate `Response::intrinsic_size`.
    pub preferred_size: Vec2,

    pub kind: SizedAtomicKind<'a>,
}

impl SizedAtomic<'_> {
    /// Was this [`crate::Atomic`] marked as `grow`?
    pub fn is_grow(&self) -> bool {
        self.grow
    }
}
