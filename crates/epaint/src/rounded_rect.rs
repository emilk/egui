use emath::{Pos2, Rect, Vec2, vec2};

use crate::CornerRadius;

/// A rectangle shape with rounded corners.
///
/// Not a painting primitive. For that, see [`crate::RectShape`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RoundedRect {
    pub rect: Rect,
    pub corner_radius: CornerRadius,
}

impl RoundedRect {
    #[inline]
    pub fn new(rect: Rect, corner_radius: impl Into<CornerRadius>) -> Self {
        Self {
            rect,
            corner_radius: corner_radius.into(),
        }
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
        let max_radius = 0.5 * rect.size().min_elem();
        let corners = [
            (f32::from(corner_radius.nw), vec2(-1.0, -1.0)),
            (f32::from(corner_radius.ne), vec2(1.0, -1.0)),
            (f32::from(corner_radius.sw), vec2(-1.0, 1.0)),
            (f32::from(corner_radius.se), vec2(1.0, 1.0)),
        ];
        for (radius, dir) in corners {
            // Same clamping as the tessellator, so we agree with the rendered shape:
            let radius = radius.min(max_radius);
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
            corner_radius: CornerRadius::ZERO,
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
            CornerRadius {
                nw: 10,
                ne: 0,
                sw: 0,
                se: 20,
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
    fn clamp_pos_oversized_radius() {
        // A radius larger than half the rect is clamped, like in the tessellator:
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0));
        let oversized = RoundedRect::new(rect, 200);
        let clamped_radius = RoundedRect::new(rect, 50);
        for pos in [pos2(0.0, 0.0), pos2(100.0, 0.0), pos2(30.0, -10.0)] {
            assert_eq!(oversized.clamp_pos(pos), clamped_radius.clamp_pos(pos));
        }
    }
}
