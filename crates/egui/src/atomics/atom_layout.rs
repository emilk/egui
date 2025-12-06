use crate::atomics::ATOMS_SMALL_VEC_SIZE;
use crate::{
    AtomKind, Atoms, FontSelection, Frame, Id, Image, IntoAtoms, Response, Sense, SizedAtom,
    SizedAtomKind, Ui, Widget,
};
use emath::{Align2, GuiRounding as _, NumExt as _, Rect, Vec2};
use epaint::text::TextWrapMode;
use epaint::{Color32, Galley};
use smallvec::SmallVec;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

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
pub struct AtomLayout<'a> {
    id: Option<Id>,
    pub atoms: Atoms<'a>,
    gap: Option<f32>,
    pub(crate) frame: Frame,
    pub(crate) sense: Sense,
    fallback_text_color: Option<Color32>,
    fallback_font: Option<FontSelection>,
    min_size: Vec2,
    wrap_mode: Option<TextWrapMode>,
    align2: Option<Align2>,
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
            wrap_mode: None,
            align2: None,
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

    /// [`AtomLayout::allocate`] and [`AllocatedAtomLayout::paint`] in one go.
    pub fn show(self, ui: &mut Ui) -> AtomLayoutResponse {
        self.allocate(ui).paint(ui)
    }

    /// Calculate sizes, create [`Galley`]s and allocate a [`Response`].
    ///
    /// Use the returned [`AllocatedAtomLayout`] for painting.
    pub fn allocate(self, ui: &mut Ui) -> AllocatedAtomLayout<'a> {
        let Self {
            id,
            mut atoms,
            gap,
            frame,
            sense,
            fallback_text_color,
            min_size,
            wrap_mode,
            align2,
            fallback_font,
        } = self;

        let fallback_font = fallback_font.unwrap_or_default();

        let wrap_mode = wrap_mode.unwrap_or_else(|| ui.wrap_mode());

        // If the TextWrapMode is not Extend, ensure there is some item marked as `shrink`.
        // If none is found, mark the first text item as `shrink`.
        if wrap_mode != TextWrapMode::Extend {
            let any_shrink = atoms.iter().any(|a| a.shrink);
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

        // The size available for the content
        let available_inner_size = ui.available_size() - frame.total_margin().sum();

        let mut desired_width = 0.0;

        // intrinsic width / height is the ideal size of the widget, e.g. the size where the
        // text is not wrapped. Used to set Response::intrinsic_size.
        let mut intrinsic_width = 0.0;
        let mut intrinsic_height = 0.0;

        let mut height: f32 = 0.0;

        let mut sized_items = SmallVec::new();

        let mut grow_count = 0;

        let mut shrink_item = None;

        let align2 = align2.unwrap_or_else(|| {
            Align2([ui.layout().horizontal_align(), ui.layout().vertical_align()])
        });

        if atoms.len() > 1 {
            let gap_space = gap * (atoms.len() as f32 - 1.0);
            desired_width += gap_space;
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

            desired_width += size.x;
            intrinsic_width += sized.intrinsic_size.x;

            height = height.at_least(size.y);
            intrinsic_height = intrinsic_height.at_least(sized.intrinsic_size.y);

            sized_items.push(sized);
        }

        if let Some((index, item)) = shrink_item {
            // The `shrink` item gets the remaining space
            let available_size_for_shrink_item = Vec2::new(
                available_inner_size.x - desired_width,
                available_inner_size.y,
            );

            let sized = item.into_sized(
                ui,
                available_size_for_shrink_item,
                Some(wrap_mode),
                fallback_font,
            );
            let size = sized.size;

            desired_width += size.x;
            intrinsic_width += sized.intrinsic_size.x;

            height = height.at_least(size.y);
            intrinsic_height = intrinsic_height.at_least(sized.intrinsic_size.y);

            sized_items.insert(index, sized);
        }

        let margin = frame.total_margin();
        let desired_size = Vec2::new(desired_width, height);
        let frame_size = (desired_size + margin.sum()).at_least(min_size);

        let (_, rect) = ui.allocate_space(frame_size);
        let mut response = ui.interact(rect, id, sense);

        response.intrinsic_size =
            Some((Vec2::new(intrinsic_width, intrinsic_height) + margin.sum()).at_least(min_size));

        AllocatedAtomLayout {
            sized_atoms: sized_items,
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

/// Instructions for painting an [`AtomLayout`].
#[derive(Clone, Debug)]
pub struct AllocatedAtomLayout<'a> {
    pub sized_atoms: SmallVec<[SizedAtom<'a>; ATOMS_SMALL_VEC_SIZE]>,
    pub frame: Frame,
    pub fallback_text_color: Color32,
    pub response: Response,
    grow_count: usize,
    // The size of the inner content, before any growing.
    desired_size: Vec2,
    align2: Align2,
    gap: f32,
}

impl<'atom> AllocatedAtomLayout<'atom> {
    pub fn iter_kinds(&self) -> impl Iterator<Item = &SizedAtomKind<'atom>> {
        self.sized_atoms.iter().map(|atom| &atom.kind)
    }

    pub fn iter_kinds_mut(&mut self) -> impl Iterator<Item = &mut SizedAtomKind<'atom>> {
        self.sized_atoms.iter_mut().map(|atom| &mut atom.kind)
    }

    pub fn iter_images(&self) -> impl Iterator<Item = &Image<'atom>> {
        self.iter_kinds().filter_map(|kind| {
            if let SizedAtomKind::Image(image, _) = kind {
                Some(image)
            } else {
                None
            }
        })
    }

    pub fn iter_images_mut(&mut self) -> impl Iterator<Item = &mut Image<'atom>> {
        self.iter_kinds_mut().filter_map(|kind| {
            if let SizedAtomKind::Image(image, _) = kind {
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
            if let SizedAtomKind::Image(image, size) = kind {
                SizedAtomKind::Image(f(image), size)
            } else {
                kind
            }
        });
    }

    /// Paint the [`Frame`] and individual [`crate::Atom`]s.
    pub fn paint(self, ui: &Ui) -> AtomLayoutResponse {
        let Self {
            sized_atoms,
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
        let grow_width = f32::max(extra_space / grow_count as f32, 0.0).floor_ui();

        let aligned_rect = if grow_count > 0 {
            align2.align_size_within_rect(Vec2::new(width_to_fill, desired_size.y), inner_rect)
        } else {
            align2.align_size_within_rect(desired_size, inner_rect)
        };

        let mut cursor = aligned_rect.left();

        let mut response = AtomLayoutResponse::empty(response);

        for sized in sized_atoms {
            let size = sized.size;
            // TODO(lucasmerlin): This is not ideal, since this might lead to accumulated rounding errors
            // https://github.com/emilk/egui/pull/5830#discussion_r2079627864
            let growth = if sized.is_grow() { grow_width } else { 0.0 };

            let frame = aligned_rect
                .with_min_x(cursor)
                .with_max_x(cursor + size.x + growth);
            cursor = frame.right() + gap;

            let align = Align2::CENTER_CENTER;
            let rect = align.align_size_within_rect(size, frame);

            match sized.kind {
                SizedAtomKind::Text(galley) => {
                    ui.painter().galley(rect.min, galley, fallback_text_color);
                }
                SizedAtomKind::Image(image, _) => {
                    image.paint_at(ui, rect);
                }
                SizedAtomKind::Custom(id) => {
                    debug_assert!(
                        !response.custom_rects.iter().any(|(i, _)| *i == id),
                        "Duplicate custom id"
                    );
                    response.custom_rects.push((id, rect));
                }
                SizedAtomKind::Empty => {}
            }
        }

        response
    }
}

/// Response from a [`AtomLayout::show`] or [`AllocatedAtomLayout::paint`].
///
/// Use [`AtomLayoutResponse::rect`] to get the response rects from [`AtomKind::Custom`].
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

    /// Use this together with [`AtomKind::Custom`] to add custom painting / child widgets.
    ///
    /// NOTE: Don't `unwrap` rects, they might be empty when the widget is not visible.
    pub fn rect(&self, id: Id) -> Option<Rect> {
        self.custom_rects
            .iter()
            .find_map(|(i, r)| if *i == id { Some(*r) } else { None })
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

impl<'a> Deref for AllocatedAtomLayout<'a> {
    type Target = [SizedAtom<'a>];

    fn deref(&self) -> &Self::Target {
        &self.sized_atoms
    }
}

impl DerefMut for AllocatedAtomLayout<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sized_atoms
    }
}
