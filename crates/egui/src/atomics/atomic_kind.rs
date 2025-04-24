use crate::{Id, Image, SizedAtomicKind, TextStyle, Ui, WidgetText};
use emath::Vec2;
use epaint::text::TextWrapMode;
use std::fmt::Formatter;

/// The different kinds of [`Atomic`]s.
#[derive(Clone, Default)]
pub enum AtomicKind<'a> {
    /// Empty, that can be used with [`AtomicExt::a_grow`] to reserve space.
    #[default]
    Empty,
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
    /// let rect = response.custom_rects.get(&id);
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
        font_size: f32,
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
                let max_size = Vec2::splat(font_size);
                let size = image.load_and_calc_size(ui, Vec2::min(available_size, max_size));
                let size = size.unwrap_or(max_size);
                (size, SizedAtomicKind::Image(image, size))
            }
            AtomicKind::Custom(id, size) => (size, SizedAtomicKind::Custom(id, size)),
            AtomicKind::Empty => (Vec2::ZERO, SizedAtomicKind::Empty),
        }
    }
}

impl<'a> From<Image<'a>> for AtomicKind<'a> {
    fn from(value: Image<'a>) -> Self {
        AtomicKind::Image(value)
    }
}

impl<'a, T> From<T> for AtomicKind<'a>
where
    T: Into<WidgetText>,
{
    fn from(value: T) -> Self {
        AtomicKind::Text(value.into())
    }
}
