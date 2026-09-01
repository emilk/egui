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
/// If you want a path of fixed width, use [`PathShape`](crate::PathShape) instead.
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

    /// Set if the stroke is on the inside, outside, or centered on the band edge.
    #[inline]
    pub fn with_stroke_kind(mut self, stroke_kind: StrokeKind) -> Self {
        self.stroke_kind = stroke_kind;
        self
    }

    /// Set the rotation of the band (in radians, clockwise) around a custom pivot point.
    ///
    /// The band keeps the position it already had at `pivot`, so calling this again
    /// with a different angle rotates around the same point.
    #[inline]
    pub fn with_angle_and_pivot(mut self, angle: f32, pivot: Pos2) -> Self {
        let pivot = pivot.to_vec2();
        // The points are stored pre-rotation, so undo each rotation to get the local offset:
        let translation =
            emath::Rot2::from_angle(-angle) * pivot - emath::Rot2::from_angle(-self.angle) * pivot;
        self.angle = angle;
        for point in &mut self.points {
            point.x += translation.x;
            point.y.min += translation.y;
            point.y.max += translation.y;
        }
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
    use emath::{TSTransform, pos2, vec2};

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
        .with_angle_and_pivot(core::f32::consts::FRAC_PI_2, Pos2::ZERO);

        band.transform(TSTransform::from_translation(vec2(2.0, 3.0)));

        assert!((band.points[0].x - 3.0).abs() < 1e-6);
        assert!((band.points[0].y.min + 1.0).abs() < 1e-6);
        assert!((band.points[0].y.max - 1.0).abs() < 1e-6);
    }

    #[test]
    fn with_angle_and_pivot_preserves_pivot() {
        let band = BandShape::filled(
            vec![
                BandPoint::new(1.0, 0.0..=2.0),
                BandPoint::new(3.0, 0.0..=2.0),
            ],
            Color32::WHITE,
        )
        .with_angle_and_pivot(core::f32::consts::FRAC_PI_2, pos2(2.0, 1.0));

        assert!((band.points[0].x - 0.0).abs() < 1e-6);
        assert!((band.points[0].y.min + 3.0).abs() < 1e-6);
        assert!((band.points[0].y.max + 1.0).abs() < 1e-6);
    }

    #[test]
    fn with_angle_and_pivot_is_relative_to_the_previous_angle() {
        let points = vec![BandPoint::new(1.0, 0.0..=2.0)];
        let pivot = pos2(2.0, 1.0);
        let once = BandShape::filled(points.clone(), Color32::WHITE)
            .with_angle_and_pivot(core::f32::consts::FRAC_PI_2, pivot);
        let twice = BandShape::filled(points, Color32::WHITE)
            .with_angle_and_pivot(core::f32::consts::FRAC_PI_4, pivot)
            .with_angle_and_pivot(core::f32::consts::FRAC_PI_2, pivot);

        assert!((once.points[0].x - twice.points[0].x).abs() < 1e-6);
        assert!((once.points[0].y.min - twice.points[0].y.min).abs() < 1e-6);
        assert!((once.points[0].y.max - twice.points[0].y.max).abs() < 1e-6);
    }

    #[test]
    fn transform_before_with_angle_and_pivot_rotates_around_translated_origin() {
        let mut band = BandShape::filled(vec![BandPoint::new(1.0, 0.0..=2.0)], Color32::WHITE);
        let center = pos2(10.0, 20.0);
        band.transform(TSTransform::from_translation(center.to_vec2()));
        let band = band.with_angle_and_pivot(core::f32::consts::FRAC_PI_2, center);

        let rotation = emath::Rot2::from_angle(band.angle);
        let min = rotation * vec2(band.points[0].x, band.points[0].y.min);
        let max = rotation * vec2(band.points[0].x, band.points[0].y.max);
        assert!((min.x - 10.0).abs() < 1e-6);
        assert!((min.y - 21.0).abs() < 1e-6);
        assert!((max.x - 8.0).abs() < 1e-6);
        assert!((max.y - 21.0).abs() < 1e-6);
    }
}
