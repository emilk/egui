use crate::{math::*, Align};

// ----------------------------------------------------------------------------

/// This describes the bounds and existing contents of an `Ui`.
/// It is what is used and updated by `Layout` when adding new widgets.
#[derive(Clone, Copy, Debug)]
pub struct Region {
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
}

// ----------------------------------------------------------------------------

/// Main layout direction
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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

/// The layout of a `Ui`, e.g. horizontal left-aligned.
#[derive(Clone, Copy, Debug, PartialEq)]
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Layout {
    /// Main axis direction
    main_dir: Direction,

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
            cross_align: Align::left(),
            cross_justify: false,
        }
    }
}

impl Layout {
    /// None align means justified, e.g. fill full width/height.
    pub(crate) fn from_parts(main_dir: Direction, cross_align: Align, cross_justify: bool) -> Self {
        Self {
            main_dir,
            cross_align,
            cross_justify,
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

    pub fn left_to_right() -> Self {
        Self {
            main_dir: Direction::LeftToRight,
            cross_align: Align::Center,
            cross_justify: false,
        }
    }

    pub fn right_to_left() -> Self {
        Self {
            main_dir: Direction::RightToLeft,
            cross_align: Align::Center,
            cross_justify: false,
        }
    }

    pub fn top_down(cross_align: Align) -> Self {
        Self {
            main_dir: Direction::TopDown,
            cross_align,
            cross_justify: false,
        }
    }

    pub fn bottom_up(cross_align: Align) -> Self {
        Self {
            main_dir: Direction::BottomUp,
            cross_align,
            cross_justify: false,
        }
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

    // ------------------------------------------------------------------------

    pub fn main_dir(self) -> Direction {
        self.main_dir
    }

    pub fn cross_align(self) -> Align {
        self.cross_align
    }

    pub fn cross_justify(self) -> bool {
        self.cross_justify
    }

    pub fn is_horizontal(self) -> bool {
        self.main_dir().is_horizontal()
    }

    pub fn is_vertical(self) -> bool {
        self.main_dir().is_vertical()
    }

    pub fn prefer_right_to_left(self) -> bool {
        self.main_dir == Direction::RightToLeft
            || self.main_dir.is_vertical() && self.cross_align == Align::Max
    }

    // ------------------------------------------------------------------------

    fn initial_cursor(self, max_rect: Rect) -> Pos2 {
        match self.main_dir {
            Direction::LeftToRight => max_rect.left_top(),
            Direction::RightToLeft => max_rect.right_top(),
            Direction::TopDown => max_rect.left_top(),
            Direction::BottomUp => max_rect.left_bottom(),
        }
    }

    pub fn region_from_max_rect(&self, max_rect: Rect) -> Region {
        let cursor = self.initial_cursor(max_rect);
        let min_rect = Rect::from_min_size(cursor, Vec2::zero());
        Region {
            min_rect,
            max_rect,
            cursor,
        }
    }

    pub fn available(&self, region: &Region) -> Rect {
        self.available_from_cursor_max_rect(region.cursor, region.max_rect)
    }

    pub fn available_finite(&self, region: &Region) -> Rect {
        self.available_from_cursor_max_rect(region.cursor, region.max_rect_finite())
    }

    /// Given the cursor in the region, how much space is available
    /// for the next widget?
    fn available_from_cursor_max_rect(self, cursor: Pos2, max_rect: Rect) -> Rect {
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

    /// Advance the cursor by this many points.
    pub fn advance_cursor(self, region: &mut Region, amount: f32) {
        match self.main_dir {
            Direction::LeftToRight => region.cursor.x += amount,
            Direction::RightToLeft => region.cursor.x -= amount,
            Direction::TopDown => region.cursor.y += amount,
            Direction::BottomUp => region.cursor.y -= amount,
        }
    }

    /// Advance the cursor by this spacing
    pub fn advance_cursor2(self, region: &mut Region, amount: Vec2) {
        if self.main_dir.is_horizontal() {
            self.advance_cursor(region, amount.x)
        } else {
            self.advance_cursor(region, amount.y)
        }
    }

    /// Reserve this much space and move the cursor.
    /// Returns where to put the widget.
    ///
    /// ## How sizes are negotiated
    /// Each widget should have a *minimum desired size* and a *desired size*.
    /// When asking for space, ask AT LEAST for you minimum, and don't ask for more than you need.
    /// If you want to fill the space, ask about `available().size()` and use that.
    ///
    /// You may get MORE space than you asked for, for instance
    /// for justified layouts, like in menus.
    ///
    /// You may get LESS space than you asked for if the current layout won't fit what you asked for.
    pub fn next_space(self, region: &Region, minimum_child_size: Vec2) -> Rect {
        let available_size = self.available_finite(region).size();
        let available_size = available_size.at_least(minimum_child_size);

        let mut child_size = minimum_child_size;
        let mut child_move = Vec2::default();

        if self.main_dir.is_horizontal() {
            if self.cross_justify {
                // fill full height
                child_size.y = child_size.y.max(available_size.y);
            } else {
                child_move.y += match self.cross_align {
                    Align::Min => 0.0,
                    Align::Center => 0.5 * (available_size.y - child_size.y),
                    Align::Max => available_size.y - child_size.y,
                };
            }
        } else {
            if self.cross_justify {
                // justified: fill full width
                child_size.x = child_size.x.max(available_size.x);
            } else {
                child_move.x += match self.cross_align {
                    Align::Min => 0.0,
                    Align::Center => 0.5 * (available_size.x - child_size.x),
                    Align::Max => available_size.x - child_size.x,
                };
            }
        }

        let child_pos = match self.main_dir {
            Direction::LeftToRight => region.cursor + child_move,
            Direction::RightToLeft => region.cursor + child_move + vec2(-child_size.x, 0.0),
            Direction::TopDown => region.cursor + child_move,
            Direction::BottomUp => region.cursor + child_move + vec2(0.0, -child_size.y),
        };

        Rect::from_min_size(child_pos, child_size)
    }
}

// ----------------------------------------------------------------------------

/// ## Debug stuff
impl Layout {
    /// Shows where the next widget is going to be placed
    pub fn debug_paint_cursor(&self, region: &Region, painter: &crate::Painter) {
        use crate::paint::*;

        let cursor = region.cursor;

        let color = color::GREEN;
        let stroke = Stroke::new(2.0, color);

        let align;

        match self.main_dir {
            Direction::LeftToRight => {
                painter.debug_arrow(cursor, vec2(1.0, 0.0), stroke);
                align = (Align::Min, Align::Min);
            }
            Direction::RightToLeft => {
                painter.debug_arrow(cursor, vec2(-1.0, 0.0), stroke);
                align = (Align::Max, Align::Min);
            }
            Direction::TopDown => {
                painter.debug_arrow(cursor, vec2(0.0, 1.0), stroke);
                align = (Align::Min, Align::Min);
            }
            Direction::BottomUp => {
                painter.debug_arrow(cursor, vec2(0.0, -1.0), stroke);
                align = (Align::Min, Align::Max);
            }
        }

        painter.text(cursor, align, "cursor", TextStyle::Monospace, color);
    }
}
