use crate::{
    AtomKind, Atoms, FontSelection, Frame, Id, Image, IntoAtoms, SizedAtom, SizedAtomKind, Ui,
    WidgetAtom,
};
use emath::{Align2, GuiRounding as _, NumExt as _, Rect, Vec2};
use epaint::text::TextWrapMode;
use epaint::{Color32, Galley};
use smallvec::SmallVec;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// The custom [`crate::Atom`] rects collected while painting, keyed by [`crate::Atom::custom`] id.
///
/// There should rarely be more than one.
pub type CustomRects = SmallVec<[(Id, Rect); 1]>;

/// Describes how a set of [`crate::Atom`]s is laid out and painted.
///
/// This is the container part of an atom-based widget: it owns the [`Atoms`], the [`Frame`]
/// painted around them, the sizing constraints (`min_size` / `max_size`), the gap between
/// atoms, alignment and text styling. It knows nothing about how the widget is shown inside a
/// [`Ui`] (that is the job of [`crate::WidgetAtom`], which wraps a `ContainerAtom` and adds an
/// [`Id`](crate::Id) and a [`Sense`](crate::Sense)).
///
/// Painting the atoms is split in two phases:
/// - [`ContainerAtom::measure`]
///   - calculates sizes
///   - converts texts to [`Galley`]s
///   - returns a [`SizedContainerAtom`]
/// - [`SizedContainerAtom::paint_at`]
///   - paints the [`Frame`]
///   - calculates individual [`crate::Atom`] positions
///   - paints each single atom
#[derive(Clone)]
pub struct ContainerAtom<'a> {
    pub atoms: Atoms<'a>,
    gap: Option<f32>,
    pub(crate) frame: Frame,
    fallback_text_color: Option<Color32>,
    fallback_font: Option<FontSelection>,
    min_size: Vec2,
    max_size: Vec2,
    wrap_mode: Option<TextWrapMode>,
    align2: Option<Align2>,
}

impl Default for ContainerAtom<'_> {
    fn default() -> Self {
        Self::new(())
    }
}

impl<'a> ContainerAtom<'a> {
    pub fn new(atoms: impl IntoAtoms<'a>) -> Self {
        Self {
            atoms: atoms.into_atoms(),
            gap: None,
            frame: Frame::default(),
            fallback_text_color: None,
            fallback_font: None,
            min_size: Vec2::ZERO,
            max_size: Vec2::INFINITY,
            wrap_mode: None,
            align2: None,
        }
    }

    /// Measure the atoms (sizing only), without allocating space or interacting.
    ///
    /// This converts texts to [`Galley`]s and calculates sizes, but it does *not* call
    /// [`Ui::allocate_space`] (so the parent cursor is left untouched) nor [`Ui::interact`].
    /// Use the returned [`SizedContainerAtom`] to paint at an arbitrary [`Rect`] via
    /// [`SizedContainerAtom::paint_at`]. This is what makes it possible to nest one atom-based
    /// widget inside another.
    ///
    /// `available_size` is the space available to the whole widget (frame included); it is
    /// clamped by `max_size`/`min_size`, exactly like [`crate::WidgetAtom::allocate`] does with
    /// [`Ui::available_size`].
    pub fn measure(self, ui: &Ui, available_size: Vec2) -> SizedContainerAtom<'a> {
        let Self {
            mut atoms,
            gap,
            frame,
            fallback_text_color,
            min_size,
            mut max_size,
            wrap_mode,
            align2,
            fallback_font,
        } = self;

        let fallback_font = fallback_font.unwrap_or_default();

        let wrap_mode = wrap_mode.unwrap_or_else(|| ui.wrap_mode());

        // If the TextWrapMode is not Extend, ensure there is some item marked as `shrink`.
        // If none is found, mark the first text item as `shrink`.
        if wrap_mode != TextWrapMode::Extend {
            let any_shrink = atoms.any_shrink();
            if !any_shrink {
                let first_text = atoms
                    .iter_mut()
                    .find(|a| matches!(a.kind, AtomKind::Text(..)));
                if let Some(atom) = first_text {
                    atom.shrink = true; // Will make the text truncate or shrink depending on wrap_mode
                }
            }
        }

        let fallback_text_color =
            fallback_text_color.unwrap_or_else(|| ui.style().visuals.text_color());
        let gap = gap.unwrap_or_else(|| ui.spacing().icon_spacing);

        // max_size has no effect in justified layouts. If we'd limit the available size here,
        // the content would be sized differently than the frame which would look weird.
        if ui.layout().horizontal_justify() {
            max_size.x = f32::INFINITY;
        }

        let available_size = available_size.at_most(max_size).at_least(min_size);

        // The size available for the content
        let available_inner_size = available_size - frame.total_margin().sum();

        let mut inner_width = 0.0;

        // intrinsic width / height is the ideal size of the widget, e.g. the size where the
        // text is not wrapped. Used to set Response::intrinsic_size.
        let mut intrinsic_width = 0.0;
        let mut intrinsic_height = 0.0;

        let mut height: f32 = 0.0;

        let mut sized_items = Vec::new();

        let mut grow_count = 0;

        let mut shrink_item = None;

        let align2 = align2.unwrap_or_else(|| {
            Align2([ui.layout().horizontal_align(), ui.layout().vertical_align()])
        });

        if atoms.len() > 1 {
            let gap_space = gap * (atoms.len() as f32 - 1.0);
            inner_width += gap_space;
            intrinsic_width += gap_space;
        }

        for (idx, item) in atoms.into_iter().enumerate() {
            if item.grow {
                grow_count += 1;
            }
            if item.shrink {
                debug_assert!(
                    shrink_item.is_none(),
                    "Only one atomic may be marked as shrink. {item:?}"
                );
                if shrink_item.is_none() {
                    shrink_item = Some((idx, item));
                    continue;
                }
            }
            let sized = item.into_sized(
                ui,
                available_inner_size,
                Some(wrap_mode),
                fallback_font.clone(),
            );
            let size = sized.size;

            inner_width += size.x;
            intrinsic_width += sized.intrinsic_size.x;

            height = height.at_least(size.y);
            intrinsic_height = intrinsic_height.at_least(sized.intrinsic_size.y);

            sized_items.push(sized);
        }

        if let Some((index, item)) = shrink_item {
            // The `shrink` item gets the remaining space
            let available_size_for_shrink_item =
                Vec2::new(available_inner_size.x - inner_width, available_inner_size.y);

            let sized = item.into_sized(
                ui,
                available_size_for_shrink_item,
                Some(wrap_mode),
                fallback_font,
            );
            let size = sized.size;

            inner_width += size.x;
            intrinsic_width += sized.intrinsic_size.x;

            height = height.at_least(size.y);
            intrinsic_height = intrinsic_height.at_least(sized.intrinsic_size.y);

            sized_items.insert(index, sized);
        }

        let margin = frame.total_margin();
        let inner_size = Vec2::new(inner_width, height);
        let outer_size = (inner_size + margin.sum()).at_least(min_size);
        let intrinsic_size =
            (Vec2::new(intrinsic_width, intrinsic_height) + margin.sum()).at_least(min_size);

        SizedContainerAtom {
            sized_atoms: sized_items,
            frame,
            fallback_text_color,
            outer_size,
            intrinsic_size,
            grow_count,
            inner_size,
            align2,
            gap,
        }
    }
}

/// Generates the layout-builder methods shared by [`ContainerAtom`] and [`WidgetAtom`] from a
/// single definition, so the two can never drift apart.
///
/// Each entry is written as it appears on [`ContainerAtom`] (mutating its own fields). The
/// matching method on [`WidgetAtom`] is generated automatically, forwarding to its inner
/// `container`. [`WidgetAtom`]-only builders (`id`, `sense`) stay inherent on [`WidgetAtom`].
macro_rules! shared_container_builders {
    (
        $(
            $(#[$meta:meta])*
            fn $name:ident($self:ident, $($arg:ident: $arg_ty:ty),* $(,)?) $body:block
        )*
    ) => {
        impl<'a> ContainerAtom<'a> {
            $(
                $(#[$meta])*
                #[inline]
                pub fn $name(mut $self, $($arg: $arg_ty),*) -> Self {
                    $body
                    $self
                }
            )*
        }

        impl<'a> WidgetAtom<'a> {
            $(
                $(#[$meta])*
                #[inline]
                pub fn $name(mut self, $($arg: $arg_ty),*) -> Self {
                    self.container = self.container.$name($($arg),*);
                    self
                }
            )*
        }
    };
}

shared_container_builders! {
    /// Set the gap between atoms.
    ///
    /// Default: `Spacing::icon_spacing`
    fn gap(self, gap: f32) {
        self.gap = Some(gap);
    }

    /// Set the [`Frame`].
    fn frame(self, frame: Frame) {
        self.frame = frame;
    }

    /// Set the fallback (default) text color.
    ///
    /// Default: [`crate::Visuals::text_color`]
    fn fallback_text_color(self, color: Color32) {
        self.fallback_text_color = Some(color);
    }

    /// Set the fallback (default) font.
    fn fallback_font(self, font: impl Into<FontSelection>) {
        self.fallback_font = Some(font.into());
    }

    /// Set the minimum size of the Widget.
    ///
    /// This will find and expand atoms with `grow: true`.
    /// If there are no growable atoms then everything will be left-aligned.
    fn min_size(self, size: Vec2) {
        self.min_size = size;
    }

    /// Set the maximum size of the Widget.
    ///
    /// By default, the size is limited by the available size in the [`Ui`].
    fn max_size(self, size: Vec2) {
        self.max_size = size;
    }

    /// Set the maximum width of the Widget.
    ///
    /// By default, the width is limited by the available width in the [`Ui`].
    fn max_width(self, width: f32) {
        self.max_size.x = width;
    }

    /// Set the maximum height of the Widget.
    ///
    /// By default, the height is limited by the available height in the [`Ui`].
    fn max_height(self, height: f32) {
        self.max_size.y = height;
    }

    /// Set the [`TextWrapMode`] for the [`crate::Atom`] marked as `shrink`.
    ///
    /// Only a single [`crate::Atom`] may shrink. If this (or `ui.wrap_mode()`) is not
    /// [`TextWrapMode::Extend`] and no item is set to shrink, the first (left-most)
    /// [`AtomKind::Text`] will be set to shrink.
    fn wrap_mode(self, wrap_mode: TextWrapMode) {
        self.wrap_mode = Some(wrap_mode);
    }

    /// Set the [`Align2`].
    ///
    /// This will align the [`crate::Atom`]s within the [`Rect`] returned by [`Ui::allocate_space`].
    ///
    /// The default is chosen based on the [`Ui`]s [`crate::Layout`]. See
    /// [this snapshot](https://github.com/emilk/egui/blob/master/tests/egui_tests/tests/snapshots/layout/button.png)
    /// for info on how the [`crate::Layout`] affects the alignment.
    fn align2(self, align2: Align2) {
        self.align2 = Some(align2);
    }
}

/// A measured [`ContainerAtom`], ready to be painted at a [`Rect`].
///
/// Produced by [`ContainerAtom::measure`]. It has not yet allocated space or interacted, so it
/// can be painted at an arbitrary [`Rect`] via [`Self::paint_at`]. This is what lets one
/// atom-based widget be nested inside another. To allocate space and interact, wrap it in a
/// [`crate::SizedWidgetAtom`] (or measure a [`crate::WidgetAtom`] directly).
#[derive(Clone, Debug)]
pub struct SizedContainerAtom<'a> {
    /// The total widget size we'll request, including the frame margin. Used to allocate space.
    ///
    /// Actual allocated size may be different.
    pub(crate) outer_size: Vec2,

    /// The size of the inner content, before any growing.
    inner_size: Vec2,

    /// The contents.
    sized_atoms: Vec<SizedAtom<'a>>,

    /// The [`Frame`] painted around the contents.
    pub frame: Frame,

    /// Set the fallback (default) text color.
    pub fallback_text_color: Color32,

    /// The intrinsic (un-wrapped, un-grown) size, including margin. Used for
    /// [`Response::set_intrinsic_size`].
    pub(crate) intrinsic_size: Vec2,

    /// How many atoms were marked as `grow`?
    grow_count: usize,

    /// How will all the atoms be aligned within the allocated rect?
    align2: Align2,

    /// The gap between each [`crate::Atom`]
    gap: f32,
}

impl<'atom> SizedContainerAtom<'atom> {
    pub fn iter_kinds(&self) -> impl Iterator<Item = &SizedAtomKind<'atom>> {
        self.sized_atoms.iter().map(|atom| &atom.kind)
    }

    pub fn iter_kinds_mut(&mut self) -> impl Iterator<Item = &mut SizedAtomKind<'atom>> {
        self.sized_atoms.iter_mut().map(|atom| &mut atom.kind)
    }

    pub fn iter_images(&self) -> impl Iterator<Item = &Image<'atom>> {
        self.iter_kinds().filter_map(|kind| {
            if let SizedAtomKind::Image { image, size: _ } = kind {
                Some(image)
            } else {
                None
            }
        })
    }

    pub fn iter_images_mut(&mut self) -> impl Iterator<Item = &mut Image<'atom>> {
        self.iter_kinds_mut().filter_map(|kind| {
            if let SizedAtomKind::Image { image, size: _ } = kind {
                Some(image)
            } else {
                None
            }
        })
    }

    pub fn iter_texts(&self) -> impl Iterator<Item = &Arc<Galley>> + use<'atom, '_> {
        self.iter_kinds().filter_map(|kind| {
            if let SizedAtomKind::Text(text) = kind {
                Some(text)
            } else {
                None
            }
        })
    }

    pub fn iter_texts_mut(&mut self) -> impl Iterator<Item = &mut Arc<Galley>> + use<'atom, '_> {
        self.iter_kinds_mut().filter_map(|kind| {
            if let SizedAtomKind::Text(text) = kind {
                Some(text)
            } else {
                None
            }
        })
    }

    pub fn map_kind<F>(&mut self, mut f: F)
    where
        F: FnMut(SizedAtomKind<'atom>) -> SizedAtomKind<'atom>,
    {
        for kind in self.iter_kinds_mut() {
            *kind = f(std::mem::take(kind));
        }
    }

    pub fn map_images<F>(&mut self, mut f: F)
    where
        F: FnMut(Image<'atom>) -> Image<'atom>,
    {
        self.map_kind(|kind| {
            if let SizedAtomKind::Image { image, size } = kind {
                SizedAtomKind::Image {
                    image: f(image),
                    size,
                }
            } else {
                kind
            }
        });
    }

    /// Paint the [`Frame`] and individual [`crate::Atom`]s within `rect`.
    ///
    /// `rect` is the full widget rect (frame included). For a top-level layout this is
    /// `response.rect`; when nested, the parent passes the cell rect it computed.
    ///
    /// Returns the [`CustomRects`] collected from [`crate::Atom::custom`] atoms, so the caller
    /// can build an [`crate::WidgetAtomResponse`].
    pub fn paint_at(self, ui: &Ui, rect: Rect) -> CustomRects {
        let Self {
            sized_atoms,
            frame,
            fallback_text_color,
            grow_count,
            inner_size,
            align2,
            gap,
            ..
        } = self;

        let inner_rect = rect - frame.total_margin();

        ui.painter().add(frame.paint(inner_rect));

        let width_to_fill = inner_rect.width();
        let extra_space = f32::max(width_to_fill - inner_size.x, 0.0);
        let grow_width = f32::max(extra_space / grow_count as f32, 0.0).floor_ui();

        let aligned_rect = if grow_count > 0 {
            align2.align_size_within_rect(Vec2::new(width_to_fill, inner_size.y), inner_rect)
        } else {
            align2.align_size_within_rect(inner_size, inner_rect)
        };

        let mut cursor = aligned_rect.left();

        let mut custom_rects = CustomRects::new();

        for sized in sized_atoms {
            let size = sized.size;
            // TODO(lucasmerlin): This is not ideal, since this might lead to accumulated rounding errors
            // https://github.com/emilk/egui/pull/5830#discussion_r2079627864
            let growth = if sized.is_grow() { grow_width } else { 0.0 };

            let frame = aligned_rect
                .with_min_x(cursor)
                .with_max_x(cursor + size.x + growth);
            cursor = frame.right() + gap;
            let rect = sized.align.align_size_within_rect(size, frame);

            if let Some(id) = sized.id {
                debug_assert!(
                    !custom_rects.iter().any(|(i, _)| *i == id),
                    "Duplicate custom id"
                );
                custom_rects.push((id, rect));
            }

            match sized.kind {
                SizedAtomKind::Text(galley) => {
                    ui.painter().galley(rect.min, galley, fallback_text_color);
                }
                SizedAtomKind::Image { image, size: _ } => {
                    image.paint_at(ui, rect);
                }
                SizedAtomKind::Empty { .. } => {}
                SizedAtomKind::Widget(widget) => {
                    // TODO(lucasmerlin): Add some kind of justify flag to the layout
                    widget.paint_at(ui, frame);
                }
                SizedAtomKind::Container(container) => {
                    // A nested container has no id/sense, so it is painted but not interacted with.
                    container.paint_at(ui, frame);
                }
            }
        }

        custom_rects
    }
}

impl<'a> Deref for ContainerAtom<'a> {
    type Target = Atoms<'a>;

    fn deref(&self) -> &Self::Target {
        &self.atoms
    }
}

impl DerefMut for ContainerAtom<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.atoms
    }
}

impl<'a> Deref for SizedContainerAtom<'a> {
    type Target = [SizedAtom<'a>];

    fn deref(&self) -> &Self::Target {
        &self.sized_atoms
    }
}

impl DerefMut for SizedContainerAtom<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sized_atoms
    }
}
