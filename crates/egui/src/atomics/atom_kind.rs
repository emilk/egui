use crate::{FontSelection, Id, Image, ImageSource, SizedAtomKind, Ui, WidgetText};
use emath::Vec2;
use epaint::text::TextWrapMode;

/// The different kinds of [`crate::Atom`]s.
#[derive(Clone, Default, Debug)]
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

    /// For custom rendering.
    ///
    /// You can get the [`crate::Rect`] with the [`Id`] from [`crate::AtomLayoutResponse`] and use a
    /// [`crate::Painter`] or [`Ui::place`] to add/draw some custom content.
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
    Custom(Id),
}

impl<'a> AtomKind<'a> {
    pub fn text(text: impl Into<WidgetText>) -> Self {
        AtomKind::Text(text.into())
    }

    pub fn image(image: impl Into<Image<'a>>) -> Self {
        AtomKind::Image(image.into())
    }

    /// Turn this [`AtomKind`] into a [`SizedAtomKind`].
    ///
    /// This converts [`WidgetText`] into [`crate::Galley`] and tries to load and size [`Image`].
    /// The first returned argument is the preferred size.
    pub fn into_sized(
        self,
        ui: &Ui,
        available_size: Vec2,
        wrap_mode: Option<TextWrapMode>,
        fallback_font: FontSelection,
    ) -> (Vec2, SizedAtomKind<'a>) {
        match self {
            AtomKind::Text(text) => {
                let wrap_mode = wrap_mode.unwrap_or_else(|| ui.wrap_mode());
                let galley = text.into_galley(ui, Some(wrap_mode), available_size.x, fallback_font);
                (galley.intrinsic_size(), SizedAtomKind::Text(galley))
            }
            AtomKind::Image(image) => {
                let size = image.load_and_calc_size(ui, available_size);
                let size = size.unwrap_or(Vec2::ZERO);
                (size, SizedAtomKind::Image(image, size))
            }
            AtomKind::Custom(id) => (Vec2::ZERO, SizedAtomKind::Custom(id)),
            AtomKind::Empty => (Vec2::ZERO, SizedAtomKind::Empty),
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
