//! The different shapes that can be painted.

use std::{any::Any, sync::Arc};

use crate::{
    text::{FontId, Fonts, Galley},
    Color32, Mesh, Stroke, TextureId,
};
use emath::*;

pub use crate::{CubicBezierShape, QuadraticBezierShape};

/// A paint primitive such as a circle or a piece of text.
/// Coordinates are all screen space points (not physical pixels).
///
/// You should generally recreate your [`Shape`]s each frame,
/// but storing them should also be fine with one exception:
/// [`Shape::Text`] depends on the current `pixels_per_point` (dpi scale)
/// and so must be recreated every time `pixels_per_point` changes.
#[must_use = "Add a Shape to a Painter"]
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// Paint nothing. This can be useful as a placeholder.
    Noop,

    /// Recursively nest more shapes - sometimes a convenience to be able to do.
    /// For performance reasons it is better to avoid it.
    Vec(Vec<Shape>),

    /// Circle with optional outline and fill.
    Circle(CircleShape),

    /// Ellipse with optional outline and fill.
    Ellipse(EllipseShape),

    /// A line between two points.
    LineSegment { points: [Pos2; 2], stroke: Stroke },

    /// A series of lines between points.
    /// The path can have a stroke and/or fill (if closed).
    Path(PathShape),

    /// Rectangle with optional outline and fill.
    Rect(RectShape),

    /// Text.
    ///
    /// This needs to be recreated if `pixels_per_point` (dpi scale) changes.
    Text(TextShape),

    /// A general triangle mesh.
    ///
    /// Can be used to display images.
    Mesh(Mesh),

    /// A quadratic [Bézier Curve](https://en.wikipedia.org/wiki/B%C3%A9zier_curve).
    QuadraticBezier(QuadraticBezierShape),

    /// A cubic [Bézier Curve](https://en.wikipedia.org/wiki/B%C3%A9zier_curve).
    CubicBezier(CubicBezierShape),

    /// Backend-specific painting.
    Callback(PaintCallback),
}

#[test]
fn shape_impl_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Shape>();
}

impl From<Vec<Self>> for Shape {
    #[inline(always)]
    fn from(shapes: Vec<Self>) -> Self {
        Self::Vec(shapes)
    }
}

impl From<Mesh> for Shape {
    #[inline(always)]
    fn from(mesh: Mesh) -> Self {
        Self::Mesh(mesh)
    }
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

    /// A horizontal line.
    pub fn hline(x: impl Into<Rangef>, y: f32, stroke: impl Into<Stroke>) -> Self {
        let x = x.into();
        Self::LineSegment {
            points: [pos2(x.min, y), pos2(x.max, y)],
            stroke: stroke.into(),
        }
    }

    /// A vertical line.
    pub fn vline(x: f32, y: impl Into<Rangef>, stroke: impl Into<Stroke>) -> Self {
        let y = y.into();
        Self::LineSegment {
            points: [pos2(x, y.min), pos2(x, y.max)],
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
        path: &[Pos2],
        color: impl Into<Color32>,
        spacing: f32,
        radius: f32,
    ) -> Vec<Self> {
        let mut shapes = Vec::new();
        points_from_line(path, spacing, radius, color.into(), &mut shapes);
        shapes
    }

    /// Turn a line into dashes.
    pub fn dashed_line(
        path: &[Pos2],
        stroke: impl Into<Stroke>,
        dash_length: f32,
        gap_length: f32,
    ) -> Vec<Self> {
        let mut shapes = Vec::new();
        dashes_from_line(
            path,
            stroke.into(),
            &[dash_length],
            &[gap_length],
            &mut shapes,
            0.,
        );
        shapes
    }

    /// Turn a line into dashes with different dash/gap lengths and a start offset.
    pub fn dashed_line_with_offset(
        path: &[Pos2],
        stroke: impl Into<Stroke>,
        dash_lengths: &[f32],
        gap_lengths: &[f32],
        dash_offset: f32,
    ) -> Vec<Self> {
        let mut shapes = Vec::new();
        dashes_from_line(
            path,
            stroke.into(),
            dash_lengths,
            gap_lengths,
            &mut shapes,
            dash_offset,
        );
        shapes
    }

    /// Turn a line into dashes. If you need to create many dashed lines use this instead of
    /// [`Self::dashed_line`].
    pub fn dashed_line_many(
        points: &[Pos2],
        stroke: impl Into<Stroke>,
        dash_length: f32,
        gap_length: f32,
        shapes: &mut Vec<Self>,
    ) {
        dashes_from_line(
            points,
            stroke.into(),
            &[dash_length],
            &[gap_length],
            shapes,
            0.,
        );
    }

    /// Turn a line into dashes with different dash/gap lengths and a start offset. If you need to
    /// create many dashed lines use this instead of [`Self::dashed_line_with_offset`].
    pub fn dashed_line_many_with_offset(
        points: &[Pos2],
        stroke: impl Into<Stroke>,
        dash_lengths: &[f32],
        gap_lengths: &[f32],
        dash_offset: f32,
        shapes: &mut Vec<Self>,
    ) {
        dashes_from_line(
            points,
            stroke.into(),
            dash_lengths,
            gap_lengths,
            shapes,
            dash_offset,
        );
    }

    /// A convex polygon with a fill and optional stroke.
    ///
    /// The most performant winding order is clockwise.
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
    pub fn ellipse_filled(center: Pos2, radius: Vec2, fill_color: impl Into<Color32>) -> Self {
        Self::Ellipse(EllipseShape::filled(center, radius, fill_color))
    }

    #[inline]
    pub fn ellipse_stroke(center: Pos2, radius: Vec2, stroke: impl Into<Stroke>) -> Self {
        Self::Ellipse(EllipseShape::stroke(center, radius, stroke))
    }

    #[inline]
    pub fn rect_filled(
        rect: Rect,
        rounding: impl Into<Rounding>,
        fill_color: impl Into<Color32>,
    ) -> Self {
        Self::Rect(RectShape::filled(rect, rounding, fill_color))
    }

    #[inline]
    pub fn rect_stroke(
        rect: Rect,
        rounding: impl Into<Rounding>,
        stroke: impl Into<Stroke>,
    ) -> Self {
        Self::Rect(RectShape::stroke(rect, rounding, stroke))
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
        let rect = anchor.anchor_size(pos, galley.size());
        Self::galley(rect.min, galley, color)
    }

    /// Any uncolored parts of the [`Galley`] (using [`Color32::PLACEHOLDER`]) will be replaced with the given color.
    ///
    /// Any non-placeholder color in the galley takes precedence over this fallback color.
    #[inline]
    pub fn galley(pos: Pos2, galley: Arc<Galley>, fallback_color: Color32) -> Self {
        TextShape::new(pos, galley, fallback_color).into()
    }

    /// All text color in the [`Galley`] will be replaced with the given color.
    #[inline]
    pub fn galley_with_override_text_color(
        pos: Pos2,
        galley: Arc<Galley>,
        text_color: Color32,
    ) -> Self {
        TextShape::new(pos, galley, text_color)
            .with_override_text_color(text_color)
            .into()
    }

    #[inline]
    #[deprecated = "Use `Shape::galley` or `Shape::galley_with_override_text_color` instead"]
    pub fn galley_with_color(pos: Pos2, galley: Arc<Galley>, text_color: Color32) -> Self {
        Self::galley_with_override_text_color(pos, galley, text_color)
    }

    #[inline]
    pub fn mesh(mesh: Mesh) -> Self {
        crate::epaint_assert!(mesh.is_valid());
        Self::Mesh(mesh)
    }

    /// An image at the given position.
    ///
    /// `uv` should normally be `Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))`
    /// unless you want to crop or flip the image.
    ///
    /// `tint` is a color multiplier. Use [`Color32::WHITE`] if you don't want to tint the image.
    pub fn image(texture_id: TextureId, rect: Rect, uv: Rect, tint: Color32) -> Self {
        let mut mesh = Mesh::with_texture(texture_id);
        mesh.add_rect_with_uv(rect, uv, tint);
        Self::mesh(mesh)
    }

    /// The visual bounding rectangle (includes stroke widths)
    pub fn visual_bounding_rect(&self) -> Rect {
        match self {
            Self::Noop => Rect::NOTHING,
            Self::Vec(shapes) => {
                let mut rect = Rect::NOTHING;
                for shape in shapes {
                    rect = rect.union(shape.visual_bounding_rect());
                }
                rect
            }
            Self::Circle(circle_shape) => circle_shape.visual_bounding_rect(),
            Self::Ellipse(ellipse_shape) => ellipse_shape.visual_bounding_rect(),
            Self::LineSegment { points, stroke } => {
                if stroke.is_empty() {
                    Rect::NOTHING
                } else {
                    Rect::from_two_pos(points[0], points[1]).expand(stroke.width / 2.0)
                }
            }
            Self::Path(path_shape) => path_shape.visual_bounding_rect(),
            Self::Rect(rect_shape) => rect_shape.visual_bounding_rect(),
            Self::Text(text_shape) => text_shape.visual_bounding_rect(),
            Self::Mesh(mesh) => mesh.calc_bounds(),
            Self::QuadraticBezier(bezier) => bezier.visual_bounding_rect(),
            Self::CubicBezier(bezier) => bezier.visual_bounding_rect(),
            Self::Callback(custom) => custom.rect,
        }
    }
}

/// ## Inspection and transforms
impl Shape {
    #[inline(always)]
    pub fn texture_id(&self) -> super::TextureId {
        if let Self::Mesh(mesh) = self {
            mesh.texture_id
        } else if let Self::Rect(rect_shape) = self {
            rect_shape.fill_texture_id
        } else {
            super::TextureId::default()
        }
    }

    /// Scale the shape by `factor`, in-place.
    ///
    /// A wrapper around [`Self::transform`].
    #[inline(always)]
    pub fn scale(&mut self, factor: f32) {
        self.transform(TSTransform::from_scaling(factor));
    }

    /// Move the shape by `delta`, in-place.
    ///
    /// A wrapper around [`Self::transform`].
    #[inline(always)]
    pub fn translate(&mut self, delta: Vec2) {
        self.transform(TSTransform::from_translation(delta));
    }

    /// Move the shape by this many points, in-place.
    ///
    /// If using a [`PaintCallback`], note that only the rect is scaled as opposed
    /// to other shapes where the stroke is also scaled.
    pub fn transform(&mut self, transform: TSTransform) {
        match self {
            Self::Noop => {}
            Self::Vec(shapes) => {
                for shape in shapes {
                    shape.transform(transform);
                }
            }
            Self::Circle(circle_shape) => {
                circle_shape.center = transform * circle_shape.center;
                circle_shape.radius *= transform.scaling;
                circle_shape.stroke.width *= transform.scaling;
            }
            Self::Ellipse(ellipse_shape) => {
                ellipse_shape.center = transform * ellipse_shape.center;
                ellipse_shape.radius *= transform.scaling;
                ellipse_shape.stroke.width *= transform.scaling;
            }
            Self::LineSegment { points, stroke } => {
                for p in points {
                    *p = transform * *p;
                }
                stroke.width *= transform.scaling;
            }
            Self::Path(path_shape) => {
                for p in &mut path_shape.points {
                    *p = transform * *p;
                }
                path_shape.stroke.width *= transform.scaling;
            }
            Self::Rect(rect_shape) => {
                rect_shape.rect = transform * rect_shape.rect;
                rect_shape.stroke.width *= transform.scaling;
                rect_shape.rounding *= transform.scaling;
            }
            Self::Text(text_shape) => {
                text_shape.pos = transform * text_shape.pos;

                // Scale text:
                let galley = Arc::make_mut(&mut text_shape.galley);
                for row in &mut galley.rows {
                    row.visuals.mesh_bounds = transform.scaling * row.visuals.mesh_bounds;
                    for v in &mut row.visuals.mesh.vertices {
                        v.pos = Pos2::new(transform.scaling * v.pos.x, transform.scaling * v.pos.y);
                    }
                }

                galley.mesh_bounds = transform.scaling * galley.mesh_bounds;
                galley.rect = transform.scaling * galley.rect;
            }
            Self::Mesh(mesh) => {
                mesh.transform(transform);
            }
            Self::QuadraticBezier(bezier_shape) => {
                bezier_shape.points[0] = transform * bezier_shape.points[0];
                bezier_shape.points[1] = transform * bezier_shape.points[1];
                bezier_shape.points[2] = transform * bezier_shape.points[2];
                bezier_shape.stroke.width *= transform.scaling;
            }
            Self::CubicBezier(cubic_curve) => {
                for p in &mut cubic_curve.points {
                    *p = transform * *p;
                }
                cubic_curve.stroke.width *= transform.scaling;
            }
            Self::Callback(shape) => {
                shape.rect = transform * shape.rect;
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

    /// The visual bounding rectangle (includes stroke width)
    pub fn visual_bounding_rect(&self) -> Rect {
        if self.fill == Color32::TRANSPARENT && self.stroke.is_empty() {
            Rect::NOTHING
        } else {
            Rect::from_center_size(
                self.center,
                Vec2::splat(self.radius * 2.0 + self.stroke.width),
            )
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

/// How to paint an ellipse.
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct EllipseShape {
    pub center: Pos2,

    /// Radius is the vector (a, b) where the width of the Ellipse is 2a and the height is 2b
    pub radius: Vec2,
    pub fill: Color32,
    pub stroke: Stroke,
}

impl EllipseShape {
    #[inline]
    pub fn filled(center: Pos2, radius: Vec2, fill_color: impl Into<Color32>) -> Self {
        Self {
            center,
            radius,
            fill: fill_color.into(),
            stroke: Default::default(),
        }
    }

    #[inline]
    pub fn stroke(center: Pos2, radius: Vec2, stroke: impl Into<Stroke>) -> Self {
        Self {
            center,
            radius,
            fill: Default::default(),
            stroke: stroke.into(),
        }
    }

    /// The visual bounding rectangle (includes stroke width)
    pub fn visual_bounding_rect(&self) -> Rect {
        if self.fill == Color32::TRANSPARENT && self.stroke.is_empty() {
            Rect::NOTHING
        } else {
            Rect::from_center_size(
                self.center,
                self.radius * 2.0 + Vec2::splat(self.stroke.width),
            )
        }
    }
}

impl From<EllipseShape> for Shape {
    #[inline(always)]
    fn from(shape: EllipseShape) -> Self {
        Self::Ellipse(shape)
    }
}

// ----------------------------------------------------------------------------

/// A path which can be stroked and/or filled (if closed).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PathShape {
    /// Filled paths should prefer clockwise order.
    pub points: Vec<Pos2>,

    /// If true, connect the first and last of the points together.
    /// This is required if `fill != TRANSPARENT`.
    pub closed: bool,

    /// Fill is only supported for convex polygons.
    pub fill: Color32,

    /// Color and thickness of the line.
    pub stroke: Stroke,
    // TODO(emilk): Add texture support either by supplying uv for each point,
    // or by some transform from points to uv (e.g. a callback or a linear transform matrix).
}

impl PathShape {
    /// A line through many points.
    ///
    /// Use [`Shape::line_segment`] instead if your line only connects two points.
    #[inline]
    pub fn line(points: Vec<Pos2>, stroke: impl Into<Stroke>) -> Self {
        Self {
            points,
            closed: false,
            fill: Default::default(),
            stroke: stroke.into(),
        }
    }

    /// A line that closes back to the start point again.
    #[inline]
    pub fn closed_line(points: Vec<Pos2>, stroke: impl Into<Stroke>) -> Self {
        Self {
            points,
            closed: true,
            fill: Default::default(),
            stroke: stroke.into(),
        }
    }

    /// A convex polygon with a fill and optional stroke.
    ///
    /// The most performant winding order is clockwise.
    #[inline]
    pub fn convex_polygon(
        points: Vec<Pos2>,
        fill: impl Into<Color32>,
        stroke: impl Into<Stroke>,
    ) -> Self {
        Self {
            points,
            closed: true,
            fill: fill.into(),
            stroke: stroke.into(),
        }
    }

    /// The visual bounding rectangle (includes stroke width)
    #[inline]
    pub fn visual_bounding_rect(&self) -> Rect {
        if self.fill == Color32::TRANSPARENT && self.stroke.is_empty() {
            Rect::NOTHING
        } else {
            Rect::from_points(&self.points).expand(self.stroke.width / 2.0)
        }
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

    /// How rounded the corners are. Use `Rounding::ZERO` for no rounding.
    pub rounding: Rounding,

    /// How to fill the rectangle.
    pub fill: Color32,

    /// The thickness and color of the outline.
    pub stroke: Stroke,

    /// If the rect should be filled with a texture, which one?
    ///
    /// The texture is multiplied with [`Self::fill`].
    pub fill_texture_id: TextureId,

    /// What UV coordinates to use for the texture?
    ///
    /// To display a texture, set [`Self::fill_texture_id`],
    /// and set this to `Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))`.
    ///
    /// Use [`Rect::ZERO`] to turn off texturing.
    pub uv: Rect,
}

impl RectShape {
    #[inline]
    pub fn new(
        rect: Rect,
        rounding: impl Into<Rounding>,
        fill_color: impl Into<Color32>,
        stroke: impl Into<Stroke>,
    ) -> Self {
        Self {
            rect,
            rounding: rounding.into(),
            fill: fill_color.into(),
            stroke: stroke.into(),
            fill_texture_id: Default::default(),
            uv: Rect::ZERO,
        }
    }

    #[inline]
    pub fn filled(
        rect: Rect,
        rounding: impl Into<Rounding>,
        fill_color: impl Into<Color32>,
    ) -> Self {
        Self {
            rect,
            rounding: rounding.into(),
            fill: fill_color.into(),
            stroke: Default::default(),
            fill_texture_id: Default::default(),
            uv: Rect::ZERO,
        }
    }

    #[inline]
    pub fn stroke(rect: Rect, rounding: impl Into<Rounding>, stroke: impl Into<Stroke>) -> Self {
        Self {
            rect,
            rounding: rounding.into(),
            fill: Default::default(),
            stroke: stroke.into(),
            fill_texture_id: Default::default(),
            uv: Rect::ZERO,
        }
    }

    /// The visual bounding rectangle (includes stroke width)
    #[inline]
    pub fn visual_bounding_rect(&self) -> Rect {
        if self.fill == Color32::TRANSPARENT && self.stroke.is_empty() {
            Rect::NOTHING
        } else {
            self.rect.expand(self.stroke.width / 2.0)
        }
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
    /// Radius of the rounding of the North-West (left top) corner.
    pub nw: f32,

    /// Radius of the rounding of the North-East (right top) corner.
    pub ne: f32,

    /// Radius of the rounding of the South-West (left bottom) corner.
    pub sw: f32,

    /// Radius of the rounding of the South-East (right bottom) corner.
    pub se: f32,
}

impl Default for Rounding {
    #[inline]
    fn default() -> Self {
        Self::ZERO
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
    /// No rounding on any corner.
    pub const ZERO: Self = Self {
        nw: 0.0,
        ne: 0.0,
        sw: 0.0,
        se: 0.0,
    };

    #[inline]
    pub const fn same(radius: f32) -> Self {
        Self {
            nw: radius,
            ne: radius,
            sw: radius,
            se: radius,
        }
    }

    /// Do all corners have the same rounding?
    #[inline]
    pub fn is_same(&self) -> bool {
        self.nw == self.ne && self.nw == self.sw && self.nw == self.se
    }

    /// Make sure each corner has a rounding of at least this.
    #[inline]
    pub fn at_least(&self, min: f32) -> Self {
        Self {
            nw: self.nw.max(min),
            ne: self.ne.max(min),
            sw: self.sw.max(min),
            se: self.se.max(min),
        }
    }

    /// Make sure each corner has a rounding of at most this.
    #[inline]
    pub fn at_most(&self, max: f32) -> Self {
        Self {
            nw: self.nw.min(max),
            ne: self.ne.min(max),
            sw: self.sw.min(max),
            se: self.se.min(max),
        }
    }
}

impl std::ops::Add for Rounding {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            nw: self.nw + rhs.nw,
            ne: self.ne + rhs.ne,
            sw: self.sw + rhs.sw,
            se: self.se + rhs.se,
        }
    }
}

impl std::ops::AddAssign for Rounding {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = Self {
            nw: self.nw + rhs.nw,
            ne: self.ne + rhs.ne,
            sw: self.sw + rhs.sw,
            se: self.se + rhs.se,
        };
    }
}

impl std::ops::AddAssign<f32> for Rounding {
    #[inline]
    fn add_assign(&mut self, rhs: f32) {
        *self = Self {
            nw: self.nw + rhs,
            ne: self.ne + rhs,
            sw: self.sw + rhs,
            se: self.se + rhs,
        };
    }
}

impl std::ops::Sub for Rounding {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            nw: self.nw - rhs.nw,
            ne: self.ne - rhs.ne,
            sw: self.sw - rhs.sw,
            se: self.se - rhs.se,
        }
    }
}

impl std::ops::SubAssign for Rounding {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = Self {
            nw: self.nw - rhs.nw,
            ne: self.ne - rhs.ne,
            sw: self.sw - rhs.sw,
            se: self.se - rhs.se,
        };
    }
}

impl std::ops::SubAssign<f32> for Rounding {
    #[inline]
    fn sub_assign(&mut self, rhs: f32) {
        *self = Self {
            nw: self.nw - rhs,
            ne: self.ne - rhs,
            sw: self.sw - rhs,
            se: self.se - rhs,
        };
    }
}

impl std::ops::Div<f32> for Rounding {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self {
        Self {
            nw: self.nw / rhs,
            ne: self.ne / rhs,
            sw: self.sw / rhs,
            se: self.se / rhs,
        }
    }
}

impl std::ops::DivAssign<f32> for Rounding {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        *self = Self {
            nw: self.nw / rhs,
            ne: self.ne / rhs,
            sw: self.sw / rhs,
            se: self.se / rhs,
        };
    }
}

impl std::ops::Mul<f32> for Rounding {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self {
            nw: self.nw * rhs,
            ne: self.ne * rhs,
            sw: self.sw * rhs,
            se: self.se * rhs,
        }
    }
}

impl std::ops::MulAssign<f32> for Rounding {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        *self = Self {
            nw: self.nw * rhs,
            ne: self.ne * rhs,
            sw: self.sw * rhs,
            se: self.se * rhs,
        };
    }
}

// ----------------------------------------------------------------------------

/// How to paint some text on screen.
///
/// This needs to be recreated if `pixels_per_point` (dpi scale) changes.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TextShape {
    /// Top left corner of the first character.
    pub pos: Pos2,

    /// The laid out text, from [`Fonts::layout_job`].
    pub galley: Arc<Galley>,

    /// Add this underline to the whole text.
    /// You can also set an underline when creating the galley.
    pub underline: Stroke,

    /// Any [`Color32::PLACEHOLDER`] in the galley will be replaced by the given color.
    /// Affects everything: backgrounds, glyphs, strikethough, underline, etc.
    pub fallback_color: Color32,

    /// If set, the text color in the galley will be ignored and replaced
    /// with the given color.
    ///
    /// This only affects the glyphs and will NOT replace background color nor strikethrough/underline color.
    pub override_text_color: Option<Color32>,

    /// If set, the text will be rendered with the given opacity in gamma space
    /// Affects everything: backgrounds, glyphs, strikethough, underline, etc.
    pub opacity_factor: f32,

    /// Rotate text by this many radians clockwise.
    /// The pivot is `pos` (the upper left corner of the text).
    pub angle: f32,
}

impl TextShape {
    /// The given fallback color will be used for any uncolored part of the galley (using [`Color32::PLACEHOLDER`]).
    ///
    /// Any non-placeholder color in the galley takes precedence over this fallback color.
    #[inline]
    pub fn new(pos: Pos2, galley: Arc<Galley>, fallback_color: Color32) -> Self {
        Self {
            pos,
            galley,
            underline: Stroke::NONE,
            fallback_color,
            override_text_color: None,
            opacity_factor: 1.0,
            angle: 0.0,
        }
    }

    /// The visual bounding rectangle
    #[inline]
    pub fn visual_bounding_rect(&self) -> Rect {
        self.galley.mesh_bounds.translate(self.pos.to_vec2())
    }

    #[inline]
    pub fn with_underline(mut self, underline: Stroke) -> Self {
        self.underline = underline;
        self
    }

    /// Use the given color for the text, regardless of what color is already in the galley.
    #[inline]
    pub fn with_override_text_color(mut self, override_text_color: Color32) -> Self {
        self.override_text_color = Some(override_text_color);
        self
    }

    /// Rotate text by this many radians clockwise.
    /// The pivot is `pos` (the upper left corner of the text).
    #[inline]
    pub fn with_angle(mut self, angle: f32) -> Self {
        self.angle = angle;
        self
    }

    /// Render text with this opacity in gamma space
    #[inline]
    pub fn with_opacity_factor(mut self, opacity_factor: f32) -> Self {
        self.opacity_factor = opacity_factor;
        self
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
    path: &[Pos2],
    spacing: f32,
    radius: f32,
    color: Color32,
    shapes: &mut Vec<Shape>,
) {
    let mut position_on_segment = 0.0;
    path.windows(2).for_each(|window| {
        let (start, end) = (window[0], window[1]);
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
    path: &[Pos2],
    stroke: Stroke,
    dash_lengths: &[f32],
    gap_lengths: &[f32],
    shapes: &mut Vec<Shape>,
    dash_offset: f32,
) {
    assert_eq!(dash_lengths.len(), gap_lengths.len());
    let mut position_on_segment = dash_offset;
    let mut drawing_dash = false;
    let mut step = 0;
    let steps = dash_lengths.len();
    path.windows(2).for_each(|window| {
        let (start, end) = (window[0], window[1]);
        let vector = end - start;
        let segment_length = vector.length();

        let mut start_point = start;
        while position_on_segment < segment_length {
            let new_point = start + vector * (position_on_segment / segment_length);
            if drawing_dash {
                // This is the end point.
                shapes.push(Shape::line_segment([start_point, new_point], stroke));
                position_on_segment += gap_lengths[step];
                // Increment step counter
                step += 1;
                if step >= steps {
                    step = 0;
                }
            } else {
                // Start a new dash.
                start_point = new_point;
                position_on_segment += dash_lengths[step];
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

/// Information passed along with [`PaintCallback`] ([`Shape::Callback`]).
pub struct PaintCallbackInfo {
    /// Viewport in points.
    ///
    /// This specifies where on the screen to paint, and the borders of this
    /// Rect is the [-1, +1] of the Normalized Device Coordinates.
    ///
    /// Note than only a portion of this may be visible due to [`Self::clip_rect`].
    ///
    /// This comes from [`PaintCallback::rect`].
    pub viewport: Rect,

    /// Clip rectangle in points.
    pub clip_rect: Rect,

    /// Pixels per point.
    pub pixels_per_point: f32,

    /// Full size of the screen, in pixels.
    pub screen_size_px: [u32; 2],
}

/// Size of the viewport in whole, physical pixels.
pub struct ViewportInPixels {
    /// Physical pixel offset for left side of the viewport.
    pub left_px: i32,

    /// Physical pixel offset for top side of the viewport.
    pub top_px: i32,

    /// Physical pixel offset for bottom side of the viewport.
    ///
    /// This is what `glViewport`, `glScissor` etc expects for the y axis.
    pub from_bottom_px: i32,

    /// Viewport width in physical pixels.
    pub width_px: i32,

    /// Viewport height in physical pixels.
    pub height_px: i32,
}

impl ViewportInPixels {
    fn from_points(rect: &Rect, pixels_per_point: f32, screen_size_px: [u32; 2]) -> Self {
        // Fractional pixel values for viewports are generally valid, but may cause sampling issues
        // and rounding errors might cause us to get out of bounds.

        // Round:
        let left_px = (pixels_per_point * rect.min.x).round() as i32; // inclusive
        let top_px = (pixels_per_point * rect.min.y).round() as i32; // inclusive
        let right_px = (pixels_per_point * rect.max.x).round() as i32; // exclusive
        let bottom_px = (pixels_per_point * rect.max.y).round() as i32; // exclusive

        // Clamp to screen:
        let screen_width = screen_size_px[0] as i32;
        let screen_height = screen_size_px[1] as i32;
        let left_px = left_px.clamp(0, screen_width);
        let right_px = right_px.clamp(left_px, screen_width);
        let top_px = top_px.clamp(0, screen_height);
        let bottom_px = bottom_px.clamp(top_px, screen_height);

        let width_px = right_px - left_px;
        let height_px = bottom_px - top_px;

        Self {
            left_px,
            top_px,
            from_bottom_px: screen_height - height_px - top_px,
            width_px,
            height_px,
        }
    }
}

#[test]
fn test_viewport_rounding() {
    for i in 0..=10_000 {
        // Two adjacent viewports should never overlap:
        let x = i as f32 / 97.0;
        let left = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0)).with_max_x(x);
        let right = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0)).with_min_x(x);

        for pixels_per_point in [0.618, 1.0, std::f32::consts::PI] {
            let left = ViewportInPixels::from_points(&left, pixels_per_point, [100, 100]);
            let right = ViewportInPixels::from_points(&right, pixels_per_point, [100, 100]);
            assert_eq!(left.left_px + left.width_px, right.left_px);
        }
    }
}

impl PaintCallbackInfo {
    /// The viewport rectangle. This is what you would use in e.g. `glViewport`.
    pub fn viewport_in_pixels(&self) -> ViewportInPixels {
        ViewportInPixels::from_points(&self.viewport, self.pixels_per_point, self.screen_size_px)
    }

    /// The "scissor" or "clip" rectangle. This is what you would use in e.g. `glScissor`.
    pub fn clip_rect_in_pixels(&self) -> ViewportInPixels {
        ViewportInPixels::from_points(&self.clip_rect, self.pixels_per_point, self.screen_size_px)
    }
}

/// If you want to paint some 3D shapes inside an egui region, you can use this.
///
/// This is advanced usage, and is backend specific.
#[derive(Clone)]
pub struct PaintCallback {
    /// Where to paint.
    ///
    /// This will become [`PaintCallbackInfo::viewport`].
    pub rect: Rect,

    /// Paint something custom (e.g. 3D stuff).
    ///
    /// The concrete value of `callback` depends on the rendering backend used. For instance, the
    /// `glow` backend requires that callback be an `egui_glow::CallbackFn` while the `wgpu`
    /// backend requires a `egui_wgpu::Callback`.
    ///
    /// If the type cannot be downcast to the type expected by the current backend the callback
    /// will not be drawn.
    ///
    /// The rendering backend is responsible for first setting the active viewport to
    /// [`Self::rect`].
    ///
    /// The rendering backend is also responsible for restoring any state, such as the bound shader
    /// program, vertex array, etc.
    ///
    /// Shape has to be clone, therefore this has to be an `Arc` instead of a `Box`.
    pub callback: Arc<dyn Any + Send + Sync>,
}

impl std::fmt::Debug for PaintCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomShape")
            .field("rect", &self.rect)
            .finish_non_exhaustive()
    }
}

impl std::cmp::PartialEq for PaintCallback {
    fn eq(&self, other: &Self) -> bool {
        // As I understand it, the problem this clippy is trying to protect against
        // can only happen if we do dynamic casts back and forth on the pointers, and we don't do that.
        #[allow(clippy::vtable_address_comparisons)]
        {
            self.rect.eq(&other.rect) && Arc::ptr_eq(&self.callback, &other.callback)
        }
    }
}

impl From<PaintCallback> for Shape {
    #[inline(always)]
    fn from(shape: PaintCallback) -> Self {
        Self::Callback(shape)
    }
}
