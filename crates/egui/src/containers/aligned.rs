use emath::{Align, Align2, ReadingDirection, Rect, Vec2, pos2};

use crate::{AsIdSalt, Direction, IdSalt, InnerResponse, Ui, UiBuilder};

/// Align some widgets of unknown size within the available space of a [`Ui`].
///
/// For instance, this can put a group of widgets in the bottom-right corner of the parent [`Ui`].
///
/// egui is an immediate mode GUI, so the size of the contents is not known until they have been added.
/// This container remembers the size from the previous pass, and uses that to place the contents.
/// On the first pass (when no size is known yet) the contents are added invisibly,
/// and a new pass is requested with [`crate::Context::request_discard`], so there is no visible flicker.
///
/// Like a [`crate::Panel`], this reserves space in the parent [`Ui`] along its main axis,
/// so you can e.g. put something at the bottom of a vertical [`Ui`] and keep adding widgets above it:
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// egui::Aligned::new(egui::Align2::RIGHT_BOTTOM).show(ui, |ui| {
///     let _ = ui.button("Bottom right");
/// });
/// ui.label("This goes at the top, in the space left above the button");
/// # });
/// ```
///
/// More precisely: if the contents are aligned to the _end_ of the parent's main axis
/// (e.g. [`Align::Max`] in a top-down [`Ui`]), the cursor is moved so that widgets added afterwards
/// end up _before_ the contents. Otherwise the cursor advances past the contents, as usual.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// egui::Aligned::new(egui::Align2::RIGHT_BOTTOM).show(ui, |ui| {
///     ui.label("Bottom right");
///     let _ = ui.button("Click me");
/// });
/// # });
/// ```
#[must_use = "You should call .show()"]
#[derive(Clone, Copy, Debug)]
pub struct Aligned {
    /// The physical anchor, used as-is by [`Self::new`] and
    /// resolved from the reading direction by [`Self::start`]/[`Self::end`].
    align2: Align2,

    /// A reading-relative horizontal anchor: `Some((is_start, vertical))`,
    /// set by [`Self::start`]/[`Self::end`] and resolved against the [`Ui`]'s
    /// reading direction in [`Self::show`].
    reading: Option<(bool, Align)>,

    id_salt: Option<IdSalt>,
}

impl Aligned {
    /// Align the contents within the available space of the parent [`Ui`].
    ///
    /// The [`Align2`] is physical: [`Align2::RIGHT_BOTTOM`] anchors at the physical
    /// right edge in both reading directions. See [`Self::start`]/[`Self::end`] for
    /// reading-relative anchors.
    pub fn new(align2: Align2) -> Self {
        Self {
            align2,
            reading: None,
            id_salt: None,
        }
    }

    /// Anchor the contents at the reading start: the left edge of the parent
    /// [`Ui`] in a left-to-right interface, the right edge in a right-to-left one.
    ///
    /// The vertical [`Align`] applies as-is.
    ///
    /// If the contents are wider than the available space, they overflow toward
    /// the reading end, keeping the reading-start edge (where the reader enters)
    /// visible.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// egui::Aligned::start(egui::Align::BOTTOM).show(ui, |ui| {
    ///     let _ = ui.button("At the reading start (bottom) edge");
    /// });
    /// # });
    /// ```
    #[inline]
    pub fn start(vertical: Align) -> Self {
        Self {
            // Resolved from the reading direction in `show`; ignored until then.
            align2: Align2::new(Align::Min, vertical),
            reading: Some((true, vertical)),
            id_salt: None,
        }
    }

    /// Anchor the contents at the reading end: the right edge of the parent
    /// [`Ui`] in a left-to-right interface, the left edge in a right-to-left one.
    ///
    /// The vertical [`Align`] applies as-is.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// egui::Aligned::end(egui::Align::BOTTOM).show(ui, |ui| {
    ///     let _ = ui.button("At the reading end (bottom) edge");
    /// });
    /// # });
    /// ```
    #[inline]
    pub fn end(vertical: Align) -> Self {
        Self {
            // Resolved from the reading direction in `show`; ignored until then.
            align2: Align2::new(Align::Max, vertical),
            reading: Some((false, vertical)),
            id_salt: None,
        }
    }

    /// A source for the unique [`crate::Id`], e.g. `.id_salt("second_aligned")` or `.id_salt(loop_index)`.
    ///
    /// Needed if you have several [`Aligned`] in the same [`Ui`].
    #[inline]
    pub fn id_salt(mut self, id_salt: impl AsIdSalt) -> Self {
        self.id_salt = Some(IdSalt::new(id_salt));
        self
    }

    /// Show the contents.
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let Self {
            align2,
            reading,
            id_salt,
        } = self;

        // Resolve a reading-relative anchor against the ui's reading direction,
        // exactly once; everything below only deals in the physical [`Align2`].
        let align2 = if let Some((is_start, vertical)) = reading {
            let direction = if ui.is_rtl() {
                ReadingDirection::Rtl
            } else {
                ReadingDirection::Ltr
            };
            let horizontal = if is_start {
                Align::start(direction)
            } else {
                Align::end(direction)
            };
            Align2::new(horizontal, vertical)
        } else {
            align2
        };
        let reading_start_anchored = reading.is_some_and(|(is_start, _)| is_start);

        let id_salt = id_salt.unwrap_or_else(|| IdSalt::new("aligned"));
        let id = ui.make_persistent_id(id_salt);
        let size_id = id.with("size");

        let available_rect = ui.available_rect_before_wrap();
        let cursor_before = ui.cursor();
        let last_size: Option<Vec2> = ui.data(|d| d.get_temp(size_id));

        // Does the contents `Ui` anchor its x axis at the right edge of its rect?
        // The child `Ui` inherits this layout, so it answers for the contents: a
        // vertical layout anchors the cross axis by `cross_align`, a horizontal
        // one flows from (and so anchors at) its `main_dir` start edge.
        let child_anchors_right_x = if ui.layout().is_vertical() {
            ui.layout().cross_align == Align::Max
        } else {
            ui.layout().main_dir() == Direction::RightToLeft
        };

        let content_rect = if let Some(size) = last_size {
            let placed = place(align2, size, available_rect, reading_start_anchored);
            let left_top = placed.left_top();
            if reading.is_some() && child_anchors_right_x {
                // A reading-relative anchor, and the contents `Ui` anchors at the
                // right edge of its max rect (a cross-align-`Max` vertical child, or
                // a right-to-left main direction): keep the full available size by
                // growing the max rect leftward from the placed right edge — the
                // mirror of the construction below. Growing rightward instead would
                // put the max rect's right edge (what the child anchors at) past
                // the anchor.
                Rect::from_min_max(
                    pos2(placed.max.x - available_rect.width(), left_top.y),
                    pos2(placed.max.x, left_top.y + available_rect.height()),
                )
            } else {
                // Keep the full available size, in case the contents grow this pass:
                Rect::from_min_size(left_top, available_rect.size())
            }
        } else {
            available_rect
        };

        let mut builder = UiBuilder::new().scope_id(id).max_rect(content_rect);
        if last_size.is_none() {
            // We don't know the size yet, so we can't place the contents correctly.
            // Add them invisibly to measure them, and then try again.
            builder = builder.invisible();
        }

        let response = ui.scope_builder(builder, add_contents);

        let size = response.response.rect.size();
        if last_size != Some(size) {
            ui.ctx().request_discard("Aligned: size changed");
        }
        ui.data_mut(|d| d.insert_temp(size_id, size));

        // Reserve space in the parent like a `Panel` would:
        // if the contents sit at the end of the parent's main axis,
        // move the cursor so that further widgets are placed _before_ the contents.
        // (`scope_builder` has already advanced the cursor _past_ the contents, which is
        // the right thing when they are at the start of the main axis, or centered.)
        //
        // We use where the contents _will_ end up (given their now known size),
        // so that this is correct also on the invisible measuring pass.
        let placed_rect = place(align2, size, available_rect, reading_start_anchored);
        let main_dir = ui.layout().main_dir();
        let axis = usize::from(main_dir.is_vertical()); // 0 = x, 1 = y
        let spacing = ui.spacing().item_spacing[axis];
        let mut cursor = cursor_before;
        match (main_dir, align2.0[axis]) {
            (Direction::LeftToRight | Direction::TopDown, Align::Max) => {
                cursor.max[axis] = placed_rect.min[axis] - spacing;
                ui.set_cursor(cursor);
            }
            (Direction::RightToLeft | Direction::BottomUp, Align::Min) => {
                cursor.min[axis] = placed_rect.max[axis] + spacing;
                ui.set_cursor(cursor);
            }
            _ => {}
        }

        response
    }
}

/// Where to put contents of the given size, aligned within `available_rect`.
///
/// Never places the contents above or to the left of the available space, even if
/// the contents are larger than the available space — except that a reading-start
/// anchor (the horizontal [`Align::Max`] side of a reading-relative [`Aligned`],
/// i.e. the right edge under rtl) instead keeps the content's right edge at the
/// available space's right edge, so oversized contents spill toward the reading
/// end and the reader's entry point keeps showing the content's start.
fn place(align2: Align2, size: Vec2, available_rect: Rect, reading_start_anchored: bool) -> Rect {
    let placed = align2.align_size_within_rect(size, available_rect);
    if reading_start_anchored && align2.0[0] == Align::Max {
        // A reading-relative anchor on the physical right edge (the reading start
        // under rtl): clamp the right edge instead of the left, mirroring the
        // top-left clamp below.
        let right_top = placed.right_top().min(available_rect.right_top());
        let min_y = placed.min.y.max(available_rect.min.y);
        Rect::from_min_size(pos2(right_top.x - size.x, min_y), size)
    } else {
        let left_top = placed.left_top().max(available_rect.left_top());
        Rect::from_min_size(left_top, size)
    }
}

#[cfg(test)]
mod tests {
    use emath::vec2;

    use crate::{Align, Align2, Aligned, Context, FontDefinitions, Rect};

    /// Run one frame of a single [`Aligned`] and return the rect of its contents, and the rect of the parent.
    fn run_frame(ctx: &Context) -> (Rect, Rect) {
        let mut rects = None;
        let output = ctx.run_ui(Default::default(), |ui| {
            let available_rect = ui.available_rect_before_wrap();
            let response = Aligned::new(Align2::RIGHT_BOTTOM).show(ui, |ui| {
                ui.allocate_space(vec2(30.0, 20.0));
            });
            rects = Some((response.response.rect, available_rect));
        });
        output.drop_without_applying_deltas();
        rects.expect("The closure was not called")
    }

    /// Run two passes (measure, then place) of a single [`Aligned`] holding
    /// contents of the given size, and return the rect of the contents, and the
    /// rect of the parent.
    ///
    /// With `wrap_in_ltr_horizontal` the [`Aligned`] sits in an explicit
    /// left-to-right horizontal [`Ui`](crate::Ui), whose contents flow from the
    /// left edge of their max rect (the interface-direction flag does not change
    /// a horizontal layout's main direction).
    fn run_aligned(
        ctx: &Context,
        aligned: Aligned,
        content_size: emath::Vec2,
        wrap_in_ltr_horizontal: bool,
    ) -> (Rect, Rect) {
        let mut rects = None;
        for _ in 0..2 {
            let output = ctx.run_ui(Default::default(), |ui| {
                let available_rect = ui.available_rect_before_wrap();
                let content_rect = if wrap_in_ltr_horizontal {
                    ui.with_layout(crate::Layout::left_to_right(Align::Min), |ui| {
                        aligned
                            .show(ui, |ui| ui.allocate_space(content_size))
                            .response
                            .rect
                    })
                    .inner
                } else {
                    aligned
                        .show(ui, |ui| ui.allocate_space(content_size))
                        .response
                        .rect
                };
                rects = Some((content_rect, available_rect));
            });
            output.drop_without_applying_deltas();
        }
        rects.expect("The closure was not called")
    }

    #[test]
    fn contents_end_up_aligned() {
        let ctx = Context::default();
        ctx.set_fonts(FontDefinitions::empty());

        // First frame measures, second frame places:
        run_frame(&ctx);
        let (content_rect, available_rect) = run_frame(&ctx);

        assert_eq!(content_rect.size(), vec2(30.0, 20.0));
        assert!((content_rect.right() - available_rect.right()).abs() < 0.01);
        assert!((content_rect.bottom() - available_rect.bottom()).abs() < 0.01);
    }

    /// `Aligned::start` hugs the reading-start edge: the left edge under ltr,
    /// the right edge under rtl. `Aligned::end` is its mirror.
    #[test]
    fn reading_constructors_hug_their_reading_edge() {
        let content_size = vec2(30.0, 20.0);

        let ltr_ctx = Context::default();
        ltr_ctx.set_fonts(FontDefinitions::empty());

        let (content_rect, available_rect) = run_aligned(
            &ltr_ctx,
            Aligned::start(Align::Min).id_salt("start"),
            content_size,
            false,
        );
        assert_eq!(content_rect.size(), content_size);
        assert!(
            (content_rect.left() - available_rect.left()).abs() < 0.01,
            "LTR: `Aligned::start` should hug the reading-start (left) edge \
             (content {content_rect:?}, available {available_rect:?})"
        );

        let (content_rect, available_rect) = run_aligned(
            &ltr_ctx,
            Aligned::end(Align::Min).id_salt("end"),
            content_size,
            false,
        );
        assert!(
            (content_rect.right() - available_rect.right()).abs() < 0.01,
            "LTR: `Aligned::end` should hug the reading-end (right) edge \
             (content {content_rect:?}, available {available_rect:?})"
        );

        let rtl_ctx = Context::default();
        rtl_ctx.set_right_to_left(true);
        rtl_ctx.set_fonts(FontDefinitions::empty());

        let (content_rect, available_rect) = run_aligned(
            &rtl_ctx,
            Aligned::start(Align::Min).id_salt("start"),
            content_size,
            false,
        );
        assert!(
            (content_rect.right() - available_rect.right()).abs() < 0.01,
            "RTL: `Aligned::start` should hug the reading-start (right) edge \
             (content {content_rect:?}, available {available_rect:?})"
        );

        let (content_rect, available_rect) = run_aligned(
            &rtl_ctx,
            Aligned::end(Align::Min).id_salt("end"),
            content_size,
            false,
        );
        assert!(
            (content_rect.left() - available_rect.left()).abs() < 0.01,
            "RTL: `Aligned::end` should hug the reading-end (left) edge \
             (content {content_rect:?}, available {available_rect:?})"
        );
    }

    /// Oversized contents anchored at the reading start spill toward the reading
    /// end: under rtl the reading-start (right) edge stays at the available right
    /// edge, instead of the top-left clamp shoving the contents rightward past it.
    /// Asserted both in a plain (vertical, cross-align-`Max` under the flag) parent
    /// and in an explicit left-to-right horizontal parent, whose contents flow from
    /// their max rect's left edge.
    ///
    /// The physical [`Aligned::new`] keeps today's contract verbatim in both
    /// directions (asserted against the behavior measured on the parent commit).
    #[test]
    fn oversized_reading_start_content_clamps_at_the_reading_start_edge() {
        // Wider than the default 10_000 x 10_000 test viewport.
        let content_size = vec2(12_000.0, 20.0);

        let rtl_ctx = Context::default();
        rtl_ctx.set_right_to_left(true);
        rtl_ctx.set_fonts(FontDefinitions::empty());

        let (content_rect, available_rect) = run_aligned(
            &rtl_ctx,
            Aligned::start(Align::Min).id_salt("start"),
            content_size,
            false,
        );
        assert!(
            (content_rect.right() - available_rect.right()).abs() < 0.01,
            "RTL: oversized `Aligned::start` contents should keep their reading-start \
             (right) edge at the available right edge and spill leftward \
             (content {content_rect:?}, available {available_rect:?})"
        );

        // In a left-to-right horizontal parent the contents flow from their max
        // rect's left edge, so the placed left edge carries them: the reading-start
        // (right) edge still lands on the available right edge.
        let (content_rect, available_rect) = run_aligned(
            &rtl_ctx,
            Aligned::start(Align::Min).id_salt("start-ltr-wrap"),
            content_size,
            true,
        );
        assert!(
            (content_rect.right() - available_rect.right()).abs() < 0.01,
            "RTL + ltr-wrapped: oversized `Aligned::start` contents should keep their \
             reading-start (right) edge at the available right edge \
             (content {content_rect:?}, available {available_rect:?})"
        );

        // The physical constructor keeps today's behavior verbatim: under the flag
        // the top-left-clamped growth rect plus the cross-align-`Max` child put the
        // contents' right edge at the available right edge; under ltr the top-left
        // clamp wins and the right edge is shoved past it (documented contract).
        let (physical_rect, available_rect) = run_aligned(
            &rtl_ctx,
            Aligned::new(Align2::RIGHT_BOTTOM).id_salt("physical"),
            content_size,
            false,
        );
        assert!(
            (physical_rect.right() - available_rect.right()).abs() < 0.01,
            "RTL: the physical `Aligned::new` keeps today's oversized behavior \
             (right edge at the available right edge) (content {physical_rect:?}, \
             available {available_rect:?})"
        );

        let ltr_ctx = Context::default();
        ltr_ctx.set_fonts(FontDefinitions::empty());

        let (content_rect, available_rect) = run_aligned(
            &ltr_ctx,
            Aligned::start(Align::Min).id_salt("start"),
            content_size,
            false,
        );
        assert!(
            (content_rect.left() - available_rect.left()).abs() < 0.01,
            "LTR: oversized `Aligned::start` contents keep their reading-start (left) \
             edge at the available left edge (content {content_rect:?}, \
             available {available_rect:?})"
        );

        let (physical_rect, available_rect) = run_aligned(
            &ltr_ctx,
            Aligned::new(Align2::RIGHT_BOTTOM).id_salt("physical"),
            content_size,
            false,
        );
        assert!(
            (physical_rect.left() - available_rect.left()).abs() < 0.01
                && physical_rect.right() > available_rect.right(),
            "LTR: the physical `Aligned::new` keeps its top-left clamp contract \
             (left edge pinned to the available left edge, right edge shoved past \
             the available right edge) (content {physical_rect:?}, \
             available {available_rect:?})"
        );
    }
}
