use crate::{
    AtomKind, Atoms, Direction, FontSelection, Frame, Id, Image, IntoAtoms, Response, Sense,
    SizedAtom, SizedAtomKind, Stroke, Ui, Widget, text_selection::LabelSelectionState,
};
use emath::{Align2, GuiRounding as _, NumExt as _, Rect, Vec2};
use epaint::text::TextWrapMode;
use epaint::{Color32, Galley};
use smallvec::SmallVec;
use std::ops::{Deref, DerefMut};
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

/// Intra-widget layout utility.
///
/// Used to lay out and paint [`crate::Atom`]s.
/// This is used internally by widgets like [`crate::Button`] and [`crate::Checkbox`].
/// You can use it to make your own widgets.
///
/// Painting the atoms can be split in two phases:
/// - [`AtomLayout::allocate`]
///   - calculates sizes
///   - converts texts to [`Galley`]s
///   - allocates a [`Response`]
///   - returns a [`AllocatedAtomLayout`]
/// - [`AllocatedAtomLayout::paint`]
///   - paints the [`Frame`]
///   - calculates individual [`crate::Atom`] positions
///   - paints each single atom
///
/// You can use this to first allocate a response and then modify, e.g., the [`Frame`] on the
/// [`AllocatedAtomLayout`] for interaction styling.
#[derive(Clone)]
pub struct AtomLayout<'a> {
    pub(crate) id: Option<Id>,
    pub atoms: Atoms<'a>,
    gap: Option<f32>,
    pub(crate) frame: Frame,
    pub(crate) sense: Sense,
    selectable: bool,
    fallback_text_color: Option<Color32>,
    fallback_font: Option<FontSelection>,
    min_size: Vec2,
    max_size: Vec2,
    wrap_mode: Option<TextWrapMode>,
    align2: Option<Align2>,
    direction: Direction,
}

impl Default for AtomLayout<'_> {
    fn default() -> Self {
        Self::new(())
    }
}

impl<'a> AtomLayout<'a> {
    pub fn new(atoms: impl IntoAtoms<'a>) -> Self {
        Self {
            id: None,
            atoms: atoms.into_atoms(),
            gap: None,
            frame: Frame::default(),
            sense: Sense::hover(),
            selectable: false,
            fallback_text_color: None,
            fallback_font: None,
            min_size: Vec2::ZERO,
            max_size: Vec2::INFINITY,
            wrap_mode: None,
            align2: None,
            direction: Direction::LeftToRight,
        }
    }

    /// Set the gap between atoms.
    ///
    /// Default: `Spacing::icon_spacing`
    #[inline]
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = Some(gap);
        self
    }

    /// Set the [`Frame`].
    #[inline]
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = frame;
        self
    }

    /// Set the [`Sense`] used when allocating the [`Response`].
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// Make the text in this layout selectable with the mouse.
    ///
    /// This is opt-in (default `false`): [`AtomLayout`] backs widgets like
    /// [`crate::Button`] and [`crate::Checkbox`] whose labels should not be
    /// selectable, so enabling it unconditionally would break them. When enabled,
    /// the layout also senses clicks and drags so the selection can be made.
    #[inline]
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// Set the fallback (default) text color.
    ///
    /// Default: [`crate::Visuals::text_color`]
    #[inline]
    pub fn fallback_text_color(mut self, color: Color32) -> Self {
        self.fallback_text_color = Some(color);
        self
    }

    /// Set the fallback (default) font.
    #[inline]
    pub fn fallback_font(mut self, font: impl Into<FontSelection>) -> Self {
        self.fallback_font = Some(font.into());
        self
    }

    /// Set the minimum size of the Widget.
    ///
    /// This will find and expand atoms with `grow: true`.
    /// If there are no growable atoms then everything will be left-aligned.
    #[inline]
    pub fn min_size(mut self, size: Vec2) -> Self {
        self.min_size = size;
        self
    }

    /// Set the maximum size of the Widget.
    ///
    /// By default, the size is limited by the available size in the [`Ui`].
    #[inline]
    pub fn max_size(mut self, size: Vec2) -> Self {
        self.max_size = size;
        self
    }

    /// Set the maximum width of the Widget.
    ///
    /// By default, the width is limited by the available width in the [`Ui`].
    #[inline]
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_size.x = width;
        self
    }

    /// Set the maximum height of the Widget.
    ///
    /// By default, the height is limited by the available height in the [`Ui`].
    #[inline]
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_size.y = height;
        self
    }

    /// Set the [`Id`] used to allocate a [`Response`].
    #[inline]
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the [`TextWrapMode`] for the [`crate::Atom`] marked as `shrink`.
    ///
    /// Only a single [`crate::Atom`] may shrink. If this (or `ui.wrap_mode()`) is not
    /// [`TextWrapMode::Extend`] and no item is set to shrink, the first (left-most)
    /// [`AtomKind::Text`] will be set to shrink.
    #[inline]
    pub fn wrap_mode(mut self, wrap_mode: TextWrapMode) -> Self {
        self.wrap_mode = Some(wrap_mode);
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
        self.align2 = Some(align2);
        self
    }

    /// Set the [`Direction`] the [`crate::Atom`]s are laid out along.
    ///
    /// The default is [`Direction::LeftToRight`] (a horizontal row). Use
    /// [`Direction::TopDown`] (or [`Direction::BottomUp`]) to stack atoms vertically.
    ///
    /// The main axis (the direction) is where `grow`/`shrink` and the gap apply; the cross axis
    /// is sized to the largest atom. [`Self::align2`] positions the whole block within the
    /// allocated [`Rect`].
    #[inline]
    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// [`AtomLayout::allocate`] and [`AllocatedAtomLayout::paint`] in one go.
    pub fn show(self, ui: &mut Ui) -> AtomLayoutResponse {
        self.allocate(ui).paint(ui)
    }

    /// Measure the atoms (sizing only), without allocating space or interacting.
    ///
    /// This converts texts to [`Galley`]s and calculates sizes, but unlike [`Self::allocate`]
    /// it does *not* call [`Ui::allocate_space`] (so the parent cursor is left untouched) nor
    /// [`Ui::interact`]. Use the returned [`SizedAtomLayout`] to paint at an arbitrary [`Rect`]
    /// via [`SizedAtomLayout::paint_at`]. This is what makes it possible to nest one
    /// [`AtomLayout`] inside another.
    ///
    /// `available_size` is the space available to the whole widget (frame included); it is
    /// clamped by `max_size`/`min_size`, exactly like [`Self::allocate`] does with
    /// [`Ui::available_size`].
    pub fn measure(self, ui: &Ui, available_size: Vec2) -> SizedAtomLayout<'a> {
        let Self {
            id,
            mut atoms,
            gap,
            frame,
            mut sense,
            selectable,
            fallback_text_color,
            min_size,
            mut max_size,
            wrap_mode,
            align2,
            fallback_font,
            direction,
        } = self;

        let fallback_font = fallback_font.unwrap_or_default();

        if selectable {
            // Mirror `Label`: sense clicks and drags so the text can be selected,
            // but don't take keyboard focus on TAB.
            let allow_drag_to_select = ui.input(|i| !i.has_touch_screen());
            let mut select_sense = if allow_drag_to_select {
                Sense::click_and_drag()
            } else {
                Sense::click()
            };
            select_sense -= Sense::FOCUSABLE;
            sense |= select_sense;
        }

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

        let id = id.unwrap_or_else(|| ui.next_auto_id());

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

        let align2 = align2.unwrap_or_else(|| {
            Align2([ui.layout().horizontal_align(), ui.layout().vertical_align()])
        });

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

        SizedAtomLayout {
            sized_atoms: sized_items,
            frame,
            fallback_text_color,
            id,
            sense,
            outer_size,
            intrinsic_size,
            grow_count,
            inner_size,
            align2,
            gap,
            direction,
            selectable,
        }
    }

    /// Calculate sizes, create [`Galley`]s and allocate a [`Response`].
    ///
    /// Use the returned [`AllocatedAtomLayout`] for painting.
    pub fn allocate(self, ui: &mut Ui) -> AllocatedAtomLayout<'a> {
        let sized = self.measure(ui, ui.available_size());

        let (_, rect) = ui.allocate_space(sized.outer_size);
        let mut response = ui.interact(rect, sized.id, sized.sense);
        response.set_intrinsic_size(sized.intrinsic_size);

        AllocatedAtomLayout { sized, response }
    }
}

/// A measured [`AtomLayout`], ready to be painted at a [`Rect`].
///
/// Produced by [`AtomLayout::measure`]. Unlike [`AllocatedAtomLayout`], it has not yet
/// allocated space or interacted, so it can be painted at an arbitrary [`Rect`] via
/// [`Self::paint_at`]. This is what lets one [`AtomLayout`] be nested inside another.
#[derive(Clone, Debug)]
pub struct SizedAtomLayout<'a> {
    /// The [`Id`] used to [`Ui::interact`] when this layout is allocated / painted.
    id: Id,

    /// The [`Sense`] used to [`Ui::interact`] when this layout is allocated / painted.
    sense: Sense,

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

    selectable: bool,
}

/// Instructions for painting an [`AtomLayout`].
///
/// This is a [`SizedAtomLayout`] that has additionally allocated space and interacted,
/// producing a [`Response`].
#[derive(Clone, Debug)]
pub struct AllocatedAtomLayout<'a> {
    /// The measured layout.
    pub sized: SizedAtomLayout<'a>,

    pub response: Response,
}

impl<'atom> SizedAtomLayout<'atom> {
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
    /// `response.rect`; when nested, the parent passes the cell rect it computed. `response`
    /// becomes the base of the returned [`AtomLayoutResponse`].
    pub fn paint_at(self, ui: &Ui, rect: Rect, response: Response) -> AtomLayoutResponse {
        let Self {
            mut sized_atoms,
            frame,
            fallback_text_color,
            grow_count,
            inner_size,
            align2,
            gap,
            direction,
            selectable,
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

        let mut response = AtomLayoutResponse::empty(response);

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
                    !response.custom_rects.iter().any(|(i, _)| *i == id),
                    "Duplicate custom id"
                );
                response.custom_rects.push((id, item_rect));
            }

            match sized.kind {
                SizedAtomKind::Text(galley) => {
                    if selectable {
                        // Route through the label selection machinery, which also
                        // paints the galley. `Stroke::NONE` keeps the rendering
                        // identical to the non-selectable path (no focus underline).
                        LabelSelectionState::label_text_selection(
                            ui,
                            &response.response,
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
                SizedAtomKind::Layout(layout) => {
                    // TODO(lucasmerlin): Add some kind of justify flag, right now nested atoms are always
                    // shown fully stretched.
                    let layout_response = ui.interact(cell, layout.id, layout.sense);
                    layout.paint_at(ui, cell, layout_response);
                }
            }
        }

        response
    }
}

impl AllocatedAtomLayout<'_> {
    /// Paint the [`Frame`] and individual [`crate::Atom`]s at the allocated [`Response`]'s rect.
    pub fn paint(self, ui: &Ui) -> AtomLayoutResponse {
        let rect = self.response.rect;
        self.sized.paint_at(ui, rect, self.response)
    }
}

/// Response from a [`AtomLayout::show`] or [`AllocatedAtomLayout::paint`].
///
/// Use [`AtomLayoutResponse::rect`] to get the response rects from [`crate::Atom::custom`].
#[derive(Clone, Debug)]
pub struct AtomLayoutResponse {
    pub response: Response,
    // There should rarely be more than one custom rect.
    custom_rects: SmallVec<[(Id, Rect); 1]>,
}

impl AtomLayoutResponse {
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

impl Deref for AtomLayoutResponse {
    type Target = Response;

    fn deref(&self) -> &Self::Target {
        &self.response
    }
}

impl DerefMut for AtomLayoutResponse {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.response
    }
}

impl Widget for AtomLayout<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

impl<'a> Deref for AtomLayout<'a> {
    type Target = Atoms<'a>;

    fn deref(&self) -> &Self::Target {
        &self.atoms
    }
}

impl DerefMut for AtomLayout<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.atoms
    }
}

impl<'a> Deref for SizedAtomLayout<'a> {
    type Target = [SizedAtom<'a>];

    fn deref(&self) -> &Self::Target {
        &self.sized_atoms
    }
}

impl DerefMut for SizedAtomLayout<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sized_atoms
    }
}

impl<'a> Deref for AllocatedAtomLayout<'a> {
    type Target = SizedAtomLayout<'a>;

    fn deref(&self) -> &Self::Target {
        &self.sized
    }
}

impl DerefMut for AllocatedAtomLayout<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sized
    }
}
