use crate::{AtomicKind, Id, SizedAtomic, Ui};
use emath::{NumExt as _, Vec2};
use epaint::text::TextWrapMode;

/// A low-level ui building block.
///
/// Implements [`From`] for [`String`], [`str`], [`crate::Image`] and much more for convenience.
/// You can directly call the `a_*` methods on anything that implements `Into<Atomic>`.
/// ```
/// # use egui::{Image, emath::Vec2};
/// use egui::AtomicExt as _;
/// let string_atomic = "Hello".atom_grow(true);
/// let image_atomic = Image::new("some_image_url").atom_size(Vec2::splat(20.0));
/// ```
#[derive(Clone, Debug)]
pub struct Atomic<'a> {
    /// See [`crate::AtomicExt::atom_size`]
    pub size: Option<Vec2>,
    /// See [`crate::AtomicExt::atom_max_size`]
    pub max_size: Vec2,
    /// See [`crate::AtomicExt::atom_grow`]
    pub grow: bool,
    /// See [`crate::AtomicExt::atom_shrink`]
    pub shrink: bool,
    /// The atom type
    pub kind: AtomicKind<'a>,
}

impl Default for Atomic<'_> {
    fn default() -> Self {
        Atomic {
            size: None,
            max_size: Vec2::INFINITY,
            grow: false,
            shrink: false,
            kind: AtomicKind::Empty,
        }
    }
}

impl<'a> Atomic<'a> {
    /// Create an empty [`Atomic`] marked as `grow`.
    ///
    /// This will expand in size, allowing all preceding atomics to be left-aligned,
    /// and all following atomics to be right-aligned
    pub fn grow() -> Self {
        Atomic {
            grow: true,
            ..Default::default()
        }
    }

    /// Create a [`AtomicKind::Custom`] with a specific size.
    pub fn custom(id: Id, size: impl Into<Vec2>) -> Self {
        Atomic {
            size: Some(size.into()),
            kind: AtomicKind::Custom(id),
            ..Default::default()
        }
    }

    /// Turn this into a [`SizedAtomic`].
    pub fn into_sized(
        self,
        ui: &Ui,
        mut available_size: Vec2,
        mut wrap_mode: Option<TextWrapMode>,
    ) -> SizedAtomic<'a> {
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

        let (preferred, kind) = self.kind.into_sized(ui, available_size, wrap_mode);

        let size = self
            .size
            .map_or_else(|| kind.size(), |s| s.at_most(self.max_size));

        SizedAtomic {
            size,
            preferred_size: preferred,
            grow: self.grow,
            kind,
        }
    }
}

impl<'a, T> From<T> for Atomic<'a>
where
    T: Into<AtomicKind<'a>>,
{
    fn from(value: T) -> Self {
        Atomic {
            kind: value.into(),
            ..Default::default()
        }
    }
}
