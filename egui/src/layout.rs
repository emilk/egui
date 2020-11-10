use crate::{math::*, Align};

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

    pub fn initial_cursor(self, max_rect: Rect) -> Pos2 {
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

    /// Given the cursor in the region, how much space is available
    /// for the next widget?
    pub fn available(self, cursor: Pos2, max_rect: Rect) -> Rect {
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
    pub fn advance_cursor(self, cursor: &mut Pos2, amount: f32) {
        match self.dir() {
            Direction::Horizontal => {
                if self.is_reversed() {
                    cursor.x -= amount;
                } else {
                    cursor.x += amount;
                }
            }
            Direction::Vertical => {
                if self.is_reversed() {
                    cursor.y -= amount;
                } else {
                    cursor.y += amount;
                }
            }
        }
    }

    /// Advance the cursor by this spacing
    pub fn advance_cursor2(self, cursor: &mut Pos2, amount: Vec2) {
        match self.dir() {
            Direction::Horizontal => self.advance_cursor(cursor, amount.x),
            Direction::Vertical => self.advance_cursor(cursor, amount.y),
        }
    }

    pub fn rect_from_cursor_size(self, cursor: Pos2, size: Vec2) -> Rect {
        let mut rect = Rect::from_min_size(cursor, size);

        match self.dir {
            Direction::Horizontal => {
                if self.reversed {
                    rect.min.x = cursor.x - size.x;
                    rect.max.x = rect.min.x - size.x
                }
            }
            Direction::Vertical => {
                if self.reversed {
                    rect.min.y = cursor.y - size.y;
                    rect.max.y = rect.min.y - size.y
                }
            }
        }

        rect
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
    /// for `Justified` aligned layouts, like in menus.
    ///
    /// You may get LESS space than you asked for if the current layout won't fit what you asked for.
    pub fn allocate_space(
        self,
        cursor: &mut Pos2,
        available_size: Vec2,
        minimum_child_size: Vec2,
    ) -> Rect {
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
            let child_pos = *cursor + child_move;
            let child_pos = match self.dir {
                Direction::Horizontal => child_pos + vec2(-child_size.x, 0.0),
                Direction::Vertical => child_pos + vec2(0.0, -child_size.y),
            };
            *cursor -= cursor_change;
            Rect::from_min_size(child_pos, child_size)
        } else {
            let child_pos = *cursor + child_move;
            *cursor += cursor_change;
            Rect::from_min_size(child_pos, child_size)
        }
    }
}

// ----------------------------------------------------------------------------

/// ## Debug stuff
impl Layout {
    /// Shows where the next widget is going to be placed
    pub fn debug_paint_cursor(&self, cursor: Pos2, painter: &crate::Painter) {
        use crate::paint::*;
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
