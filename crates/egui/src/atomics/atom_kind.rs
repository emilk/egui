use crate::{FontSelection, Image, ImageSource, SizedAtomKind, Ui, WidgetText};
use emath::Vec2;
use epaint::text::TextWrapMode;
use std::fmt::Debug;

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

/// See [`AtomKind::Closure`]
// We need 'static in the result (or need to introduce another lifetime on the enum).
// Otherwise, a single 'static Atom would force the closure to be 'static.
pub type AtomClosure<'a> = Box<dyn FnOnce(&Ui, IntoSizedArgs) -> IntoSizedResult<'static> + 'a>;

/// The different kinds of [`crate::Atom`]s.
#[derive(Default)]
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

    /// A custom closure that produces a sized atom.
    ///
    /// The vec2 passed in is the available size to this atom. The returned vec2 should be the
    /// preferred / intrinsic size.
    ///
    /// Note: This api is experimental, expect breaking changes here.
    /// When cloning, this will be cloned as [`AtomKind::Empty`].
    Closure(AtomClosure<'a>),
}

impl Clone for AtomKind<'_> {
    fn clone(&self) -> Self {
        match self {
            AtomKind::Empty => AtomKind::Empty,
            AtomKind::Text(text) => AtomKind::Text(text.clone()),
            AtomKind::Image(image) => AtomKind::Image(image.clone()),
            AtomKind::Closure(_) => {
                log::warn!("Cannot clone atom closures");
                AtomKind::Empty
            }
        }
    }
}

impl Debug for AtomKind<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AtomKind::Empty => write!(f, "AtomKind::Empty"),
            AtomKind::Text(text) => write!(f, "AtomKind::Text({text:?})"),
            AtomKind::Image(image) => write!(f, "AtomKind::Image({image:?})"),
            AtomKind::Closure(_) => write!(f, "AtomKind::Closure(<closure>)"),
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

    /// See [`Self::Closure`]
    pub fn closure(func: impl FnOnce(&Ui, IntoSizedArgs) -> IntoSizedResult<'static> + 'a) -> Self {
        AtomKind::Closure(Box::new(func))
    }

    /// Turn this [`AtomKind`] into a [`SizedAtomKind`].
    ///
    /// This converts [`WidgetText`] into [`crate::Galley`] and tries to load and size [`Image`].
    /// The first returned argument is the preferred size.
    pub fn into_sized(
        self,
        ui: &Ui,
        IntoSizedArgs {
            available_size,
            wrap_mode,
            fallback_font,
        }: IntoSizedArgs,
    ) -> IntoSizedResult<'a> {
        match self {
            AtomKind::Text(text) => {
                let galley = text.into_galley(ui, Some(wrap_mode), available_size.x, fallback_font);
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
                    sized: SizedAtomKind::Image { image, size },
                }
            }
            AtomKind::Empty => IntoSizedResult {
                intrinsic_size: Vec2::ZERO,
                sized: SizedAtomKind::Empty { size: None },
            },
            AtomKind::Closure(func) => func(
                ui,
                IntoSizedArgs {
                    available_size,
                    wrap_mode,
                    fallback_font,
                },
            ),
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
