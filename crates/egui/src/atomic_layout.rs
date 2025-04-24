use crate::{
    FontSelection, Frame, Id, Image, Response, Sense, Style, TextStyle, Ui, Widget, WidgetText,
};
use ahash::{HashMap, HashMapExt};
use emath::{Align2, NumExt, Rect, Vec2};
use epaint::text::TextWrapMode;
use epaint::{Color32, Fonts, Galley};
use std::fmt::Formatter;
use std::sync::Arc;

/// Intra-widget layout utility.
///
/// Used to lay out and paint [`Atomic`]s.
/// This is used internally by widgets like [`crate::Button`] and [`crate::Checkbox`].
/// You can use it to make your own widgets.
///
/// Painting the atomics can be split in two phases:
/// - [`AtomicLayout::allocate`]
///   - calculates sizes
///   - converts texts to [`Galley`]s
///   - allocates a [`Response`]
///   - returns a [`AllocatedAtomicLayout`]
/// - [`AllocatedAtomicLayout::paint`]
///   - paints the [`Frame`]
///   - calculates individual [`Atomic`] positions
///   - paints each single atomic
///
/// You can use this to first allocate a response and then modify, e.g., the [`Frame`] on the
/// [`AllocatedAtomicLayout`] for interaction styling.
pub struct AtomicLayout<'a> {
    id: Option<Id>,
    pub atomics: Atomics<'a>,
    gap: Option<f32>,
    pub(crate) frame: Frame,
    pub(crate) sense: Sense,
    fallback_text_color: Option<Color32>,
    min_size: Vec2,
    wrap_mode: Option<TextWrapMode>,
    align2: Option<Align2>,
}

impl Default for AtomicLayout<'_> {
    fn default() -> Self {
        Self::new(())
    }
}

impl<'a> AtomicLayout<'a> {
    pub fn new(atomics: impl IntoAtomics<'a>) -> Self {
        Self {
            id: None,
            atomics: atomics.into_atomics(),
            gap: None,
            frame: Frame::default(),
            sense: Sense::hover(),
            fallback_text_color: None,
            min_size: Vec2::ZERO,
            wrap_mode: None,
            align2: None,
        }
    }

    /// Insert a new [`Atomic`] at the end of the list (left side).
    pub fn push(mut self, atomic: impl Into<Atomic<'a>>) -> Self {
        self.atomics.push(atomic.into());
        self
    }

    /// Insert a new [`Atomic`] at the beginning of the list (right side).
    pub fn push_front(mut self, atomic: impl Into<Atomic<'a>>) -> Self {
        self.atomics.push_front(atomic.into());
        self
    }

    /// Set the gap between atomics.
    ///
    /// Default: `Spacing::icon_spacing`
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = Some(gap);
        self
    }

    /// Set the [`Frame`].
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = frame;
        self
    }

    /// Set the [`Sense`] used when allocating the [`Response`].
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// Set the fallback (default) text color.
    ///
    /// Default: [`crate::Visuals::text_color`]
    pub fn fallback_text_color(mut self, color: Color32) -> Self {
        self.fallback_text_color = Some(color);
        self
    }

    /// Set the minimum size of the Widget.
    pub fn min_size(mut self, size: Vec2) -> Self {
        self.min_size = size;
        self
    }

    /// Set the [`Id`] used to allocate a [`Response`].
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the [`TextWrapMode`] for the [`Atomic`] marked as `shrink`.
    ///
    /// Only a single [`Atomic`] may shrink. If this (or `ui.wrap_mode()`) is not
    /// [`TextWrapMode::Extend`] and no item is set to shrink, the first (right-most)
    /// [`AtomicKind::Text`] will be set to shrink.
    pub fn wrap_mode(mut self, wrap_mode: TextWrapMode) -> Self {
        self.wrap_mode = Some(wrap_mode);
        self
    }

    /// Set the [`Align2`].
    ///
    /// The default is chosen based on the [`Ui`]s [`crate::Layout`]. See
    /// [this snapshot](https://github.com/emilk/egui/blob/master/tests/egui_tests/tests/snapshots/layout/button.png)
    /// for info on how the [`Layout`] affects the alignment.
    pub fn align2(mut self, align2: Align2) -> Self {
        self.align2 = Some(align2);
        self
    }

    /// [`AtomicLayout::allocate`] and [`AllocatedAtomicLayout::paint`] in one go.
    pub fn show(self, ui: &mut Ui) -> AtomicLayoutResponse {
        self.allocate(ui).paint(ui)
    }

    /// Calculate sizes, create [`Galley`]s and allocate a [`Response`].
    ///
    /// Use the returned [`AllocatedAtomicLayout`] for painting.
    pub fn allocate(self, ui: &mut Ui) -> AllocatedAtomicLayout<'a> {
        let Self {
            id,
            mut atomics,
            gap,
            frame,
            sense,
            fallback_text_color,
            min_size,
            wrap_mode,
            align2,
        } = self;

        let wrap_mode = wrap_mode.unwrap_or(ui.wrap_mode());

        // If the TextWrapMode is not Extend, ensure there is some item marked as `shrink`.
        // If none is found, mark the first text item as `shrink`.
        if !matches!(wrap_mode, TextWrapMode::Extend) {
            let any_shrink = atomics.iter().any(|a| a.shrink);
            if !any_shrink {
                let first_text = atomics
                    .iter_mut()
                    .find(|a| matches!(a.kind, AtomicKind::Text(..)));
                if let Some(atomic) = first_text {
                    atomic.shrink = true;
                }
            }
        }

        let id = id.unwrap_or_else(|| ui.next_auto_id());

        let fallback_text_color =
            fallback_text_color.unwrap_or_else(|| ui.style().visuals.text_color());
        let gap = gap.unwrap_or(ui.spacing().icon_spacing);

        // The size available for the content
        let available_inner_size = ui.available_size() - frame.total_margin().sum();

        let mut desired_width = 0.0;
        let mut preferred_width = 0.0;
        let mut preferred_height = 0.0;

        let mut height: f32 = 0.0;

        let mut sized_items = Vec::new();

        let mut grow_count = 0;

        let mut shrink_item = None;

        let align2 = align2.unwrap_or_else(|| {
            Align2([ui.layout().horizontal_align(), ui.layout().vertical_align()])
        });

        if atomics.0.len() > 1 {
            let gap_space = gap * (atomics.0.len() as f32 - 1.0);
            desired_width += gap_space;
            preferred_width += gap_space;
        }

        let default_font_height = || {
            let font_selection = FontSelection::default();
            let font_id = font_selection.resolve(ui.style());
            ui.fonts(|f| f.row_height(&font_id))
        };

        let max_font_size = ui
            .fonts(|fonts| {
                atomics
                    .0
                    .iter()
                    .filter_map(|a| a.get_min_height_for_image(fonts, ui.style()))
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            })
            .unwrap_or_else(default_font_height);

        for (idx, item) in atomics.0.into_iter().enumerate() {
            if item.shrink {
                debug_assert!(
                    shrink_item.is_none(),
                    "Only one atomic may be marked as shrink"
                );
                if shrink_item.is_none() {
                    shrink_item = Some((idx, item));
                    continue;
                }
            }
            if item.grow {
                grow_count += 1;
            }
            let sized = item.into_sized(ui, available_inner_size, max_font_size, Some(wrap_mode));
            let size = sized.size;

            desired_width += size.x;
            preferred_width += sized.preferred_size.x;

            height = height.at_least(size.y);
            preferred_height = preferred_height.at_least(sized.preferred_size.y);

            sized_items.push(sized);
        }

        if let Some((index, item)) = shrink_item {
            // The `shrink` item gets the remaining space
            let shrunk_size = Vec2::new(
                available_inner_size.x - desired_width,
                available_inner_size.y,
            );
            let sized = item.into_sized(ui, shrunk_size, max_font_size, Some(wrap_mode));
            let size = sized.size;

            desired_width += size.x;
            preferred_width += sized.preferred_size.x;

            height = height.at_least(size.y);
            preferred_height = preferred_height.at_least(sized.preferred_size.y);

            sized_items.insert(index, sized);
        }

        let margin = frame.total_margin();
        let desired_size = Vec2::new(desired_width, height);
        let frame_size = (desired_size + margin.sum()).at_least(min_size);

        let (_, rect) = ui.allocate_space(frame_size);
        let mut response = ui.interact(rect, id, sense);

        response.intrinsic_size =
            Some((Vec2::new(preferred_width, preferred_height) + margin.sum()).at_least(min_size));

        AllocatedAtomicLayout {
            sized_atomics: sized_items,
            frame,
            fallback_text_color,
            response,
            grow_count,
            desired_size,
            align2,
            gap,
        }
    }
}

/// Instructions for painting an [`AtomicLayout`].
#[derive(Clone, Debug)]
pub struct AllocatedAtomicLayout<'a> {
    pub sized_atomics: Vec<SizedAtomic<'a>>,
    pub frame: Frame,
    pub fallback_text_color: Color32,
    pub response: Response,
    grow_count: usize,
    // The size of the inner content, before any growing.
    desired_size: Vec2,
    align2: Align2,
    gap: f32,
}

impl<'a> AllocatedAtomicLayout<'a> {
    /// Paint the [`Frame`] and individual [`Atomic`]s.
    pub fn paint(self, ui: &Ui) -> AtomicLayoutResponse {
        let Self {
            sized_atomics: sized_items,
            frame,
            fallback_text_color,
            response,
            grow_count,
            desired_size,
            align2,
            gap,
        } = self;

        let inner_rect = response.rect - self.frame.total_margin();

        ui.painter().add(frame.paint(inner_rect));

        let width_to_fill = inner_rect.width();
        let extra_space = f32::max(width_to_fill - desired_size.x, 0.0);
        let grow_width = f32::max(extra_space / grow_count as f32, 0.0);

        let aligned_rect = if grow_count > 0 {
            align2.align_size_within_rect(Vec2::new(width_to_fill, desired_size.y), inner_rect)
        } else {
            align2.align_size_within_rect(desired_size, inner_rect)
        };

        let mut cursor = aligned_rect.left();

        let mut response = AtomicLayoutResponse {
            response,
            custom_rects: HashMap::new(),
        };

        for sized in sized_items {
            let size = sized.size;
            let growth = if sized.grow { grow_width } else { 0.0 };

            let frame = aligned_rect
                .with_min_x(cursor)
                .with_max_x(cursor + size.x + growth);
            cursor = frame.right() + gap;

            let align = Align2::CENTER_CENTER;
            let rect = align.align_size_within_rect(size, frame);

            match sized.kind {
                SizedAtomicKind::Text(galley) => {
                    ui.painter().galley(rect.min, galley, fallback_text_color);
                }
                SizedAtomicKind::Image(image, _) => {
                    image.paint_at(ui, rect);
                }
                SizedAtomicKind::Custom(id, _) => {
                    response.custom_rects.insert(id, rect);
                }
                SizedAtomicKind::Empty => {}
            }
        }

        response
    }
}

/// Response from a [`AtomicLayout::show`] or [`AllocatedAtomicLayout::paint`].
///
/// Use the `custom_rects` together with [`AtomicKind::Custom`] to add child widgets to a widget.
///
/// NOTE: Don't `unwrap` rects, they might be empty when the widget is not visible.
#[derive(Clone, Debug)]
pub struct AtomicLayoutResponse {
    pub response: Response,
    pub custom_rects: HashMap<Id, Rect>,
}

/// The different kinds of [`Atomic`]s.
#[derive(Clone, Default)]
pub enum AtomicKind<'a> {
    /// Empty, that can be used with [`Atomic::a_grow`] to reserve space.
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
                    galley.size(), // TODO
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

/// A sized [`AtomicKind`].
#[derive(Clone, Default, Debug)]
pub enum SizedAtomicKind<'a> {
    #[default]
    Empty,
    Text(Arc<Galley>),
    Image(Image<'a>, Vec2),
    Custom(Id, Vec2),
}

impl SizedAtomicKind<'_> {
    /// Get the calculated size.
    pub fn size(&self) -> Vec2 {
        match self {
            SizedAtomicKind::Text(galley) => galley.size(),
            SizedAtomicKind::Image(_, size) | SizedAtomicKind::Custom(_, size) => *size,
            SizedAtomicKind::Empty => Vec2::ZERO,
        }
    }
}

/// A low-level ui building block.
///
/// Implements [`From`] for [`String`], [`str`], [`Image`] and much more for convenience.
/// You can directly call the `a_*` methods on anything that implements `Into<Atomic>`.
/// ```
/// # use egui::{Image, emath::Vec2};
/// use egui::AtomicExt;
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

/// A [`Atomic`] which has been sized.
#[derive(Clone, Debug)]
pub struct SizedAtomic<'a> {
    pub grow: bool,
    pub size: Vec2,
    pub preferred_size: Vec2,
    pub kind: SizedAtomicKind<'a>,
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
    fn get_min_height_for_image(&self, fonts: &Fonts, style: &Style) -> Option<f32> {
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

/// A list of [`Atomic`]s.
#[derive(Clone, Debug, Default)]
pub struct Atomics<'a>(Vec<Atomic<'a>>);

impl<'a> Atomics<'a> {
    pub fn push(&mut self, atomic: impl Into<Atomic<'a>>) {
        self.0.push(atomic.into());
    }

    pub fn push_front(&mut self, atomic: impl Into<Atomic<'a>>) {
        self.0.insert(0, atomic.into());
    }

    pub fn iter(&self) -> impl Iterator<Item = &Atomic<'a>> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Atomic<'a>> {
        self.0.iter_mut()
    }

    /// Concatenate and return the text contents.
    // TODO(lucasmerlin): It might not always make sense to return the concatenated text, e.g.
    // in a submenu button there is a right text '⏵' which is now passed to the screen reader.
    pub fn text(&self) -> Option<String> {
        let mut string: Option<String> = None;
        for atomic in &self.0 {
            if let AtomicKind::Text(text) = &atomic.kind {
                if let Some(string) = &mut string {
                    string.push(' ');
                    string.push_str(text.text());
                } else {
                    string = Some(text.text().to_owned());
                }
            }
        }
        string
    }
}

/// Helper trait to convert a tuple of atomics into [`Atomics`].
///
/// ```
/// use egui::{Atomics, Image, IntoAtomics, RichText};
/// let atomics: Atomics = (
///     "Some text",
///     RichText::new("Some RichText"),
///     Image::new("some_image_url"),
/// ).into_atomics();
/// ```
impl<'a, T> IntoAtomics<'a> for T
where
    T: Into<Atomic<'a>>,
{
    fn collect(self, atomics: &mut Atomics<'a>) {
        atomics.push(self);
    }
}

pub trait IntoAtomics<'a> {
    fn collect(self, atomics: &mut Atomics<'a>);

    fn into_atomics(self) -> Atomics<'a>
    where
        Self: Sized,
    {
        let mut atomics = Atomics(Vec::new());
        self.collect(&mut atomics);
        atomics
    }
}

impl<'a> IntoAtomics<'a> for Atomics<'a> {
    fn collect(self, atomics: &mut Self) {
        atomics.0.extend(self.0);
    }
}

macro_rules! all_the_atomics {
    ($($T:ident),*) => {
        impl<'a, $($T),*> IntoAtomics<'a> for ($($T),*)
        where
            $($T: IntoAtomics<'a>),*
        {
            fn collect(self, _atomics: &mut Atomics<'a>) {
                #[allow(non_snake_case)]
                let ($($T),*) = self;
                $($T.collect(_atomics);)*
            }
        }
    };
}

all_the_atomics!();
all_the_atomics!(T0, T1);
all_the_atomics!(T0, T1, T2);
all_the_atomics!(T0, T1, T2, T3);
all_the_atomics!(T0, T1, T2, T3, T4);
all_the_atomics!(T0, T1, T2, T3, T4, T5);

impl Widget for AtomicLayout<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}
