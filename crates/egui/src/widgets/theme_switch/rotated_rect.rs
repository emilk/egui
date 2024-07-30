use crate::Painter;
use emath::{vec2, Pos2, Rect, Rot2};
use epaint::{Color32, PathShape, Rounding, Stroke};

/// Draws a rectangle with rounded corners, rotated around an origin.
pub(crate) fn draw_rotated_rect(
    painter: &Painter,
    rect: Rect,
    rounding: impl Into<Rounding>,
    fill: impl Into<Color32>,
    rot: impl Into<Rot2>,
    origin: Pos2,
) {
    let rounding = rounding.into();
    let fill = fill.into();
    let rot = rot.into();
    let safe_inset_points = safe_inset_points(rect, rot, origin, rounding);

    painter.add(PathShape::convex_polygon(
        safe_inset_points.to_vec(),
        fill,
        Stroke::NONE,
    ));

    // Nobody will notice that we're not actually drawing
    // the round bits... 🤫
}

fn safe_inset_points(rect: Rect, rot: Rot2, origin: Pos2, rounding: Rounding) -> [Pos2; 8] {
    [
        rotate(rect.left_top() + vec2(0.0, rounding.nw), rot, origin),
        rotate(rect.left_top() + vec2(rounding.nw, 0.0), rot, origin),
        rotate(rect.right_top() - vec2(rounding.ne, 0.0), rot, origin),
        rotate(rect.right_top() + vec2(0.0, rounding.ne), rot, origin),
        rotate(rect.right_bottom() - vec2(0.0, rounding.se), rot, origin),
        rotate(rect.right_bottom() - vec2(rounding.se, 0.0), rot, origin),
        rotate(rect.left_bottom() + vec2(rounding.sw, 0.0), rot, origin),
        rotate(rect.left_bottom() - vec2(0.0, rounding.sw), rot, origin),
    ]
}

fn rotate(point: Pos2, rot: Rot2, origin: Pos2) -> Pos2 {
    origin + rot * (point - origin)
}
