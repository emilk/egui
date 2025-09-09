use crate::*;

/// How to paint an ellipse.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct EllipseShape {
    pub center: Pos2,

    /// Radius is the vector (a, b) where the width of the Ellipse is 2a and the height is 2b
    pub radius: Vec2,
    pub fill: Color32,
    pub stroke: Stroke,
}

impl EllipseShape {
    #[inline]
    pub fn filled(center: Pos2, radius: Vec2, fill_color: impl Into<Color32>) -> Self {
        Self {
            center,
            radius,
            fill: fill_color.into(),
            stroke: Default::default(),
        }
    }

    #[inline]
    pub fn stroke(center: Pos2, radius: Vec2, stroke: impl Into<Stroke>) -> Self {
        Self {
            center,
            radius,
            fill: Default::default(),
            stroke: stroke.into(),
        }
    }

    /// The visual bounding rectangle (includes stroke width)
    pub fn visual_bounding_rect(&self) -> Rect {
        if self.fill == Color32::TRANSPARENT && self.stroke.is_empty() {
            Rect::NOTHING
        } else {
            Rect::from_center_size(
                self.center,
                self.radius * 2.0 + Vec2::splat(self.stroke.width),
            )
        }
    }
}

impl From<EllipseShape> for Shape {
    #[inline(always)]
    fn from(shape: EllipseShape) -> Self {
        Self::Ellipse(shape)
    }
}
