//! The different shapes that can be painted.

use std::sync::Arc;

use emath::{pos2, Align2, Pos2, Rangef, Rect, TSTransform, Vec2};

use crate::{
    stroke::PathStroke,
    text::{FontId, Fonts, Galley},
    Color32, CornerRadius, Mesh, Stroke, StrokeKind, TextureId,
};

use super::{
    ArcPieShape, CircleShape, CubicBezierShape, EllipseShape, PaintCallback, PathShape,
    QuadraticBezierShape, RectShape, TextShape,
};

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

    /// An arc or pie with a given start and end angle.
    ArcPie(ArcPieShape),

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
    ///
    /// Wrapped in an [`Arc`] to minimize the size of [`Shape`].
    Mesh(Arc<Mesh>),

    /// A quadratic [Bézier Curve](https://en.wikipedia.org/wiki/B%C3%A9zier_curve).
    QuadraticBezier(QuadraticBezierShape),

    /// A cubic [Bézier Curve](https://en.wikipedia.org/wiki/B%C3%A9zier_curve).
    CubicBezier(CubicBezierShape),

    /// Backend-specific painting.
    Callback(PaintCallback),
}

#[test]
fn shape_size() {
    assert_eq!(
        std::mem::size_of::<Shape>(), 64,
        "Shape changed size! If it shrank - good! Update this test. If it grew - bad! Try to find a way to avoid it."
    );
    assert!(
        std::mem::size_of::<Shape>() <= 64,
        "Shape is getting way too big!"
    );
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
        Self::Mesh(mesh.into())
    }
}

impl From<Arc<Mesh>> for Shape {
    #[inline(always)]
    fn from(mesh: Arc<Mesh>) -> Self {
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
    pub fn line(points: Vec<Pos2>, stroke: impl Into<PathStroke>) -> Self {
        Self::Path(PathShape::line(points, stroke))
    }

    /// A line that closes back to the start point again.
    #[inline]
    pub fn closed_line(points: Vec<Pos2>, stroke: impl Into<PathStroke>) -> Self {
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
        stroke: impl Into<PathStroke>,
    ) -> Self {
        Self::Path(PathShape::convex_polygon(points, fill, stroke))
    }

    /// Generates an arc with a given start and end angle.
    ///
    /// This function creates an arc centered at a specified point, with a specified radius.
    /// The arc starts at the `start_angle` and ends at the `end_angle`.
    /// Angles are specified in radians, with positive angles indicating clockwise rotation and negative angles indicating counterclockwise rotation.
    ///
    /// # Arguments
    ///
    /// * `center` - The center point of the arc.
    /// * `radius` - The radius of the arc.
    /// * `start_angle` - The start angle of the arc, in radians.
    /// * `end_angle` - The end angle of the arc, in radians.
    /// * `stroke` - The stroke of the arc.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use epaint::{pos2, Color32, Shape, Stroke};
    /// let arc = Shape::arc(pos2(100.0, 100.0), 50.0, 0.0, std::f32::consts::PI, Stroke::new(3.0, Color32::RED));
    /// ```
    pub fn arc(
        center: Pos2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        stroke: impl Into<PathStroke>,
    ) -> Self {
        Self::ArcPie(ArcPieShape::arc(
            center,
            radius,
            start_angle,
            end_angle,
            stroke,
        ))
    }

    /// Generates an pie with a given start and end angle.
    ///
    /// This function creates an arc centered at a specified point, with a specified radius.
    /// The pie starts at the `start_angle` and ends at the `end_angle`.
    /// Angles are specified in radians, with positive angles indicating clockwise rotation and negative angles indicating counterclockwise rotation.
    ///
    /// # Arguments
    ///
    /// * `center` - The center point of the pie.
    /// * `radius` - The radius of the pie.
    /// * `start_angle` - The start angle of the pie, in radians.
    /// * `end_angle` - The end angle of the pie, in radians.
    /// * `stroke` - The stroke of the pie.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use epaint::{pos2, Color32, Shape, Stroke};
    /// let pie = Shape::pie(pos2(100.0, 100.0), 50.0, 0.0, std::f32::consts::PI, Color32::BLUE, Stroke::new(3.0, Color32::RED));
    /// ```
    pub fn pie(
        center: Pos2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        fill: impl Into<Color32>,
        stroke: impl Into<PathStroke>,
    ) -> Self {
        Self::ArcPie(ArcPieShape::pie(
            center,
            radius,
            start_angle,
            end_angle,
            fill,
            stroke,
        ))
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

    /// See also [`Self::rect_stroke`].
    #[inline]
    pub fn rect_filled(
        rect: Rect,
        corner_radius: impl Into<CornerRadius>,
        fill_color: impl Into<Color32>,
    ) -> Self {
        Self::Rect(RectShape::filled(rect, corner_radius, fill_color))
    }

    /// See also [`Self::rect_filled`].
    #[inline]
    pub fn rect_stroke(
        rect: Rect,
        corner_radius: impl Into<CornerRadius>,
        stroke: impl Into<Stroke>,
        stroke_kind: StrokeKind,
    ) -> Self {
        Self::Rect(RectShape::stroke(rect, corner_radius, stroke, stroke_kind))
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
    pub fn mesh(mesh: impl Into<Arc<Mesh>>) -> Self {
        let mesh = mesh.into();
        debug_assert!(mesh.is_valid());
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
            Self::ArcPie(arc_pie_shape) => arc_pie_shape.visual_bounding_rect(),
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
    pub fn texture_id(&self) -> crate::TextureId {
        if let Self::Mesh(mesh) = self {
            mesh.texture_id
        } else if let Self::Rect(rect_shape) = self {
            rect_shape.fill_texture_id()
        } else {
            crate::TextureId::default()
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
            Self::ArcPie(arc_pie_shape) => {
                arc_pie_shape.center = transform * arc_pie_shape.center;
                arc_pie_shape.radius *= transform.scaling;
                arc_pie_shape.stroke.width *= transform.scaling;
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
                rect_shape.corner_radius *= transform.scaling;
                rect_shape.stroke.width *= transform.scaling;
                rect_shape.blur_width *= transform.scaling;
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
                Arc::make_mut(mesh).transform(transform);
            }
            Self::QuadraticBezier(bezier) => {
                for p in &mut bezier.points {
                    *p = transform * *p;
                }
                bezier.stroke.width *= transform.scaling;
            }
            Self::CubicBezier(bezier) => {
                for p in &mut bezier.points {
                    *p = transform * *p;
                }
                bezier.stroke.width *= transform.scaling;
            }
            Self::Callback(shape) => {
                shape.rect = transform * shape.rect;
            }
        }
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
    for window in path.windows(2) {
        let (start, end) = (window[0], window[1]);
        let vector = end - start;
        let segment_length = vector.length();
        while position_on_segment < segment_length {
            let new_point = start + vector * (position_on_segment / segment_length);
            shapes.push(Shape::circle_filled(new_point, radius, color));
            position_on_segment += spacing;
        }
        position_on_segment -= segment_length;
    }
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
    for window in path.windows(2) {
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
    }
}
