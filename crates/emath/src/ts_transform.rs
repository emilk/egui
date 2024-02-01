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
    /// Scaling applied first, scaled around (0, 0).
    pub scaling: f32,

    /// Translation amount, applied after translation.
    pub translation: Vec2,
}

impl Eq for TSTransform {}

impl Default for TSTransform {
    #[inline]
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl TSTransform {
    pub const IDENTITY: Self = Self {
        translation: Vec2::ZERO,
        scaling: 1.0,
    };

    /// The translation is applied first, then scaling around 0, 0.
    #[inline]
    pub fn new(translation: Vec2, scaling: f32) -> Self {
        Self {
            translation,
            scaling,
        }
    }

    #[inline]
    pub fn from_translation(translation: Vec2) -> Self {
        Self::new(translation, 1.0)
    }

    #[inline]
    pub fn from_scaling(scaling: f32) -> Self {
        Self::new(Vec2::ZERO, scaling)
    }

    /// Reverses the transformation from screen space to layer space.
    ///
    /// ```
    /// # use emath::{pos2, vec2, TSTransform};
    /// let p1 = pos2(2.0, 3.0);
    /// let p2 = pos2(12.0, 5.0);
    /// let ts = TSTransform::new(vec2(2.0, 3.0), 2.0);
    /// let inv = ts.inverse();
    /// assert_eq!(inv.mul_pos(p1), pos2(0.0, 0.0));
    /// assert_eq!(inv.mul_pos(p2), pos2(5.0, 1.0));
    /// ```
    #[inline]
    pub fn inverse(&self) -> Self {
        Self::new(-self.translation / self.scaling, 1.0 / self.scaling)
    }

    /// Transforms the given coordinate by translation then scaling.
    ///
    /// ```
    /// # use emath::{pos2, vec2, TSTransform};
    /// let p1 = pos2(0.0, 0.0);
    /// let p2 = pos2(5.0, 1.0);
    /// let ts = TSTransform::new(vec2(2.0, 3.0), 2.0);
    /// assert_eq!(ts.mul_pos(p1), pos2(2.0, 3.0));
    /// assert_eq!(ts.mul_pos(p2), pos2(12.0, 5.0));
    /// ```
    #[inline]
    pub fn mul_pos(&self, pos: Pos2) -> Pos2 {
        self.scaling * pos + self.translation
    }

    /// Transforms the given rectangle by translation then scaling.
    ///
    /// ```
    /// # use emath::{pos2, vec2, Rect, TSTransform};
    /// let rect = Rect::from_min_max(pos2(5.0, 5.0), pos2(15.0, 10.0));
    /// let ts = TSTransform::new(vec2(1.0, 0.0), 3.0);
    /// let transformed = ts.mul_rect(rect);
    /// assert_eq!(transformed.min, pos2(16.0, 15.0));
    /// assert_eq!(transformed.max, pos2(46.0, 30.0));
    /// ```
    #[inline]
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

    #[inline]
    fn mul(self, pos: Pos2) -> Pos2 {
        self.mul_pos(pos)
    }
}

/// Transforms the position.
impl std::ops::Mul<Rect> for TSTransform {
    type Output = Rect;

    #[inline]
    fn mul(self, rect: Rect) -> Rect {
        self.mul_rect(rect)
    }
}
