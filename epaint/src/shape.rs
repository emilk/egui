use std::ops::Range;

use crate::{
    text::{Fonts, Galley, TextStyle},
    Color32, Mesh, Stroke,
};
use emath::*;

/// A paint primitive such as a circle or a piece of text.
/// Coordinates are all screen space points (not physical pixels).
#[must_use = "Add a Shape to a Painter"]
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// Paint nothing. This can be useful as a placeholder.
    Noop,
    /// Recursively nest more shapes - sometimes a convenience to be able to do.
    /// For performance reasons it is better to avoid it.
    Vec(Vec<Shape>),
    Circle(CircleShape),
    /// A line between two points.
    LineSegment {
        points: [Pos2; 2],
        stroke: Stroke,
    },
    /// A series of lines between points.
    /// The path can have a stroke and/or fill (if closed).
    Path(PathShape),
    Rect(RectShape),
    Text(TextShape),
    Mesh(Mesh),
    // https://github.com/emilk/egui/issues/1120
    QuadraticBezier(QuadraticBezierShape),
    CubicBezier(CubicBezierShape),
}

/// ## Constructors
impl Shape {
    /// A line between two points.
    /// More efficient than calling [`Self::line`].
    #[inline]
    pub fn line_segment(points: [Pos2; 2], stroke: impl Into<Stroke>) -> Self {
        Self::LineSegment {
            points,
            stroke: stroke.into(),
        }
    }

    /// A line through many points.
    ///
    /// Use [`Self::line_segment`] instead if your line only connects two points.
    #[inline]
    pub fn line(points: Vec<Pos2>, stroke: impl Into<Stroke>) -> Self {
        Self::Path(PathShape::line(points, stroke))
    }

    /// A line that closes back to the start point again.
    #[inline]
    pub fn closed_line(points: Vec<Pos2>, stroke: impl Into<Stroke>) -> Self {
        Self::Path(PathShape::closed_line(points, stroke))
    }

    /// Turn a line into equally spaced dots.
    pub fn dotted_line(
        points: &[Pos2],
        color: impl Into<Color32>,
        spacing: f32,
        radius: f32,
    ) -> Vec<Self> {
        let mut shapes = Vec::new();
        points_from_line(points, spacing, radius, color.into(), &mut shapes);
        shapes
    }

    /// Turn a line into dashes.
    pub fn dashed_line(
        points: &[Pos2],
        stroke: impl Into<Stroke>,
        dash_length: f32,
        gap_length: f32,
    ) -> Vec<Self> {
        let mut shapes = Vec::new();
        dashes_from_line(points, stroke.into(), dash_length, gap_length, &mut shapes);
        shapes
    }

    /// Turn a line into dashes. If you need to create many dashed lines use this instead of
    /// [`Self::dashed_line`]
    pub fn dashed_line_many(
        points: &[Pos2],
        stroke: impl Into<Stroke>,
        dash_length: f32,
        gap_length: f32,
        shapes: &mut Vec<Shape>,
    ) {
        dashes_from_line(points, stroke.into(), dash_length, gap_length, shapes);
    }

    /// A convex polygon with a fill and optional stroke.
    #[inline]
    pub fn convex_polygon(
        points: Vec<Pos2>,
        fill: impl Into<Color32>,
        stroke: impl Into<Stroke>,
    ) -> Self {
        Self::Path(PathShape::convex_polygon(points, fill, stroke))
    }

    #[inline]
    pub fn circle_filled(center: Pos2, radius: f32, fill_color: impl Into<Color32>) -> Self {
        Self::Circle(CircleShape::filled(center, radius, fill_color))
    }

    #[inline]
    pub fn circle_stroke(center: Pos2, radius: f32, stroke: impl Into<Stroke>) -> Self {
        Self::Circle(CircleShape::stroke(center, radius, stroke))
    }

    #[inline]
    pub fn rect_filled(rect: Rect, corner_radius: f32, fill_color: impl Into<Color32>) -> Self {
        Self::Rect(RectShape::filled(rect, corner_radius, fill_color))
    }

    #[inline]
    pub fn rect_stroke(rect: Rect, corner_radius: f32, stroke: impl Into<Stroke>) -> Self {
        Self::Rect(RectShape::stroke(rect, corner_radius, stroke))
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn text(
        fonts: &Fonts,
        pos: Pos2,
        anchor: Align2,
        text: impl ToString,
        text_style: TextStyle,
        color: Color32,
    ) -> Self {
        let galley = fonts.layout_no_wrap(text.to_string(), text_style, color);
        let rect = anchor.anchor_rect(Rect::from_min_size(pos, galley.size()));
        Self::galley(rect.min, galley)
    }

    #[inline]
    pub fn galley(pos: Pos2, galley: crate::mutex::Arc<Galley>) -> Self {
        TextShape::new(pos, galley).into()
    }

    pub fn mesh(mesh: Mesh) -> Self {
        crate::epaint_assert!(mesh.is_valid());
        Self::Mesh(mesh)
    }
}

/// ## Inspection and transforms
impl Shape {
    #[inline(always)]
    pub fn texture_id(&self) -> super::TextureId {
        if let Shape::Mesh(mesh) = self {
            mesh.texture_id
        } else {
            super::TextureId::default()
        }
    }

    /// Move the shape by this many points, in-place.
    pub fn translate(&mut self, delta: Vec2) {
        match self {
            Shape::Noop => {}
            Shape::Vec(shapes) => {
                for shape in shapes {
                    shape.translate(delta);
                }
            }
            Shape::Circle(circle_shape) => {
                circle_shape.center += delta;
            }
            Shape::LineSegment { points, .. } => {
                for p in points {
                    *p += delta;
                }
            }
            Shape::Path(path_shape) => {
                for p in &mut path_shape.points {
                    *p += delta;
                }
            }
            Shape::Rect(rect_shape) => {
                rect_shape.rect = rect_shape.rect.translate(delta);
            }
            Shape::Text(text_shape) => {
                text_shape.pos += delta;
            }
            Shape::Mesh(mesh) => {
                mesh.translate(delta);
            }
            Shape::QuadraticBezier(bezier_shape) => {
                bezier_shape.points[0] += delta;
                bezier_shape.points[1] += delta;
                bezier_shape.points[2] += delta;
            }
            Shape::CubicBezier(cubie_curve) => {
                for p in &mut cubie_curve.points {
                    *p += delta;
                }
            }
        }
    }
}

// ----------------------------------------------------------------------------

/// How to paint a circle.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct CircleShape {
    pub center: Pos2,
    pub radius: f32,
    pub fill: Color32,
    pub stroke: Stroke,
}

impl CircleShape {
    #[inline]
    pub fn filled(center: Pos2, radius: f32, fill_color: impl Into<Color32>) -> Self {
        Self {
            center,
            radius,
            fill: fill_color.into(),
            stroke: Default::default(),
        }
    }

    #[inline]
    pub fn stroke(center: Pos2, radius: f32, stroke: impl Into<Stroke>) -> Self {
        Self {
            center,
            radius,
            fill: Default::default(),
            stroke: stroke.into(),
        }
    }
}

impl From<CircleShape> for Shape {
    #[inline(always)]
    fn from(shape: CircleShape) -> Self {
        Self::Circle(shape)
    }
}

// ----------------------------------------------------------------------------

/// A path which can be stroked and/or filled (if closed).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PathShape {
    pub points: Vec<Pos2>,
    /// If true, connect the first and last of the points together.
    /// This is required if `fill != TRANSPARENT`.
    pub closed: bool,
    /// Fill is only supported for convex polygons.
    pub fill: Color32,
    pub stroke: Stroke,
}

impl PathShape {
    /// A line through many points.
    ///
    /// Use [`Shape::line_segment`] instead if your line only connects two points.
    #[inline]
    pub fn line(points: Vec<Pos2>, stroke: impl Into<Stroke>) -> Self {
        PathShape {
            points,
            closed: false,
            fill: Default::default(),
            stroke: stroke.into(),
        }
    }

    /// A line that closes back to the start point again.
    #[inline]
    pub fn closed_line(points: Vec<Pos2>, stroke: impl Into<Stroke>) -> Self {
        PathShape {
            points,
            closed: true,
            fill: Default::default(),
            stroke: stroke.into(),
        }
    }

    /// A convex polygon with a fill and optional stroke.
    #[inline]
    pub fn convex_polygon(
        points: Vec<Pos2>,
        fill: impl Into<Color32>,
        stroke: impl Into<Stroke>,
    ) -> Self {
        PathShape {
            points,
            closed: true,
            fill: fill.into(),
            stroke: stroke.into(),
        }
    }

    /// Screen-space bounding rectangle.
    #[inline]
    pub fn bounding_rect(&self) -> Rect {
        Rect::from_points(&self.points).expand(self.stroke.width)
    }
}

impl From<PathShape> for Shape {
    #[inline(always)]
    fn from(shape: PathShape) -> Self {
        Self::Path(shape)
    }
}

// ----------------------------------------------------------------------------

/// How to paint a rectangle.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RectShape {
    pub rect: Rect,
    /// How rounded the corners are. Use `0.0` for no rounding.
    pub corner_radius: f32,
    pub fill: Color32,
    pub stroke: Stroke,
}

impl RectShape {
    #[inline]
    pub fn filled(rect: Rect, corner_radius: f32, fill_color: impl Into<Color32>) -> Self {
        Self {
            rect,
            corner_radius,
            fill: fill_color.into(),
            stroke: Default::default(),
        }
    }

    #[inline]
    pub fn stroke(rect: Rect, corner_radius: f32, stroke: impl Into<Stroke>) -> Self {
        Self {
            rect,
            corner_radius,
            fill: Default::default(),
            stroke: stroke.into(),
        }
    }

    /// Screen-space bounding rectangle.
    #[inline]
    pub fn bounding_rect(&self) -> Rect {
        self.rect.expand(self.stroke.width)
    }
}

impl From<RectShape> for Shape {
    #[inline(always)]
    fn from(shape: RectShape) -> Self {
        Self::Rect(shape)
    }
}

// ----------------------------------------------------------------------------

/// How to paint some text on screen.
#[derive(Clone, Debug, PartialEq)]
pub struct TextShape {
    /// Top left corner of the first character.
    pub pos: Pos2,

    /// The layed out text, from [`Fonts::layout_job`].
    pub galley: crate::mutex::Arc<Galley>,

    /// Add this underline to the whole text.
    /// You can also set an underline when creating the galley.
    pub underline: Stroke,

    /// If set, the text color in the galley will be ignored and replaced
    /// with the given color.
    /// This will NOT replace background color nor strikethrough/underline color.
    pub override_text_color: Option<Color32>,

    /// Rotate text by this many radians clock-wise.
    /// The pivot is `pos` (the upper left corner of the text).
    pub angle: f32,
}

impl TextShape {
    #[inline]
    pub fn new(pos: Pos2, galley: crate::mutex::Arc<Galley>) -> Self {
        Self {
            pos,
            galley,
            underline: Stroke::none(),
            override_text_color: None,
            angle: 0.0,
        }
    }

    /// Screen-space bounding rectangle.
    #[inline]
    pub fn bounding_rect(&self) -> Rect {
        self.galley.mesh_bounds.translate(self.pos.to_vec2())
    }
}

impl From<TextShape> for Shape {
    #[inline(always)]
    fn from(shape: TextShape) -> Self {
        Self::Text(shape)
    }
}

// ----------------------------------------------------------------------------

/// Creates equally spaced filled circles from a line.
fn points_from_line(
    line: &[Pos2],
    spacing: f32,
    radius: f32,
    color: Color32,
    shapes: &mut Vec<Shape>,
) {
    let mut position_on_segment = 0.0;
    line.windows(2).for_each(|window| {
        let start = window[0];
        let end = window[1];
        let vector = end - start;
        let segment_length = vector.length();
        while position_on_segment < segment_length {
            let new_point = start + vector * (position_on_segment / segment_length);
            shapes.push(Shape::circle_filled(new_point, radius, color));
            position_on_segment += spacing;
        }
        position_on_segment -= segment_length;
    });
}

/// Creates dashes from a line.
fn dashes_from_line(
    line: &[Pos2],
    stroke: Stroke,
    dash_length: f32,
    gap_length: f32,
    shapes: &mut Vec<Shape>,
) {
    let mut position_on_segment = 0.0;
    let mut drawing_dash = false;
    line.windows(2).for_each(|window| {
        let start = window[0];
        let end = window[1];
        let vector = end - start;
        let segment_length = vector.length();

        let mut start_point = start;
        while position_on_segment < segment_length {
            let new_point = start + vector * (position_on_segment / segment_length);
            if drawing_dash {
                // This is the end point.
                shapes.push(Shape::line_segment([start_point, new_point], stroke));
                position_on_segment += gap_length;
            } else {
                // Start a new dash.
                start_point = new_point;
                position_on_segment += dash_length;
            }
            drawing_dash = !drawing_dash;
        }

        // If the segment ends and the dash is not finished, add the segment's end point.
        if drawing_dash {
            shapes.push(Shape::line_segment([start_point, end], stroke));
        }

        position_on_segment -= segment_length;
    });
}

// ----------------------------------------------------------------------------

/// How to paint a cubic Bezier curve on screen.
/// The definition: https://en.wikipedia.org/wiki/B%C3%A9zier_curve
/// This implementation is only for cubic Bezier curve, or the Bezier curve of degree 3.
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
    /// Creates a cubic Bezier curve based on 4 points and stroke.
    /// The first point is the starting point and the last one is the ending point of the curve.
    /// The middle points are the control points.
    /// The number of points must be 4.
    pub fn from_points_stroke(points: Vec<Pos2>, stroke: impl Into<Stroke>) -> Self {
        crate::epaint_assert!(
            points.len() == 4,
            "Cubic needs 4 points"
        );
        Self {
            points:points.try_into().unwrap(),
            closed: false,
            fill: Default::default(),
            stroke: stroke.into(),
        }
    }
    
    /// Screen-space bounding rectangle.
    pub fn bounding_rect(&self) -> Rect {
        //temporary solution
        let (mut min_x,mut max_x) = if self.points[0].x < self.points[3].x {
            (self.points[0].x,self.points[3].x)}else{(self.points[3].x,self.points[0].x)};
        let (mut min_y,mut max_y) = if self.points[0].y < self.points[3].y {
            (self.points[0].y,self.points[3].y)}else{(self.points[3].y,self.points[0].y)};
        
        // find the inflection points and get the x value
        cubic_for_each_local_extremum(self.points[0].x,self.points[1].x,self.points[2].x,self.points[3].x,&mut |t|{
            let x = self.sample(t).x;
            if x < min_x {min_x = x}
            if x > max_x {max_x = x}
        });

        // find the inflection points and get the y value
        cubic_for_each_local_extremum(self.points[0].y,self.points[1].y,self.points[2].y,self.points[3].y,&mut |t|{
            let y = self.sample(t).y;
            if y < min_y {min_y = y}
            if y > max_y {max_y = y}
        });

        
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

    /// Calculate the point (x,y) at t based on the cubic bezier curve equation.
    /// t is in [0.0,1.0]
    /// https://en.wikipedia.org/wiki/B%C3%A9zier_curve#Cubic_B.C3.A9zier_curves
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

    /// find a set of points that approximate the cubic bezier curve.
    /// the number of points is determined by the tolerance.
    /// the points may not be evenly distributed in the range [0.0,1.0] (t value)
    pub fn flatten(&self, tolerance:Option<f32>)->Vec<Pos2>{
        let tolerance =tolerance.unwrap_or( (self.points[0].x-self.points[3].x).abs()*0.001);
        let mut result = Vec::new();
        result.push(self.points[0]);
        self.for_each_flattened_with_t(tolerance, &mut |p,_t|{
            result.push(p);
        });
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
    /// create a new quadratic bezier shape based on the 3 points and stroke.
    /// the first point is the starting point and the last one is the ending point of the curve.
    /// the middle point is the control points.
    /// the points should be in the order [start, control, end]
    /// 
    pub fn from_points_stroke(points: Vec<Pos2>, stroke: impl Into<Stroke>) -> Self {
        crate::epaint_assert!(
            points.len() == 3,
            "Quadratic needs 3 points"
        );

        QuadraticBezierShape {
            points: points.try_into().unwrap(), // it's safe to unwrap because we just checked
            closed: false,
            fill: Default::default(),
            stroke: stroke.into(),
        }
    }
    
    /// bounding box of the quadratic bezier shape
    pub fn bounding_rect(&self) -> Rect {
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

        quadratic_for_each_local_extremum(self.points[0].x, self.points[1].x, self.points[2].x, &mut |t|{
            let x = self.sample(t).x;
            if x < min_x {
                min_x = x;
            }
            if x > max_x {
                max_x = x;
            }
        });

        quadratic_for_each_local_extremum(self.points[0].y, self.points[1].y, self.points[2].y, &mut |t|{
            let y = self.sample(t).y;
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
        });
        
        Rect {
            min: Pos2 { x: min_x, y: min_y },
            max: Pos2 { x: max_x, y: max_y },
        }
    }

    /// Calculate the point (x,y) at t based on the quadratic bezier curve equation.
    /// t is in [0.0,1.0]
    /// https://en.wikipedia.org/wiki/B%C3%A9zier_curve#Quadratic_B.C3.A9zier_curves
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

    /// find a set of points that approximate the quadratic bezier curve.
    /// the number of points is determined by the tolerance.
    /// the points may not be evenly distributed in the range [0.0,1.0] (t value)
    pub fn flatten(&self, tolerance:Option<f32>)->Vec<Pos2>{
        let tolerance =tolerance.unwrap_or( (self.points[0].x-self.points[2].x).abs()*0.001);
        let mut result = Vec::new();
        self.for_each_flattened_with_t(tolerance, &mut |p,_t|{
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
            
            callback(self.sample(t),t);
        }

        callback(self.sample(1.0),1.0);
    }
}

impl From<QuadraticBezierShape> for Shape {
    #[inline(always)]
    fn from(shape: QuadraticBezierShape) -> Self {
        Self::QuadraticBezier(shape)
    }
}

// lyon_geom::flatten_cubic.rs
// copied from https://docs.rs/lyon_geom/latest/lyon_geom/
pub fn flatten_cubic_bezier_with_t<F: FnMut(Pos2,f32)>(
    curve: &CubicBezierShape,
    tolerance: f32,
    callback: &mut F,
) 
{
    // debug_assert!(tolerance >= S::EPSILON * S::EPSILON);
    let quadratics_tolerance = tolerance * 0.2;
    let flattening_tolerance = tolerance * 0.8;

    let num_quadratics = curve.num_quadratics( quadratics_tolerance);
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
        let scale =
            cross.abs() / ((ddx * ddx + ddy * ddy).sqrt() * (parabola_to - parabola_from).abs());

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
            is_point = ((to.x - from.x) * (to.x - from.x) + (to.y - from.y) * (to.y - from.y))
                .sqrt()
                < tolerance * tolerance;
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
        let t = (u - self.inv_integral_from) * self.div_inv_integral_diff;

        t
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

fn single_curve_approximation(curve:&CubicBezierShape) -> QuadraticBezierShape {
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

fn quadratic_for_each_local_extremum<F:FnMut(f32)>(p0:f32,p1:f32,p2:f32, cb:&mut F){
    // A quadratic bezier curve can be derived by a linear function:
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

fn cubic_for_each_local_extremum<F: FnMut(f32)>(p0:f32,p1:f32,p2:f32,p3:f32, cb:&mut F){
    // See www.faculty.idc.ac.il/arik/quality/appendixa.html for an explanation
    // A cubic bezier curve can be derivated by the following equation:
    // B'(t) = 3(1-t)^2(p1-p0) + 6(1-t)t(p2-p1) + 3t^2(p3-p2) or
    // f(x) = a * x² + b * x + c
    let a = 3.0 * (p3 + 3.0 * (p1-p2) - p0);
    let b = 6.0 * (p2 - 2.0 * p1 + p0);
    let c = 3.0 * (p1-p0);

    let in_range = |t:f32| t<=1.0 && t>=0.0;

    // linear situation
    if a == 0.0 {
        if b != 0.0 {
            let t =  - c / b;
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
    fn test_quadratic_bounding_box(){
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
        let bbox = curve.bounding_rect();
        assert!( (bbox.min.x-72.96).abs()<0.01);
        assert!( (bbox.min.y-27.78).abs()<0.01);
        
        assert!( (bbox.max.x-180.0).abs() < 0.01);
        assert!( (bbox.max.y-170.0).abs() < 0.01);

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.1, &mut |pos,_t| {
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
        let bbox = curve.bounding_rect();
        assert!( (bbox.min.x-10.0).abs()<0.01);
        assert!( (bbox.min.y-10.0).abs()<0.01);
        
        assert!( (bbox.max.x-130.42).abs() < 0.01);
        assert!( (bbox.max.y-170.0).abs() < 0.01);

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.1, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 25);
    }

    #[test]
    fn test_quadratic_dfferent_tolerance(){
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
        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(1.0, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 9);

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.1, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 25);

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 77);

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.001, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 240);
    }
    #[test]
    fn test_cubic_bounding_box(){
        let curve = CubicBezierShape {
            points: [
                Pos2::new(10.0, 10.0),
                Pos2::new(110.0, 170.0),
                Pos2::new(180.0, 30.0),
                Pos2::new(270.0, 210.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let bbox = curve.bounding_rect();
        assert_eq!(bbox.min.x, 10.0);
        assert_eq!(bbox.min.y, 10.0);
        assert_eq!(bbox.max.x, 270.0);
        assert_eq!(bbox.max.y, 210.0);

        let curve = CubicBezierShape {
            points: [
                Pos2::new(10.0, 10.0),
                Pos2::new(110.0, 170.0),
                Pos2::new(270.0, 210.0),
                Pos2::new(180.0, 30.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let bbox = curve.bounding_rect();
        assert_eq!(bbox.min.x, 10.0);
        assert_eq!(bbox.min.y, 10.0);
        assert!( (bbox.max.x-206.50).abs() < 0.01);
        assert!( (bbox.max.y-148.48).abs() < 0.01);
        
        let curve = CubicBezierShape {
            points: [
                Pos2::new(110.0, 170.0),
                Pos2::new(10.0, 10.0),
                Pos2::new(270.0, 210.0),
                Pos2::new(180.0, 30.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let bbox = curve.bounding_rect();
        assert!( (bbox.min.x-86.71).abs()<0.01);
        assert!( (bbox.min.y-30.0).abs()<0.01);
        
        assert!( (bbox.max.x-199.27).abs() < 0.01);
        assert!( (bbox.max.y-170.0).abs() < 0.01);
    }
    #[test]
    fn test_cubic_different_tolerance_flattening() {
        let curve = CubicBezierShape {
            points: [
                Pos2::new(0.0, 0.0),
                Pos2::new(100.0, 0.0),
                Pos2::new(100.0, 100.0),
                Pos2::new(100.0, 200.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(1.0, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 10);

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.5, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 13);

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.1, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 28);

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 83);

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.001, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 248);
    }

    #[test]
    fn test_cubic_different_shape_flattening() {
        let curve = CubicBezierShape {
            points: [
                Pos2::new(90.0, 110.0),
                Pos2::new(30.0, 170.0),
                Pos2::new(210.0, 170.0),
                Pos2::new(170.0, 110.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 117);

        let curve = CubicBezierShape {
            points: [
                Pos2::new(90.0, 110.0),
                Pos2::new(90.0, 170.0),
                Pos2::new(170.0, 170.0),
                Pos2::new(170.0, 110.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 91);

        let curve = CubicBezierShape {
            points: [
                Pos2::new(90.0, 110.0),
                Pos2::new(110.0, 170.0),
                Pos2::new(150.0, 170.0),
                Pos2::new(170.0, 110.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 75);

        let curve = CubicBezierShape {
            points: [
                Pos2::new(90.0, 110.0),
                Pos2::new(110.0, 170.0),
                Pos2::new(230.0, 110.0),
                Pos2::new(170.0, 110.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 100);

        let curve = CubicBezierShape {
            points: [
                Pos2::new(90.0, 110.0),
                Pos2::new(110.0, 170.0),
                Pos2::new(210.0, 70.0),
                Pos2::new(170.0, 110.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 71);

        let curve = CubicBezierShape {
            points: [
                Pos2::new(90.0, 110.0),
                Pos2::new(110.0, 170.0),
                Pos2::new(150.0, 50.0),
                Pos2::new(170.0, 110.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 88);
    }

    #[test]
    fn test_quadrtic_flattening() {
        let curve = QuadraticBezierShape {
            points: [
                Pos2::new(0.0, 0.0),
                Pos2::new(80.0, 200.0),
                Pos2::new(100.0, 30.0),
            ],
            closed: false,
            fill: Default::default(),
            stroke: Default::default(),
        };

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(1.0, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 9);

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.5, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 11);

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.1, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 24);

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.01, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 72);

        let mut result = Vec::new();
        result.push(curve.points[0]); //add the start point
        curve.for_each_flattened_with_t(0.001, &mut |pos,_t| {
            result.push(pos);
        });
        
        assert_eq!(result.len(), 223);
    }
}