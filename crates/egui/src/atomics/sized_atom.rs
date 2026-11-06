use crate::SizedAtomKind;
use emath::Vec2;

/// A [`crate::Atom`] which has been sized.
#[derive(Clone, Debug)]
pub struct SizedAtom<'a> {
    pub id: Option<crate::IdSalt>,

    pub(crate) grow: bool,

    /// The size of the atom.
    ///
    /// Used for placing this atom in a [`crate::ContainerAtom`], the cursor will advance by
    /// size.x + gap.
    pub size: Vec2,

    /// Intrinsic size of the atom. This is used to calculate `Response::intrinsic_size`.
    pub intrinsic_size: Vec2,

    /// How will the atom be aligned in its available space?
    pub align: emath::Align2,

    /// Align the content to the reading-end edge of the cell along the row's main
    /// axis, instead of the physical main-axis component of [`Self::align`]?
    pub main_axis_end: bool,

    pub kind: SizedAtomKind<'a>,
}

impl SizedAtom<'_> {
    /// Was this [`crate::Atom`] marked as `grow`?
    pub fn is_grow(&self) -> bool {
        self.grow
    }
}
