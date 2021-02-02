use crate::*;

pub(crate) struct Placer {
    /// If set this will take precedence over [`layout`].
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

    pub(crate) fn set_grid(&mut self, grid: grid::GridLayout) {
        self.grid = Some(grid);
    }

    pub(crate) fn save_grid(&mut self) {
        if let Some(grid) = &mut self.grid {
            grid.save();
        }
    }

    pub(crate) fn is_grid(&self) -> bool {
        self.grid.is_some()
    }

    pub(crate) fn layout(&self) -> &Layout {
        &self.layout
    }

    pub(crate) fn prefer_right_to_left(&self) -> bool {
        self.layout.prefer_right_to_left()
    }

    pub(crate) fn min_rect(&self) -> Rect {
        self.region.min_rect
    }

    pub(crate) fn max_rect(&self) -> Rect {
        self.region.max_rect
    }

    pub(crate) fn max_rect_finite(&self) -> Rect {
        self.region.max_rect_finite()
    }

    pub(crate) fn force_set_min_rect(&mut self, min_rect: Rect) {
        self.region.min_rect = min_rect;
    }

    pub(crate) fn cursor(&self) -> Pos2 {
        self.region.cursor
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
    }

    pub(crate) fn available_rect_before_wrap_finite(&self) -> Rect {
        if let Some(grid) = &self.grid {
            grid.available_rect_finite(&self.region)
        } else {
            self.layout.available_rect_before_wrap_finite(&self.region)
        }
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
    /// Use `justify_or_align` to get the inner `widget_rect`.
    pub(crate) fn next_space(&self, child_size: Vec2, item_spacing: Vec2) -> Rect {
        if let Some(grid) = &self.grid {
            grid.next_cell(self.region.cursor, child_size)
        } else {
            self.layout
                .next_space(&self.region, child_size, item_spacing)
        }
    }

    /// Apply justify or alignment after calling `next_space`.
    pub(crate) fn justify_or_align(&self, rect: Rect, child_size: Vec2) -> Rect {
        if let Some(grid) = &self.grid {
            grid.justify_or_align(rect, child_size)
        } else {
            self.layout.justify_or_align(rect, child_size)
        }
    }

    /// Advance the cursor by this many points.
    /// [`Self::min_rect`] will expand to contain the cursor.
    pub(crate) fn advance_cursor(&mut self, amount: f32) {
        debug_assert!(
            self.grid.is_none(),
            "You cannot advance the cursor when in a grid layout"
        );
        self.layout.advance_cursor(&mut self.region.cursor, amount);

        self.region
            .expand_to_include_rect(Rect::from_min_size(self.cursor(), Vec2::zero()));
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
        if let Some(grid) = &mut self.grid {
            grid.advance(&mut self.region.cursor, frame_rect, widget_rect)
        } else {
            self.layout.advance_after_rects(
                &mut self.region.cursor,
                frame_rect,
                widget_rect,
                item_spacing,
            )
        }
        self.region.expand_to_include_rect(widget_rect);
    }

    /// Move to the next row in a grid layout or wrapping layout.
    /// Otherwise does nothing.
    pub(crate) fn end_row(&mut self, item_spacing: Vec2, painter: &Painter) {
        if let Some(grid) = &mut self.grid {
            grid.end_row(&mut self.region.cursor, painter)
        } else {
            self.layout.end_row(&mut self.region, item_spacing)
        }
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

    /// Set the maximum width of the ui.
    /// You won't be able to shrink it below the current minimum size.
    pub(crate) fn set_max_width(&mut self, width: f32) {
        #![allow(clippy::float_cmp)]
        let Self { layout, region, .. } = self;
        if layout.main_dir() == Direction::RightToLeft {
            debug_assert_eq!(region.min_rect.max.x, region.max_rect.max.x);
            region.max_rect.min.x = region.max_rect.max.x - width.at_least(region.min_rect.width());
        } else {
            debug_assert_eq!(region.min_rect.min.x, region.max_rect.min.x);
            region.max_rect.max.x = region.max_rect.min.x + width.at_least(region.min_rect.width());
        }
    }

    /// Set the maximum height of the ui.
    /// You won't be able to shrink it below the current minimum size.
    pub(crate) fn set_max_height(&mut self, height: f32) {
        #![allow(clippy::float_cmp)]
        let Self { layout, region, .. } = self;
        if layout.main_dir() == Direction::BottomUp {
            debug_assert_eq!(region.min_rect.max.y, region.max_rect.max.y);
            region.max_rect.min.y =
                region.max_rect.max.y - height.at_least(region.min_rect.height());
        } else {
            debug_assert_eq!(region.min_rect.min.y, region.max_rect.min.y);
            region.max_rect.max.y =
                region.max_rect.min.y + height.at_least(region.min_rect.height());
        }
    }

    /// Set the minimum width of the ui.
    /// This can't shrink the ui, only make it larger.
    pub(crate) fn set_min_width(&mut self, width: f32) {
        #![allow(clippy::float_cmp)]
        let Self { layout, region, .. } = self;
        if layout.main_dir() == Direction::RightToLeft {
            debug_assert_eq!(region.min_rect.max.x, region.max_rect.max.x);
            let min_rect = &mut region.min_rect;
            min_rect.min.x = min_rect.min.x.min(min_rect.max.x - width);
        } else {
            debug_assert_eq!(region.min_rect.min.x, region.max_rect.min.x);
            let min_rect = &mut region.min_rect;
            min_rect.max.x = min_rect.max.x.max(min_rect.min.x + width);
        }
        region.max_rect = region.max_rect.union(region.min_rect);
    }

    /// Set the minimum height of the ui.
    /// This can't shrink the ui, only make it larger.
    pub(crate) fn set_min_height(&mut self, height: f32) {
        #![allow(clippy::float_cmp)]
        let Self { layout, region, .. } = self;
        if layout.main_dir() == Direction::BottomUp {
            debug_assert_eq!(region.min_rect.max.y, region.max_rect.max.y);
            let min_rect = &mut region.min_rect;
            min_rect.min.y = min_rect.min.y.min(min_rect.max.y - height);
        } else {
            debug_assert_eq!(region.min_rect.min.y, region.max_rect.min.y);
            let min_rect = &mut region.min_rect;
            min_rect.max.y = min_rect.max.y.max(min_rect.min.y + height);
        }
        region.max_rect = region.max_rect.union(region.min_rect);
    }
}

impl Placer {
    pub(crate) fn debug_paint_cursor(&self, painter: &crate::Painter) {
        let color = Color32::GREEN;
        let stroke = Stroke::new(2.0, color);

        if let Some(grid) = &self.grid {
            painter.rect_stroke(grid.next_cell(self.cursor(), Vec2::splat(0.0)), 1.0, stroke)
        } else {
            self.layout
                .debug_paint_cursor(&self.region, stroke, painter)
        }
    }
}
