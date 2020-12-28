use crate::math::{pos2, Rect};

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

impl Align {
    pub fn left() -> Self {
        Self::Min
    }
    pub fn right() -> Self {
        Self::Max
    }
    pub fn top() -> Self {
        Self::Min
    }
    pub fn bottom() -> Self {
        Self::Max
    }

    pub(crate) fn scroll_center_factor(&self) -> f32 {
        match self {
            Self::Min => 0.0,
            Self::Center => 0.5,
            Self::Max => 1.0,
        }
    }
}

impl Default for Align {
    fn default() -> Align {
        Align::Min
    }
}

pub type Align2 = (Align, Align);

pub const LEFT_BOTTOM: Align2 = (Align::Min, Align::Max);
pub const LEFT_CENTER: Align2 = (Align::Min, Align::Center);
pub const LEFT_TOP: Align2 = (Align::Min, Align::Min);
pub const CENTER_BOTTOM: Align2 = (Align::Center, Align::Max);
pub const CENTER_CENTER: Align2 = (Align::Center, Align::Center);
pub const CENTER_TOP: Align2 = (Align::Center, Align::Min);
pub const RIGHT_BOTTOM: Align2 = (Align::Max, Align::Max);
pub const RIGHT_CENTER: Align2 = (Align::Max, Align::Center);
pub const RIGHT_TOP: Align2 = (Align::Max, Align::Min);

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
