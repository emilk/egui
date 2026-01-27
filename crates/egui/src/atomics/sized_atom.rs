use crate::SizedAtomKind;
use emath::Vec2;

/// A [`crate::Atom`] which has been sized.
#[derive(Clone, Debug)]
pub struct SizedAtom<'a> {
    pub(crate) grow: bool,

    /// The size of the atom.
    ///
    /// Used for placing this atom in [`crate::AtomLayout`], the cursor will advance by
    /// size.x + gap.
    pub size: Vec2,

    /// Intrinsic size of the atom. This is used to calculate `Response::intrinsic_size`.
    pub intrinsic_size: Vec2,

    pub kind: SizedAtomKind<'a>,
}

impl SizedAtom<'_> {
    /// Was this [`crate::Atom`] marked as `grow`?
    pub fn is_grow(&self) -> bool {
        self.grow
    }
}
