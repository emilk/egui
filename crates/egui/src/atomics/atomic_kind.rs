use crate::{Id, Image, ImageSource, SizedAtomicKind, TextStyle, Ui, WidgetText};
use emath::Vec2;
use epaint::text::TextWrapMode;
use std::fmt::Formatter;

/// The different kinds of [`Atomic`]s.
#[derive(Clone, Default)]
pub enum AtomicKind<'a> {
    /// Empty, that can be used with [`AtomicExt::a_grow`] to reserve space.
    #[default]
    Empty,

    /// Text atomic.
    ///
    /// Truncation within [`crate::AtomicLayout`] works like this:
    /// -
    /// - if `wrap_mode` is not Extend
    ///   - if no atomic is `shrink`
    ///     - the first text atomic is selected and will be marked as `shrink`
    ///   - the atomic marked as `shrink` will shrink / wrap based on the selected wrap mode
    ///   - any other text atomics will have `wrap_mode` extend
    /// - if `wrap_mode` is extend, Text will extend as expected.
    ///
    /// Generally, `wrap_mode` should only be set via [`crate::Style`] or
    /// [`crate::AtomicLayout::wrap_mode`], as setting a wrap mode on a [`WidgetText`] atomic
    /// that is not `shrink` will have unexpected results.
    Text(WidgetText),

    Image(Image<'a>),

    /// For custom rendering.
    ///
    /// You can get the [`Rect`] with the [`Id`] from [`AtomicLayoutResponse`] and use a
    /// [`crate::Painter`] or [`Ui::put`] to add/draw some custom content.
    ///
    /// Example:
    /// ```
    /// # use egui::{AtomicKind, Button, Id, __run_test_ui};
    /// # use emath::Vec2;
    /// # __run_test_ui(|ui| {
    /// let id = Id::new("my_button");
    /// let response = Button::new(("Hi!", AtomicKind::Custom(id, Vec2::splat(18.0)))).atomic_ui(ui);
    ///
    /// let rect = response.get_rect(id);
    /// if let Some(rect) = rect {
    ///     ui.put(*rect, Button::new("⏵"));
    /// }
    /// # });
    /// ```
    Custom(Id, Vec2),
}

impl std::fmt::Debug for AtomicKind<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AtomicKind::Empty => write!(f, "AtomicKind::Empty"),
            AtomicKind::Text(text) => write!(f, "AtomicKind::Text({})", text.text()),
            AtomicKind::Image(image) => write!(f, "AtomicKind::Image({image:?})"),
            AtomicKind::Custom(id, size) => write!(f, "AtomicKind::Custom({id:?}, {size:?})"),
        }
    }
}

impl<'a> AtomicKind<'a> {
    pub fn text(text: impl Into<WidgetText>) -> Self {
        AtomicKind::Text(text.into())
    }

    pub fn image(image: impl Into<Image<'a>>) -> Self {
        AtomicKind::Image(image.into())
    }

    pub fn custom(id: Id, size: Vec2) -> Self {
        AtomicKind::Custom(id, size)
    }

    /// Turn this [`AtomicKind`] into a [`SizedAtomicKind`].
    ///
    /// This converts [`WidgetText`] into [`Galley`] and tries to load and size [`Image`].
    /// The first returned argument is the preferred size.
    pub fn into_sized(
        self,
        ui: &Ui,
        available_size: Vec2,
        wrap_mode: Option<TextWrapMode>,
    ) -> (Vec2, SizedAtomicKind<'a>) {
        match self {
            AtomicKind::Text(text) => {
                let galley = text.into_galley(ui, wrap_mode, available_size.x, TextStyle::Button);
                (
                    galley.size(), // TODO(lucasmerlin): calculate the preferred size
                    SizedAtomicKind::Text(galley),
                )
            }
            AtomicKind::Image(image) => {
                let size = image.load_and_calc_size(ui, available_size);
                let size = size.unwrap_or(Vec2::ZERO);
                (size, SizedAtomicKind::Image(image, size))
            }
            AtomicKind::Custom(id, size) => (size, SizedAtomicKind::Custom(id, size)),
            AtomicKind::Empty => (Vec2::ZERO, SizedAtomicKind::Empty),
        }
    }
}

impl<'a> From<ImageSource<'a>> for AtomicKind<'a> {
    fn from(value: ImageSource<'a>) -> Self {
        AtomicKind::Image(value.into())
    }
}

impl<'a> From<Image<'a>> for AtomicKind<'a> {
    fn from(value: Image<'a>) -> Self {
        AtomicKind::Image(value)
    }
}

impl<T> From<T> for AtomicKind<'_>
where
    T: Into<WidgetText>,
{
    fn from(value: T) -> Self {
        AtomicKind::Text(value.into())
    }
}
