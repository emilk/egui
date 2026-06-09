use crate::{
    AtomKind, Atoms, Direction, FontSelection, Frame, Id, Image, IntoAtoms, Response, Sense,
    SizedAtom, SizedAtomKind, Ui, Widget,
};
use emath::{Align2, GuiRounding as _, NumExt as _, Rect, Vec2};
use epaint::text::TextWrapMode;
use epaint::{Color32, Galley};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::Arc;

/// Frame-pass-local memoization cache for [`AtomLayout::measure_rc`].
///
/// Keyed by an [`Rc::as_ptr`] identity plus the available-size bits. Both are stable within a
/// single top-level measure pass (nested layouts are held alive via `Rc`), so repeatedly measuring
/// the same nested layout at the same size — which a deep tree of `grow` layouts does `O(2^depth)`
/// times — becomes a cache hit instead of a full re-measure.
pub(crate) type MeasureCache<'a> = HashMap<(usize, u64), SizedAtomLayout<'a>>;

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

/// Build a [`Rect`] that extends `main_len` along the main axis from `main_min`, and `cross_len`
/// along the cross axis from `cross_min`, for `direction`.
#[inline]
fn rect_from_main_cross(
    direction: Direction,
    main_min: f32,
    main_len: f32,
    cross_min: f32,
    cross_len: f32,
) -> Rect {
    let min = main_cross_vec(direction, main_min, cross_min).to_pos2();
    Rect::from_min_size(min, main_cross_vec(direction, main_len, cross_len))
}

/// Group already-sized atoms into lines for flex-like wrapping.
///
/// Walks `atoms` in order, accumulating along the main axis; when adding the next atom would
/// exceed `max_main` (and the current line is non-empty) a new line is started. Atoms are never
/// split. `gap` is added between atoms on a line. Always returns at least one line (possibly
/// empty, if there are no atoms).
fn pack_lines(
    atoms: &[SizedAtom<'_>],
    main_axis: usize,
    cross_axis: usize,
    max_main: f32,
    gap: f32,
) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut main_extent = 0.0;
    let mut cross_extent: f32 = 0.0;
    let mut grow_count = 0;

    for (i, atom) in atoms.iter().enumerate() {
        let atom_main = atom.size[main_axis];
        let atom_cross = atom.size[cross_axis];
        let is_first_on_line = i == start;
        let main_with_atom = if is_first_on_line {
            atom_main
        } else {
            main_extent + gap + atom_main
        };

        if !is_first_on_line && main_with_atom > max_main {
            // Doesn't fit: flush the current line and start a new one with this atom.
            lines.push(Line {
                range: start..i,
                main_extent,
                cross_extent,
                grow_count,
            });
            start = i;
            main_extent = atom_main;
            cross_extent = atom_cross;
            grow_count = usize::from(atom.grow);
        } else {
            main_extent = main_with_atom;
            cross_extent = cross_extent.max(atom_cross);
            grow_count += usize::from(atom.grow);
        }
    }

    lines.push(Line {
        range: start..atoms.len(),
        main_extent,
        cross_extent,
        grow_count,
    });

    lines
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
    fallback_text_color: Option<Color32>,
    fallback_font: Option<FontSelection>,
    min_size: Vec2,
    max_size: Vec2,
    wrap_mode: Option<TextWrapMode>,
    align2: Option<Align2>,
    direction: Direction,
    wrap: bool,
    cross_justify: bool,
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
            fallback_text_color: None,
            fallback_font: None,
            min_size: Vec2::ZERO,
            max_size: Vec2::INFINITY,
            wrap_mode: None,
            align2: None,
            direction: Direction::LeftToRight,
            wrap: false,
            cross_justify: false,
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

    /// Wrap [`crate::Atom`]s onto multiple lines when they exceed the available main extent
    /// (flex-like wrapping).
    ///
    /// Atoms are treated as atomic units for line-breaking: an atom either fits on the current
    /// line or moves to the next one (it is not split). Each line is packed and grown
    /// independently along the main axis; lines are stacked along the cross axis.
    ///
    /// Wrapping is mutually exclusive with the implicit single-atom text shrink: when `wrap` is
    /// set, no atom is automatically marked as `shrink`. The same `gap` is used between atoms on
    /// a line and between lines.
    #[inline]
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Stretch the content along the cross axis to fill the [`Rect`] this layout is painted into
    /// (flexbox `align-items: stretch`).
    ///
    /// By default the content takes its natural cross size and is positioned within the available
    /// cross space by [`Self::align2`]. This matters when the layout is painted into a `Rect`
    /// larger than its measured size along the cross axis — most commonly when it is a nested,
    /// `grow`ing [`crate::Atom`] in a parent layout: with `cross_justify` its own content (a full
    /// width mock image, a row of tags, …) expands to fill the grown size instead of hugging the
    /// start. Extra cross space is shared evenly between (wrapped) lines.
    #[inline]
    pub fn cross_justify(mut self, cross_justify: bool) -> Self {
        self.cross_justify = cross_justify;
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
    pub fn measure(&self, ui: &Ui, available_size: Vec2) -> SizedAtomLayout<'a> {
        self.measure_impl(ui, available_size, &mut MeasureCache::default())
    }

    /// Measure a nested layout held by an [`Rc`], memoizing the result in `cache`.
    ///
    /// A grown nested `Layout` atom is re-measured (the cross-after-main reflow) at its grown
    /// size, recursively. Without memoization a deep tree of `grow` layouts re-measures its
    /// descendants `O(2^depth)` times. Keyed by the layout's [`Rc::as_ptr`] identity and the
    /// available size — both stable within a pass — repeated `(layout, size)` measures become
    /// cache hits. The `Rc` is held by the caller (the `Layout` atom / reflow source), which is
    /// why the identity lives here rather than in [`Self::measure_impl`].
    pub(crate) fn measure_rc(
        layout: &Rc<Self>,
        ui: &Ui,
        available_size: Vec2,
        cache: &mut MeasureCache<'a>,
    ) -> SizedAtomLayout<'a> {
        let key = (
            Rc::as_ptr(layout) as usize,
            (u64::from(available_size.x.to_bits()) << 32) | u64::from(available_size.y.to_bits()),
        );
        if let Some(cached) = cache.get(&key) {
            return cached.clone();
        }
        let result = layout.measure_impl(ui, available_size, cache);
        cache.insert(key, result.clone());
        result
    }

    /// The measure body. Threads `cache` so nested [`Rc`] layouts are memoized via
    /// [`Self::measure_rc`]; it does not memoize its own result (a top-level layout is measured
    /// once, and a nested one is keyed by its `Rc` at the call site).
    pub(crate) fn measure_impl(
        &self,
        ui: &Ui,
        available_size: Vec2,
        cache: &mut MeasureCache<'a>,
    ) -> SizedAtomLayout<'a> {
        let atoms = &self.atoms;
        let frame = self.frame;
        let sense = self.sense;
        let min_size = self.min_size;
        let mut max_size = self.max_size;
        let direction = self.direction;
        let wrap = self.wrap;
        let cross_justify = self.cross_justify;

        let fallback_font = self.fallback_font.clone().unwrap_or_default();

        let wrap_mode = self.wrap_mode.unwrap_or_else(|| ui.wrap_mode());

        // If the TextWrapMode is not Extend, ensure there is some item marked as `shrink`.
        // If none is found, the first text item acts as the `shrink` item. We size from `&self`
        // and can't mutate the atom, so we record its index and treat it as `shrink` below.
        // When `wrap` (flex wrapping) is enabled the shrink mechanism is disabled (atoms wrap
        // onto new lines instead of one atom shrinking to fit a single line).
        let auto_shrink_index = if wrap_mode != TextWrapMode::Extend && !wrap && !atoms.any_shrink()
        {
            atoms
                .iter()
                .position(|a| matches!(a.kind, AtomKind::Text(..)))
        } else {
            None
        };

        let id = self.id.unwrap_or_else(|| ui.next_auto_id());

        let fallback_text_color = self
            .fallback_text_color
            .unwrap_or_else(|| ui.style().visuals.text_color());
        let gap = self.gap.unwrap_or_else(|| ui.spacing().icon_spacing);

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

        let align2 = self.align2.unwrap_or_else(|| {
            Align2([ui.layout().horizontal_align(), ui.layout().vertical_align()])
        });

        if atoms.len() > 1 {
            let gap_space = gap * (atoms.len() as f32 - 1.0);
            inner_main += gap_space;
            intrinsic_main += gap_space;
        }

        for (idx, item) in atoms.iter().enumerate() {
            if item.grow {
                grow_count += 1;
            }
            // When wrapping, `shrink` atoms are laid out like any other atom (no single-atom
            // shrink-to-fit), so don't divert them into the shrink path. `auto_shrink_index`
            // promotes the first text atom to `shrink` when none was set explicitly.
            if (item.shrink || Some(idx) == auto_shrink_index) && !wrap {
                debug_assert!(
                    shrink_item.is_none(),
                    "Only one atomic may be marked as shrink. {item:?}"
                );
                if shrink_item.is_none() {
                    shrink_item = Some((idx, item));
                    continue;
                }
            }
            let sized = item.as_sized(
                ui,
                available_inner_size,
                Some(wrap_mode),
                fallback_font.clone(),
                cache,
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

            // `Atom::as_sized` reads `self.shrink` (a non-shrink atom with no max width is forced
            // to `Extend`). The auto-selected first-text atom isn't flagged `shrink`, so size a
            // copy with `shrink` set to keep the previous truncate/wrap behavior.
            let sized = if item.shrink {
                item.as_sized(
                    ui,
                    available_size_for_shrink_item,
                    Some(wrap_mode),
                    fallback_font,
                    cache,
                )
            } else {
                let mut item = item.clone();
                item.shrink = true;
                item.as_sized(
                    ui,
                    available_size_for_shrink_item,
                    Some(wrap_mode),
                    fallback_font,
                    cache,
                )
            };
            let size = sized.size;

            inner_main += size[main_axis];
            intrinsic_main += sized.intrinsic_size[main_axis];

            cross_size = cross_size.at_least(size[cross_axis]);
            intrinsic_cross = intrinsic_cross.at_least(sized.intrinsic_size[cross_axis]);

            sized_items.insert(index, sized);
        }

        // Group the (flat) sized atoms into lines. Without wrapping that's a single line
        // spanning everything, which reproduces the previous single-line behavior exactly.
        let mut lines = if wrap {
            pack_lines(
                &sized_items,
                main_axis,
                cross_axis,
                available_inner_size[main_axis],
                gap,
            )
        } else {
            vec![Line {
                range: 0..sized_items.len(),
                main_extent: inner_main,
                cross_extent: cross_size,
                grow_count,
            }]
        };

        // Inner main = widest line. `grow` doesn't change it (it only fills slack within a line).
        let inner_main = lines.iter().map(|l| l.main_extent).fold(0.0_f32, f32::max);

        let margin = frame.total_margin();

        // Flexbox §9.3→§9.4 ordering: resolve `grow` and *then* re-measure each grown nested
        // layout at its grown main extent, so its reflowed cross size feeds the line's cross
        // extent. Otherwise the cross size (line height) is computed from each atom's *natural*
        // (pre-grow) main size, committed into `outer_size`, and only paint resolves `grow` — so a
        // nested layout that re-wraps narrower content when grown (a card whose tags collapse onto
        // one line) leaves the line taller than its reflowed contents (a gap above the footer).
        //
        // This mirrors the re-measure `paint_at` already does for grown nested layouts, but does
        // it here so the reflowed height propagates into the parent's own size. It only fires when
        // the layout fills past its content along the main axis (`fill_main > line.main_extent` —
        // e.g. `min_size` forces it to the available width, the gallery case). When nothing fills,
        // `grow` has no slack to distribute, so non-fill layouts are completely unaffected.
        let fill_main = (min_size[main_axis] - margin.sum()[main_axis]).max(inner_main);
        for line in &mut lines {
            if line.grow_count == 0 {
                continue;
            }
            let grow_main = ((fill_main - line.main_extent) / line.grow_count as f32).floor_ui();
            if grow_main <= 0.0 {
                continue;
            }
            let mut line_cross: f32 = 0.0;
            for sized in &mut sized_items[line.range.clone()] {
                if sized.grow
                    && let SizedAtomKind::Layout {
                        source,
                        sized: inner,
                    } = &mut sized.kind
                {
                    let grown = main_cross_vec(
                        direction,
                        sized.size[main_axis] + grow_main,
                        available_inner_size[cross_axis],
                    );
                    let remeasured = AtomLayout::measure_rc(source, ui, grown, cache);
                    sized.size[cross_axis] = remeasured.outer_size[cross_axis];
                    **inner = remeasured;
                }
                line_cross = line_cross.max(sized.size[cross_axis]);
            }
            line.cross_extent = line_cross;
        }

        // Inner cross = stacked line cross extents + inter-line gaps (post-reflow).
        let inner_cross = lines.iter().map(|l| l.cross_extent).sum::<f32>()
            + gap * lines.len().saturating_sub(1) as f32;

        let inner_size = main_cross_vec(direction, inner_main, inner_cross);
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
            lines,
            inner_size,
            align2,
            gap,
            direction,
            cross_justify,
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

/// One (possibly wrapped) line of atoms within a [`SizedAtomLayout`].
///
/// `range` indexes into [`SizedAtomLayout::sized_atoms`], which is kept as a single flat `Vec`
/// (so the `iter_*`/`map_*` helpers keep working); the lines just describe how to group it.
/// For a non-wrapping layout there is exactly one line spanning all atoms.
#[derive(Clone, Debug)]
struct Line {
    /// Range into [`SizedAtomLayout::sized_atoms`].
    range: std::ops::Range<usize>,

    /// Sum of atom main extents on this line plus the inter-atom gaps.
    main_extent: f32,

    /// The largest atom cross extent on this line.
    cross_extent: f32,

    /// How many atoms on this line are marked `grow`.
    grow_count: usize,
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

    /// The atoms grouped into (possibly wrapped) lines. Always at least one line.
    lines: Vec<Line>,

    /// How will all the atoms be aligned within the allocated rect?
    align2: Align2,

    /// The gap between each [`crate::Atom`]
    gap: f32,

    /// The axis the atoms are laid out along. The main axis carries `grow`/`shrink`/`gap`.
    direction: Direction,

    /// Stretch the content along the cross axis to fill the painted [`Rect`]. See
    /// [`AtomLayout::cross_justify`].
    cross_justify: bool,
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
            sized_atoms,
            frame,
            fallback_text_color,
            lines,
            inner_size,
            align2,
            gap,
            direction,
            cross_justify,
            ..
        } = self;

        let inner_rect = rect - frame.total_margin();

        ui.painter().add(frame.paint(inner_rect));

        let (main_axis, cross_axis) = main_cross_axis(direction);

        // We position atoms along the main axis (the `direction`) and stack lines along the cross
        // axis. For a single (non-wrapped) line this reduces to the original single-line layout.
        let main_range = if direction.is_horizontal() {
            inner_rect.x_range()
        } else {
            inner_rect.y_range()
        };
        let cross_range = if direction.is_horizontal() {
            inner_rect.y_range()
        } else {
            inner_rect.x_range()
        };
        let main_to_fill = inner_rect.size()[main_axis];

        // With `cross_justify` the content stretches to fill the cross extent of `inner_rect`:
        // any extra cross space is shared evenly between lines and the stack starts at the edge.
        // Otherwise the stack takes its natural cross size and `align2` positions it.
        let extra_cross = f32::max(cross_range.span() - inner_size[cross_axis], 0.0);
        let line_grow_cross = if cross_justify && !lines.is_empty() {
            (extra_cross / lines.len() as f32).floor_ui()
        } else {
            0.0
        };
        let mut cross_cursor = if cross_justify {
            cross_range.min
        } else {
            align2.0[cross_axis]
                .align_size_within_range(inner_size[cross_axis], cross_range)
                .min
        };

        // Split the flat `sized_atoms` into per-line groups. The line ranges are contiguous and
        // ordered, so we can just take them off the front in order.
        let mut atoms_iter = sized_atoms.into_iter();

        let mut response = AtomLayoutResponse::empty(response);

        for line in lines {
            let mut line_atoms: Vec<SizedAtom<'_>> =
                (&mut atoms_iter).take(line.range.len()).collect();

            // Per-line growth: extra main space is split between this line's `grow` atoms.
            let extra_space = f32::max(main_to_fill - line.main_extent, 0.0);
            let grow_main = if line.grow_count > 0 {
                f32::max(extra_space / line.grow_count as f32, 0.0).floor_ui()
            } else {
                0.0
            };

            // When something on this line grows, the line fills the available main extent;
            // otherwise it's the line's own extent. `align2` then positions it along the main axis.
            let block_main = if line.grow_count > 0 {
                main_to_fill
            } else {
                line.main_extent
            };
            let line_main = align2.0[main_axis].align_size_within_range(block_main, main_range);

            // The rect this line occupies: `block_main` along the main axis, and its cross extent
            // (this line's height) along the cross axis, grown to fill if `cross_justify` is set.
            let line_cross = line.cross_extent + line_grow_cross;
            let line_rect = rect_from_main_cross(
                direction,
                line_main.min,
                line_main.span(),
                cross_cursor,
                line_cross,
            );
            cross_cursor += line_cross + gap;

            // For reversed directions the first atom sits at the far end, so we lay them out in
            // reverse and otherwise share the same forward cursor logic.
            if matches!(direction, Direction::RightToLeft | Direction::BottomUp) {
                line_atoms.reverse();
            }

            // The cursor walks the main axis from the start of the aligned line.
            let mut cursor = line_rect.min.to_vec2()[main_axis];

            for sized in line_atoms {
                let size = sized.size;
                // TODO(lucasmerlin): This is not ideal, since this might lead to accumulated rounding errors
                // https://github.com/emilk/egui/pull/5830#discussion_r2079627864
                let growth = if sized.is_grow() { grow_main } else { 0.0 };

                let atom_main = size[main_axis] + growth;

                // The cell spans this line's cross extent fully and `atom_main` along the main axis.
                let cell = main_cross_rect(direction, line_rect, cursor, cursor + atom_main);
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
                        ui.painter()
                            .galley(item_rect.min, galley, fallback_text_color);
                    }
                    SizedAtomKind::Image { image, size: _ } => {
                        image.paint_at(ui, item_rect);
                    }
                    SizedAtomKind::Empty { .. } => {}
                    SizedAtomKind::Layout { source, sized } => {
                        let layout_response = ui.interact(cell, sized.id, sized.sense);
                        // The atom was measured at its natural size, but `grow`/`shrink` may have
                        // changed the cell it's painted into. If so, re-measure the layout at the
                        // actual cell so its own contents re-wrap / reflow to fit (a nested layout
                        // can't otherwise know how much it grew). When the size is unchanged we
                        // reuse the already-measured layout.
                        let resized = (cell.size() - sized.outer_size).abs().max_elem() > 0.5;
                        if resized {
                            source
                                .measure(ui, cell.size())
                                .paint_at(ui, cell, layout_response);
                        } else {
                            sized.paint_at(ui, cell, layout_response);
                        }
                    }
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
