use crate::{AtomKind, FontSelection, Id, SizedAtom, Ui};
use emath::{NumExt as _, Vec2};
use epaint::text::TextWrapMode;

/// A low-level ui building block.
///
/// Implements [`From`] for [`String`], [`str`], [`crate::Image`] and much more for convenience.
/// You can directly call the `atom_*` methods on anything that implements `Into<Atom>`.
/// ```
/// # use egui::{Image, emath::Vec2};
/// use egui::AtomExt as _;
/// let string_atom = "Hello".atom_grow(true);
/// let image_atom = Image::new("some_image_url").atom_size(Vec2::splat(20.0));
/// ```
#[derive(Clone, Debug)]
pub struct Atom<'a> {
    /// See [`crate::AtomExt::atom_size`]
    pub size: Option<Vec2>,

    /// See [`crate::AtomExt::atom_max_size`]
    pub max_size: Vec2,

    /// See [`crate::AtomExt::atom_grow`]
    pub grow: bool,

    /// See [`crate::AtomExt::atom_shrink`]
    pub shrink: bool,

    /// The atom type
    pub kind: AtomKind<'a>,
}

impl Default for Atom<'_> {
    fn default() -> Self {
        Atom {
            size: None,
            max_size: Vec2::INFINITY,
            grow: false,
            shrink: false,
            kind: AtomKind::Empty,
        }
    }
}

impl<'a> Atom<'a> {
    /// Create an empty [`Atom`] marked as `grow`.
    ///
    /// This will expand in size, allowing all preceding atoms to be left-aligned,
    /// and all following atoms to be right-aligned
    pub fn grow() -> Self {
        Atom {
            grow: true,
            ..Default::default()
        }
    }

    /// Create a [`AtomKind::Custom`] with a specific size.
    pub fn custom(id: Id, size: impl Into<Vec2>) -> Self {
        Atom {
            size: Some(size.into()),
            kind: AtomKind::Custom(id),
            ..Default::default()
        }
    }

    /// Turn this into a [`SizedAtom`].
    pub fn into_sized(
        self,
        ui: &Ui,
        mut available_size: Vec2,
        mut wrap_mode: Option<TextWrapMode>,
        fallback_font: FontSelection,
    ) -> SizedAtom<'a> {
        if !self.shrink && self.max_size.x.is_infinite() {
            wrap_mode = Some(TextWrapMode::Extend);
        }
        available_size = available_size.at_most(self.max_size);
        if let Some(size) = self.size {
            available_size = available_size.at_most(size);
        }
        if self.max_size.x.is_finite() {
            wrap_mode = Some(TextWrapMode::Truncate);
        }

        let (intrinsic, kind) = self
            .kind
            .into_sized(ui, available_size, wrap_mode, fallback_font);

        let size = self
            .size
            .map_or_else(|| kind.size(), |s| s.at_most(self.max_size));

        SizedAtom {
            size,
            intrinsic_size: intrinsic.at_least(self.size.unwrap_or_default()),
            grow: self.grow,
            kind,
        }
    }
}

impl<'a, T> From<T> for Atom<'a>
where
    T: Into<AtomKind<'a>>,
{
    fn from(value: T) -> Self {
        Atom {
            kind: value.into(),
            ..Default::default()
        }
    }
}
