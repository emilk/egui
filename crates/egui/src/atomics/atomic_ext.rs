use crate::Atomic;
use emath::Vec2;

/// A trait for conveniently building [`Atomic`]s.
pub trait AtomicExt<'a> {
    /// Set the atomic to a fixed size.
    ///
    /// If [`Atomic::grow`] is `true`, this will be the minimum width.
    /// If [`Atomic::shrink`] is `true`, this will be the maximum width.
    /// If both are true, the width will have no effect.
    fn atom_size(self, size: Vec2) -> Atomic<'a>;

    /// Grow this atomic to the available space.
    fn atom_grow(self, grow: bool) -> Atomic<'a>;

    /// Shrink this atomic if there isn't enough space.
    ///
    /// NOTE: Only a single [`Atomic`] may shrink for each widget.
    fn atom_shrink(self, shrink: bool) -> Atomic<'a>;
}

impl<'a, T> AtomicExt<'a> for T
where
    T: Into<Atomic<'a>> + Sized,
{
    fn atom_size(self, size: Vec2) -> Atomic<'a> {
        let mut atomic = self.into();
        atomic.size = Some(size);
        atomic
    }

    fn atom_grow(self, grow: bool) -> Atomic<'a> {
        let mut atomic = self.into();
        atomic.grow = grow;
        atomic
    }

    fn atom_shrink(self, shrink: bool) -> Atomic<'a> {
        let mut atomic = self.into();
        atomic.shrink = shrink;
        atomic
    }
}
