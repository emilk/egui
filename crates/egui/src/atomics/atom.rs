use crate::{AtomKind, FontSelection, Id, IntoSizedArgs, IntoSizedResult, SizedAtom, Ui};
use emath::{Align2, NumExt as _, Vec2};
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
    /// See [`crate::AtomExt::atom_id`]
    pub id: Option<Id>,

    /// See [`crate::AtomExt::atom_size`]
    pub size: Option<Vec2>,

    /// See [`crate::AtomExt::atom_max_size`]
    pub max_size: Vec2,

    /// See [`crate::AtomExt::atom_grow`]
    pub grow: bool,

    /// See [`crate::AtomExt::atom_shrink`]
    pub shrink: bool,

    /// See [`crate::AtomExt::atom_align`]
    pub align: Align2,

    /// The atom type / content
    pub kind: AtomKind<'a>,
}

impl Default for Atom<'_> {
    fn default() -> Self {
        Atom {
            id: None,
            size: None,
            max_size: Vec2::INFINITY,
            grow: false,
            shrink: false,
            align: Align2::CENTER_CENTER,
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
    ///
    /// Example:
    /// ```
    /// # use egui::{AtomExt, AtomKind, Atom, Button, Id, __run_test_ui};
    /// # use emath::Vec2;
    /// # __run_test_ui(|ui| {
    /// let id = Id::new("my_button");
    /// let response = Button::new(("Hi!", Atom::custom(id, Vec2::splat(18.0)))).atom_ui(ui);
    ///
    /// let rect = response.rect(id);
    /// if let Some(rect) = rect {
    ///     ui.place(rect, Button::new("⏵"));
    /// }
    /// # });
    /// ```
    pub fn custom(id: Id, size: impl Into<Vec2>) -> Self {
        Atom {
            size: Some(size.into()),
            kind: AtomKind::Empty,
            id: Some(id),
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

        let id = self.id;

        let wrap_mode = wrap_mode.unwrap_or_else(|| ui.wrap_mode());
        let IntoSizedResult {
            intrinsic_size,
            sized,
        } = self.kind.into_sized(
            ui,
            IntoSizedArgs {
                available_size,
                wrap_mode,
                fallback_font,
            },
        );

        let size = self
            .size
            .map_or_else(|| sized.size(), |s| s.at_most(self.max_size));

        SizedAtom {
            id,
            size,
            intrinsic_size: intrinsic_size.at_least(self.size.unwrap_or_default()),
            grow: self.grow,
            align: self.align,
            kind: sized,
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
