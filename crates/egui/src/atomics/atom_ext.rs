use crate::{Atom, FontSelection, Ui};
use emath::Vec2;

/// A trait for conveniently building [`Atom`]s.
///
/// The functions are prefixed with `atom_` to avoid conflicts with e.g. [`crate::RichText::size`].
pub trait AtomExt<'a> {
    /// Set the atom to a fixed size.
    ///
    /// If [`Atom::grow`] is `true`, this will be the minimum width.
    /// If [`Atom::shrink`] is `true`, this will be the maximum width.
    /// If both are true, the width will have no effect.
    ///
    /// [`Self::atom_max_size`] will limit size.
    ///
    /// See [`crate::AtomKind`] docs to see how the size affects the different types.
    fn atom_size(self, size: Vec2) -> Atom<'a>;

    /// Grow this atom to the available space.
    ///
    /// This will affect the size of the [`Atom`] in the main direction. Since
    /// [`crate::AtomLayout`] today only supports horizontal layout, it will affect the width.
    ///
    /// You can also combine this with [`Self::atom_shrink`] to make it always take exactly the
    /// remaining space.
    fn atom_grow(self, grow: bool) -> Atom<'a>;

    /// Shrink this atom if there isn't enough space.
    ///
    /// This will affect the size of the [`Atom`] in the main direction. Since
    /// [`crate::AtomLayout`] today only supports horizontal layout, it will affect the width.
    ///
    /// NOTE: Only a single [`Atom`] may shrink for each widget.
    ///
    /// If no atom was set to shrink and `wrap_mode != TextWrapMode::Extend`, the first
    /// `AtomKind::Text` is set to shrink.
    fn atom_shrink(self, shrink: bool) -> Atom<'a>;

    /// Set the maximum size of this atom.
    ///
    /// Will not affect the space taken by `grow` (All atoms marked as grow will always grow
    /// equally to fill the available space).
    fn atom_max_size(self, max_size: Vec2) -> Atom<'a>;

    /// Set the maximum width of this atom.
    ///
    /// Will not affect the space taken by `grow` (All atoms marked as grow will always grow
    /// equally to fill the available space).
    fn atom_max_width(self, max_width: f32) -> Atom<'a>;

    /// Set the maximum height of this atom.
    fn atom_max_height(self, max_height: f32) -> Atom<'a>;

    /// Set the max height of this atom to match the font size.
    ///
    /// This is useful for e.g. limiting the height of icons in buttons.
    fn atom_max_height_font_size(self, ui: &Ui) -> Atom<'a>
    where
        Self: Sized,
    {
        let font_selection = FontSelection::default();
        let font_id = font_selection.resolve(ui.style());
        let height = ui.fonts(|f| f.row_height(&font_id));
        self.atom_max_height(height)
    }
}

impl<'a, T> AtomExt<'a> for T
where
    T: Into<Atom<'a>> + Sized,
{
    fn atom_size(self, size: Vec2) -> Atom<'a> {
        let mut atom = self.into();
        atom.size = Some(size);
        atom
    }

    fn atom_grow(self, grow: bool) -> Atom<'a> {
        let mut atom = self.into();
        atom.grow = grow;
        atom
    }

    fn atom_shrink(self, shrink: bool) -> Atom<'a> {
        let mut atom = self.into();
        atom.shrink = shrink;
        atom
    }

    fn atom_max_size(self, max_size: Vec2) -> Atom<'a> {
        let mut atom = self.into();
        atom.max_size = max_size;
        atom
    }

    fn atom_max_width(self, max_width: f32) -> Atom<'a> {
        let mut atom = self.into();
        atom.max_size.x = max_width;
        atom
    }

    fn atom_max_height(self, max_height: f32) -> Atom<'a> {
        let mut atom = self.into();
        atom.max_size.y = max_height;
        atom
    }
}
