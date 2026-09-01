use emath::{Pos2, Rangef, Rect, TSTransform};

use crate::{Color32, Shape, Stroke, StrokeKind};

/// A sample in a [`BandShape`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct BandPoint {
    /// The horizontal position of this sample.
    pub x: f32,

    /// The vertical extent of the band at [`Self::x`].
    pub y: Rangef,
}

impl BandPoint {
    #[inline]
    pub fn new(x: f32, y: impl Into<Rangef>) -> Self {
        Self { x, y: y.into() }
    }

    pub(crate) fn is_valid(self) -> bool {
        self.x.is_finite()
            && self.y.min.is_finite()
            && self.y.max.is_finite()
            && self.y.min <= self.y.max
    }
}

/// A varying-width band along a direction.
///
/// The samples are x-monotone in the band's local coordinate system. Invalid points,
/// reversed ranges, and spans with non-increasing x are ignored.
///
/// If you want a path of fixed width, use [`PathShape`] instead.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct BandShape {
    /// The samples that define the lower and upper boundaries, sorted by [`BandPoint::x`].
    pub points: Vec<BandPoint>,

    /// The fill color of the band.
    pub fill: Color32,

    /// An optional stroke for the two boundaries.
    ///
    /// The stroke is painted on top of the fill. The end caps are left open.
    pub stroke: Stroke,

    /// Whether the stroke is inside, outside, or centered on the band edge.
    pub stroke_kind: StrokeKind,

    /// Rotate the band by this many radians clockwise around the origin `(0, 0)`.
    pub angle: f32,
}

impl BandShape {
    #[inline]
    pub fn new(
        points: Vec<BandPoint>,
        fill: impl Into<Color32>,
        stroke: impl Into<Stroke>,
    ) -> Self {
        Self {
            points,
            fill: fill.into(),
            stroke: stroke.into(),
            stroke_kind: StrokeKind::Middle,
            angle: 0.0,
        }
    }

    #[inline]
    pub fn filled(points: Vec<BandPoint>, fill: impl Into<Color32>) -> Self {
        Self::new(points, fill, Stroke::NONE)
    }

    #[inline]
    pub fn stroke(points: Vec<BandPoint>, stroke: impl Into<Stroke>) -> Self {
        Self::new(points, Color32::TRANSPARENT, stroke)
    }

    /// Set the rotation of the band (in radians, clockwise).
    /// The band rotates around the origin.
    #[inline]
    pub fn with_angle(mut self, angle: f32) -> Self {
        self.angle = angle;
        self
    }

    /// Transform the band in-place.
    pub fn transform(&mut self, transform: TSTransform) {
        let translation = emath::Rot2::from_angle(-self.angle) * transform.translation;
        for point in &mut self.points {
            point.x = transform.scaling * point.x + translation.x;
            point.y.min = transform.scaling * point.y.min + translation.y;
            point.y.max = transform.scaling * point.y.max + translation.y;
        }
        self.stroke.width *= transform.scaling;
    }

    /// The visual bounding rectangle, including the stroke width.
    pub fn visual_bounding_rect(&self) -> Rect {
        if !self.angle.is_finite() || (self.fill == Color32::TRANSPARENT && self.stroke.is_empty())
        {
            return Rect::NOTHING;
        }

        let rect = self.local_bounding_rect();
        if rect == Rect::NOTHING {
            return Rect::NOTHING;
        }

        let stroke_width = match self.stroke_kind {
            StrokeKind::Inside => 0.0,
            StrokeKind::Middle => self.stroke.width / 2.0,
            StrokeKind::Outside => self.stroke.width,
        };
        rect.expand(stroke_width)
            .rotate_bb(emath::Rot2::from_angle(self.angle))
    }

    fn local_bounding_rect(&self) -> Rect {
        let mut rect = Rect::NOTHING;
        for &[left, right] in self.points.array_windows() {
            if left.is_valid() && right.is_valid() && left.x < right.x {
                rect.extend_with(Pos2::new(left.x, left.y.min));
                rect.extend_with(Pos2::new(left.x, left.y.max));
                rect.extend_with(Pos2::new(right.x, right.y.min));
                rect.extend_with(Pos2::new(right.x, right.y.max));
            }
        }
        rect
    }
}

impl From<BandShape> for Shape {
    #[inline(always)]
    fn from(shape: BandShape) -> Self {
        Self::Band(shape)
    }
}

#[cfg(test)]
mod tests {
    use emath::{TSTransform, vec2};

    use super::*;

    #[test]
    fn transform_preserves_rotation() {
        let mut band = BandShape::filled(
            vec![
                BandPoint::new(0.0, 1.0..=3.0),
                BandPoint::new(2.0, 2.0..=4.0),
            ],
            Color32::WHITE,
        )
        .with_angle(core::f32::consts::FRAC_PI_2);

        band.transform(TSTransform::from_translation(vec2(2.0, 3.0)));

        assert!((band.points[0].x - 3.0).abs() < 1e-6);
        assert!((band.points[0].y.min + 1.0).abs() < 1e-6);
        assert!((band.points[0].y.max - 1.0).abs() < 1e-6);
    }
}
