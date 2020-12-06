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

/// `Layout` direction (horizontal or vertical).
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Direction {
    Horizontal,
    Vertical,
}

impl Default for Direction {
    fn default() -> Direction {
        Direction::Vertical
    }
}

// ----------------------------------------------------------------------------

/// The layout of a `Ui`, e.g. horizontal left-aligned.
#[derive(Clone, Copy, Debug, PartialEq)]
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Layout {
    /// Lay out things horizontally or vertically? Main axis.
    dir: Direction,

    /// How to align things on the cross axis.
    /// For vertical layouts: put things to left, center or right?
    /// For horizontal layouts: put things to top, center or bottom?
    /// `None` means justified, which means full width (vertical layout) or height (horizontal layouts).
    align: Option<Align>,

    /// Lay out things in reversed order, i.e. from the right or bottom-up.
    reversed: bool,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            dir: Direction::Vertical,
            align: Some(Align::Min),
            reversed: false,
        }
    }
}

impl Layout {
    /// None align means justified, e.g. fill full width/height.
    pub fn from_dir_align(dir: Direction, align: Option<Align>) -> Self {
        Self {
            dir,
            align,
            reversed: false,
        }
    }

    pub fn vertical(align: Align) -> Self {
        Self {
            dir: Direction::Vertical,
            align: Some(align),
            reversed: false,
        }
    }

    pub fn horizontal(align: Align) -> Self {
        Self {
            dir: Direction::Horizontal,
            align: Some(align),
            reversed: false,
        }
    }

    /// Full-width layout.
    /// Nice for menus etc where each button is full width.
    pub fn justified(dir: Direction) -> Self {
        Self {
            dir,
            align: None,
            reversed: false,
        }
    }

    #[must_use]
    pub fn reverse(self) -> Self {
        Self {
            dir: self.dir,
            align: self.align,
            reversed: !self.reversed,
        }
    }

    #[must_use]
    pub fn with_reversed(self, reversed: bool) -> Self {
        if reversed {
            self.reverse()
        } else {
            self
        }
    }

    pub fn dir(self) -> Direction {
        self.dir
    }

    pub fn align(self) -> Option<Align> {
        self.align
    }

    pub fn is_reversed(self) -> bool {
        self.reversed
    }

    fn initial_cursor(self, max_rect: Rect) -> Pos2 {
        match self.dir {
            Direction::Horizontal => {
                if self.reversed {
                    max_rect.right_top()
                } else {
                    max_rect.left_top()
                }
            }
            Direction::Vertical => {
                if self.reversed {
                    max_rect.left_bottom()
                } else {
                    max_rect.left_top()
                }
            }
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
        match self.dir {
            Direction::Horizontal => {
                rect.min.y = cursor.y;
                if self.reversed {
                    rect.max.x = cursor.x;
                } else {
                    rect.min.x = cursor.x;
                }
            }
            Direction::Vertical => {
                rect.min.x = cursor.x;
                if self.reversed {
                    rect.max.y = cursor.y;
                } else {
                    rect.min.y = cursor.y;
                }
            }
        }
        rect
    }

    /// Advance the cursor by this many points.
    pub fn advance_cursor(self, region: &mut Region, amount: f32) {
        match self.dir() {
            Direction::Horizontal => {
                if self.is_reversed() {
                    region.cursor.x -= amount;
                } else {
                    region.cursor.x += amount;
                }
            }
            Direction::Vertical => {
                if self.is_reversed() {
                    region.cursor.y -= amount;
                } else {
                    region.cursor.y += amount;
                }
            }
        }
    }

    /// Advance the cursor by this spacing
    pub fn advance_cursor2(self, region: &mut Region, amount: Vec2) {
        match self.dir() {
            Direction::Horizontal => self.advance_cursor(region, amount.x),
            Direction::Vertical => self.advance_cursor(region, amount.y),
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
        let mut cursor_change = Vec2::default();

        match self.dir {
            Direction::Horizontal => {
                if let Some(align) = self.align {
                    child_move.y += match align {
                        Align::Min => 0.0,
                        Align::Center => 0.5 * (available_size.y - child_size.y),
                        Align::Max => available_size.y - child_size.y,
                    };
                } else {
                    // justified: fill full height
                    child_size.y = child_size.y.max(available_size.y);
                }

                cursor_change.x += child_size.x;
            }
            Direction::Vertical => {
                if let Some(align) = self.align {
                    child_move.x += match align {
                        Align::Min => 0.0,
                        Align::Center => 0.5 * (available_size.x - child_size.x),
                        Align::Max => available_size.x - child_size.x,
                    };
                } else {
                    // justified: fill full width
                    child_size.x = child_size.x.max(available_size.x);
                };
                cursor_change.y += child_size.y;
            }
        }

        if self.is_reversed() {
            let child_pos = region.cursor + child_move;
            let child_pos = match self.dir {
                Direction::Horizontal => child_pos + vec2(-child_size.x, 0.0),
                Direction::Vertical => child_pos + vec2(0.0, -child_size.y),
            };
            Rect::from_min_size(child_pos, child_size)
        } else {
            let child_pos = region.cursor + child_move;
            Rect::from_min_size(child_pos, child_size)
        }
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

        match self.dir {
            Direction::Horizontal => {
                if self.reversed {
                    painter.debug_arrow(cursor, vec2(-1.0, 0.0), stroke);
                    align = (Align::Max, Align::Min);
                } else {
                    painter.debug_arrow(cursor, vec2(1.0, 0.0), stroke);
                    align = (Align::Min, Align::Min);
                }
            }
            Direction::Vertical => {
                if self.reversed {
                    painter.debug_arrow(cursor, vec2(0.0, -1.0), stroke);
                    align = (Align::Min, Align::Max);
                } else {
                    painter.debug_arrow(cursor, vec2(0.0, 1.0), stroke);
                    align = (Align::Min, Align::Min);
                }
            }
        }

        painter.text(cursor, align, "cursor", TextStyle::Monospace, color);
    }
}
