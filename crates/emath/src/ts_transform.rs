use crate::{Pos2, Rect, Vec2};

/// Linearly transforms positions via a translation, then a scaling.
///
/// [`TSTransform`] first translates points, then scales them with the scaling origin
/// at `0, 0` (the top left corner)
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct TSTransform {
    /// Translation amount.
    pub translation: Vec2,
    /// Scaling amount after translation, scaled around (0, 0).
    pub scaling: f32,
}

impl Eq for TSTransform {}

impl Default for TSTransform {
    #[inline]
    fn default() -> Self {
        TSTransform {
            translation: Vec2::ZERO,
            scaling: 1.0,
        }
    }
}

impl TSTransform {
    /// The translation is applied first, then scaling around 0, 0.
    pub fn new(translation: Vec2, scaling: f32) -> Self {
        Self {
            translation,
            scaling,
        }
    }

    pub fn from_translation(translation: Vec2) -> Self {
        Self::new(translation, 1.0)
    }

    pub fn from_scaling(scaling: f32) -> Self {
        Self::new(Vec2::ZERO, scaling)
    }

    /// Reverses the transformation from screen space to layer space.
    pub fn invert_pos(&self, pos: Pos2) -> Pos2 {
        // First, reverse scale around 0, 0, then reverse transform.
        let pos = pos / self.scaling;
        pos - self.translation
    }

    /// Reverses the transformation from screen space to layer space.
    pub fn invert_rect(&self, rect: Rect) -> Rect {
        Rect::from_min_max(self.invert_pos(rect.min), self.invert_pos(rect.max))
    }

    /// Transforms the given coordinate by translation then scaling.
    pub fn mul_pos(&self, pos: Pos2) -> Pos2 {
        let pos = pos + self.translation;
        pos * self.scaling
    }

    /// Transforms the given rectangle by translation then scaling.
    pub fn mul_rect(&self, rect: Rect) -> Rect {
        Rect {
            min: self.mul_pos(rect.min),
            max: self.mul_pos(rect.max),
        }
    }
}

/// Transforms the position.
impl std::ops::Mul<Pos2> for TSTransform {
    type Output = Pos2;

    fn mul(self, pos: Pos2) -> Pos2 {
        self.mul_pos(pos)
    }
}

/// Transforms the position.
impl std::ops::Mul<Rect> for TSTransform {
    type Output = Rect;

    fn mul(self, rect: Rect) -> Rect {
        self.mul_rect(rect)
    }
}
