use emath::{Pos2, Rect, Vec2, vec2};

use crate::CornerRadiusF32;

/// A rectangle geometry with rounded corners.
///
/// Not a painting primitive. For that, see [`crate::RectShape`].
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RoundedRect {
    rect: Rect,
    corner_radius: CornerRadiusF32,
}

impl RoundedRect {
    /// The corner radius is clamped to half the size of the rectangle,
    /// like in the tessellator, so that we agree with the rendered shape.
    #[inline]
    pub fn new(rect: Rect, corner_radius: impl Into<CornerRadiusF32>) -> Self {
        let max_radius = 0.5 * rect.size().min_elem();
        Self {
            rect,
            corner_radius: corner_radius.into().at_most(max_radius).at_least(0.0),
        }
    }

    #[inline]
    pub fn rect(&self) -> Rect {
        self.rect
    }

    #[inline]
    pub fn corner_radius(&self) -> CornerRadiusF32 {
        self.corner_radius
    }

    /// Split into the rectangle and the corner radius.
    #[inline]
    pub fn into_parts(self) -> (Rect, CornerRadiusF32) {
        let Self {
            rect,
            corner_radius,
        } = self;
        (rect, corner_radius)
    }

    /// Expand the rectangle and the corner radii by the given amount.
    #[inline]
    #[must_use]
    pub fn expand(self, amount: f32) -> Self {
        Self::new(
            self.rect.expand(amount),
            self.corner_radius + CornerRadiusF32::same(amount),
        )
    }

    /// Clamp the given position to lie within this rounded rectangle.
    ///
    /// Positions in the corner regions are projected onto the corner arcs.
    pub fn clamp_pos(&self, pos: Pos2) -> Pos2 {
        let Self {
            rect,
            corner_radius,
        } = *self;
        let pos = rect.clamp(pos);
        let corners = [
            (corner_radius.nw, vec2(-1.0, -1.0)),
            (corner_radius.ne, vec2(1.0, -1.0)),
            (corner_radius.sw, vec2(-1.0, 1.0)),
            (corner_radius.se, vec2(1.0, 1.0)),
        ];
        for (radius, dir) in corners {
            let arc_center = rect.center() + dir * (rect.size() / 2.0 - Vec2::splat(radius));
            let offset = pos - arc_center;
            if 0.0 < offset.x * dir.x && 0.0 < offset.y * dir.y && radius < offset.length() {
                return arc_center + (radius / offset.length()) * offset;
            }
        }
        pos
    }
}

impl From<Rect> for RoundedRect {
    #[inline]
    fn from(rect: Rect) -> Self {
        Self {
            rect,
            corner_radius: CornerRadiusF32::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use emath::pos2;

    use super::*;

    #[test]
    fn clamp_pos() {
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0));
        let rounded = RoundedRect::new(
            rect,
            CornerRadiusF32 {
                nw: 10.0,
                ne: 0.0,
                sw: 0.0,
                se: 20.0,
            },
        );

        // Interior point is untouched:
        assert_eq!(rounded.clamp_pos(pos2(50.0, 50.0)), pos2(50.0, 50.0));

        // Sharp corner is untouched:
        assert_eq!(rounded.clamp_pos(pos2(100.0, 0.0)), pos2(100.0, 0.0));

        // Outside the rect is clamped to the edge:
        assert_eq!(rounded.clamp_pos(pos2(-10.0, 50.0)), pos2(0.0, 50.0));

        // Rounded corner is projected onto the arc:
        let clamped = rounded.clamp_pos(pos2(0.0, 0.0));
        let arc_center = pos2(10.0, 10.0);
        assert!((clamped - arc_center).length() - 10.0 < 0.001);
        let expected = 10.0 - 10.0 / core::f32::consts::SQRT_2;
        assert!((clamped - pos2(expected, expected)).length() < 0.001);

        // Point on the arc stays put:
        assert_eq!(rounded.clamp_pos(pos2(10.0, 0.0)), pos2(10.0, 0.0));
    }

    #[test]
    fn expand() {
        let rect = Rect::from_min_max(pos2(10.0, 10.0), pos2(90.0, 90.0));
        let expanded = RoundedRect::new(rect, 20.0).expand(10.0);
        assert_eq!(
            expanded.rect(),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0))
        );
        assert_eq!(expanded.corner_radius(), CornerRadiusF32::same(30.0));
    }

    #[test]
    fn oversized_radius_is_clamped() {
        // A radius larger than half the rect is clamped, like in the tessellator:
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0));
        assert_eq!(RoundedRect::new(rect, 200.0), RoundedRect::new(rect, 50.0));
        assert_eq!(
            RoundedRect::new(rect, 200.0).corner_radius(),
            CornerRadiusF32::same(50.0)
        );
    }
}
