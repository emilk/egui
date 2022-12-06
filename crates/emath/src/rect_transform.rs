use crate::{pos2, remap, remap_clamp, Pos2, Rect, Vec2};

/// Linearly transforms positions from one [`Rect`] to another.
///
/// [`RectTransform`] stores the rectangles, and therefore supports clamping and culling.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct RectTransform {
    from: Rect,
    to: Rect,
}

impl RectTransform {
    pub fn identity(from_and_to: Rect) -> Self {
        Self::from_to(from_and_to, from_and_to)
    }

    pub fn from_to(from: Rect, to: Rect) -> Self {
        Self { from, to }
    }

    pub fn from(&self) -> &Rect {
        &self.from
    }

    pub fn to(&self) -> &Rect {
        &self.to
    }

    /// The scale factors.
    pub fn scale(&self) -> Vec2 {
        self.to.size() / self.from.size()
    }

    pub fn inverse(&self) -> RectTransform {
        Self::from_to(self.to, self.from)
    }

    /// Transforms the given coordinate in the `from` space to the `to` space.
    pub fn transform_pos(&self, pos: Pos2) -> Pos2 {
        pos2(
            remap(pos.x, self.from.x_range(), self.to.x_range()),
            remap(pos.y, self.from.y_range(), self.to.y_range()),
        )
    }

    /// Transforms the given rectangle in the `in`-space to a rectangle in the `out`-space.
    pub fn transform_rect(&self, rect: Rect) -> Rect {
        Rect {
            min: self.transform_pos(rect.min),
            max: self.transform_pos(rect.max),
        }
    }

    /// Transforms the given coordinate in the `from` space to the `to` space,
    /// clamping if necessary.
    pub fn transform_pos_clamped(&self, pos: Pos2) -> Pos2 {
        pos2(
            remap_clamp(pos.x, self.from.x_range(), self.to.x_range()),
            remap_clamp(pos.y, self.from.y_range(), self.to.y_range()),
        )
    }
}

/// Transforms the position.
impl std::ops::Mul<Pos2> for RectTransform {
    type Output = Pos2;

    fn mul(self, pos: Pos2) -> Pos2 {
        self.transform_pos(pos)
    }
}

/// Transforms the position.
impl std::ops::Mul<Pos2> for &RectTransform {
    type Output = Pos2;

    fn mul(self, pos: Pos2) -> Pos2 {
        self.transform_pos(pos)
    }
}
