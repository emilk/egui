use serde_derive::{Deserialize, Serialize};

use crate::{math::*, style::Style};

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Horizontal,
    Vertical,
}

impl Default for Direction {
    fn default() -> Direction {
        Direction::Vertical
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
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
pub fn align_rect(rect: Rect, align: (Align, Align)) -> Rect {
    let x = match align.0 {
        Align::Min => rect.left(),
        Align::Center => rect.left() - 0.5 * rect.width(),
        Align::Max => rect.left() - rect.width(),
    };
    let y = match align.1 {
        Align::Min => rect.top(),
        Align::Center => rect.top() - 0.5 * rect.height(),
        Align::Max => rect.top() - rect.height(),
    };
    Rect::from_min_size(pos2(x, y), rect.size())
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct Layout {
    /// Lay out things horizontally or vertically?
    dir: Direction,

    /// For vertical layouts: put things to left, center or right?
    /// For horizontal layouts: put things to top, center or bottom?
    /// None means justified, which means full width (vertical layout) or height (horizontal layouts).
    align: Option<Align>,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            dir: Direction::Vertical,
            align: Some(Align::Min),
        }
    }
}

impl Layout {
    /// None align means justified, e.g. fill full width/height.
    pub fn from_dir_align(dir: Direction, align: Option<Align>) -> Self {
        Self { dir, align }
    }

    pub fn vertical(align: Align) -> Self {
        Self {
            dir: Direction::Vertical,
            align: Some(align),
        }
    }

    pub fn horizontal(align: Align) -> Self {
        Self {
            dir: Direction::Horizontal,
            align: Some(align),
        }
    }

    /// Full-width layout.
    /// Nice for menues etc where each button is full width.
    pub fn justified(dir: Direction) -> Self {
        Self { dir, align: None }
    }

    pub fn dir(&self) -> Direction {
        self.dir
    }

    /// Reserve this much space and move the cursor.
    /// Returns where to put the widget.
    ///
    /// # How sizes are negotiated
    /// Each widget should have a *minimum desired size* and a *desired size*.
    /// When asking for space, ask AT LEAST for you minimum, and don't ask for more than you need.
    /// If you want to fill the space, ask about `available().size()` and use that.
    ///
    /// You may get MORE space than you asked for, for instance
    /// for `Justified` aligned layouts, like in menus.
    ///
    /// You may get LESS space than you asked for if the current layout won't fit what you asked for.
    pub fn allocate_space(
        &self,
        cursor: &mut Pos2,
        style: &Style,
        available_size: Vec2,
        mut child_size: Vec2,
    ) -> Rect {
        let available_size = available_size.max(child_size);

        let mut child_pos = *cursor;
        if self.dir == Direction::Horizontal {
            if let Some(align) = self.align {
                if align != Align::Min {
                    debug_assert!(available_size.y.is_finite());
                    debug_assert!(child_size.y.is_finite());
                }

                child_pos.y += match align {
                    Align::Min => 0.0,
                    Align::Center => 0.5 * (available_size.y - child_size.y),
                    Align::Max => available_size.y - child_size.y,
                };
            } else {
                // justified: fill full height
                child_size.y = child_size.y.max(available_size.y);
            }

            cursor.x += child_size.x;
            cursor.x += style.item_spacing.x; // Where to put next thing, if there is a next thing
        } else {
            if let Some(align) = self.align {
                if align != Align::Min {
                    debug_assert!(available_size.y.is_finite());
                    debug_assert!(child_size.y.is_finite());
                }

                child_pos.x += match align {
                    Align::Min => 0.0,
                    Align::Center => 0.5 * (available_size.x - child_size.x),
                    Align::Max => available_size.x - child_size.x,
                };
            } else {
                // justified: fill full width
                child_size.x = child_size.x.max(available_size.x);
            };
            cursor.y += child_size.y;
            cursor.y += style.item_spacing.y; // Where to put next thing, if there is a next thing
        }

        Rect::from_min_size(child_pos, child_size)
    }
}
