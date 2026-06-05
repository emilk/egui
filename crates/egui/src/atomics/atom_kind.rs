use crate::{AtomLayout, FontSelection, Image, ImageSource, SizedAtomKind, Ui, WidgetText};
use emath::Vec2;
use epaint::text::TextWrapMode;
use std::fmt::Debug;
use std::rc::Rc;

/// Args passed when sizing an [`super::Atom`]
pub struct IntoSizedArgs {
    pub available_size: Vec2,
    pub wrap_mode: TextWrapMode,
    pub fallback_font: FontSelection,
}

/// Result returned when sizing an [`super::Atom`]
pub struct IntoSizedResult<'a> {
    pub intrinsic_size: Vec2,
    pub sized: SizedAtomKind<'a>,
}

/// The different kinds of [`crate::Atom`]s.
#[derive(Clone, Default)]
pub enum AtomKind<'a> {
    /// Empty, that can be used with [`crate::AtomExt::atom_grow`] to reserve space.
    #[default]
    Empty,

    /// Text atom.
    ///
    /// Truncation within [`crate::AtomLayout`] works like this:
    /// -
    /// - if `wrap_mode` is not Extend
    ///   - if no atom is `shrink`
    ///     - the first text atom is selected and will be marked as `shrink`
    ///   - the atom marked as `shrink` will shrink / wrap based on the selected wrap mode
    ///   - any other text atoms will have `wrap_mode` extend
    /// - if `wrap_mode` is extend, Text will extend as expected.
    ///
    /// Unless [`crate::AtomExt::atom_max_width`] is set, `wrap_mode` should only be set via [`crate::Style`] or
    /// [`crate::AtomLayout::wrap_mode`], as setting a wrap mode on a [`WidgetText`] atom
    /// that is not `shrink` will have unexpected results.
    ///
    /// The size is determined by converting the [`WidgetText`] into a galley and using the galleys
    /// size. You can use [`crate::AtomExt::atom_size`] to override this, and [`crate::AtomExt::atom_max_width`]
    /// to limit the width (Causing the text to wrap or truncate, depending on the `wrap_mode`.
    /// [`crate::AtomExt::atom_max_height`] has no effect on text.
    Text(WidgetText),

    /// Image atom.
    ///
    /// By default the size is determined via [`Image::calc_size`].
    /// You can use [`crate::AtomExt::atom_max_size`] or [`crate::AtomExt::atom_size`] to customize the size.
    /// There is also a helper [`crate::AtomExt::atom_max_height_font_size`] to set the max height to the
    /// default font height, which is convenient for icons.
    Image(Image<'a>),

    /// A nested [`AtomLayout`], letting you embed an atom-based widget as a single atom
    /// inside another [`AtomLayout`].
    ///
    /// The nested layout is measured (sized) when the parent is sized, and painted (and
    /// interacted with) at the cell rect the parent computes for it. The `Arc` lets the parent
    /// keep the (unsized) layout around cheaply so a grown atom can be re-measured at its painted
    /// size without deep-cloning it. See [`SizedAtomKind::Layout`].
    Layout(Rc<AtomLayout<'a>>),
}

impl Debug for AtomKind<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AtomKind::Empty => write!(f, "AtomKind::Empty"),
            AtomKind::Text(text) => write!(f, "AtomKind::Text({text:?})"),
            AtomKind::Image(image) => write!(f, "AtomKind::Image({image:?})"),
            AtomKind::Layout(_) => write!(f, "AtomKind::Layout(<layout>)"),
        }
    }
}

impl<'a> AtomKind<'a> {
    /// See [`Self::Text`]
    pub fn text(text: impl Into<WidgetText>) -> Self {
        AtomKind::Text(text.into())
    }

    /// See [`Self::Image`]
    pub fn image(image: impl Into<Image<'a>>) -> Self {
        AtomKind::Image(image.into())
    }

    /// Size this [`AtomKind`] into a [`SizedAtomKind`].
    ///
    /// This converts [`WidgetText`] into [`crate::Galley`] and tries to load and size [`Image`].
    /// The first returned argument is the preferred size.
    ///
    /// Takes `&self` so an atom can be sized repeatedly (e.g. re-measured at a grown size when a
    /// nested layout reflows) without consuming it. The returned [`SizedAtomKind`] is owned (texts
    /// produce a [`crate::Galley`], images and nested layouts are shared via cheap clones), so it
    /// does not borrow `self`.
    pub fn as_sized(
        &self,
        ui: &Ui,
        IntoSizedArgs {
            available_size,
            wrap_mode,
            fallback_font,
        }: IntoSizedArgs,
        cache: &mut super::MeasureCache<'a>,
    ) -> IntoSizedResult<'a> {
        match self {
            AtomKind::Text(text) => {
                let galley =
                    text.clone()
                        .into_galley(ui, Some(wrap_mode), available_size.x, fallback_font);
                IntoSizedResult {
                    intrinsic_size: galley.intrinsic_size(),
                    sized: SizedAtomKind::Text(galley),
                }
            }
            AtomKind::Image(image) => {
                let size = image.load_and_calc_size(ui, available_size);
                let size = size.unwrap_or(Vec2::ZERO);
                IntoSizedResult {
                    intrinsic_size: size,
                    sized: SizedAtomKind::Image {
                        image: image.clone(),
                        size,
                    },
                }
            }
            AtomKind::Empty => IntoSizedResult {
                intrinsic_size: Vec2::ZERO,
                sized: SizedAtomKind::Empty { size: None },
            },
            AtomKind::Layout(layout) => {
                // Measure at the natural size for the parent's sizing, but keep a shared handle to
                // the original layout so a grown atom can be re-measured at its painted size in
                // `paint_at` (cheap `Rc` clone, no deep copy). `measure_rc` shares the `cache`
                // (keyed by the `Rc`'s identity) so a deep tree of `grow` layouts doesn't
                // re-measure its descendants exponentially.
                let sized = AtomLayout::measure_rc(layout, ui, available_size, cache);
                IntoSizedResult {
                    intrinsic_size: sized.intrinsic_size,
                    sized: SizedAtomKind::Layout {
                        source: Rc::clone(layout),
                        sized: Box::new(sized),
                    },
                }
            }
        }
    }
}

impl<'a> From<ImageSource<'a>> for AtomKind<'a> {
    fn from(value: ImageSource<'a>) -> Self {
        AtomKind::Image(value.into())
    }
}

impl<'a> From<Image<'a>> for AtomKind<'a> {
    fn from(value: Image<'a>) -> Self {
        AtomKind::Image(value)
    }
}

impl<T> From<T> for AtomKind<'_>
where
    T: Into<WidgetText>,
{
    fn from(value: T) -> Self {
        AtomKind::Text(value.into())
    }
}

impl<'a> From<AtomLayout<'a>> for AtomKind<'a> {
    fn from(layout: AtomLayout<'a>) -> Self {
        AtomKind::Layout(Rc::new(layout))
    }
}
