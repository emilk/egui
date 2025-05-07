use crate::{Atomic, FontSelection, Ui};
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

    /// Set the maximum size of this atomic.
    fn atom_max_size(self, max_size: Vec2) -> Atomic<'a>;

    /// Set the maximum width of this atomic.
    fn atom_max_width(self, max_width: f32) -> Atomic<'a>;

    /// Set the maximum height of this atomic.
    fn atom_max_height(self, max_height: f32) -> Atomic<'a>;

    /// Set the max height of this atomic to match the font size.
    ///
    /// This is useful for e.g. limiting the height of icons in buttons.
    fn atom_max_height_font_size(self, ui: &Ui) -> Atomic<'a>
    where
        Self: Sized,
    {
        let font_selection = FontSelection::default();
        let font_id = font_selection.resolve(ui.style());
        let height = ui.fonts(|f| f.row_height(&font_id));
        self.atom_max_height(height)
    }
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

    fn atom_max_size(self, max_size: Vec2) -> Atomic<'a> {
        let mut atomic = self.into();
        atomic.max_size = max_size;
        atomic
    }

    fn atom_max_width(self, max_width: f32) -> Atomic<'a> {
        let mut atomic = self.into();
        atomic.max_size.x = max_width;
        atomic
    }

    fn atom_max_height(self, max_height: f32) -> Atomic<'a> {
        let mut atomic = self.into();
        atomic.max_size.y = max_height;
        atomic
    }
}
