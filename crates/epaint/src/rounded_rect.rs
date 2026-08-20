use emath::{Pos2, Rect, Vec2, vec2};

use crate::CornerRadius;

/// A rectangle with rounded corners.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RoundedRect {
    pub rect: Rect,
    pub rounding: CornerRadius,
}

impl RoundedRect {
    #[inline]
    pub fn new(rect: Rect, rounding: impl Into<CornerRadius>) -> Self {
        Self {
            rect,
            rounding: rounding.into(),
        }
    }

    /// Clamp the given position to lie within this rounded rectangle.
    ///
    /// Positions in the corner regions are projected onto the corner arcs.
    pub fn clamp_pos(&self, pos: Pos2) -> Pos2 {
        let Self { rect, rounding } = *self;
        let pos = rect.clamp(pos);
        let corners = [
            (f32::from(rounding.nw), vec2(-1.0, -1.0)),
            (f32::from(rounding.ne), vec2(1.0, -1.0)),
            (f32::from(rounding.sw), vec2(-1.0, 1.0)),
            (f32::from(rounding.se), vec2(1.0, 1.0)),
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
            rounding: CornerRadius::ZERO,
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
}
