use crate::{Align, Layout, Painter, Pos2, Rect, Region, Vec2, grid, vec2};
use emath::GuiRounding as _;

#[cfg(debug_assertions)]
use crate::{Align2, Color32, Stroke};

pub(crate) struct Placer {
    /// If set this will take precedence over [`crate::layout`].
    grid: Option<grid::GridLayout>,
    layout: Layout,
    region: Region,
}

impl Placer {
    pub(crate) fn new(max_rect: Rect, layout: Layout) -> Self {
        let region = layout.region_from_max_rect(max_rect);
        Self {
            grid: None,
            layout,
            region,
        }
    }

    #[inline(always)]
    pub(crate) fn set_grid(&mut self, grid: grid::GridLayout) {
        self.grid = Some(grid);
    }

    pub(crate) fn save_grid(&mut self) {
        if let Some(grid) = &mut self.grid {
            grid.save();
        }
    }

    #[inline(always)]
    pub(crate) fn grid(&self) -> Option<&grid::GridLayout> {
        self.grid.as_ref()
    }

    #[inline(always)]
    pub(crate) fn is_grid(&self) -> bool {
        self.grid.is_some()
    }

    #[inline(always)]
    pub(crate) fn layout(&self) -> &Layout {
        &self.layout
    }

    #[inline(always)]
    pub(crate) fn prefer_right_to_left(&self, ui_is_rtl: bool) -> bool {
        self.layout.prefer_right_to_left()
            || ui_is_rtl && self.layout.is_vertical() && self.layout.cross_align == Align::Min
    }

    #[inline(always)]
    pub(crate) fn min_rect(&self) -> Rect {
        self.region.min_rect
    }

    #[inline(always)]
    pub(crate) fn max_rect(&self) -> Rect {
        self.region.max_rect
    }

    #[inline(always)]
    pub(crate) fn force_set_min_rect(&mut self, min_rect: Rect) {
        self.region.min_rect = min_rect;
    }

    #[inline(always)]
    pub(crate) fn cursor(&self) -> Rect {
        self.region.cursor
    }

    #[inline(always)]
    pub(crate) fn set_cursor(&mut self, cursor: Rect) {
        self.region.cursor = cursor;
    }
}

impl Placer {
    pub(crate) fn align_size_within_rect(&self, size: Vec2, outer: Rect) -> Rect {
        if let Some(grid) = &self.grid {
            grid.align_size_within_rect(size, outer)
        } else {
            self.layout.align_size_within_rect(size, outer)
        }
    }

    pub(crate) fn available_rect_before_wrap(&self) -> Rect {
        if let Some(grid) = &self.grid {
            grid.available_rect(&self.region)
        } else {
            self.layout.available_rect_before_wrap(&self.region)
        }
        .round_ui()
    }

    /// Amount of space available for a widget.
    /// For wrapping layouts, this is the maximum (after wrap).
    pub(crate) fn available_size(&self) -> Vec2 {
        if let Some(grid) = &self.grid {
            grid.available_rect(&self.region).size()
        } else {
            self.layout.available_size(&self.region)
        }
    }

    /// Returns where to put the next widget that is of the given size.
    /// The returned `frame_rect` will always be justified along the cross axis.
    /// This is what you then pass to `advance_after_rects`.
    /// Use `justify_and_align` to get the inner `widget_rect`.
    pub(crate) fn next_space(&self, child_size: Vec2, item_spacing: Vec2) -> Rect {
        debug_assert!(
            0.0 <= child_size.x && 0.0 <= child_size.y,
            "Negative child size: {child_size:?}"
        );
        self.region.sanity_check();
        if let Some(grid) = &self.grid {
            grid.next_cell(self.region.cursor, child_size)
        } else {
            self.layout
                .next_frame(&self.region, child_size, item_spacing)
        }
    }

    /// Where do we expect a zero-sized widget to be placed?
    pub(crate) fn next_widget_position(&self) -> Pos2 {
        if let Some(grid) = &self.grid {
            grid.next_cell(self.region.cursor, Vec2::ZERO).center()
        } else {
            self.layout.next_widget_position(&self.region)
        }
    }

    /// Apply justify or alignment after calling `next_space`.
    pub(crate) fn justify_and_align(&self, rect: Rect, child_size: Vec2) -> Rect {
        debug_assert!(!rect.any_nan(), "rect: {rect:?}");
        debug_assert!(!child_size.any_nan(), "child_size is NaN: {child_size:?}");

        if let Some(grid) = &self.grid {
            grid.justify_and_align(rect, child_size)
        } else {
            self.layout.justify_and_align(rect, child_size)
        }
    }

    /// Advance the cursor by this many points.
    /// [`Self::min_rect`] will expand to contain the cursor.
    ///
    /// Note that `advance_cursor` isn't supported when in a grid layout.
    pub(crate) fn advance_cursor(&mut self, amount: f32) {
        debug_assert!(
            self.grid.is_none(),
            "You cannot advance the cursor when in a grid layout"
        );
        self.layout.advance_cursor(&mut self.region, amount);
    }

    /// Advance cursor after a widget was added to a specific rectangle
    /// and expand the region `min_rect`.
    ///
    /// * `frame_rect`: the frame inside which a widget was e.g. centered
    /// * `widget_rect`: the actual rect used by the widget
    pub(crate) fn advance_after_rects(
        &mut self,
        frame_rect: Rect,
        widget_rect: Rect,
        item_spacing: Vec2,
    ) {
        debug_assert!(!frame_rect.any_nan(), "frame_rect: {frame_rect:?}");
        debug_assert!(
            !widget_rect.any_nan(),
            "widget_rect is NaN: {widget_rect:?}"
        );
        self.region.sanity_check();

        if let Some(grid) = &mut self.grid {
            grid.advance(&mut self.region.cursor, frame_rect, widget_rect);
        } else {
            self.layout.advance_after_rects(
                &mut self.region.cursor,
                frame_rect,
                widget_rect,
                item_spacing,
            );
        }

        // The reported `min_rect` must hug the content. A frame stretched to the available
        // width (vertical layouts with `Align::Center` cross alignment, or justified ones —
        // see `Layout::next_frame_ignore_wrap`) is placement-only and must not widen the
        // minimum size; for every other vertical layout the frame already equals the widget
        // rect. Horizontal layouts and grids keep folding the frame: there the frame is
        // load-bearing (row height / cell size), not a measurement defect.
        let measured_rect = if self.grid.is_none() && self.layout.is_vertical() {
            widget_rect
        } else {
            frame_rect
        };
        self.expand_to_include_rect(measured_rect);

        self.region.sanity_check();
    }

    /// Move to the next row in a grid layout or wrapping layout.
    /// Otherwise does nothing.
    pub(crate) fn end_row(&mut self, item_spacing: Vec2, painter: &Painter) {
        if let Some(grid) = &mut self.grid {
            grid.end_row(&mut self.region.cursor, painter);
        } else {
            self.layout.end_row(&mut self.region, item_spacing);
        }
    }

    /// Set row height in horizontal wrapping layout.
    pub(crate) fn set_row_height(&mut self, height: f32) {
        self.layout.set_row_height(&mut self.region, height);
    }
}

impl Placer {
    /// Expand the `min_rect` and `max_rect` of this ui to include a child at the given rect.
    pub(crate) fn expand_to_include_rect(&mut self, rect: Rect) {
        self.region.expand_to_include_rect(rect);
    }

    /// Expand the `min_rect` and `max_rect` of this ui to include a child at the given x-coordinate.
    pub(crate) fn expand_to_include_x(&mut self, x: f32) {
        self.region.expand_to_include_x(x);
    }

    /// Expand the `min_rect` and `max_rect` of this ui to include a child at the given y-coordinate.
    pub(crate) fn expand_to_include_y(&mut self, y: f32) {
        self.region.expand_to_include_y(y);
    }

    fn next_widget_space_ignore_wrap_justify(&self, size: Vec2) -> Rect {
        self.layout
            .next_widget_space_ignore_wrap_justify(&self.region, size)
    }

    // Anchoring of the width setters (`set_max_width` / `set_min_width`):
    // they claim their span with the layout-derived anchor
    // (`Layout::horizontal_align`) — a vertical body claims from its
    // `cross_align` edge — in every pass and every interface direction. The
    // anchor must stay layout-derived: `Layout::region_from_max_rect` seeds
    // `min_rect` as a zero-size rect at the `cross_align` edge,
    // `Region::expand_to_include_x` unions any claim into it, and the
    // `region.max_rect |= region.min_rect` guard below re-anchors every
    // result to the body's content edge — so a claim from any other edge
    // would only balloon the measured size to the offered span. In
    // `Align::Max` bodies under a right-to-left interface (the popup/menu
    // bodies) the layout-derived anchor already IS the reading edge, so
    // reading-edge capping works there. The regression pins live in
    // `crates/egui/tests/reading_edge_width.rs`.
    /// Set the maximum width of the ui.
    /// You won't be able to shrink it below the current minimum size.
    pub(crate) fn set_max_width(&mut self, width: f32) {
        let rect = self.next_widget_space_ignore_wrap_justify(vec2(width, 0.0));
        let region = &mut self.region;
        region.max_rect.min.x = rect.min.x;
        region.max_rect.max.x = rect.max.x;
        region.max_rect |= region.min_rect; // make sure we didn't shrink too much

        region.cursor.min.x = region.max_rect.min.x;
        region.cursor.max.x = region.max_rect.max.x;

        region.sanity_check();
    }

    /// Set the maximum height of the ui.
    /// You won't be able to shrink it below the current minimum size.
    pub(crate) fn set_max_height(&mut self, height: f32) {
        let rect = self.next_widget_space_ignore_wrap_justify(vec2(0.0, height));
        let region = &mut self.region;
        region.max_rect.min.y = rect.min.y;
        region.max_rect.max.y = rect.max.y;
        region.max_rect |= region.min_rect; // make sure we didn't shrink too much

        region.cursor.min.y = region.max_rect.min.y;
        region.cursor.max.y = region.max_rect.max.y;

        region.sanity_check();
    }

    /// Set the minimum width of the ui.
    /// This can't shrink the ui, only make it larger.
    pub(crate) fn set_min_width(&mut self, width: f32) {
        if width <= 0.0 {
            return;
        }
        let rect = self.next_widget_space_ignore_wrap_justify(vec2(width, 0.0));
        self.region.expand_to_include_x(rect.min.x);
        self.region.expand_to_include_x(rect.max.x);
    }

    /// Set the minimum height of the ui.
    /// This can't shrink the ui, only make it larger.
    pub(crate) fn set_min_height(&mut self, height: f32) {
        if height <= 0.0 {
            return;
        }
        let rect = self.next_widget_space_ignore_wrap_justify(vec2(0.0, height));
        self.region.expand_to_include_y(rect.min.y);
        self.region.expand_to_include_y(rect.max.y);
    }
}

impl Placer {
    #[cfg(debug_assertions)]
    pub(crate) fn debug_paint_cursor(&self, painter: &crate::Painter, text: impl ToString) {
        let stroke = Stroke::new(1.0, Color32::DEBUG_COLOR);

        if let Some(grid) = &self.grid {
            let rect = grid.next_cell(self.cursor(), Vec2::splat(0.0));
            painter.rect_stroke(rect, 1.0, stroke, epaint::StrokeKind::Inside);
            let align = Align2::CENTER_CENTER;
            painter.debug_text(align.pos_in_rect(&rect), align, stroke.color, text);
        } else {
            self.layout
                .paint_text_at_cursor(painter, &self.region, stroke, text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Placer;
    use crate::{Align, Context, Direction, Layout, RawInput, Rect, UiBuilder, Vec2, pos2};

    fn placer(layout: Layout) -> Placer {
        Placer::new(
            Rect::from_min_size(pos2(0.0, 0.0), crate::vec2(100.0, 100.0)),
            layout,
        )
    }

    /// A `Ui` built under a right-to-left interface resolves the effective
    /// cross align of a vertical `Align::Min` body to `Max` at construction;
    /// under a left-to-right interface it stays `Min`.
    #[test]
    fn ui_construction_resolves_the_effective_cross_align_of_vertical_min_bodies() {
        for (rtl, expected) in [(false, Align::Min), (true, Align::Max)] {
            let ctx = Context::default();
            ctx.set_right_to_left(rtl);
            let output = ctx.run_ui(
                RawInput {
                    screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), Vec2::splat(200.0))),
                    ..Default::default()
                },
                |ui| {
                    assert_eq!(ui.layout().cross_align(), expected, "root ui, rtl = {rtl}");
                    let child = ui.new_child(UiBuilder::new().layout(Layout::top_down(Align::Min)));
                    assert_eq!(
                        child.layout().cross_align(),
                        expected,
                        "child ui, rtl = {rtl}"
                    );
                },
            );
            output.drop_without_applying_deltas();
        }
    }

    /// The RTL term applies only to vertical `Align::Min` layouts,
    /// and `rtl = false` matches the old pure-layout answers.
    #[test]
    fn prefer_right_to_left_truth_table() {
        let layouts = [
            Layout::left_to_right(Align::Center),
            Layout::right_to_left(Align::Center),
            Layout::top_down(Align::Min),
            Layout::top_down(Align::Center),
            Layout::top_down(Align::Max),
            Layout::top_down_justified(Align::Min),
            Layout::top_down_justified(Align::Max),
        ];

        for layout in layouts {
            let placer = placer(layout);

            // With the flag off, the answer is exactly the old pure-layout one:
            assert_eq!(
                placer.prefer_right_to_left(false),
                layout.prefer_right_to_left(),
                "{layout:?} with ui_is_rtl=false"
            );

            // With the flag on, only vertical `Align::Min` layouts gain the RTL term:
            let expected = layout.prefer_right_to_left()
                || layout.is_vertical() && layout.cross_align == Align::Min;
            assert_eq!(
                placer.prefer_right_to_left(true),
                expected,
                "{layout:?} with ui_is_rtl=true"
            );
        }
    }

    #[test]
    fn right_to_left_interface_mirrors_nested_rows_of_vertical_bodies() {
        // `ui.vertical` uses this exact layout:
        let body = Layout::top_down(Align::Min);
        assert!(placer(body).prefer_right_to_left(true));
        assert!(!placer(body).prefer_right_to_left(false));

        // Horizontal rows are never mirrored by the interface term:
        let row = Layout::left_to_right(Align::Center);
        assert!(!placer(row).prefer_right_to_left(true));

        // A layout that already runs right-to-left is right-to-left either way:
        let rtl_row = Layout::right_to_left(Align::Center);
        assert!(placer(rtl_row).prefer_right_to_left(true));
        assert!(placer(rtl_row).prefer_right_to_left(false));

        // Vertical `Align::Max` is right-to-left regardless of the interface direction:
        let max_body = Layout::top_down_justified(Align::Max);
        assert!(placer(max_body).prefer_right_to_left(true));
        assert!(placer(max_body).prefer_right_to_left(false));

        // Sanity: `Direction::RightToLeft` main dir is what makes the pure-layout term true.
        assert_eq!(rtl_row.main_dir, Direction::RightToLeft);
    }

    /// The reported `min_rect` hugs the widget for vertical layouts — even cross-`Center`,
    /// whose frame is stretched to the available width and is placement-only — while
    /// horizontal layouts keep folding the frame, whose cross extent is the row height.
    #[test]
    fn advance_after_rects_min_rect_hugs_widget_for_vertical_center() {
        // Vertical cross-Center: the frame is stretched to the available width,
        // but the reported minimum must stay content-tight:
        let mut placer = Placer::new(
            Rect::from_min_size(pos2(0.0, 0.0), crate::vec2(300.0, 100.0)),
            Layout::top_down(Align::Center),
        );
        let frame = placer.next_space(crate::vec2(50.0, 10.0), Vec2::ZERO);
        assert_eq!(
            frame.width(),
            300.0,
            "the frame is stretched to the available width"
        );
        let widget = placer.justify_and_align(frame, crate::vec2(50.0, 10.0));
        placer.advance_after_rects(frame, widget, Vec2::ZERO);
        assert_eq!(placer.min_rect().width(), 50.0);

        // Horizontal cross-Center: the frame is stretched to the available height, and the
        // reported minimum must keep that frame (a widget fold would shrink every row):
        let mut placer = Placer::new(
            Rect::from_min_size(pos2(0.0, 0.0), crate::vec2(300.0, 20.0)),
            Layout::left_to_right(Align::Center),
        );
        let frame = placer.next_space(crate::vec2(50.0, 15.0), Vec2::ZERO);
        assert_eq!(
            frame.height(),
            20.0,
            "the frame is stretched to the available height"
        );
        let widget = placer.justify_and_align(frame, crate::vec2(50.0, 15.0));
        placer.advance_after_rects(frame, widget, Vec2::ZERO);
        assert_eq!(placer.min_rect().height(), 20.0);
    }
}
