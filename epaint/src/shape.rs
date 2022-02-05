use crate::{
    text::{FontId, Fonts, Galley},
    Color32, Mesh, Stroke,
};
use crate::{CubicBezierShape, QuadraticBezierShape};
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
    pub fn rect_filled(
        rect: Rect,
        corner_radius: impl Into<Rounding>,
        fill_color: impl Into<Color32>,
    ) -> Self {
        Self::Rect(RectShape::filled(rect, corner_radius, fill_color))
    }

    #[inline]
    pub fn rect_stroke(
        rect: Rect,
        corner_radius: impl Into<Rounding>,
        stroke: impl Into<Stroke>,
    ) -> Self {
        Self::Rect(RectShape::stroke(rect, corner_radius, stroke))
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn text(
        fonts: &Fonts,
        pos: Pos2,
        anchor: Align2,
        text: impl ToString,
        font_id: FontId,
        color: Color32,
    ) -> Self {
        let galley = fonts.layout_no_wrap(text.to_string(), font_id, color);
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
    /// How rounded the corners are. Use `Rounding::none()` for no rounding.
    pub corner_radius: Rounding,
    pub fill: Color32,
    pub stroke: Stroke,
}

impl RectShape {
    #[inline]
    pub fn filled(
        rect: Rect,
        corner_radius: impl Into<Rounding>,
        fill_color: impl Into<Color32>,
    ) -> Self {
        Self {
            rect,
            corner_radius: corner_radius.into(),
            fill: fill_color.into(),
            stroke: Default::default(),
        }
    }

    #[inline]
    pub fn stroke(
        rect: Rect,
        corner_radius: impl Into<Rounding>,
        stroke: impl Into<Stroke>,
    ) -> Self {
        Self {
            rect,
            corner_radius: corner_radius.into(),
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

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
/// How rounded the corners of things should be
pub struct Rounding {
    pub nw: f32,
    pub ne: f32,
    pub sw: f32,
    pub se: f32,
}

impl Default for Rounding {
    #[inline]
    fn default() -> Self {
        Self::none()
    }
}

impl From<f32> for Rounding {
    #[inline]
    fn from(radius: f32) -> Self {
        Self {
            nw: radius,
            ne: radius,
            sw: radius,
            se: radius,
        }
    }
}

impl Rounding {
    #[inline]
    pub fn same(radius: f32) -> Self {
        Self {
            nw: radius,
            ne: radius,
            sw: radius,
            se: radius,
        }
    }

    #[inline]
    pub fn none() -> Self {
        Self {
            nw: 0.0,
            ne: 0.0,
            sw: 0.0,
            se: 0.0,
        }
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
