use crate::SizedAtomKind;
use emath::Vec2;

/// A [`crate::Atom`] which has been sized.
#[derive(Clone, Debug)]
pub struct SizedAtom<'a> {
    pub id: Option<crate::Id>,

    pub(crate) grow: bool,

    pub(crate) ignore_spacing: bool,

    /// The size of the atom.
    ///
    /// Used for placing this atom in [`crate::AtomLayout`], the cursor will advance by
    /// size.x + gap.
    pub size: Vec2,

    /// Intrinsic size of the atom. This is used to calculate `Response::intrinsic_size`.
    pub intrinsic_size: Vec2,

    /// How will the atom be aligned in its available space?
    pub align: emath::Align2,

    pub kind: SizedAtomKind<'a>,
}

impl SizedAtom<'_> {
    /// Was this [`crate::Atom`] marked as `grow`?
    pub fn is_grow(&self) -> bool {
        self.grow
    }

    /// Was this [`crate::Atom`] marked as `ignore_spacing`?
    pub fn ignore_spacing(&self) -> bool {
        self.ignore_spacing
    }
}
