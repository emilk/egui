use crate::{
    AtomKind, AtomPaintArgs, Atoms, Direction, FontSelection, Frame, IdSalt, Image, IntoAtoms,
    Response, SizedAtom, SizedAtomKind, Stroke, Ui, WidgetAtom,
    text_selection::LabelSelectionState,
};
use core::ops::{Deref, DerefMut};
use emath::{Align2, GuiRounding as _, NumExt as _, Rect, Vec2};
use epaint::text::TextWrapMode;
use epaint::{Color32, Galley};
use smallvec::SmallVec;
use std::sync::Arc;

/// The `(main, cross)` axis indices for `direction`, for indexing a [`Vec2`] (0 = x, 1 = y).
#[inline]
fn main_cross_axis(direction: Direction) -> (usize, usize) {
    let main = usize::from(!direction.is_horizontal());
    (main, 1 - main)
}

/// Build a [`Vec2`] from `main`/`cross` components for `direction`.
#[inline]
fn main_cross_vec(direction: Direction, main: f32, cross: f32) -> Vec2 {
    if direction.is_horizontal() {
        Vec2::new(main, cross)
    } else {
        Vec2::new(cross, main)
    }
}

/// Build a cell [`Rect`] spanning `aligned_rect` fully on the cross axis and `[min_main, max_main]`
/// along the main axis.
#[inline]
fn main_cross_rect(direction: Direction, aligned_rect: Rect, min_main: f32, max_main: f32) -> Rect {
    if direction.is_horizontal() {
        Rect::from_x_y_ranges(min_main..=max_main, aligned_rect.y_range())
    } else {
        Rect::from_x_y_ranges(aligned_rect.x_range(), min_main..=max_main)
    }
}

/// The custom atom rects collected while painting, keyed by atom salt.
///
/// There should rarely be more than one.
pub type CustomRects = SmallVec<[(IdSalt, Rect); 1]>;

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
    direction: Direction,
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
            direction: Direction::LeftToRight,
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
            direction,
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
        // This only applies along the main axis (the direction we lay atoms out along).
        if direction.is_horizontal() {
            if ui.layout().horizontal_justify() {
                max_size.x = f32::INFINITY;
            }
        } else if ui.layout().vertical_justify() {
            max_size.y = f32::INFINITY;
        }

        let available_size = available_size.at_most(max_size).at_least(min_size);

        // The size available for the content
        let available_inner_size = available_size - frame.total_margin().sum();

        // We work in main/cross axis terms so the same code handles horizontal and vertical
        // layouts. For a horizontal `direction`, main = x and cross = y; for vertical it's
        // swapped. `grow`/`shrink`/`gap` apply along the main axis; the cross axis is sized to
        // the largest atom. `main_axis`/`cross_axis` index into a `Vec2` (0 = x, 1 = y).
        let (main_axis, cross_axis) = main_cross_axis(direction);

        let mut inner_main = 0.0;

        // intrinsic main / cross is the ideal size of the widget, e.g. the size where the
        // text is not wrapped. Used to set Response::intrinsic_size.
        let mut intrinsic_main = 0.0;
        let mut intrinsic_cross: f32 = 0.0;

        let mut cross_size: f32 = 0.0;

        let mut sized_items = Vec::new();

        let mut grow_count = 0;

        let mut shrink_item = None;

        let align2 = align2.unwrap_or_else(|| ui.layout().align2());

        if atoms.len() > 1 {
            let gap_space = gap * (atoms.len() as f32 - 1.0);
            inner_main += gap_space;
            intrinsic_main += gap_space;
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

            inner_main += size[main_axis];
            intrinsic_main += sized.intrinsic_size[main_axis];

            cross_size = cross_size.at_least(size[cross_axis]);
            intrinsic_cross = intrinsic_cross.at_least(sized.intrinsic_size[cross_axis]);

            sized_items.push(sized);
        }

        if let Some((index, item)) = shrink_item {
            // The `shrink` item gets the remaining space along the main axis.
            let available_size_for_shrink_item = main_cross_vec(
                direction,
                available_inner_size[main_axis] - inner_main,
                available_inner_size[cross_axis],
            );

            let sized = item.into_sized(
                ui,
                available_size_for_shrink_item,
                Some(wrap_mode),
                fallback_font,
            );
            let size = sized.size;

            inner_main += size[main_axis];
            intrinsic_main += sized.intrinsic_size[main_axis];

            cross_size = cross_size.at_least(size[cross_axis]);
            intrinsic_cross = intrinsic_cross.at_least(sized.intrinsic_size[cross_axis]);

            sized_items.insert(index, sized);
        }

        let margin = frame.total_margin();
        let inner_size = main_cross_vec(direction, inner_main, cross_size);
        let outer_size = (inner_size + margin.sum()).at_least(min_size);
        let intrinsic_size = (main_cross_vec(direction, intrinsic_main, intrinsic_cross)
            + margin.sum())
        .at_least(min_size);

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
            direction,
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

    /// Set the gap between atoms if no gap has been set yet.
    fn fallback_gap(self, gap: f32) {
        self.gap = self.gap.or(Some(gap));
    }

    /// Set the direction the atoms are laid out along.
    fn direction(self, direction: Direction) {
        self.direction = direction;
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

    /// The axis the atoms are laid out along. The main axis carries `grow`/`shrink`/`gap`.
    direction: Direction,
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
            *kind = f(core::mem::take(kind));
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
        self.paint_at_with_selection(ui, rect, None)
    }

    pub(crate) fn paint_at_with_selection(
        self,
        ui: &Ui,
        rect: Rect,
        selection_response: Option<&Response>,
    ) -> CustomRects {
        let Self {
            mut sized_atoms,
            frame,
            fallback_text_color,
            grow_count,
            inner_size,
            align2,
            gap,
            direction,
            ..
        } = self;

        let inner_rect = rect - frame.total_margin();

        ui.painter().add(frame.paint(inner_rect));

        let (main_axis, cross_axis) = main_cross_axis(direction);

        // We position atoms along the main axis (the `direction`) and span the cross axis.
        let main_to_fill = inner_rect.size()[main_axis];
        let inner_main = inner_size[main_axis];
        let extra_space = f32::max(main_to_fill - inner_main, 0.0);
        let grow_main = f32::max(extra_space / grow_count as f32, 0.0).floor_ui();

        // When something grows, the block fills the available main extent; otherwise it's the
        // content's inner size. `align2` then positions the block within `inner_rect`.
        let block_main = if grow_count > 0 {
            main_to_fill
        } else {
            inner_main
        };
        let block_size = main_cross_vec(direction, block_main, inner_size[cross_axis]);
        let aligned_rect = align2.align_size_within_rect(block_size, inner_rect);

        // For reversed directions the first atom sits at the far end, so we lay them out in
        // reverse and otherwise share the same forward cursor logic.
        if matches!(direction, Direction::RightToLeft | Direction::BottomUp) {
            sized_atoms.reverse();
        }

        // The cursor walks the main axis from the start (left/top) of the aligned block.
        let mut cursor = aligned_rect.min.to_vec2()[main_axis];

        let mut custom_rects = CustomRects::new();

        for sized in sized_atoms {
            let size = sized.size;
            // TODO(lucasmerlin): This is not ideal, since this might lead to accumulated rounding errors
            // https://github.com/emilk/egui/pull/5830#discussion_r2079627864
            let growth = if sized.is_grow() { grow_main } else { 0.0 };

            let atom_main = size[main_axis] + growth;

            // The cell spans the cross axis fully and `atom_main` along the main axis.
            let cell = main_cross_rect(direction, aligned_rect, cursor, cursor + atom_main);
            cursor += atom_main + gap;
            let item_rect = sized.align.align_size_within_rect(size, cell);

            if let Some(id) = sized.id {
                debug_assert!(
                    !custom_rects.iter().any(|(i, _)| *i == id),
                    "Duplicate custom id"
                );
                custom_rects.push((id, item_rect));
            }

            match sized.kind {
                SizedAtomKind::Text(galley) => {
                    if let Some(response) = selection_response {
                        // Route through the label selection machinery, which also
                        // paints the galley. `Stroke::NONE` keeps the rendering
                        // identical to the non-selectable path (no focus underline).
                        LabelSelectionState::label_text_selection(
                            ui,
                            response,
                            item_rect.min,
                            galley,
                            fallback_text_color,
                            Stroke::NONE,
                        );
                    } else {
                        ui.painter()
                            .galley(item_rect.min, galley, fallback_text_color);
                    }
                }
                SizedAtomKind::Image { image, size: _ } => {
                    image.paint_at(ui, item_rect);
                }
                SizedAtomKind::Empty { .. } => {}
                SizedAtomKind::Paint { paint, size: _ } => {
                    paint(
                        ui,
                        AtomPaintArgs {
                            rect: item_rect,
                            fallback_text_color,
                        },
                    );
                }
                SizedAtomKind::Widget(widget) => {
                    // TODO(lucasmerlin): Add some kind of justify flag to the layout
                    widget.paint_at(ui, cell);
                }
                SizedAtomKind::Container(container) => {
                    // A nested container has no id/sense, so it is painted but not interacted with.
                    container.paint_at(ui, cell);
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
