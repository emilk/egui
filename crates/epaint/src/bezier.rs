#![allow(clippy::many_single_char_names)]
use std::ops::Range;

use crate::{shape::Shape, Color32, PathShape, Stroke};
use emath::*;

// ----------------------------------------------------------------------------

/// A cubic [Bézier Curve](https://en.wikipedia.org/wiki/B%C3%A9zier_curve).
///
/// See also [`QuadraticBezierShape`].
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct CubicBezierShape {
    /// The first point is the starting point and the last one is the ending point of the curve.
    /// The middle points are the control points.
    pub points: [Pos2; 4],
    pub closed: bool,

    pub fill: Color32,
    pub stroke: Stroke,
}

impl CubicBezierShape {
    /// Creates a cubic Bézier curve based on 4 points and stroke.
    ///
    /// The first point is the starting point and the last one is the ending point of the curve.
    /// The middle points are the control points.
    pub fn from_points_stroke(
        points: [Pos2; 4],
        closed: bool,
        fill: Color32,
        stroke: impl Into<Stroke>,
    ) -> Self {
        Self {
            points,
            closed,
            fill,
            stroke: stroke.into(),
        }
    }

    /// Transform the curve with the given transform.
    pub fn transform(&self, transform: &RectTransform) -> Self {
        let mut points = [Pos2::default(); 4];
        for (i, origin_point) in self.points.iter().enumerate() {
            points[i] = transform * *origin_point;
        }
        CubicBezierShape {
            points,
            closed: self.closed,
            fill: self.fill,
            stroke: self.stroke,
        }
    }

    /// Convert the cubic Bézier curve to one or two [`PathShape`]'s.
    /// When the curve is closed and it has to intersect with the base line, it will be converted into two shapes.
    /// Otherwise, it will be converted into one shape.
    /// The `tolerance` will be used to control the max distance between the curve and the base line.
    /// The `epsilon` is used when comparing two floats.
    pub fn to_path_shapes(&self, tolerance: Option<f32>, epsilon: Option<f32>) -> Vec<PathShape> {
        let mut pathshapes = Vec::new();
        let mut points_vec = self.flatten_closed(tolerance, epsilon);
        for points in points_vec.drain(..) {
            let pathshape = PathShape {
                points,
                closed: self.closed,
                fill: self.fill,
                stroke: self.stroke,
            };
            pathshapes.push(pathshape);
        }
        pathshapes
    }

    /// The visual bounding rectangle (includes stroke width)
    pub fn visual_bounding_rect(&self) -> Rect {
        if self.fill == Color32::TRANSPARENT && self.stroke.is_empty() {
            Rect::NOTHING
        } else {
            self.logical_bounding_rect().expand(self.stroke.width / 2.0)
        }
    }

    /// Logical bounding rectangle (ignoring stroke width)
    pub fn logical_bounding_rect(&self) -> Rect {
        //temporary solution
        let (mut min_x, mut max_x) = if self.points[0].x < self.points[3].x {
            (self.points[0].x, self.points[3].x)
        } else {
            (self.points[3].x, self.points[0].x)
        };
        let (mut min_y, mut max_y) = if self.points[0].y < self.points[3].y {
            (self.points[0].y, self.points[3].y)
        } else {
            (self.points[3].y, self.points[0].y)
        };

        // find the inflection points and get the x value
        cubic_for_each_local_extremum(
            self.points[0].x,
            self.points[1].x,
            self.points[2].x,
            self.points[3].x,
            &mut |t| {
                let x = self.sample(t).x;
                if x < min_x {
                    min_x = x;
                }
                if x > max_x {
                    max_x = x;
                }
            },
        );

        // find the inflection points and get the y value
        cubic_for_each_local_extremum(
            self.points[0].y,
            self.points[1].y,
            self.points[2].y,
            self.points[3].y,
            &mut |t| {
                let y = self.sample(t).y;
                if y < min_y {
                    min_y = y;
                }
                if y > max_y {
                    max_y = y;
                }
            },
        );

        Rect {
            min: Pos2 { x: min_x, y: min_y },
            max: Pos2 { x: max_x, y: max_y },
        }
    }

    /// split the original cubic curve into a new one within a range.
    pub fn split_range(&self, t_range: Range<f32>) -> Self {
        crate::epaint_assert!(
            t_range.start >= 0.0 && t_range.end <= 1.0 && t_range.start <= t_range.end,
            "range should be in [0.0,1.0]"
        );

        let from = self.sample(t_range.start);
        let to = self.sample(t_range.end);

        let d_from = self.points[1] - self.points[0].to_vec2();
        let d_ctrl = self.points[2] - self.points[1].to_vec2();
        let d_to = self.points[3] - self.points[2].to_vec2();
        let q = QuadraticBezierShape {
            points: [d_from, d_ctrl, d_to],
            closed: self.closed,
            fill: self.fill,
            stroke: self.stroke,
        };
        let delta_t = t_range.end - t_range.start;
        let q_start = q.sample(t_range.start);
        let q_end = q.sample(t_range.end);
        let ctrl1 = from + q_start.to_vec2() * delta_t;
        let ctrl2 = to - q_end.to_vec2() * delta_t;
        CubicBezierShape {
            points: [from, ctrl1, ctrl2, to],
            closed: self.closed,
            fill: self.fill,
            stroke: self.stroke,
        }
    }

    // copied from lyon::geom::flattern_cubic.rs
    // Computes the number of quadratic bézier segments to approximate a cubic one.
    // Derived by Raph Levien from section 10.6 of Sedeberg's CAGD notes
    // https://scholarsarchive.byu.edu/cgi/viewcontent.cgi?article=1000&context=facpub#section.10.6
    // and the error metric from the caffein owl blog post http://caffeineowl.com/graphics/2d/vectorial/cubic2quad01.html
    pub fn num_quadratics(&self, tolerance: f32) -> u32 {
        crate::epaint_assert!(tolerance > 0.0, "the tolerance should be positive");

        let x =
            self.points[0].x - 3.0 * self.points[1].x + 3.0 * self.points[2].x - self.points[3].x;
        let y =
            self.points[0].y - 3.0 * self.points[1].y + 3.0 * self.points[2].y - self.points[3].y;
        let err = x * x + y * y;

        (err / (432.0 * tolerance * tolerance))
            .powf(1.0 / 6.0)
            .ceil()
            .max(1.0) as u32
    }

    /// Find out the t value for the point where the curve is intersected with the base line.
    /// The base line is the line from P0 to P3.
    /// If the curve only has two intersection points with the base line, they should be 0.0 and 1.0.
    /// In this case, the "fill" will be simple since the curve is a convex line.
    /// If the curve has more than two intersection points with the base line, the "fill" will be a problem.
    /// We need to find out where is the 3rd t value (0<t<1)
    /// And the original cubic curve will be split into two curves (0.0..t and t..1.0).
    /// B(t) = (1-t)^3*P0 + 3*t*(1-t)^2*P1 + 3*t^2*(1-t)*P2 + t^3*P3
    /// or B(t) = (P3 - 3*P2 + 3*P1 - P0)*t^3 + (3*P2 - 6*P1 + 3*P0)*t^2 + (3*P1 - 3*P0)*t + P0
    /// this B(t) should be on the line between P0 and P3. Therefore:
    /// (B.x - P0.x)/(P3.x - P0.x) = (B.y - P0.y)/(P3.y - P0.y), or:
    /// B.x * (P3.y - P0.y) - B.y * (P3.x - P0.x) + P0.x * (P0.y - P3.y) + P0.y * (P3.x - P0.x) = 0
    /// B.x = (P3.x - 3 * P2.x + 3 * P1.x - P0.x) * t^3 + (3 * P2.x - 6 * P1.x + 3 * P0.x) * t^2 + (3 * P1.x - 3 * P0.x) * t + P0.x
    /// B.y = (P3.y - 3 * P2.y + 3 * P1.y - P0.y) * t^3 + (3 * P2.y - 6 * P1.y + 3 * P0.y) * t^2 + (3 * P1.y - 3 * P0.y) * t + P0.y
    /// Combine the above three equations and iliminate B.x and B.y, we get:
    /// t^3 * ( (P3.x - 3*P2.x + 3*P1.x - P0.x) * (P3.y - P0.y) - (P3.y - 3*P2.y + 3*P1.y - P0.y) * (P3.x - P0.x))
    /// + t^2 * ( (3 * P2.x - 6 * P1.x + 3 * P0.x) * (P3.y - P0.y) - (3 * P2.y - 6 * P1.y + 3 * P0.y) * (P3.x - P0.x))
    /// + t^1 * ( (3 * P1.x - 3 * P0.x) * (P3.y - P0.y) - (3 * P1.y - 3 * P0.y) * (P3.x - P0.x))
    /// + (P0.x * (P3.y - P0.y) - P0.y * (P3.x - P0.x)) + P0.x * (P0.y - P3.y) + P0.y * (P3.x - P0.x)
    /// = 0
    /// or a * t^3 + b * t^2 + c * t + d = 0
    ///
    /// let x = t - b / (3 * a), then we have:
    /// x^3 + p * x + q = 0, where:
    /// p = (3.0 * a * c - b^2) / (3.0 * a^2)
    /// q = (2.0 * b^3 - 9.0 * a * b * c + 27.0 * a^2 * d) / (27.0 * a^3)
    ///
    /// when p > 0, there will be one real root, two complex roots
    /// when p = 0, there will be two real roots, when p=q=0, there will be three real roots but all 0.
    /// when p < 0, there will be three unique real roots. this is what we need. (x1, x2, x3)
    ///  t = x + b / (3 * a), then we have: t1, t2, t3.
    /// the one between 0.0 and 1.0 is what we need.
    /// <`https://baike.baidu.com/item/%E4%B8%80%E5%85%83%E4%B8%89%E6%AC%A1%E6%96%B9%E7%A8%8B/8388473 /`>
    ///
    pub fn find_cross_t(&self, epsilon: f32) -> Option<f32> {
        let p0 = self.points[0];
        let p1 = self.points[1];
        let p2 = self.points[2];
        let p3 = self.points[3];

        let a = (p3.x - 3.0 * p2.x + 3.0 * p1.x - p0.x) * (p3.y - p0.y)
            - (p3.y - 3.0 * p2.y + 3.0 * p1.y - p0.y) * (p3.x - p0.x);
        let b = (3.0 * p2.x - 6.0 * p1.x + 3.0 * p0.x) * (p3.y - p0.y)
            - (3.0 * p2.y - 6.0 * p1.y + 3.0 * p0.y) * (p3.x - p0.x);
        let c =
            (3.0 * p1.x - 3.0 * p0.x) * (p3.y - p0.y) - (3.0 * p1.y - 3.0 * p0.y) * (p3.x - p0.x);
        let d = p0.x * (p3.y - p0.y) - p0.y * (p3.x - p0.x)
            + p0.x * (p0.y - p3.y)
            + p0.y * (p3.x - p0.x);

        let h = -b / (3.0 * a);
        let p = (3.0 * a * c - b * b) / (3.0 * a * a);
        let q = (2.0 * b * b * b - 9.0 * a * b * c + 27.0 * a * a * d) / (27.0 * a * a * a);

        if p > 0.0 {
            return None;
        }
        let r = (-1.0 * (p / 3.0).powi(3)).sqrt();
        let theta = (-1.0 * q / (2.0 * r)).acos() / 3.0;

        let t1 = 2.0 * r.cbrt() * theta.cos() + h;
        let t2 = 2.0 * r.cbrt() * (theta + 120.0 * std::f32::consts::PI / 180.0).cos() + h;
        let t3 = 2.0 * r.cbrt() * (theta + 240.0 * std::f32::consts::PI / 180.0).cos() + h;

        if t1 > epsilon && t1 < 1.0 - epsilon {
            return Some(t1);
        }
        if t2 > epsilon && t2 < 1.0 - epsilon {
            return Some(t2);
        }
        if t3 > epsilon && t3 < 1.0 - epsilon {
            return Some(t3);
        }
        None
    }

    /// Calculate the point (x,y) at t based on the cubic Bézier curve equation.
    /// t is in [0.0,1.0]
    /// [Bézier Curve](https://en.wikipedia.org/wiki/B%C3%A9zier_curve#Cubic_B.C3.A9zier_curves)
    ///
    pub fn sample(&self, t: f32) -> Pos2 {
        crate::epaint_assert!(
            t >= 0.0 && t <= 1.0,
            "the sample value should be in [0.0,1.0]"
        );

        let h = 1.0 - t;
        let a = t * t * t;
        let b = 3.0 * t * t * h;
        let c = 3.0 * t * h * h;
        let d = h * h * h;
        let result = self.points[3].to_vec2() * a
            + self.points[2].to_vec2() * b
            + self.points[1].to_vec2() * c
            + self.points[0].to_vec2() * d;
        result.to_pos2()
    }

    /// find a set of points that approximate the cubic Bézier curve.
    /// the number of points is determined by the tolerance.
    /// the points may not be evenly distributed in the range [0.0,1.0] (t value)
    pub fn flatten(&self, tolerance: Option<f32>) -> Vec<Pos2> {
        let tolerance = tolerance.unwrap_or((self.points[0].x - self.points[3].x).abs() * 0.001);
        let mut result = vec![self.points[0]];
        self.for_each_flattened_with_t(tolerance, &mut |p, _t| {
            result.push(p);
        });
        result
    }

    /// find a set of points that approximate the cubic Bézier curve.
    /// the number of points is determined by the tolerance.
    /// the points may not be evenly distributed in the range [0.0,1.0] (t value)
    /// this api will check whether the curve will cross the base line or not when closed = true.
    /// The result will be a vec of vec of Pos2. it will store two closed aren in different vec.
    /// The epsilon is used to compare a float value.
    pub fn flatten_closed(&self, tolerance: Option<f32>, epsilon: Option<f32>) -> Vec<Vec<Pos2>> {
        let tolerance = tolerance.unwrap_or((self.points[0].x - self.points[3].x).abs() * 0.001);
        let epsilon = epsilon.unwrap_or(1.0e-5);
        let mut result = Vec::new();
        let mut first_half = Vec::new();
        let mut second_half = Vec::new();
        let mut flipped = false;
        first_half.push(self.points[0]);

        let cross = self.find_cross_t(epsilon);
        match cross {
            Some(cross) => {
                if self.closed {
                    self.for_each_flattened_with_t(tolerance, &mut |p, t| {
                        if t < cross {
                            first_half.push(p);
                        } else {
                            if !flipped {
                                // when just crossed the base line, flip the order of the points
                                // add the cross point to the first half as the last point
                                // and add the cross point to the second half as the first point
                                flipped = true;
                                let cross_point = self.sample(cross);
                                first_half.push(cross_point);
                                second_half.push(cross_point);
                            }
                            second_half.push(p);
                        }
                    });
                } else {
                    self.for_each_flattened_with_t(tolerance, &mut |p, _t| {
                        first_half.push(p);
                    });
                }
            }
            None => {
                self.for_each_flattened_with_t(tolerance, &mut |p, _t| {
                    first_half.push(p);
                });
            }
        }

        result.push(first_half);
        if !second_half.is_empty() {
            result.push(second_half);
        }
        result
    }
    // from lyon_geom::cubic_bezier.rs
    /// Iterates through the curve invoking a callback at each point.
    pub fn for_each_flattened_with_t<F: FnMut(Pos2, f32)>(&self, tolerance: f32, callback: &mut F) {
        flatten_cubic_bezier_with_t(self, tolerance, callback);
    }
}

impl From<CubicBezierShape> for Shape {
    #[inline(always)]
    fn from(shape: CubicBezierShape) -> Self {
        Self::CubicBezier(shape)
    }
}

// ----------------------------------------------------------------------------

/// A quadratic [Bézier Curve](https://en.wikipedia.org/wiki/B%C3%A9zier_curve).
///
/// See also [`CubicBezierShape`].
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct QuadraticBezierShape {
    /// The first point is the starting point and the last one is the ending point of the curve.
    /// The middle point is the control points.
    pub points: [Pos2; 3],
    pub closed: bool,

    pub fill: Color32,
    pub stroke: Stroke,
}

impl QuadraticBezierShape {
    /// Create a new quadratic Bézier shape based on the 3 points and stroke.
    ///
    /// The first point is the starting point and the last one is the ending point of the curve.
    /// The middle point is the control points.
    /// The points should be in the order [start, control, end]
    pub fn from_points_stroke(
        points: [Pos2; 3],
        closed: bool,
        fill: Color32,
        stroke: impl Into<Stroke>,
    ) -> Self {
        QuadraticBezierShape {
            points,
            closed,
            fill,
            stroke: stroke.into(),
        }
    }

    /// Transform the curve with the given transform.
    pub fn transform(&self, transform: &RectTransform) -> Self {
        let mut points = [Pos2::default(); 3];
        for (i, origin_point) in self.points.iter().enumerate() {
            points[i] = transform * *origin_point;
        }
        QuadraticBezierShape {
            points,
            closed: self.closed,
            fill: self.fill,
            stroke: self.stroke,
        }
    }

    /// Convert the quadratic Bézier curve to one [`PathShape`].
    /// The `tolerance` will be used to control the max distance between the curve and the base line.
    pub fn to_path_shape(&self, tolerance: Option<f32>) -> PathShape {
        let points = self.flatten(tolerance);
        PathShape {
            points,
            closed: self.closed,
            fill: self.fill,
            stroke: self.stroke,
        }
    }

    /// The visual bounding rectangle (includes stroke width)
    pub fn visual_bounding_rect(&self) -> Rect {
        if self.fill == Color32::TRANSPARENT && self.stroke.is_empty() {
            Rect::NOTHING
        } else {
            self.logical_bounding_rect().expand(self.stroke.width / 2.0)
        }
    }

    /// Logical bounding rectangle (ignoring stroke width)
    pub fn logical_bounding_rect(&self) -> Rect {
        let (mut min_x, mut max_x) = if self.points[0].x < self.points[2].x {
            (self.points[0].x, self.points[2].x)
        } else {
            (self.points[2].x, self.points[0].x)
        };
        let (mut min_y, mut max_y) = if self.points[0].y < self.points[2].y {
            (self.points[0].y, self.points[2].y)
        } else {
            (self.points[2].y, self.points[0].y)
        };

        quadratic_for_each_local_extremum(
            self.points[0].x,
            self.points[1].x,
            self.points[2].x,
            &mut |t| {
                let x = self.sample(t).x;
                if x < min_x {
                    min_x = x;
                }
                if x > max_x {
                    max_x = x;
                }
            },
        );

        quadratic_for_each_local_extremum(
            self.points[0].y,
            self.points[1].y,
            self.points[2].y,
            &mut |t| {
                let y = self.sample(t).y;
                if y < min_y {
                    min_y = y;
                }
                if y > max_y {
                    max_y = y;
                }
            },
        );

        Rect {
            min: Pos2 { x: min_x, y: min_y },
            max: Pos2 { x: max_x, y: max_y },
        }
    }

    /// Calculate the point (x,y) at t based on the quadratic Bézier curve equation.
    /// t is in [0.0,1.0]
    /// [Bézier Curve](https://en.wikipedia.org/wiki/B%C3%A9zier_curve#Quadratic_B.C3.A9zier_curves)
    ///
    pub fn sample(&self, t: f32) -> Pos2 {
        crate::epaint_assert!(
            t >= 0.0 && t <= 1.0,
            "the sample value should be in [0.0,1.0]"
        );

        let h = 1.0 - t;
        let a = t * t;
        let b = 2.0 * t * h;
        let c = h * h;
        let result = self.points[2].to_vec2() * a
            + self.points[1].to_vec2() * b
            + self.points[0].to_vec2() * c;
        result.to_pos2()
    }

    /// find a set of points that approximate the quadratic Bézier curve.
    /// the number of points is determined by the tolerance.
    /// the points may not be evenly distributed in the range [0.0,1.0] (t value)
    pub fn flatten(&self, tolerance: Option<f32>) -> Vec<Pos2> {
        let tolerance = tolerance.unwrap_or((self.points[0].x - self.points[2].x).abs() * 0.001);
        let mut result = vec![self.points[0]];
        self.for_each_flattened_with_t(tolerance, &mut |p, _t| {
            result.push(p);
        });
        result
    }

    // copied from https://docs.rs/lyon_geom/latest/lyon_geom/
    /// Compute a flattened approximation of the curve, invoking a callback at
    /// each step.
    ///
    /// The callback takes the point and corresponding curve parameter at each step.
    ///
    /// This implements the algorithm described by Raph Levien at
    /// <https://raphlinus.github.io/graphics/curves/2019/12/23/flatten-quadbez.html>
    pub fn for_each_flattened_with_t<F>(&self, tolerance: f32, callback: &mut F)
    where
        F: FnMut(Pos2, f32),
    {
        let params = FlatteningParameters::from_curve(self, tolerance);
        if params.is_point {
            return;
        }

        let count = params.count as u32;
        for index in 1..count {
            let t = params.t_at_iteration(index as f32);

            callback(self.sample(t), t);
        }

        callback(self.sample(1.0), 1.0);
    }
}

impl From<QuadraticBezierShape> for Shape {
    #[inline(always)]
    fn from(shape: QuadraticBezierShape) -> Self {
        Self::QuadraticBezier(shape)
    }
}

// ----------------------------------------------------------------------------

// lyon_geom::flatten_cubic.rs
// copied from https://docs.rs/lyon_geom/latest/lyon_geom/
fn flatten_cubic_bezier_with_t<F: FnMut(Pos2, f32)>(
    curve: &CubicBezierShape,
    tolerance: f32,
    callback: &mut F,
) {
    // debug_assert!(tolerance >= S::EPSILON * S::EPSILON);
    let quadratics_tolerance = tolerance * 0.2;
    let flattening_tolerance = tolerance * 0.8;

    let num_quadratics = curve.num_quadratics(quadratics_tolerance);
    let step = 1.0 / num_quadratics as f32;
    let n = num_quadratics;
    let mut t0 = 0.0;
    for _ in 0..(n - 1) {
        let t1 = t0 + step;

        let quadratic = single_curve_approximation(&curve.split_range(t0..t1));
        quadratic.for_each_flattened_with_t(flattening_tolerance, &mut |point, t_sub| {
            let t = t0 + step * t_sub;
            callback(point, t);
        });

        t0 = t1;
    }

    // Do the last step manually to make sure we finish at t = 1.0 exactly.
    let quadratic = single_curve_approximation(&curve.split_range(t0..1.0));
    quadratic.for_each_flattened_with_t(flattening_tolerance, &mut |point, t_sub| {
        let t = t0 + step * t_sub;
        callback(point, t);
    });
}

// from lyon_geom::quadratic_bezier.rs
// copied from https://docs.rs/lyon_geom/latest/lyon_geom/
struct FlatteningParameters {
    count: f32,
    integral_from: f32,
    integral_step: f32,
    inv_integral_from: f32,
    div_inv_integral_diff: f32,
    is_point: bool,
}

impl FlatteningParameters {
    // https://raphlinus.github.io/graphics/curves/2019/12/23/flatten-quadbez.html
    pub fn from_curve(curve: &QuadraticBezierShape, tolerance: f32) -> Self {
        // Map the quadratic bézier segment to y = x^2 parabola.
        let from = curve.points[0];
        let ctrl = curve.points[1];
        let to = curve.points[2];

        let ddx = 2.0 * ctrl.x - from.x - to.x;
        let ddy = 2.0 * ctrl.y - from.y - to.y;
        let cross = (to.x - from.x) * ddy - (to.y - from.y) * ddx;
        let inv_cross = 1.0 / cross;
        let parabola_from = ((ctrl.x - from.x) * ddx + (ctrl.y - from.y) * ddy) * inv_cross;
        let parabola_to = ((to.x - ctrl.x) * ddx + (to.y - ctrl.y) * ddy) * inv_cross;
        // Note, scale can be NaN, for example with straight lines. When it happens the NaN will
        // propagate to other parameters. We catch it all by setting the iteration count to zero
        // and leave the rest as garbage.
        let scale = cross.abs() / (ddx.hypot(ddy) * (parabola_to - parabola_from).abs());

        let integral_from = approx_parabola_integral(parabola_from);
        let integral_to = approx_parabola_integral(parabola_to);
        let integral_diff = integral_to - integral_from;

        let inv_integral_from = approx_parabola_inv_integral(integral_from);
        let inv_integral_to = approx_parabola_inv_integral(integral_to);
        let div_inv_integral_diff = 1.0 / (inv_integral_to - inv_integral_from);

        // the original author thinks it can be stored as integer if it's not generic.
        // but if so, we have to handle the edge case of the integral being infinite.
        let mut count = (0.5 * integral_diff.abs() * (scale / tolerance).sqrt()).ceil();
        let mut is_point = false;
        // If count is NaN the curve can be approximated by a single straight line or a point.
        if !count.is_finite() {
            count = 0.0;
            is_point = (to.x - from.x).hypot(to.y - from.y) < tolerance * tolerance;
        }

        let integral_step = integral_diff / count;

        FlatteningParameters {
            count,
            integral_from,
            integral_step,
            inv_integral_from,
            div_inv_integral_diff,
            is_point,
        }
    }

    fn t_at_iteration(&self, iteration: f32) -> f32 {
        let u = approx_parabola_inv_integral(self.integral_from + self.integral_step * iteration);
        (u - self.inv_integral_from) * self.div_inv_integral_diff
    }
}

/// Compute an approximation to integral (1 + 4x^2) ^ -0.25 dx used in the flattening code.
fn approx_parabola_integral(x: f32) -> f32 {
    let d: f32 = 0.67;
    let quarter = 0.25;
    x / (1.0 - d + (d.powi(4) + quarter * x * x).sqrt().sqrt())
}

/// Approximate the inverse of the function above.
fn approx_parabola_inv_integral(x: f32) -> f32 {
    let b = 0.39;
    let quarter = 0.25;
    x * (1.0 - b + (b * b + quarter * x * x).sqrt())
}

fn single_curve_approximation(curve: &CubicBezierShape) -> QuadraticBezierShape {
    let c1_x = (curve.points[1].x * 3.0 - curve.points[0].x) * 0.5;
    let c1_y = (curve.points[1].y * 3.0 - curve.points[0].y) * 0.5;
    let c2_x = (curve.points[2].x * 3.0 - curve.points[3].x) * 0.5;
    let c2_y = (curve.points[2].y * 3.0 - curve.points[3].y) * 0.5;
    let c = Pos2 {
        x: (c1_x + c2_x) * 0.5,
        y: (c1_y + c2_y) * 0.5,
    };
    QuadraticBezierShape {
        points: [curve.points[0], c, curve.points[3]],
        closed: curve.closed,
        fill: curve.fill,
        stroke: curve.stroke,
    }
}

fn quadratic_for_each_local_extremum<F: FnMut(f32)>(p0: f32, p1: f32, p2: f32, cb: &mut F) {
    // A quadratic Bézier curve can be derived by a linear function:
    // p(t) = p0 + t(p1 - p0) + t^2(p2 - 2p1 + p0)
    // The derivative is:
    // p'(t) = (p1 - p0) + 2(p2 - 2p1 + p0)t or:
    // f(x) = a* x + b
    let a = p2 - 2.0 * p1 + p0;
    // let b = p1 - p0;
    // no need to check for zero, since we're only interested in local extrema
    if a == 0.0 {
        return;
    }

    let t = (p0 - p1) / a;
    if t > 0.0 && t < 1.0 {
        cb(t);
    }
}

fn cubic_for_each_local_extremum<F: FnMut(f32)>(p0: f32, p1: f32, p2: f32, p3: f32, cb: &mut F) {
    // See www.faculty.idc.ac.il/arik/quality/appendixa.html for an explanation
    // A cubic Bézier curve can be derivated by the following equation:
    // B'(t) = 3(1-t)^2(p1-p0) + 6(1-t)t(p2-p1) + 3t^2(p3-p2) or
    // f(x) = a * x² + b * x + c
    let a = 3.0 * (p3 + 3.0 * (p1 - p2) - p0);
    let b = 6.0 * (p2 - 2.0 * p1 + p0);
    let c = 3.0 * (p1 - p0);

    let in_range = |t: f32| t <= 1.0 && t >= 0.0;

    // linear situation
    if a == 0.0 {
        if b != 0.0 {
            let t = -c / b;
            if in_range(t) {
                cb(t);
            }
        }
        return;
    }

    let discr = b * b - 4.0 * a * c;
    // no Real solution
    if discr < 0.0 {
        return;
    }

    // one Real solution
    if discr == 0.0 {
        let t = -b / (2.0 * a);
        if in_range(t) {
            cb(t);
        }
        return;
    }

    // two Real solutions
    let discr = discr.sqrt();
    let t1 = (-b - discr) / (2.0 * a);
    let t2 = (-b + discr) / (2.0 * a);
    if in_range(t1) {
        cb(t1);
    }
    if in_range(t2) {
        cb(t2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quadratic_bounding_box() {
        let curve = QuadraticBezierShape {
            points: [
                Pos2 { x: 110.0, y: 170.0 },
                Pos2 { x: 10.0, y: 10.0 },
                Pos2 { x: 180.0, y: 30.0 },
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };
        let bbox = curve.logical_bounding_rect();
        assert!((bbox.min.x - 72.96).abs() < 0.01);
        assert!((bbox.min.y - 27.78).abs() < 0.01);

        assert!((bbox.max.x - 180.0).abs() < 0.01);
        assert!((bbox.max.y - 170.0).abs() < 0.01);

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.1, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 26);

        let curve = QuadraticBezierShape {
            points: [
                Pos2 { x: 110.0, y: 170.0 },
                Pos2 { x: 180.0, y: 30.0 },
                Pos2 { x: 10.0, y: 10.0 },
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };
        let bbox = curve.logical_bounding_rect();
        assert!((bbox.min.x - 10.0).abs() < 0.01);
        assert!((bbox.min.y - 10.0).abs() < 0.01);

        assert!((bbox.max.x - 130.42).abs() < 0.01);
        assert!((bbox.max.y - 170.0).abs() < 0.01);

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.1, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 25);
    }

    #[test]
    fn test_quadratic_dfferent_tolerance() {
        let curve = QuadraticBezierShape {
            points: [
                Pos2 { x: 110.0, y: 170.0 },
                Pos2 { x: 180.0, y: 30.0 },
                Pos2 { x: 10.0, y: 10.0 },
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };
        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(1.0, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 9);

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.1, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 25);

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 77);

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.001, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 240);
    }

    #[test]
    fn test_cubic_bounding_box() {
        let curve = CubicBezierShape {
            points: [
                pos2(10.0, 10.0),
                pos2(110.0, 170.0),
                pos2(180.0, 30.0),
                pos2(270.0, 210.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let bbox = curve.logical_bounding_rect();
        assert_eq!(bbox.min.x, 10.0);
        assert_eq!(bbox.min.y, 10.0);
        assert_eq!(bbox.max.x, 270.0);
        assert_eq!(bbox.max.y, 210.0);

        let curve = CubicBezierShape {
            points: [
                pos2(10.0, 10.0),
                pos2(110.0, 170.0),
                pos2(270.0, 210.0),
                pos2(180.0, 30.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let bbox = curve.logical_bounding_rect();
        assert_eq!(bbox.min.x, 10.0);
        assert_eq!(bbox.min.y, 10.0);
        assert!((bbox.max.x - 206.50).abs() < 0.01);
        assert!((bbox.max.y - 148.48).abs() < 0.01);

        let curve = CubicBezierShape {
            points: [
                pos2(110.0, 170.0),
                pos2(10.0, 10.0),
                pos2(270.0, 210.0),
                pos2(180.0, 30.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let bbox = curve.logical_bounding_rect();
        assert!((bbox.min.x - 86.71).abs() < 0.01);
        assert!((bbox.min.y - 30.0).abs() < 0.01);

        assert!((bbox.max.x - 199.27).abs() < 0.01);
        assert!((bbox.max.y - 170.0).abs() < 0.01);
    }

    #[test]
    fn test_cubic_different_tolerance_flattening() {
        let curve = CubicBezierShape {
            points: [
                pos2(0.0, 0.0),
                pos2(100.0, 0.0),
                pos2(100.0, 100.0),
                pos2(100.0, 200.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(1.0, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 10);

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.5, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 13);

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.1, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 28);

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 83);

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.001, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 248);
    }

    #[test]
    fn test_cubic_different_shape_flattening() {
        let curve = CubicBezierShape {
            points: [
                pos2(90.0, 110.0),
                pos2(30.0, 170.0),
                pos2(210.0, 170.0),
                pos2(170.0, 110.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 117);

        let curve = CubicBezierShape {
            points: [
                pos2(90.0, 110.0),
                pos2(90.0, 170.0),
                pos2(170.0, 170.0),
                pos2(170.0, 110.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 91);

        let curve = CubicBezierShape {
            points: [
                pos2(90.0, 110.0),
                pos2(110.0, 170.0),
                pos2(150.0, 170.0),
                pos2(170.0, 110.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 75);

        let curve = CubicBezierShape {
            points: [
                pos2(90.0, 110.0),
                pos2(110.0, 170.0),
                pos2(230.0, 110.0),
                pos2(170.0, 110.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 100);

        let curve = CubicBezierShape {
            points: [
                pos2(90.0, 110.0),
                pos2(110.0, 170.0),
                pos2(210.0, 70.0),
                pos2(170.0, 110.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 71);

        let curve = CubicBezierShape {
            points: [
                pos2(90.0, 110.0),
                pos2(110.0, 170.0),
                pos2(150.0, 50.0),
                pos2(170.0, 110.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 88);
    }

    #[test]
    fn test_quadrtic_flattening() {
        let curve = QuadraticBezierShape {
            points: [pos2(0.0, 0.0), pos2(80.0, 200.0), pos2(100.0, 30.0)],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(1.0, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 9);

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.5, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 11);

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.1, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 24);

        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 72);
        let mut result = vec![curve.points[0]]; //add the start point
        curve.for_each_flattened_with_t(0.001, &mut |pos, _t| {
            result.push(pos);
        });

        assert_eq!(result.len(), 223);
    }
}
