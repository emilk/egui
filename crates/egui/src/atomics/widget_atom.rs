use crate::{
    Align2, Color32, ContainerAtom, FontSelection, Frame, Id, IntoAtoms, Response, Sense,
    SizedContainerAtom, Ui, Widget,
};
use emath::{Rect, Vec2};
use epaint::text::TextWrapMode;
use smallvec::SmallVec;
use std::ops::{Deref, DerefMut};

/// An atom-based widget: a [`ContainerAtom`] plus everything needed to show it inside a [`Ui`].
///
/// The [`ContainerAtom`] defines how the [`crate::Atom`]s are laid out and painted (frame, sizes,
/// gap, alignment). The `WidgetAtom` wraps it and adds the [`Id`] and [`Sense`] used to allocate
/// a [`Response`] and interact. This is used internally by widgets like [`crate::Button`] and
/// [`crate::Checkbox`], and you can use it to make your own widgets.
///
/// Painting can be split in two phases:
/// - [`WidgetAtom::allocate`]
///   - measures the [`ContainerAtom`] (see [`ContainerAtom::measure`])
///   - allocates a [`Response`]
///   - returns an [`AllocatedWidgetAtom`]
/// - [`AllocatedWidgetAtom::paint`]
///   - paints the [`Frame`] and each single atom
///
/// You can use this to first allocate a response and then modify, e.g., the [`Frame`] on the
/// [`AllocatedWidgetAtom`] for interaction styling.
#[derive(Clone)]
pub struct WidgetAtom<'a> {
    id: Option<Id>,
    pub(crate) sense: Sense,
    pub container: ContainerAtom<'a>,
}

impl Default for WidgetAtom<'_> {
    fn default() -> Self {
        Self::new(())
    }
}

impl<'a> WidgetAtom<'a> {
    pub fn new(atoms: impl IntoAtoms<'a>) -> Self {
        Self {
            id: None,
            sense: Sense::hover(),
            container: ContainerAtom::new(atoms),
        }
    }

    /// Set the [`Id`] used to allocate a [`Response`].
    #[inline]
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the [`Sense`] used when allocating the [`Response`].
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// Set the gap between atoms.
    ///
    /// Default: `Spacing::icon_spacing`
    #[inline]
    pub fn gap(mut self, gap: f32) -> Self {
        self.container = self.container.gap(gap);
        self
    }

    /// Set the [`Frame`].
    #[inline]
    pub fn frame(mut self, frame: Frame) -> Self {
        self.container = self.container.frame(frame);
        self
    }

    /// Set the fallback (default) text color.
    ///
    /// Default: [`crate::Visuals::text_color`]
    #[inline]
    pub fn fallback_text_color(mut self, color: Color32) -> Self {
        self.container = self.container.fallback_text_color(color);
        self
    }

    /// Set the fallback (default) font.
    #[inline]
    pub fn fallback_font(mut self, font: impl Into<FontSelection>) -> Self {
        self.container = self.container.fallback_font(font);
        self
    }

    /// Set the minimum size of the Widget.
    ///
    /// This will find and expand atoms with `grow: true`.
    /// If there are no growable atoms then everything will be left-aligned.
    #[inline]
    pub fn min_size(mut self, size: Vec2) -> Self {
        self.container = self.container.min_size(size);
        self
    }

    /// Set the maximum size of the Widget.
    ///
    /// By default, the size is limited by the available size in the [`Ui`].
    #[inline]
    pub fn max_size(mut self, size: Vec2) -> Self {
        self.container = self.container.max_size(size);
        self
    }

    /// Set the maximum width of the Widget.
    ///
    /// By default, the width is limited by the available width in the [`Ui`].
    #[inline]
    pub fn max_width(mut self, width: f32) -> Self {
        self.container = self.container.max_width(width);
        self
    }

    /// Set the maximum height of the Widget.
    ///
    /// By default, the height is limited by the available height in the [`Ui`].
    #[inline]
    pub fn max_height(mut self, height: f32) -> Self {
        self.container = self.container.max_height(height);
        self
    }

    /// Set the [`TextWrapMode`] for the [`crate::Atom`] marked as `shrink`.
    ///
    /// Only a single [`crate::Atom`] may shrink. If this (or `ui.wrap_mode()`) is not
    /// [`TextWrapMode::Extend`] and no item is set to shrink, the first (left-most)
    /// [`crate::AtomKind::Text`] will be set to shrink.
    #[inline]
    pub fn wrap_mode(mut self, wrap_mode: TextWrapMode) -> Self {
        self.container = self.container.wrap_mode(wrap_mode);
        self
    }

    /// Set the [`Align2`].
    ///
    /// This will align the [`crate::Atom`]s within the [`Rect`] returned by [`Ui::allocate_space`].
    ///
    /// The default is chosen based on the [`Ui`]s [`crate::Layout`]. See
    /// [this snapshot](https://github.com/emilk/egui/blob/master/tests/egui_tests/tests/snapshots/layout/button.png)
    /// for info on how the [`crate::Layout`] affects the alignment.
    #[inline]
    pub fn align2(mut self, align2: Align2) -> Self {
        self.container = self.container.align2(align2);
        self
    }

    /// [`WidgetAtom::allocate`] and [`AllocatedWidgetAtom::paint`] in one go.
    pub fn show(self, ui: &mut Ui) -> WidgetAtomResponse {
        self.allocate(ui).paint(ui)
    }

    /// Measure the atoms (sizing only), without allocating space or interacting.
    ///
    /// This resolves the [`Id`] and measures the [`ContainerAtom`] (see
    /// [`ContainerAtom::measure`]), but unlike [`Self::allocate`] it does *not* call
    /// [`Ui::allocate_space`] nor [`Ui::interact`]. Use the returned [`SizedWidgetAtom`] to
    /// allocate later via [`SizedWidgetAtom::allocate`], or to paint at an arbitrary [`Rect`] via
    /// [`SizedWidgetAtom::paint_at`].
    ///
    /// `available_size` is the space available to the whole widget (frame included); it is
    /// clamped by `max_size`/`min_size`, exactly like [`Self::allocate`] does with
    /// [`Ui::available_size`].
    pub fn measure(self, ui: &Ui, available_size: Vec2) -> SizedWidgetAtom<'a> {
        let Self {
            id,
            sense,
            container,
        } = self;
        let id = id.unwrap_or_else(|| ui.next_auto_id());
        let container = container.measure(ui, available_size);
        SizedWidgetAtom {
            id,
            sense,
            container,
        }
    }

    /// Calculate sizes, create [`crate::Galley`]s and allocate a [`Response`].
    ///
    /// Use the returned [`AllocatedWidgetAtom`] for painting.
    pub fn allocate(self, ui: &mut Ui) -> AllocatedWidgetAtom<'a> {
        self.measure(ui, ui.available_size()).allocate(ui)
    }
}

/// A measured [`WidgetAtom`]: a [`SizedContainerAtom`] plus the [`Id`] and [`Sense`] needed to
/// allocate a [`Response`].
///
/// Produced by [`WidgetAtom::measure`]. Unlike [`AllocatedWidgetAtom`], it has not yet allocated
/// space or interacted. Call [`Self::allocate`] to do so, or [`Self::paint_at`] to interact and
/// paint at an arbitrary [`Rect`] (used when nesting one atom-based widget inside another).
#[derive(Clone, Debug)]
pub struct SizedWidgetAtom<'a> {
    /// The [`Id`] used to [`Ui::interact`] when this widget is allocated / painted.
    id: Id,

    /// The [`Sense`] used to [`Ui::interact`] when this widget is allocated / painted.
    sense: Sense,

    /// The measured container.
    pub container: SizedContainerAtom<'a>,
}

impl<'a> SizedWidgetAtom<'a> {
    /// Allocate space and interact, producing an [`AllocatedWidgetAtom`] ready for painting.
    pub fn allocate(self, ui: &mut Ui) -> AllocatedWidgetAtom<'a> {
        let (_, rect) = ui.allocate_space(self.container.outer_size);
        let mut response = ui.interact(rect, self.id, self.sense);
        response.set_intrinsic_size(self.container.intrinsic_size);

        AllocatedWidgetAtom {
            container: self.container,
            response,
        }
    }

    /// Interact at `rect` and paint the [`Frame`] and atoms there.
    ///
    /// Unlike [`Self::allocate`] this does not call [`Ui::allocate_space`]; it interacts at the
    /// given `rect` using this widget's [`Id`] and [`Sense`]. This is used when nesting one
    /// atom-based widget inside another.
    pub fn paint_at(self, ui: &Ui, rect: Rect) -> WidgetAtomResponse {
        let response = ui.interact(rect, self.id, self.sense);
        let custom_rects = self.container.paint_at(ui, rect);
        WidgetAtomResponse {
            response,
            custom_rects,
        }
    }
}

/// Instructions for painting a [`WidgetAtom`].
///
/// This is a [`SizedContainerAtom`] that has additionally allocated space and interacted,
/// producing a [`Response`].
#[derive(Clone, Debug)]
pub struct AllocatedWidgetAtom<'a> {
    /// The measured container.
    pub container: SizedContainerAtom<'a>,

    pub response: Response,
}

impl AllocatedWidgetAtom<'_> {
    /// Paint the [`Frame`] and individual [`crate::Atom`]s at the allocated [`Response`]'s rect.
    pub fn paint(self, ui: &Ui) -> WidgetAtomResponse {
        let rect = self.response.rect;
        let custom_rects = self.container.paint_at(ui, rect);
        WidgetAtomResponse {
            response: self.response,
            custom_rects,
        }
    }
}

/// Response from a [`WidgetAtom::show`] or [`AllocatedWidgetAtom::paint`].
///
/// Use [`WidgetAtomResponse::rect`] to get the response rects from [`crate::Atom::custom`].
#[derive(Clone, Debug)]
pub struct WidgetAtomResponse {
    pub response: Response,
    // There should rarely be more than one custom rect.
    pub(crate) custom_rects: SmallVec<[(Id, Rect); 1]>,
}

impl WidgetAtomResponse {
    pub fn empty(response: Response) -> Self {
        Self {
            response,
            custom_rects: Default::default(),
        }
    }

    pub fn custom_rects(&self) -> impl Iterator<Item = (Id, Rect)> + '_ {
        self.custom_rects.iter().copied()
    }

    /// Use this together with [`crate::Atom::custom`] to add custom painting / child widgets.
    ///
    /// NOTE: Don't `unwrap` rects, they might be empty when the widget is not visible.
    pub fn rect(&self, id: Id) -> Option<Rect> {
        self.custom_rects
            .iter()
            .find_map(|(i, r)| if *i == id { Some(*r) } else { None })
    }
}

impl Deref for WidgetAtomResponse {
    type Target = Response;

    fn deref(&self) -> &Self::Target {
        &self.response
    }
}

impl DerefMut for WidgetAtomResponse {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.response
    }
}

impl Widget for WidgetAtom<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

impl<'a> Deref for WidgetAtom<'a> {
    type Target = ContainerAtom<'a>;

    fn deref(&self) -> &Self::Target {
        &self.container
    }
}

impl DerefMut for WidgetAtom<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.container
    }
}

impl<'a> Deref for SizedWidgetAtom<'a> {
    type Target = SizedContainerAtom<'a>;

    fn deref(&self) -> &Self::Target {
        &self.container
    }
}

impl DerefMut for SizedWidgetAtom<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.container
    }
}

impl<'a> Deref for AllocatedWidgetAtom<'a> {
    type Target = SizedContainerAtom<'a>;

    fn deref(&self) -> &Self::Target {
        &self.container
    }
}

impl DerefMut for AllocatedWidgetAtom<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.container
    }
}

/// `AtomLayout` was split into [`WidgetAtom`] (id, sense, allocation) and [`ContainerAtom`]
/// (layout & painting). [`WidgetAtom`] is the direct replacement.
#[deprecated = "Renamed to `WidgetAtom`"]
pub type AtomLayout<'a> = WidgetAtom<'a>;

/// `SizedAtomLayout` was split into [`SizedWidgetAtom`] (id, sense) and [`SizedContainerAtom`]
/// (the measured contents). [`SizedWidgetAtom`] is the direct replacement.
#[deprecated = "Renamed to `SizedWidgetAtom`"]
pub type SizedAtomLayout<'a> = SizedWidgetAtom<'a>;

/// Renamed to [`AllocatedWidgetAtom`].
#[deprecated = "Renamed to `AllocatedWidgetAtom`"]
pub type AllocatedAtomLayout<'a> = AllocatedWidgetAtom<'a>;

/// Renamed to [`WidgetAtomResponse`].
#[deprecated = "Renamed to `WidgetAtomResponse`"]
pub type AtomLayoutResponse = WidgetAtomResponse;
