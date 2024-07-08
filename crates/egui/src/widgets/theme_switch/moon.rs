use super::arc::ArcShape;
use crate::epaint::{CubicBezierShape, PathShape, PathStroke};
use crate::{Color32, Painter, Pos2, Vec2};
use std::f32::consts::{PI, TAU};

/// Draws an outlined moon symbol in the waxing crescent phase.
pub(crate) fn moon(painter: &Painter, center: Pos2, radius: f32, color: Color32) {
    let stroke_width = radius / 5.0;

    let start = 0.04 * TAU;
    let start_vec = radius * Vec2::angled(start);
    let size = 0.65 * TAU;
    let end_vec = radius * Vec2::angled(start + size);

    let direction_angle = start - (TAU - size) / 2.;
    let direction = Vec2::angled(direction_angle);

    // We want to draw a circle with the same radius somewhere on the line
    // `direction` such that it intersects with our first circle at `start` and `end`.
    // The connection between the start and end points is a chord of our occluding circle.
    let chord = start_vec - end_vec;
    let angle = 2.0 * (chord.length() / (2.0 * radius)).asin();
    let sagitta = radius * (1.0 - (angle / 2.0).cos());
    let apothem = radius - sagitta;
    let occluding_center = center + midpoint(start_vec, end_vec) + apothem * direction;

    let occlusion_start = direction_angle + PI - angle / 2.;
    let occlusion_end = direction_angle + PI + angle / 2.;

    let main_arc = ArcShape::new(
        center,
        radius,
        start..=(start + size),
        Color32::TRANSPARENT,
        (stroke_width, color),
    );
    let occluding_arc = ArcShape::new(
        occluding_center,
        radius,
        occlusion_end..=occlusion_start,
        Color32::TRANSPARENT,
        (stroke_width, color),
    );

    // We join the beziers together to a path which improves
    // the drawing of the joints somewhat.
    let path = to_path(
        main_arc
            .approximate_as_beziers()
            .chain(occluding_arc.approximate_as_beziers()),
        (stroke_width, color),
    );

    painter.add(path);
}

fn midpoint(a: Vec2, b: Vec2) -> Vec2 {
    0.5 * (a + b)
}

fn to_path(
    beziers: impl IntoIterator<Item = CubicBezierShape>,
    stroke: impl Into<PathStroke>,
) -> PathShape {
    let points = beziers.into_iter().flat_map(|b| b.flatten(None)).collect();
    PathShape {
        points,
        closed: false,
        fill: Default::default(),
        stroke: stroke.into(),
    }
}
