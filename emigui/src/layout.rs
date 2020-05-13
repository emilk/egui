use serde_derive::{Deserialize, Serialize};

use crate::math::*;

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

    /// Full width/height.
    /// Use this when you want
    Justified,
}

impl Default for Align {
    fn default() -> Align {
        Align::Min
    }
}

/// Give a position within the rect, specified by the aligns
pub fn align_rect(rect: Rect, align: (Align, Align)) -> Rect {
    let x = match align.0 {
        Align::Min | Align::Justified => rect.left(),
        Align::Center => rect.left() - 0.5 * rect.width(),
        Align::Max => rect.left() - rect.width(),
    };
    let y = match align.1 {
        Align::Min | Align::Justified => rect.top(),
        Align::Center => rect.top() - 0.5 * rect.height(),
        Align::Max => rect.top() - rect.height(),
    };
    Rect::from_min_size(pos2(x, y), rect.size())
}
