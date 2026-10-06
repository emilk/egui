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

    /// Rotate ellipse by this many radians clockwise around its center.
    pub angle: f32,
}

impl EllipseShape {
    #[inline]
    pub fn filled(center: Pos2, radius: Vec2, fill_color: impl Into<Color32>) -> Self {
        Self {
            center,
            radius,
            fill: fill_color.into(),
            stroke: Default::default(),
            angle: 0.0,
        }
    }

    #[inline]
    pub fn stroke(center: Pos2, radius: Vec2, stroke: impl Into<Stroke>) -> Self {
        Self {
            center,
            radius,
            fill: Default::default(),
            stroke: stroke.into(),
            angle: 0.0,
        }
    }

    /// Set the rotation of the ellipse (in radians, clockwise).
    /// The ellipse rotates around its center.
    #[inline]
    pub fn with_angle(mut self, angle: f32) -> Self {
        self.angle = angle;
        self
    }

    /// Set the rotation of the ellipse (in radians, clockwise) around a custom pivot point.
    #[inline]
    pub fn with_angle_and_pivot(mut self, angle: f32, pivot: Pos2) -> Self {
        self.angle = angle;
        let rot = emath::Rot2::from_angle(angle);
        self.center = pivot + rot * (self.center - pivot);
        self
    }

    /// The visual bounding rectangle (includes stroke width)
    pub fn visual_bounding_rect(&self) -> Rect {
        if self.fill == Color32::TRANSPARENT && self.stroke.is_empty() {
            Rect::NOTHING
        } else {
            let rect = Rect::from_center_size(
                Pos2::ZERO,
                self.radius * 2.0 + Vec2::splat(self.stroke.width),
            );
            rect.rotate_bb(emath::Rot2::from_angle(self.angle))
                .translate(self.center.to_vec2())
        }
    }
}

impl From<EllipseShape> for Shape {
    #[inline(always)]
    fn from(shape: EllipseShape) -> Self {
        Self::Ellipse(shape)
    }
}
