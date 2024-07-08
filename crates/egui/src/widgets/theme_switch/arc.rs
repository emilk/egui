use emath::{vec2, Pos2, Vec2};
use epaint::{Color32, CubicBezierShape, Stroke};
use std::f32::consts::FRAC_PI_2;
use std::ops::RangeInclusive;

pub(crate) fn approximate_with_beziers(
    center: impl Into<Pos2>,
    radius: impl Into<f32>,
    range: impl Into<RangeInclusive<f32>>,
    fill: impl Into<Color32>,
    stroke: impl Into<Stroke>,
) -> impl Iterator<Item = CubicBezierShape> + Clone {
    let fill = fill.into();
    let stroke = stroke.into();
    approximate_with_beziers_impl(center.into(), radius.into(), range.into())
        .map(move |p| CubicBezierShape::from_points_stroke(p, false, fill, stroke))
}

// Implementation based on:
// Riškus, Aleksas. (2006). Approximation of a cubic bezier curve by circular arcs and vice versa.
// Information Technology and Control. 35.

fn approximate_with_beziers_impl(
    center: Pos2,
    radius: f32,
    range: RangeInclusive<f32>,
) -> impl Iterator<Item = [Pos2; 4]> + Clone {
    QuarterTurnsIter(Some(range))
        .map(move |r| approximate_with_bezier(center, radius, *r.start(), *r.end()))
}

fn approximate_with_bezier(center: Pos2, radius: f32, start: f32, end: f32) -> [Pos2; 4] {
    let p1 = center + radius * Vec2::angled(start);
    let p4 = center + radius * Vec2::angled(end);

    let a = p1 - center;
    let b = p4 - center;
    let q1 = a.length_sq();
    let q2 = q1 + a.dot(b);
    let k2 = (4.0 / 3.0) * ((2.0 * q1 * q2).sqrt() - q2) / (a.x * b.y - a.y * b.x);

    let p2 = center + vec2(a.x - k2 * a.y, a.y + k2 * a.x);
    let p3 = center + vec2(b.x + k2 * b.y, b.y - k2 * b.x);

    [p1, p2, p3, p4]
}

// We can approximate at most one quadrant of the circle
// so we divide it up into individual chunks that we then approximate
// using bezier curves.
#[derive(Clone)]
struct QuarterTurnsIter(Option<RangeInclusive<f32>>);

const QUARTER_TURN: f32 = FRAC_PI_2;
impl Iterator for QuarterTurnsIter {
    type Item = RangeInclusive<f32>;

    fn next(&mut self) -> Option<Self::Item> {
        let (start, end) = self.0.clone()?.into_inner();
        let distance = end - start;
        if distance < QUARTER_TURN {
            self.0 = None;
            Some(start..=end)
        } else {
            let new_start = start + (QUARTER_TURN * distance.signum());
            self.0 = Some(new_start..=end);
            Some(start..=new_start)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some((start, end)) = self.0.clone().map(|x| x.into_inner()) {
            let turns = (start - end).abs() / QUARTER_TURN;
            let lower = turns.floor() as usize;
            let upper = turns.ceil() as usize;
            (lower, Some(upper))
        } else {
            (0, None)
        }
    }
}
