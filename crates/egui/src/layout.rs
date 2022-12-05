use crate::{egui_assert, emath::*, Align};
use std::f32::INFINITY;

// ----------------------------------------------------------------------------

/// This describes the bounds and existing contents of an [`Ui`][`crate::Ui`].
/// It is what is used and updated by [`Layout`] when adding new widgets.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Region {
    /// This is the minimal size of the [`Ui`](crate::Ui).
    /// When adding new widgets, this will generally expand.
    ///
    /// Always finite.
    ///
    /// The bounding box of all child widgets, but not necessarily a tight bounding box
    /// since [`Ui`](crate::Ui) can start with a non-zero min_rect size.
    pub min_rect: Rect,

    /// The maximum size of this [`Ui`](crate::Ui). This is a *soft max*
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
    /// If something has already been added, this will point to `style.spacing.item_spacing` beyond the latest child.
    /// The cursor can thus be `style.spacing.item_spacing` pixels outside of the min_rect.
    pub(crate) cursor: Rect,
}

impl Region {
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
        self.cursor.extend_with_x(x);
    }

    /// Ensure we are big enough to contain the given Y-coordinate.
    /// This is sometimes useful to expand an ui to stretch to a certain place.
    pub fn expand_to_include_y(&mut self, y: f32) {
        self.min_rect.extend_with_y(y);
        self.max_rect.extend_with_y(y);
        self.cursor.extend_with_y(y);
    }

    pub fn sanity_check(&self) {
        egui_assert!(!self.min_rect.any_nan());
        egui_assert!(!self.max_rect.any_nan());
        egui_assert!(!self.cursor.any_nan());
    }
}

// ----------------------------------------------------------------------------

/// Layout direction, one of [`LeftToRight`](Direction::LeftToRight), [`RightToLeft`](Direction::RightToLeft), [`TopDown`](Direction::TopDown), [`BottomUp`](Direction::BottomUp).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
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
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
///     ui.label("world!");
///     ui.label("Hello");
/// });
/// # });
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Layout {
    /// Main axis direction
    pub main_dir: Direction,

    /// If true, wrap around when reading the end of the main direction.
    /// For instance, for `main_dir == Direction::LeftToRight` this will
    /// wrap to a new row when we reach the right side of the `max_rect`.
    pub main_wrap: bool,

    /// How to align things on the main axis.
    pub main_align: Align,

    /// Justify the main axis?
    pub main_justify: bool,

    /// How to align things on the cross axis.
    /// For vertical layouts: put things to left, center or right?
    /// For horizontal layouts: put things to top, center or bottom?
    pub cross_align: Align,

    /// Justify the cross axis?
    /// For vertical layouts justify mean all widgets get maximum width.
    /// For horizontal layouts justify mean all widgets get maximum height.
    pub cross_justify: bool,
}

impl Default for Layout {
    fn default() -> Self {
        // TODO(emilk): Get from `Style` instead.
        Self::top_down(Align::LEFT) // This is a very euro-centric default.
    }
}

/// ## Constructors
impl Layout {
    /// Place elements horizontally, left to right.
    ///
    /// The `valign` parameter controls how to align elements vertically.
    #[inline(always)]
    pub fn left_to_right(valign: Align) -> Self {
        Self {
            main_dir: Direction::LeftToRight,
            main_wrap: false,
            main_align: Align::Center, // looks best to e.g. center text within a button
            main_justify: false,
            cross_align: valign,
            cross_justify: false,
        }
    }

    /// Place elements horizontally, right to left.
    ///
    /// The `valign` parameter controls how to align elements vertically.
    #[inline(always)]
    pub fn right_to_left(valign: Align) -> Self {
        Self {
            main_dir: Direction::RightToLeft,
            main_wrap: false,
            main_align: Align::Center, // looks best to e.g. center text within a button
            main_justify: false,
            cross_align: valign,
            cross_justify: false,
        }
    }

    /// Place elements vertically, top to bottom.
    ///
    /// Use the provided horizontal alignment.
    #[inline(always)]
    pub fn top_down(halign: Align) -> Self {
        Self {
            main_dir: Direction::TopDown,
            main_wrap: false,
            main_align: Align::Center, // looks best to e.g. center text within a button
            main_justify: false,
            cross_align: halign,
            cross_justify: false,
        }
    }

    /// Top-down layout justified so that buttons etc fill the full available width.
    #[inline(always)]
    pub fn top_down_justified(halign: Align) -> Self {
        Self::top_down(halign).with_cross_justify(true)
    }

    /// Place elements vertically, bottom up.
    ///
    /// Use the provided horizontal alignment.
    #[inline(always)]
    pub fn bottom_up(halign: Align) -> Self {
        Self {
            main_dir: Direction::BottomUp,
            main_wrap: false,
            main_align: Align::Center, // looks best to e.g. center text within a button
            main_justify: false,
            cross_align: halign,
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

    /// For when you want to add a single widget to a layout, and that widget
    /// should use up all available space.
    ///
    /// Only one widget may be added to the inner `Ui`!
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

    /// Wrap widgets when we overflow the main axis?
    ///
    /// For instance, for left-to-right layouts, setting this to `true` will
    /// put widgets on a new row if we would overflow the right side of [`crate::Ui::max_rect`].
    #[inline(always)]
    pub fn with_main_wrap(self, main_wrap: bool) -> Self {
        Self { main_wrap, ..self }
    }

    /// The alignment to use on the main axis.
    #[inline(always)]
    pub fn with_main_align(self, main_align: Align) -> Self {
        Self { main_align, ..self }
    }

    /// The alignment to use on the cross axis.
    ///
    /// The "cross" axis is the one orthogonal to the main axis.
    /// For instance: in left-to-right layout, the main axis is horizontal and the cross axis is vertical.
    #[inline(always)]
    pub fn with_cross_align(self, cross_align: Align) -> Self {
        Self {
            cross_align,
            ..self
        }
    }

    /// Justify widgets on the main axis?
    ///
    /// Justify here means "take up all available space".
    #[inline(always)]
    pub fn with_main_justify(self, main_justify: bool) -> Self {
        Self {
            main_justify,
            ..self
        }
    }

    /// Justify widgets along the cross axis?
    ///
    /// Justify here means "take up all available space".
    ///
    /// The "cross" axis is the one orthogonal to the main axis.
    /// For instance: in left-to-right layout, the main axis is horizontal and the cross axis is vertical.
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

    /// e.g. for adjusting the placement of something.
    /// * in horizontal layout: left or right?
    /// * in vertical layout: same as [`Self::horizontal_align`].
    pub fn horizontal_placement(&self) -> Align {
        match self.main_dir {
            Direction::LeftToRight => Align::LEFT,
            Direction::RightToLeft => Align::RIGHT,
            Direction::TopDown | Direction::BottomUp => self.cross_align,
        }
    }

    /// e.g. for when aligning text within a button.
    pub fn horizontal_align(&self) -> Align {
        if self.is_horizontal() {
            self.main_align
        } else {
            self.cross_align
        }
    }

    /// e.g. for when aligning text within a button.
    pub fn vertical_align(&self) -> Align {
        if self.is_vertical() {
            self.main_align
        } else {
            self.cross_align
        }
    }

    /// e.g. for when aligning text within a button.
    fn align2(&self) -> Align2 {
        Align2([self.horizontal_align(), self.vertical_align()])
    }

    pub fn horizontal_justify(&self) -> bool {
        if self.is_horizontal() {
            self.main_justify
        } else {
            self.cross_justify
        }
    }

    pub fn vertical_justify(&self) -> bool {
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
        egui_assert!(size.x >= 0.0 && size.y >= 0.0);
        egui_assert!(!outer.is_negative());
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
        egui_assert!(!max_rect.any_nan());
        egui_assert!(max_rect.is_finite());
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
        egui_assert!(!cursor.any_nan());
        egui_assert!(!max_rect.any_nan());
        egui_assert!(max_rect.is_finite());

        // NOTE: in normal top-down layout the cursor has moved below the current max_rect,
        // but the available shouldn't be negative.

        // ALSO: with wrapping layouts, cursor jumps to new row before expanding max_rect.

        let mut avail = max_rect;

        match self.main_dir {
            Direction::LeftToRight => {
                avail.min.x = cursor.min.x;
                avail.max.x = avail.max.x.max(cursor.min.x);
                avail.max.x = avail.max.x.max(avail.min.x);
                avail.max.y = avail.max.y.max(avail.min.y);
            }
            Direction::RightToLeft => {
                avail.max.x = cursor.max.x;
                avail.min.x = avail.min.x.min(cursor.max.x);
                avail.min.x = avail.min.x.min(avail.max.x);
                avail.max.y = avail.max.y.max(avail.min.y);
            }
            Direction::TopDown => {
                avail.min.y = cursor.min.y;
                avail.max.y = avail.max.y.max(cursor.min.y);
                avail.max.x = avail.max.x.max(avail.min.x);
                avail.max.y = avail.max.y.max(avail.min.y);
            }
            Direction::BottomUp => {
                avail.max.y = cursor.max.y;
                avail.min.y = avail.min.y.min(cursor.max.y);
                avail.max.x = avail.max.x.max(avail.min.x);
                avail.min.y = avail.min.y.min(avail.max.y);
            }
        }

        // We can use the cursor to restrict the available region.
        // For instance, we use this to restrict the available space of a parent Ui
        // after adding a panel to it.
        // We also use it for wrapping layouts.
        avail = avail.intersect(cursor);

        // Make sure it isn't negative:
        if avail.max.x < avail.min.x {
            let x = 0.5 * (avail.min.x + avail.max.x);
            avail.min.x = x;
            avail.max.x = x;
        }
        if avail.max.y < avail.min.y {
            let y = 0.5 * (avail.min.y + avail.max.y);
            avail.min.y = y;
            avail.max.y = y;
        }

        egui_assert!(!avail.any_nan());

        avail
    }

    /// Returns where to put the next widget that is of the given size.
    /// The returned `frame_rect` [`Rect`] will always be justified along the cross axis.
    /// This is what you then pass to `advance_after_rects`.
    /// Use `justify_and_align` to get the inner `widget_rect`.
    pub(crate) fn next_frame(&self, region: &Region, child_size: Vec2, spacing: Vec2) -> Rect {
        region.sanity_check();
        egui_assert!(child_size.x >= 0.0 && child_size.y >= 0.0);

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
        region.sanity_check();
        egui_assert!(child_size.x >= 0.0 && child_size.y >= 0.0);

        let available_rect = self.available_rect_before_wrap(region);

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

        egui_assert!(!frame_rect.any_nan());
        egui_assert!(!frame_rect.is_negative());

        frame_rect
    }

    /// Apply justify (fill width/height) and/or alignment after calling `next_space`.
    pub(crate) fn justify_and_align(&self, frame: Rect, mut child_size: Vec2) -> Rect {
        egui_assert!(child_size.x >= 0.0 && child_size.y >= 0.0);
        egui_assert!(!frame.is_negative());

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
        egui_assert!(!rect.any_nan());
        egui_assert!(!rect.is_negative());
        egui_assert!((rect.width() - size.x).abs() < 1.0 || size.x == f32::INFINITY);
        egui_assert!((rect.height() - size.y).abs() < 1.0 || size.y == f32::INFINITY);
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
        egui_assert!(!cursor.any_nan());
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
        } else {
            // Make sure we also expand where we consider adding things (the cursor):
            if self.is_horizontal() {
                cursor.min.y = cursor.min.y.min(frame_rect.min.y);
                cursor.max.y = cursor.max.y.max(frame_rect.max.y);
            } else {
                cursor.min.x = cursor.min.x.min(frame_rect.min.x);
                cursor.max.x = cursor.max.x.max(frame_rect.max.x);
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
    pub(crate) fn paint_text_at_cursor(
        &self,
        painter: &crate::Painter,
        region: &Region,
        stroke: epaint::Stroke,
        text: impl ToString,
    ) {
        let cursor = region.cursor;
        let next_pos = self.next_widget_position(region);

        let l = 64.0;

        let align = match self.main_dir {
            Direction::LeftToRight => {
                painter.line_segment([cursor.left_top(), cursor.left_bottom()], stroke);
                painter.arrow(next_pos, vec2(l, 0.0), stroke);
                Align2([Align::LEFT, self.vertical_align()])
            }
            Direction::RightToLeft => {
                painter.line_segment([cursor.right_top(), cursor.right_bottom()], stroke);
                painter.arrow(next_pos, vec2(-l, 0.0), stroke);
                Align2([Align::RIGHT, self.vertical_align()])
            }
            Direction::TopDown => {
                painter.line_segment([cursor.left_top(), cursor.right_top()], stroke);
                painter.arrow(next_pos, vec2(0.0, l), stroke);
                Align2([self.horizontal_align(), Align::TOP])
            }
            Direction::BottomUp => {
                painter.line_segment([cursor.left_bottom(), cursor.right_bottom()], stroke);
                painter.arrow(next_pos, vec2(0.0, -l), stroke);
                Align2([self.horizontal_align(), Align::BOTTOM])
            }
        };

        painter.debug_text(next_pos, align, stroke.color, text);
    }
}
