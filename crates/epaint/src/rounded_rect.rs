use emath::{Pos2, Rect, Vec2, vec2};

use crate::CornerRadius;

/// A rectangle shape with rounded corners.
///
/// Not a painting primitive. For that, see [`crate::RectShape`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RoundedRect {
    rect: Rect,
    corner_radius: CornerRadius,
}

impl RoundedRect {
    /// The corner radius is clamped to half the size of the rectangle,
    /// like in the tessellator, so that we agree with the rendered shape.
    #[inline]
    pub fn new(rect: Rect, corner_radius: impl Into<CornerRadius>) -> Self {
        let max_radius = (0.5 * rect.size().min_elem()).clamp(0.0, 255.0) as u8;
        Self {
            rect,
            corner_radius: corner_radius.into().at_most(max_radius),
        }
    }

    #[inline]
    pub fn rect(&self) -> Rect {
        self.rect
    }

    #[inline]
    pub fn corner_radius(&self) -> CornerRadius {
        self.corner_radius
    }

    /// Expand the rectangle and the corner radii by the given amount.
    #[inline]
    #[must_use]
    pub fn expand(self, amount: u8) -> Self {
        Self::new(
            self.rect.expand(f32::from(amount)),
            self.corner_radius + amount,
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
            (f32::from(corner_radius.nw), vec2(-1.0, -1.0)),
            (f32::from(corner_radius.ne), vec2(1.0, -1.0)),
            (f32::from(corner_radius.sw), vec2(-1.0, 1.0)),
            (f32::from(corner_radius.se), vec2(1.0, 1.0)),
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
    fn expand() {
        let rect = Rect::from_min_max(pos2(10.0, 10.0), pos2(90.0, 90.0));
        let expanded = RoundedRect::new(rect, 20).expand(10);
        assert_eq!(
            expanded.rect(),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0))
        );
        assert_eq!(expanded.corner_radius(), CornerRadius::same(30));
    }

    #[test]
    fn oversized_radius_is_clamped() {
        // A radius larger than half the rect is clamped, like in the tessellator:
        let rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0));
        assert_eq!(RoundedRect::new(rect, 200), RoundedRect::new(rect, 50));
        assert_eq!(
            RoundedRect::new(rect, 200).corner_radius(),
            CornerRadius::same(50)
        );
    }
}
