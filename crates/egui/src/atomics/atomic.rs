use crate::{AtomicKind, SizedAtomic, Style, Ui};
use emath::Vec2;
use epaint::text::TextWrapMode;
use epaint::Fonts;

/// A low-level ui building block.
///
/// Implements [`From`] for [`String`], [`str`], [`crate::Image`] and much more for convenience.
/// You can directly call the `a_*` methods on anything that implements `Into<Atomic>`.
/// ```
/// # use egui::{Image, emath::Vec2};
/// use egui::AtomicExt as _;
/// let string_atomic = "Hello".a_grow(true);
/// let image_atomic = Image::new("some_image_url").a_size(Vec2::splat(20.0));
/// ```
#[derive(Clone, Debug)]
pub struct Atomic<'a> {
    pub size: Option<Vec2>,
    pub grow: bool,
    pub shrink: bool,
    pub kind: AtomicKind<'a>,
}

impl<'a> Atomic<'a> {
    /// Create an empty [`Atomic`] marked as `grow`.
    pub fn grow() -> Self {
        Atomic {
            size: None,
            grow: true,
            shrink: false,
            kind: AtomicKind::Empty,
        }
    }

    /// Heuristic to find the best height for an image.
    /// Basically returns the height if this is not an [`Image`].
    pub(crate) fn get_min_height_for_image(&self, fonts: &Fonts, style: &Style) -> Option<f32> {
        self.size.map(|s| s.y).or_else(|| {
            match &self.kind {
                AtomicKind::Text(text) => Some(text.font_height(fonts, style)),
                AtomicKind::Custom(_, size) => Some(size.y),
                // Since this method is used to calculate the best height for an image, we always return
                // None for images.
                AtomicKind::Empty | AtomicKind::Image(_) => None,
            }
        })
    }

    /// Turn this into a [`SizedAtomic`].
    pub fn into_sized(
        self,
        ui: &Ui,
        available_size: Vec2,
        font_size: f32,
        wrap_mode: Option<TextWrapMode>,
    ) -> SizedAtomic<'a> {
        let (preferred, kind) = self
            .kind
            .into_sized(ui, available_size, font_size, wrap_mode);
        SizedAtomic {
            size: self.size.unwrap_or_else(|| kind.size()),
            preferred_size: preferred,
            grow: self.grow,
            kind,
        }
    }
}

/// A trait for conveniently building [`Atomic`]s.
pub trait AtomicExt<'a> {
    /// Set the atomic to a fixed size.
    fn a_size(self, size: Vec2) -> Atomic<'a>;

    /// Grow this atomic to the available space.
    fn a_grow(self, grow: bool) -> Atomic<'a>;

    /// Shrink this atomic if there isn't enough space.
    ///
    /// NOTE: Only a single [`Atomic`] may shrink for each widget.
    fn a_shrink(self, shrink: bool) -> Atomic<'a>;
}

impl<'a, T> AtomicExt<'a> for T
where
    T: Into<Atomic<'a>> + Sized,
{
    fn a_size(self, size: Vec2) -> Atomic<'a> {
        let mut atomic = self.into();
        atomic.size = Some(size);
        atomic
    }

    fn a_grow(self, grow: bool) -> Atomic<'a> {
        let mut atomic = self.into();
        atomic.grow = grow;
        atomic
    }

    /// NOTE: Only a single atomic may be marked as shrink
    fn a_shrink(self, shrink: bool) -> Atomic<'a> {
        let mut atomic = self.into();
        atomic.shrink = shrink;
        atomic
    }
}

impl<'a, T> From<T> for Atomic<'a>
where
    T: Into<AtomicKind<'a>>,
{
    fn from(value: T) -> Self {
        Atomic {
            size: None,
            grow: false,
            shrink: false,
            kind: value.into(),
        }
    }
}
