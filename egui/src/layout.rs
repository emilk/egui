use crate::{math::*, style::Style};

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

/// left/center/right or top/center/bottom alignment for e.g. anchors and `Layout`s.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Align {
    /// Left/Top
    Min,

    /// Note: requires a bounded/known available_width.
    Center,

    /// Right/Bottom
    /// Note: requires a bounded/known available_width.
    Max,
}

impl Default for Align {
    fn default() -> Align {
        Align::Min
    }
}

/// Used e.g. to anchor a piece of text to a part of the rectangle.
/// Give a position within the rect, specified by the aligns
pub(crate) fn anchor_rect(rect: Rect, anchor: (Align, Align)) -> Rect {
    let x = match anchor.0 {
        Align::Min => rect.left(),
        Align::Center => rect.left() - 0.5 * rect.width(),
        Align::Max => rect.left() - rect.width(),
    };
    let y = match anchor.1 {
        Align::Min => rect.top(),
        Align::Center => rect.top() - 0.5 * rect.height(),
        Align::Max => rect.top() - rect.height(),
    };
    Rect::from_min_size(pos2(x, y), rect.size())
}

// ----------------------------------------------------------------------------

/// The layout of a `Ui`, e.g. horizontal left-aligned.
#[derive(Clone, Copy, Debug, PartialEq)]
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Layout {
    /// Lay out things horizontally or vertically?
    dir: Direction,

    /// For vertical layouts: put things to left, center or right?
    /// For horizontal layouts: put things to top, center or bottom?
    /// None means justified, which means full width (vertical layout) or height (horizontal layouts).
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
    /// Nice for menues etc where each button is full width.
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

    pub fn dir(self) -> Direction {
        self.dir
    }

    pub fn is_reversed(self) -> bool {
        self.reversed
    }

    /// Given the cursor in the region, how much space is available
    /// for the next widget?
    pub fn available(self, cursor: Pos2, rect: Rect) -> Rect {
        if self.reversed {
            Rect::from_min_max(rect.min, cursor)
        } else {
            Rect::from_min_max(cursor, rect.max)
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
    /// for `Justified` aligned layouts, like in menus.
    ///
    /// You may get LESS space than you asked for if the current layout won't fit what you asked for.
    pub fn allocate_space(
        self,
        cursor: &mut Pos2,
        style: &Style,
        available_size: Vec2,
        mut child_size: Vec2,
    ) -> Rect {
        let available_size = available_size.max(child_size);

        let mut child_move = Vec2::default();
        let mut cursor_change = Vec2::default();

        if self.dir == Direction::Horizontal {
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
            cursor_change.x += style.item_spacing.x; // Where to put next thing, if there is a next thing
        } else {
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
            cursor_change.y += style.item_spacing.y; // Where to put next thing, if there is a next thing
        }

        if self.is_reversed() {
            // reverse: cursor starts at bottom right corner of new widget.

            let child_pos = if self.dir == Direction::Horizontal {
                pos2(
                    cursor.x - child_size.x,
                    cursor.y - available_size.y + child_move.y,
                )
            } else {
                pos2(
                    cursor.x - available_size.x + child_move.x,
                    cursor.y - child_size.y,
                )
            };
            // let child_pos = *cursor - child_move - child_size;
            *cursor -= cursor_change;
            Rect::from_min_size(child_pos, child_size)
        } else {
            let child_pos = *cursor + child_move;
            *cursor += cursor_change;
            Rect::from_min_size(child_pos, child_size)
        }
    }
}
