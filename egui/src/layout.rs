use crate::{math::*, Align};

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
    /// If something has already been added, this will point ot `style.spacing.item_spacing` beyond the latest child.
    /// The cursor can thus be `style.spacing.item_spacing` pixels outside of the min_rect.
    pub(crate) cursor: Pos2,
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

    /// Ensure we are big enough to contain the given x-coordinate.
    /// This is sometimes useful to expand an ui to stretch to a certain place.
    pub fn expand_to_include_x(&mut self, x: f32) {
        self.min_rect.min.x = self.min_rect.min.x.min(x);
        self.min_rect.max.x = self.min_rect.max.x.max(x);

        self.max_rect.min.x = self.max_rect.min.x.min(x);
        self.max_rect.max.x = self.max_rect.max.x.max(x);
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
    pub fn is_horizontal(self) -> bool {
        match self {
            Direction::LeftToRight | Direction::RightToLeft => true,
            Direction::TopDown | Direction::BottomUp => false,
        }
    }

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
    pub fn top_down_justified(cross_align: Align) -> Self {
        Self::top_down(cross_align).with_cross_justify(true)
    }

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

    pub fn with_main_wrap(self, main_wrap: bool) -> Self {
        Self { main_wrap, ..self }
    }

    pub fn with_cross_align(self, cross_align: Align) -> Self {
        Self {
            cross_align,
            ..self
        }
    }

    pub fn with_cross_justify(self, cross_justify: bool) -> Self {
        Self {
            cross_justify,
            ..self
        }
    }
}

/// ## Inspectors
impl Layout {
    pub fn main_dir(&self) -> Direction {
        self.main_dir
    }

    pub fn main_wrap(&self) -> bool {
        self.main_wrap
    }

    pub fn cross_align(&self) -> Align {
        self.cross_align
    }

    pub fn cross_justify(&self) -> bool {
        self.cross_justify
    }

    pub fn is_horizontal(&self) -> bool {
        self.main_dir().is_horizontal()
    }

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
        self.align2().align_size_within_rect(size, outer)
    }

    fn initial_cursor(&self, max_rect: Rect) -> Pos2 {
        match self.main_dir {
            Direction::LeftToRight => max_rect.left_top(),
            Direction::RightToLeft => max_rect.right_top(),
            Direction::TopDown => max_rect.left_top(),
            Direction::BottomUp => max_rect.left_bottom(),
        }
    }

    pub(crate) fn region_from_max_rect(&self, max_rect: Rect) -> Region {
        let cursor = self.initial_cursor(max_rect);
        let min_rect = Rect::from_min_size(cursor, Vec2::ZERO);
        Region {
            min_rect,
            max_rect,
            cursor,
        }
    }

    pub(crate) fn available_rect_before_wrap(&self, region: &Region) -> Rect {
        self.available_from_cursor_max_rect(region.cursor, region.max_rect)
    }

    fn available_size_before_wrap(&self, region: &Region) -> Vec2 {
        self.available_rect_before_wrap(region).size()
    }

    pub(crate) fn available_rect_before_wrap_finite(&self, region: &Region) -> Rect {
        self.available_from_cursor_max_rect(region.cursor, region.max_rect_finite())
    }

    fn available_size_before_wrap_finite(&self, region: &Region) -> Vec2 {
        self.available_rect_before_wrap_finite(region).size()
    }

    /// Amount of space available for a widget.
    /// For wrapping layouts, this is the maximum (after wrap).
    pub(crate) fn available_size(&self, r: &Region) -> Vec2 {
        if self.main_wrap {
            if self.main_dir.is_horizontal() {
                vec2(r.max_rect.width(), r.max_rect.bottom() - r.cursor.y)
            } else {
                vec2(r.max_rect.right() - r.cursor.x, r.max_rect.height())
            }
        } else {
            self.available_from_cursor_max_rect(r.cursor, r.max_rect)
                .size()
        }
    }

    /// Given the cursor in the region, how much space is available
    /// for the next widget?
    fn available_from_cursor_max_rect(&self, cursor: Pos2, max_rect: Rect) -> Rect {
        let mut rect = max_rect;

        match self.main_dir {
            Direction::LeftToRight => {
                rect.min.x = cursor.x;
                rect.min.y = cursor.y;
            }
            Direction::RightToLeft => {
                rect.max.x = cursor.x;
                rect.min.y = cursor.y;
            }
            Direction::TopDown => {
                rect.min.x = cursor.x;
                rect.min.y = cursor.y;
            }
            Direction::BottomUp => {
                rect.min.x = cursor.x;
                rect.max.y = cursor.y;
            }
        }

        rect
    }

    /// Returns where to put the next widget that is of the given size.
    /// The returned `frame_rect` `Rect` will always be justified along the cross axis.
    /// This is what you then pass to `advance_after_rects`.
    /// Use `justify_and_align` to get the inner `widget_rect`.
    #[allow(clippy::collapsible_if)]
    pub(crate) fn next_space(
        &self,
        region: &Region,
        mut child_size: Vec2,
        item_spacing: Vec2,
    ) -> Rect {
        let mut cursor = region.cursor;

        if self.main_wrap {
            let available_size = self.available_size_before_wrap(region);
            match self.main_dir {
                Direction::LeftToRight => {
                    if available_size.x < child_size.x && region.max_rect.left() < cursor.x {
                        // New row
                        cursor = pos2(
                            region.max_rect.left(),
                            region.max_rect.bottom() + item_spacing.y,
                        );
                    }
                }
                Direction::RightToLeft => {
                    if available_size.x < child_size.x && cursor.x < region.max_rect.right() {
                        // New row
                        cursor = pos2(
                            region.max_rect.right(),
                            region.max_rect.bottom() + item_spacing.y,
                        );
                    }
                }
                Direction::TopDown => {
                    if available_size.y < child_size.y && region.max_rect.top() < cursor.y {
                        // New column
                        cursor = pos2(
                            region.max_rect.right() + item_spacing.x,
                            region.max_rect.top(),
                        );
                    }
                }
                Direction::BottomUp => {
                    if available_size.y < child_size.y && cursor.y < region.max_rect.bottom() {
                        // New column
                        cursor = pos2(
                            region.max_rect.right() + item_spacing.x,
                            region.max_rect.bottom(),
                        );
                    }
                }
            }
        }

        let available_size = self.available_size_before_wrap_finite(region);

        if self.is_vertical() || self.horizontal_justify() {
            child_size.x = child_size.x.at_least(available_size.x); // fill full width
        }
        if self.is_horizontal() || self.vertical_justify() {
            child_size.y = child_size.y.at_least(available_size.y); // fill full height
        }

        let child_pos = match self.main_dir {
            Direction::LeftToRight => cursor,
            Direction::RightToLeft => cursor + vec2(-child_size.x, 0.0),
            Direction::TopDown => cursor,
            Direction::BottomUp => cursor + vec2(0.0, -child_size.y),
        };

        Rect::from_min_size(child_pos, child_size)
    }

    /// Apply justify (fill width/height) and/or alignment after calling `next_space`.
    pub(crate) fn justify_and_align(&self, rect: Rect, mut child_size: Vec2) -> Rect {
        if self.horizontal_justify() {
            child_size.x = child_size.x.at_least(rect.width()); // fill full width
        }
        if self.vertical_justify() {
            child_size.y = child_size.y.at_least(rect.height()); // fill full height
        }
        self.align_size_within_rect(child_size, rect)
    }

    /// Advance the cursor by this many points.
    pub(crate) fn advance_cursor(&self, cursor: &mut Pos2, amount: f32) {
        match self.main_dir {
            Direction::LeftToRight => cursor.x += amount,
            Direction::RightToLeft => cursor.x -= amount,
            Direction::TopDown => cursor.y += amount,
            Direction::BottomUp => cursor.y -= amount,
        }
    }

    /// Advance cursor after a widget was added to a specific rectangle.
    ///
    /// * `frame_rect`: the frame inside which a widget was e.g. centered
    /// * `widget_rect`: the actual rect used by the widget
    pub(crate) fn advance_after_rects(
        &self,
        cursor: &mut Pos2,
        frame_rect: Rect,
        widget_rect: Rect,
        item_spacing: Vec2,
    ) {
        *cursor = match self.main_dir {
            Direction::LeftToRight => pos2(widget_rect.right() + item_spacing.x, frame_rect.top()),
            Direction::RightToLeft => pos2(widget_rect.left() - item_spacing.x, frame_rect.top()),
            Direction::TopDown => pos2(frame_rect.left(), widget_rect.bottom() + item_spacing.y),
            Direction::BottomUp => pos2(frame_rect.left(), widget_rect.top() - item_spacing.y),
        };
    }

    /// Move to the next row in a wrapping layout.
    /// Otherwise does nothing.
    pub(crate) fn end_row(&mut self, region: &mut Region, item_spacing: Vec2) {
        if self.main_wrap && self.is_horizontal() {
            // New row
            region.cursor = pos2(
                region.max_rect.left(),
                region.max_rect.bottom() + item_spacing.y,
            );
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
        use crate::paint::*;

        let cursor = region.cursor;

        let align;

        let l = 64.0;

        match self.main_dir {
            Direction::LeftToRight => {
                painter.arrow(cursor, vec2(l, 0.0), stroke);
                align = Align2::LEFT_TOP;
            }
            Direction::RightToLeft => {
                painter.arrow(cursor, vec2(-l, 0.0), stroke);
                align = Align2::RIGHT_TOP;
            }
            Direction::TopDown => {
                painter.arrow(cursor, vec2(0.0, l), stroke);
                align = Align2::LEFT_TOP;
            }
            Direction::BottomUp => {
                painter.arrow(cursor, vec2(0.0, -l), stroke);
                align = Align2::LEFT_BOTTOM;
            }
        }

        painter.text(cursor, align, "cursor", TextStyle::Monospace, stroke.color);
    }
}
