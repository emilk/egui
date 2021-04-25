use crate::{emath::*, Align};
use std::f32::INFINITY;

// ----------------------------------------------------------------------------

/// This describes the bounds and existing contents of an [`Ui`][`crate::Ui`].
/// It is what is used and updated by [`Layout`] when adding new widgets.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Region {
    /// This is the minimal size of the `Ui`.
    /// When adding new widgets, this will generally expand.
    ///
    /// Always finite.
    ///
    /// The bounding box of all child widgets, but not necessarily a tight bounding box
    /// since `Ui` can start with a non-zero min_rect size.
    pub min_rect: Rect,

    /// The maximum size of this `Ui`. This is a *soft max*
    /// meaning new widgets will *try* not to expand beyond it,
    /// but if they have to, they will.
    ///
    /// Text will wrap at `max_rect.right()`.
    /// Some widgets (like separator lines) will try to fill the full `max_rect` width of the ui.
    ///
    /// `max_rect` will always be at least the size of `min_rect`.
    ///
    /// If the `max_rect` size is zero, it is a signal that child widgets should be as small as possible.
    /// If the `max_rect` size is infinite, it is a signal that child widgets should take up as much room as they want.
    pub max_rect: Rect,

    /// Where the next widget will be put.
    ///
    /// One side of this will always be infinite: the direction in which new widgets will be added.
    /// The opposing side is what is incremented.
    /// The crossing sides are initialized to `max_rect`.
    ///
    /// So one can think of `cursor` as a constraint on the available region.
    ///
    /// If something has already been added, this will point ot `style.spacing.item_spacing` beyond the latest child.
    /// The cursor can thus be `style.spacing.item_spacing` pixels outside of the min_rect.
    pub(crate) cursor: Rect,
}

impl Region {
    /// This is like `max_rect`, but will never be infinite.
    /// If the desired rect is infinite ("be as big as you want")
    /// this will be bounded by `min_rect` instead.
    pub fn max_rect_finite(&self) -> Rect {
        let mut result = self.max_rect;
        if !result.min.x.is_finite() {
            result.min.x = self.min_rect.min.x;
        }
        if !result.min.y.is_finite() {
            result.min.y = self.min_rect.min.y;
        }
        if !result.max.x.is_finite() {
            result.max.x = self.min_rect.max.x;
        }
        if !result.max.y.is_finite() {
            result.max.y = self.min_rect.max.y;
        }
        result
    }

    /// Expand the `min_rect` and `max_rect` of this ui to include a child at the given rect.
    pub fn expand_to_include_rect(&mut self, rect: Rect) {
        self.min_rect = self.min_rect.union(rect);
        self.max_rect = self.max_rect.union(rect);
    }

    /// Ensure we are big enough to contain the given X-coordinate.
    /// This is sometimes useful to expand an ui to stretch to a certain place.
    pub fn expand_to_include_x(&mut self, x: f32) {
        self.min_rect.extend_with_x(x);
        self.max_rect.extend_with_x(x);
    }

    /// Ensure we are big enough to contain the given Y-coordinate.
    /// This is sometimes useful to expand an ui to stretch to a certain place.
    pub fn expand_to_include_y(&mut self, y: f32) {
        self.min_rect.extend_with_y(y);
        self.max_rect.extend_with_y(y);
    }
}

// ----------------------------------------------------------------------------

/// Layout direction, one of `LeftToRight`, `RightToLeft`, `TopDown`, `BottomUp`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(rename_all = "snake_case"))]
pub enum Direction {
    LeftToRight,
    RightToLeft,
    TopDown,
    BottomUp,
}

impl Direction {
    #[inline(always)]
    pub fn is_horizontal(self) -> bool {
        match self {
            Direction::LeftToRight | Direction::RightToLeft => true,
            Direction::TopDown | Direction::BottomUp => false,
        }
    }

    #[inline(always)]
    pub fn is_vertical(self) -> bool {
        match self {
            Direction::LeftToRight | Direction::RightToLeft => false,
            Direction::TopDown | Direction::BottomUp => true,
        }
    }
}

// ----------------------------------------------------------------------------

/// The layout of a [`Ui`][`crate::Ui`], e.g. "vertical & centered".
#[derive(Clone, Copy, Debug, PartialEq)]
// #[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct Layout {
    /// Main axis direction
    main_dir: Direction,

    /// If true, wrap around when reading the end of the main direction.
    /// For instance, for `main_dir == Direction::LeftToRight` this will
    /// wrap to a new row when we reach the right side of the `max_rect`.
    main_wrap: bool,

    /// How to align things on the main axis.
    main_align: Align,

    /// Justify the main axis?
    main_justify: bool,

    /// How to align things on the cross axis.
    /// For vertical layouts: put things to left, center or right?
    /// For horizontal layouts: put things to top, center or bottom?
    cross_align: Align,

    /// Justify the cross axis?
    /// For vertical layouts justify mean all widgets get maximum width.
    /// For horizontal layouts justify mean all widgets get maximum height.
    cross_justify: bool,
}

impl Default for Layout {
    fn default() -> Self {
        // TODO: Get from `Style` instead.
        // This is a very euro-centric default.
        Self {
            main_dir: Direction::TopDown,
            main_wrap: false,
            main_align: Align::TOP,
            main_justify: false,
            cross_align: Align::LEFT,
            cross_justify: false,
        }
    }
}

/// ## Constructors
impl Layout {
    #[inline(always)]
    pub fn left_to_right() -> Self {
        Self {
            main_dir: Direction::LeftToRight,
            main_wrap: false,
            main_align: Align::Center, // looks best to e.g. center text within a button
            main_justify: false,
            cross_align: Align::Center,
            cross_justify: false,
        }
    }

    #[inline(always)]
    pub fn right_to_left() -> Self {
        Self {
            main_dir: Direction::RightToLeft,
            main_wrap: false,
            main_align: Align::Center, // looks best to e.g. center text within a button
            main_justify: false,
            cross_align: Align::Center,
            cross_justify: false,
        }
    }

    #[inline(always)]
    pub fn top_down(cross_align: Align) -> Self {
        Self {
            main_dir: Direction::TopDown,
            main_wrap: false,
            main_align: Align::Center, // looks best to e.g. center text within a button
            main_justify: false,
            cross_align,
            cross_justify: false,
        }
    }

    /// Top-down layout justifed so that buttons etc fill the full available width.
    #[inline(always)]
    pub fn top_down_justified(cross_align: Align) -> Self {
        Self::top_down(cross_align).with_cross_justify(true)
    }

    #[inline(always)]
    pub fn bottom_up(cross_align: Align) -> Self {
        Self {
            main_dir: Direction::BottomUp,
            main_wrap: false,
            main_align: Align::Center, // looks best to e.g. center text within a button
            main_justify: false,
            cross_align,
            cross_justify: false,
        }
    }

    #[inline(always)]
    pub fn from_main_dir_and_cross_align(main_dir: Direction, cross_align: Align) -> Self {
        Self {
            main_dir,
            main_wrap: false,
            main_align: Align::Center, // looks best to e.g. center text within a button
            main_justify: false,
            cross_align,
            cross_justify: false,
        }
    }

    #[inline(always)]
    pub fn centered_and_justified(main_dir: Direction) -> Self {
        Self {
            main_dir,
            main_wrap: false,
            main_align: Align::Center,
            main_justify: true,
            cross_align: Align::Center,
            cross_justify: true,
        }
    }

    #[deprecated = "Use `top_down`"]
    pub fn vertical(cross_align: Align) -> Self {
        Self::top_down(cross_align)
    }

    #[deprecated = "Use `left_to_right`"]
    pub fn horizontal(cross_align: Align) -> Self {
        Self::left_to_right().with_cross_align(cross_align)
    }

    #[inline(always)]
    pub fn with_main_wrap(self, main_wrap: bool) -> Self {
        Self { main_wrap, ..self }
    }

    #[inline(always)]
    pub fn with_cross_align(self, cross_align: Align) -> Self {
        Self {
            cross_align,
            ..self
        }
    }

    #[inline(always)]
    pub fn with_cross_justify(self, cross_justify: bool) -> Self {
        Self {
            cross_justify,
            ..self
        }
    }
}

/// ## Inspectors
impl Layout {
    #[inline(always)]
    pub fn main_dir(&self) -> Direction {
        self.main_dir
    }

    #[inline(always)]
    pub fn main_wrap(&self) -> bool {
        self.main_wrap
    }

    #[inline(always)]
    pub fn cross_align(&self) -> Align {
        self.cross_align
    }

    #[inline(always)]
    pub fn cross_justify(&self) -> bool {
        self.cross_justify
    }

    #[inline(always)]
    pub fn is_horizontal(&self) -> bool {
        self.main_dir().is_horizontal()
    }

    #[inline(always)]
    pub fn is_vertical(&self) -> bool {
        self.main_dir().is_vertical()
    }

    pub fn prefer_right_to_left(&self) -> bool {
        self.main_dir == Direction::RightToLeft
            || self.main_dir.is_vertical() && self.cross_align == Align::Max
    }

    fn horizontal_align(&self) -> Align {
        if self.is_horizontal() {
            self.main_align
        } else {
            self.cross_align
        }
    }

    fn vertical_align(&self) -> Align {
        if self.is_vertical() {
            self.main_align
        } else {
            self.cross_align
        }
    }

    fn align2(&self) -> Align2 {
        Align2([self.horizontal_align(), self.vertical_align()])
    }

    fn horizontal_justify(&self) -> bool {
        if self.is_horizontal() {
            self.main_justify
        } else {
            self.cross_justify
        }
    }

    fn vertical_justify(&self) -> bool {
        if self.is_vertical() {
            self.main_justify
        } else {
            self.cross_justify
        }
    }
}

/// ## Doing layout
impl Layout {
    pub fn align_size_within_rect(&self, size: Vec2, outer: Rect) -> Rect {
        debug_assert!(size.x >= 0.0 && size.y >= 0.0);
        debug_assert!(!outer.is_negative());
        self.align2().align_size_within_rect(size, outer)
    }

    fn initial_cursor(&self, max_rect: Rect) -> Rect {
        let mut cursor = max_rect;

        match self.main_dir {
            Direction::LeftToRight => {
                cursor.max.x = INFINITY;
            }
            Direction::RightToLeft => {
                cursor.min.x = -INFINITY;
            }
            Direction::TopDown => {
                cursor.max.y = INFINITY;
            }
            Direction::BottomUp => {
                cursor.min.y = -INFINITY;
            }
        }

        cursor
    }

    pub(crate) fn region_from_max_rect(&self, max_rect: Rect) -> Region {
        debug_assert!(!max_rect.any_nan());
        let mut region = Region {
            min_rect: Rect::NOTHING, // temporary
            max_rect,
            cursor: self.initial_cursor(max_rect),
        };
        let seed = self.next_widget_position(&region);
        region.min_rect = Rect::from_center_size(seed, Vec2::ZERO);
        region
    }

    pub(crate) fn available_rect_before_wrap(&self, region: &Region) -> Rect {
        self.available_from_cursor_max_rect(region.cursor, region.max_rect)
    }

    pub(crate) fn available_rect_before_wrap_finite(&self, region: &Region) -> Rect {
        self.available_from_cursor_max_rect(region.cursor, region.max_rect_finite())
    }

    /// Amount of space available for a widget.
    /// For wrapping layouts, this is the maximum (after wrap).
    pub(crate) fn available_size(&self, r: &Region) -> Vec2 {
        if self.main_wrap {
            if self.main_dir.is_horizontal() {
                vec2(r.max_rect.width(), r.cursor.height())
            } else {
                vec2(r.cursor.width(), r.max_rect.height())
            }
        } else {
            self.available_from_cursor_max_rect(r.cursor, r.max_rect)
                .size()
        }
    }

    /// Given the cursor in the region, how much space is available
    /// for the next widget?
    fn available_from_cursor_max_rect(&self, cursor: Rect, max_rect: Rect) -> Rect {
        // NOTE: in normal top-down layout the cursor has moved below the current max_rect,
        // but the available shouldn't be negative.

        // ALSO: with wrapping layouts, cursor jumps to new row before expanding max_rect

        let mut avail = max_rect;

        match self.main_dir {
            Direction::LeftToRight => {
                avail.min.x = cursor.min.x;
                avail.max.x = avail.max.x.max(cursor.min.x);
                if self.main_wrap {
                    avail.min.y = cursor.min.y;
                    avail.max.y = cursor.max.y;
                }
                avail.max.x = avail.max.x.max(avail.min.x);
                avail.max.y = avail.max.y.max(avail.min.y);
            }
            Direction::RightToLeft => {
                avail.max.x = cursor.max.x;
                avail.min.x = avail.min.x.min(cursor.max.x);
                if self.main_wrap {
                    avail.min.y = cursor.min.y;
                    avail.max.y = cursor.max.y;
                }
                avail.min.x = avail.min.x.min(avail.max.x);
                avail.max.y = avail.max.y.max(avail.min.y);
            }
            Direction::TopDown => {
                avail.min.y = cursor.min.y;
                avail.max.y = avail.max.y.max(cursor.min.y);
                if self.main_wrap {
                    avail.min.x = cursor.min.x;
                    avail.max.x = cursor.max.x;
                }
                avail.max.x = avail.max.x.max(avail.min.x);
                avail.max.y = avail.max.y.max(avail.min.y);
            }
            Direction::BottomUp => {
                avail.min.y = avail.min.y.min(cursor.max.y);
                if self.main_wrap {
                    avail.min.x = cursor.min.x;
                    avail.max.x = cursor.max.x;
                }
                avail.max.x = avail.max.x.max(avail.min.x);
                avail.min.y = avail.min.y.min(avail.max.y);
            }
        }

        avail
    }

    /// Returns where to put the next widget that is of the given size.
    /// The returned `frame_rect` `Rect` will always be justified along the cross axis.
    /// This is what you then pass to `advance_after_rects`.
    /// Use `justify_and_align` to get the inner `widget_rect`.
    pub(crate) fn next_frame(&self, region: &Region, child_size: Vec2, spacing: Vec2) -> Rect {
        debug_assert!(child_size.x >= 0.0 && child_size.y >= 0.0);

        if self.main_wrap {
            let available_size = self.available_rect_before_wrap(region).size();

            let Region {
                mut cursor,
                mut max_rect,
                min_rect,
            } = *region;

            match self.main_dir {
                Direction::LeftToRight => {
                    if available_size.x < child_size.x && max_rect.left() < cursor.left() {
                        // New row
                        let new_row_height = cursor.height().max(child_size.y);
                        // let new_top = cursor.bottom() + spacing.y;
                        let new_top = min_rect.bottom() + spacing.y; // tighter packing
                        cursor = Rect::from_min_max(
                            pos2(max_rect.left(), new_top),
                            pos2(INFINITY, new_top + new_row_height),
                        );
                        max_rect.max.y = max_rect.max.y.max(cursor.max.y);
                    }
                }
                Direction::RightToLeft => {
                    if available_size.x < child_size.x && cursor.right() < max_rect.right() {
                        // New row
                        let new_row_height = cursor.height().max(child_size.y);
                        // let new_top = cursor.bottom() + spacing.y;
                        let new_top = min_rect.bottom() + spacing.y; // tighter packing
                        cursor = Rect::from_min_max(
                            pos2(-INFINITY, new_top),
                            pos2(max_rect.right(), new_top + new_row_height),
                        );
                        max_rect.max.y = max_rect.max.y.max(cursor.max.y);
                    }
                }
                Direction::TopDown => {
                    if available_size.y < child_size.y && max_rect.top() < cursor.top() {
                        // New column
                        let new_col_width = cursor.width().max(child_size.x);
                        cursor = Rect::from_min_max(
                            pos2(cursor.right() + spacing.x, max_rect.top()),
                            pos2(cursor.right() + spacing.x + new_col_width, INFINITY),
                        );
                        max_rect.max.x = max_rect.max.x.max(cursor.max.x);
                    }
                }
                Direction::BottomUp => {
                    if available_size.y < child_size.y && cursor.bottom() < max_rect.bottom() {
                        // New column
                        let new_col_width = cursor.width().max(child_size.x);
                        cursor = Rect::from_min_max(
                            pos2(cursor.right() + spacing.x, -INFINITY),
                            pos2(
                                cursor.right() + spacing.x + new_col_width,
                                max_rect.bottom(),
                            ),
                        );
                        max_rect.max.x = max_rect.max.x.max(cursor.max.x);
                    }
                }
            }

            // Use the new cursor:
            let region = Region {
                min_rect,
                max_rect,
                cursor,
            };

            self.next_frame_ignore_wrap(&region, child_size)
        } else {
            self.next_frame_ignore_wrap(region, child_size)
        }
    }

    fn next_frame_ignore_wrap(&self, region: &Region, child_size: Vec2) -> Rect {
        debug_assert!(child_size.x >= 0.0 && child_size.y >= 0.0);

        let available_rect = self.available_rect_before_wrap_finite(region);

        let mut frame_size = child_size;

        if (self.is_vertical() && self.horizontal_align() == Align::Center)
            || self.horizontal_justify()
        {
            frame_size.x = frame_size.x.max(available_rect.width()); // fill full width
        }
        if (self.is_horizontal() && self.vertical_align() == Align::Center)
            || self.vertical_justify()
        {
            frame_size.y = frame_size.y.max(available_rect.height()); // fill full height
        }

        let align2 = match self.main_dir {
            Direction::LeftToRight => Align2([Align::LEFT, self.vertical_align()]),
            Direction::RightToLeft => Align2([Align::RIGHT, self.vertical_align()]),
            Direction::TopDown => Align2([self.horizontal_align(), Align::TOP]),
            Direction::BottomUp => Align2([self.horizontal_align(), Align::BOTTOM]),
        };

        let mut frame_rect = align2.align_size_within_rect(frame_size, available_rect);

        if self.is_horizontal() && frame_rect.top() < region.cursor.top() {
            // for horizontal layouts we always want to expand down,
            // or we will overlap the row above.
            // This is a bit hacky. Maybe we should do it for vertical layouts too.
            frame_rect = frame_rect.translate(Vec2::Y * (region.cursor.top() - frame_rect.top()));
        }

        frame_rect
    }

    /// Apply justify (fill width/height) and/or alignment after calling `next_space`.
    pub(crate) fn justify_and_align(&self, frame: Rect, mut child_size: Vec2) -> Rect {
        debug_assert!(child_size.x >= 0.0 && child_size.y >= 0.0);
        debug_assert!(!frame.is_negative());

        if self.horizontal_justify() {
            child_size.x = child_size.x.at_least(frame.width()); // fill full width
        }
        if self.vertical_justify() {
            child_size.y = child_size.y.at_least(frame.height()); // fill full height
        }
        self.align_size_within_rect(child_size, frame)
    }

    pub(crate) fn next_widget_space_ignore_wrap_justify(
        &self,
        region: &Region,
        size: Vec2,
    ) -> Rect {
        let frame = self.next_frame_ignore_wrap(region, size);
        let rect = self.align_size_within_rect(size, frame);
        debug_assert!((rect.size() - size).length() < 1.0);
        rect
    }

    /// Where would the next tiny widget be centered?
    pub(crate) fn next_widget_position(&self, region: &Region) -> Pos2 {
        self.next_widget_space_ignore_wrap_justify(region, Vec2::ZERO)
            .center()
    }

    /// Advance the cursor by this many points, and allocate in region.
    pub(crate) fn advance_cursor(&self, region: &mut Region, amount: f32) {
        match self.main_dir {
            Direction::LeftToRight => {
                region.cursor.min.x += amount;
                region.expand_to_include_x(region.cursor.min.x);
            }
            Direction::RightToLeft => {
                region.cursor.max.x -= amount;
                region.expand_to_include_x(region.cursor.max.x);
            }
            Direction::TopDown => {
                region.cursor.min.y += amount;
                region.expand_to_include_y(region.cursor.min.y);
            }
            Direction::BottomUp => {
                region.cursor.max.y -= amount;
                region.expand_to_include_y(region.cursor.max.y);
            }
        }
    }

    /// Advance cursor after a widget was added to a specific rectangle.
    ///
    /// * `frame_rect`: the frame inside which a widget was e.g. centered
    /// * `widget_rect`: the actual rect used by the widget
    pub(crate) fn advance_after_rects(
        &self,
        cursor: &mut Rect,
        frame_rect: Rect,
        widget_rect: Rect,
        item_spacing: Vec2,
    ) {
        if self.main_wrap {
            if cursor.intersects(frame_rect.shrink(1.0)) {
                // make row/column larger if necessary
                *cursor = cursor.union(frame_rect);
            } else {
                // this is a new row or column. We temporarily use NAN for what will be filled in later.
                match self.main_dir {
                    Direction::LeftToRight => {
                        *cursor = Rect::from_min_max(
                            pos2(f32::NAN, frame_rect.min.y),
                            pos2(INFINITY, frame_rect.max.y),
                        );
                    }
                    Direction::RightToLeft => {
                        *cursor = Rect::from_min_max(
                            pos2(-INFINITY, frame_rect.min.y),
                            pos2(f32::NAN, frame_rect.max.y),
                        );
                    }
                    Direction::TopDown => {
                        *cursor = Rect::from_min_max(
                            pos2(frame_rect.min.x, f32::NAN),
                            pos2(frame_rect.max.x, INFINITY),
                        );
                    }
                    Direction::BottomUp => {
                        *cursor = Rect::from_min_max(
                            pos2(frame_rect.min.x, -INFINITY),
                            pos2(frame_rect.max.x, f32::NAN),
                        );
                    }
                };
            }
        }

        match self.main_dir {
            Direction::LeftToRight => {
                cursor.min.x = widget_rect.max.x + item_spacing.x;
            }
            Direction::RightToLeft => {
                cursor.max.x = widget_rect.min.x - item_spacing.x;
            }
            Direction::TopDown => {
                cursor.min.y = widget_rect.max.y + item_spacing.y;
            }
            Direction::BottomUp => {
                cursor.max.y = widget_rect.min.y - item_spacing.y;
            }
        };
    }

    /// Move to the next row in a wrapping layout.
    /// Otherwise does nothing.
    pub(crate) fn end_row(&mut self, region: &mut Region, spacing: Vec2) {
        if self.main_wrap {
            match self.main_dir {
                Direction::LeftToRight => {
                    let new_top = region.cursor.bottom() + spacing.y;
                    region.cursor = Rect::from_min_max(
                        pos2(region.max_rect.left(), new_top),
                        pos2(INFINITY, new_top + region.cursor.height()),
                    );
                }
                Direction::RightToLeft => {
                    let new_top = region.cursor.bottom() + spacing.y;
                    region.cursor = Rect::from_min_max(
                        pos2(-INFINITY, new_top),
                        pos2(region.max_rect.right(), new_top + region.cursor.height()),
                    );
                }
                Direction::TopDown | Direction::BottomUp => {}
            }
        }
    }

    /// Set row height in horizontal wrapping layout.
    pub(crate) fn set_row_height(&mut self, region: &mut Region, height: f32) {
        if self.main_wrap && self.is_horizontal() {
            region.cursor.max.y = region.cursor.min.y + height;
        }
    }
}

// ----------------------------------------------------------------------------

/// ## Debug stuff
impl Layout {
    /// Shows where the next widget is going to be placed
    pub(crate) fn debug_paint_cursor(
        &self,
        region: &Region,
        stroke: epaint::Stroke,
        painter: &crate::Painter,
    ) {
        use epaint::*;

        let cursor = region.cursor;
        let next_pos = self.next_widget_position(region);

        let align;

        let l = 64.0;

        match self.main_dir {
            Direction::LeftToRight => {
                painter.line_segment([cursor.left_top(), cursor.left_bottom()], stroke);
                painter.arrow(next_pos, vec2(l, 0.0), stroke);
                align = Align2([Align::LEFT, self.vertical_align()]);
            }
            Direction::RightToLeft => {
                painter.line_segment([cursor.right_top(), cursor.right_bottom()], stroke);
                painter.arrow(next_pos, vec2(-l, 0.0), stroke);
                align = Align2([Align::RIGHT, self.vertical_align()]);
            }
            Direction::TopDown => {
                painter.line_segment([cursor.left_top(), cursor.right_top()], stroke);
                painter.arrow(next_pos, vec2(0.0, l), stroke);
                align = Align2([self.horizontal_align(), Align::TOP]);
            }
            Direction::BottomUp => {
                painter.line_segment([cursor.left_bottom(), cursor.right_bottom()], stroke);
                painter.arrow(next_pos, vec2(0.0, -l), stroke);
                align = Align2([self.horizontal_align(), Align::BOTTOM]);
            }
        }

        painter.text(
            next_pos,
            align,
            "cursor",
            TextStyle::Monospace,
            Color32::WHITE,
        );
    }
}
