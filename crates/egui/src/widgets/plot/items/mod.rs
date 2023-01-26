//! Contains items that can be added to a plot.

use std::ops::RangeInclusive;

use epaint::util::FloatOrd;
use epaint::Mesh;

use crate::*;

use super::{Cursor, LabelFormatter, PlotBounds, ScreenTransform};
use rect_elem::*;
use values::{ClosestElem, PlotGeometry};

pub use bar::Bar;
pub use box_elem::{BoxElem, BoxSpread};
pub use values::{LineStyle, MarkerShape, Orientation, PlotPoint, PlotPoints};

mod bar;
mod box_elem;
mod rect_elem;
mod values;

const DEFAULT_FILL_ALPHA: f32 = 0.05;

/// Container to pass-through several parameters related to plot visualization
pub(super) struct PlotConfig<'a> {
    pub ui: &'a Ui,
    pub transform: &'a ScreenTransform,
    pub show_x: bool,
    pub show_y: bool,
}

/// Trait shared by things that can be drawn in the plot.
pub(super) trait PlotItem {
    fn shapes(&self, ui: &mut Ui, transform: &ScreenTransform, shapes: &mut Vec<Shape>);

    /// For plot-items which are generated based on x values (plotting functions).
    fn initialize(&mut self, x_range: RangeInclusive<f64>);

    fn name(&self) -> &str;

    fn color(&self) -> Color32;

    fn highlight(&mut self);

    fn highlighted(&self) -> bool;

    fn geometry(&self) -> PlotGeometry<'_>;

    fn bounds(&self) -> PlotBounds;

    fn find_closest(&self, point: Pos2, transform: &ScreenTransform) -> Option<ClosestElem> {
        match self.geometry() {
            PlotGeometry::None => None,

            PlotGeometry::Points(points) => points
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    let pos = transform.position_from_point(value);
                    let dist_sq = point.distance_sq(pos);
                    ClosestElem { index, dist_sq }
                })
                .min_by_key(|e| e.dist_sq.ord()),

            PlotGeometry::Rects => {
                panic!("If the PlotItem is made of rects, it should implement find_closest()")
            }
        }
    }

    fn on_hover(
        &self,
        elem: ClosestElem,
        shapes: &mut Vec<Shape>,
        cursors: &mut Vec<Cursor>,
        plot: &PlotConfig<'_>,
        label_formatter: &LabelFormatter,
    ) {
        let points = match self.geometry() {
            PlotGeometry::Points(points) => points,
            PlotGeometry::None => {
                panic!("If the PlotItem has no geometry, on_hover() must not be called")
            }
            PlotGeometry::Rects => {
                panic!("If the PlotItem is made of rects, it should implement on_hover()")
            }
        };

        let line_color = if plot.ui.visuals().dark_mode {
            Color32::from_gray(100).additive()
        } else {
            Color32::from_black_alpha(180)
        };

        // this method is only called, if the value is in the result set of find_closest()
        let value = points[elem.index];
        let pointer = plot.transform.position_from_point(&value);
        shapes.push(Shape::circle_filled(pointer, 3.0, line_color));

        rulers_at_value(
            pointer,
            value,
            self.name(),
            plot,
            shapes,
            cursors,
            label_formatter,
        );
    }
}

// ----------------------------------------------------------------------------

/// A horizontal line in a plot, filling the full width
#[derive(Clone, Debug, PartialEq)]
pub struct HLine {
    pub(super) y: f64,
    pub(super) stroke: Stroke,
    pub(super) name: String,
    pub(super) highlight: bool,
    pub(super) style: LineStyle,
}

impl HLine {
    pub fn new(y: impl Into<f64>) -> Self {
        Self {
            y: y.into(),
            stroke: Stroke::new(1.0, Color32::TRANSPARENT),
            name: String::default(),
            highlight: false,
            style: LineStyle::Solid,
        }
    }

    /// Highlight this line in the plot by scaling up the line.
    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Add a stroke.
    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Stroke width. A high value means the plot thickens.
    pub fn width(mut self, width: impl Into<f32>) -> Self {
        self.stroke.width = width.into();
        self
    }

    /// Stroke color. Default is `Color32::TRANSPARENT` which means a color will be auto-assigned.
    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        self.stroke.color = color.into();
        self
    }

    /// Set the line's style. Default is `LineStyle::Solid`.
    pub fn style(mut self, style: LineStyle) -> Self {
        self.style = style;
        self
    }

    /// Name of this horizontal line.
    ///
    /// This name will show up in the plot legend, if legends are turned on.
    ///
    /// Multiple plot items may share the same name, in which case they will also share an entry in
    /// the legend.
    #[allow(clippy::needless_pass_by_value)]
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }
}

impl PlotItem for HLine {
    fn shapes(&self, ui: &mut Ui, transform: &ScreenTransform, shapes: &mut Vec<Shape>) {
        let HLine {
            y,
            stroke,
            highlight,
            style,
            ..
        } = self;

        // Round to minimize aliasing:
        let points = vec![
            ui.ctx().round_pos_to_pixels(
                transform.position_from_point(&PlotPoint::new(transform.bounds().min[0], *y)),
            ),
            ui.ctx().round_pos_to_pixels(
                transform.position_from_point(&PlotPoint::new(transform.bounds().max[0], *y)),
            ),
        ];
        style.style_line(points, *stroke, *highlight, shapes);
    }

    fn initialize(&mut self, _x_range: RangeInclusive<f64>) {}

    fn name(&self) -> &str {
        &self.name
    }

    fn color(&self) -> Color32 {
        self.stroke.color
    }

    fn highlight(&mut self) {
        self.highlight = true;
    }

    fn highlighted(&self) -> bool {
        self.highlight
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotGeometry::None
    }

    fn bounds(&self) -> PlotBounds {
        let mut bounds = PlotBounds::NOTHING;
        bounds.min[1] = self.y;
        bounds.max[1] = self.y;
        bounds
    }
}

/// A vertical line in a plot, filling the full width
#[derive(Clone, Debug, PartialEq)]
pub struct VLine {
    pub(super) x: f64,
    pub(super) stroke: Stroke,
    pub(super) name: String,
    pub(super) highlight: bool,
    pub(super) style: LineStyle,
}

impl VLine {
    pub fn new(x: impl Into<f64>) -> Self {
        Self {
            x: x.into(),
            stroke: Stroke::new(1.0, Color32::TRANSPARENT),
            name: String::default(),
            highlight: false,
            style: LineStyle::Solid,
        }
    }

    /// Highlight this line in the plot by scaling up the line.
    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Add a stroke.
    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Stroke width. A high value means the plot thickens.
    pub fn width(mut self, width: impl Into<f32>) -> Self {
        self.stroke.width = width.into();
        self
    }

    /// Stroke color. Default is `Color32::TRANSPARENT` which means a color will be auto-assigned.
    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        self.stroke.color = color.into();
        self
    }

    /// Set the line's style. Default is `LineStyle::Solid`.
    pub fn style(mut self, style: LineStyle) -> Self {
        self.style = style;
        self
    }

    /// Name of this vertical line.
    ///
    /// This name will show up in the plot legend, if legends are turned on.
    ///
    /// Multiple plot items may share the same name, in which case they will also share an entry in
    /// the legend.
    #[allow(clippy::needless_pass_by_value)]
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }
}

impl PlotItem for VLine {
    fn shapes(&self, ui: &mut Ui, transform: &ScreenTransform, shapes: &mut Vec<Shape>) {
        let VLine {
            x,
            stroke,
            highlight,
            style,
            ..
        } = self;

        // Round to minimize aliasing:
        let points = vec![
            ui.ctx().round_pos_to_pixels(
                transform.position_from_point(&PlotPoint::new(*x, transform.bounds().min[1])),
            ),
            ui.ctx().round_pos_to_pixels(
                transform.position_from_point(&PlotPoint::new(*x, transform.bounds().max[1])),
            ),
        ];
        style.style_line(points, *stroke, *highlight, shapes);
    }

    fn initialize(&mut self, _x_range: RangeInclusive<f64>) {}

    fn name(&self) -> &str {
        &self.name
    }

    fn color(&self) -> Color32 {
        self.stroke.color
    }

    fn highlight(&mut self) {
        self.highlight = true;
    }

    fn highlighted(&self) -> bool {
        self.highlight
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotGeometry::None
    }

    fn bounds(&self) -> PlotBounds {
        let mut bounds = PlotBounds::NOTHING;
        bounds.min[0] = self.x;
        bounds.max[0] = self.x;
        bounds
    }
}

/// A series of values forming a path.
pub struct Line {
    pub(super) series: PlotPoints,
    pub(super) stroke: Stroke,
    pub(super) name: String,
    pub(super) highlight: bool,
    pub(super) fill: Option<f32>,
    pub(super) style: LineStyle,
}

impl Line {
    pub fn new(series: impl Into<PlotPoints>) -> Self {
        Self {
            series: series.into(),
            stroke: Stroke::new(1.0, Color32::TRANSPARENT),
            name: Default::default(),
            highlight: false,
            fill: None,
            style: LineStyle::Solid,
        }
    }

    /// Highlight this line in the plot by scaling up the line.
    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Add a stroke.
    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Stroke width. A high value means the plot thickens.
    pub fn width(mut self, width: impl Into<f32>) -> Self {
        self.stroke.width = width.into();
        self
    }

    /// Stroke color. Default is `Color32::TRANSPARENT` which means a color will be auto-assigned.
    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        self.stroke.color = color.into();
        self
    }

    /// Fill the area between this line and a given horizontal reference line.
    pub fn fill(mut self, y_reference: impl Into<f32>) -> Self {
        self.fill = Some(y_reference.into());
        self
    }

    /// Set the line's style. Default is `LineStyle::Solid`.
    pub fn style(mut self, style: LineStyle) -> Self {
        self.style = style;
        self
    }

    /// Name of this line.
    ///
    /// This name will show up in the plot legend, if legends are turned on.
    ///
    /// Multiple plot items may share the same name, in which case they will also share an entry in
    /// the legend.
    #[allow(clippy::needless_pass_by_value)]
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }
}

/// Returns the x-coordinate of a possible intersection between a line segment from `p1` to `p2` and
/// a horizontal line at the given y-coordinate.
fn y_intersection(p1: &Pos2, p2: &Pos2, y: f32) -> Option<f32> {
    ((p1.y > y && p2.y < y) || (p1.y < y && p2.y > y))
        .then_some(((y * (p1.x - p2.x)) - (p1.x * p2.y - p1.y * p2.x)) / (p1.y - p2.y))
}

impl PlotItem for Line {
    fn shapes(&self, _ui: &mut Ui, transform: &ScreenTransform, shapes: &mut Vec<Shape>) {
        let Self {
            series,
            stroke,
            highlight,
            mut fill,
            style,
            ..
        } = self;

        let values_tf: Vec<_> = series
            .points()
            .iter()
            .map(|v| transform.position_from_point(v))
            .collect();
        let n_values = values_tf.len();

        // Fill the area between the line and a reference line, if required.
        if n_values < 2 {
            fill = None;
        }
        if let Some(y_reference) = fill {
            let mut fill_alpha = DEFAULT_FILL_ALPHA;
            if *highlight {
                fill_alpha = (2.0 * fill_alpha).at_most(1.0);
            }
            let y = transform
                .position_from_point(&PlotPoint::new(0.0, y_reference))
                .y;
            let fill_color = Rgba::from(stroke.color)
                .to_opaque()
                .multiply(fill_alpha)
                .into();
            let mut mesh = Mesh::default();
            let expected_intersections = 20;
            mesh.reserve_triangles((n_values - 1) * 2);
            mesh.reserve_vertices(n_values * 2 + expected_intersections);
            values_tf.windows(2).for_each(|w| {
                let i = mesh.vertices.len() as u32;
                mesh.colored_vertex(w[0], fill_color);
                mesh.colored_vertex(pos2(w[0].x, y), fill_color);
                if let Some(x) = y_intersection(&w[0], &w[1], y) {
                    let point = pos2(x, y);
                    mesh.colored_vertex(point, fill_color);
                    mesh.add_triangle(i, i + 1, i + 2);
                    mesh.add_triangle(i + 2, i + 3, i + 4);
                } else {
                    mesh.add_triangle(i, i + 1, i + 2);
                    mesh.add_triangle(i + 1, i + 2, i + 3);
                }
            });
            let last = values_tf[n_values - 1];
            mesh.colored_vertex(last, fill_color);
            mesh.colored_vertex(pos2(last.x, y), fill_color);
            shapes.push(Shape::Mesh(mesh));
        }
        style.style_line(values_tf, *stroke, *highlight, shapes);
    }

    fn initialize(&mut self, x_range: RangeInclusive<f64>) {
        self.series.generate_points(x_range);
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn color(&self) -> Color32 {
        self.stroke.color
    }

    fn highlight(&mut self) {
        self.highlight = true;
    }

    fn highlighted(&self) -> bool {
        self.highlight
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotGeometry::Points(self.series.points())
    }

    fn bounds(&self) -> PlotBounds {
        self.series.bounds()
    }
}

/// A convex polygon.
pub struct Polygon {
    pub(super) series: PlotPoints,
    pub(super) stroke: Stroke,
    pub(super) name: String,
    pub(super) highlight: bool,
    pub(super) fill_alpha: f32,
    pub(super) style: LineStyle,
}

impl Polygon {
    pub fn new(series: impl Into<PlotPoints>) -> Self {
        Self {
            series: series.into(),
            stroke: Stroke::new(1.0, Color32::TRANSPARENT),
            name: Default::default(),
            highlight: false,
            fill_alpha: DEFAULT_FILL_ALPHA,
            style: LineStyle::Solid,
        }
    }

    /// Highlight this polygon in the plot by scaling up the stroke and reducing the fill
    /// transparency.
    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Add a custom stroke.
    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Set the stroke width.
    pub fn width(mut self, width: impl Into<f32>) -> Self {
        self.stroke.width = width.into();
        self
    }

    /// Stroke color. Default is `Color32::TRANSPARENT` which means a color will be auto-assigned.
    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        self.stroke.color = color.into();
        self
    }

    /// Alpha of the filled area.
    pub fn fill_alpha(mut self, alpha: impl Into<f32>) -> Self {
        self.fill_alpha = alpha.into();
        self
    }

    /// Set the outline's style. Default is `LineStyle::Solid`.
    pub fn style(mut self, style: LineStyle) -> Self {
        self.style = style;
        self
    }

    /// Name of this polygon.
    ///
    /// This name will show up in the plot legend, if legends are turned on.
    ///
    /// Multiple plot items may share the same name, in which case they will also share an entry in
    /// the legend.
    #[allow(clippy::needless_pass_by_value)]
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }
}

impl PlotItem for Polygon {
    fn shapes(&self, _ui: &mut Ui, transform: &ScreenTransform, shapes: &mut Vec<Shape>) {
        let Self {
            series,
            stroke,
            highlight,
            mut fill_alpha,
            style,
            ..
        } = self;

        if *highlight {
            fill_alpha = (2.0 * fill_alpha).at_most(1.0);
        }

        let mut values_tf: Vec<_> = series
            .points()
            .iter()
            .map(|v| transform.position_from_point(v))
            .collect();

        let fill = Rgba::from(stroke.color).to_opaque().multiply(fill_alpha);

        let shape = Shape::convex_polygon(values_tf.clone(), fill, Stroke::NONE);
        shapes.push(shape);
        values_tf.push(*values_tf.first().unwrap());
        style.style_line(values_tf, *stroke, *highlight, shapes);
    }

    fn initialize(&mut self, x_range: RangeInclusive<f64>) {
        self.series.generate_points(x_range);
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn color(&self) -> Color32 {
        self.stroke.color
    }

    fn highlight(&mut self) {
        self.highlight = true;
    }

    fn highlighted(&self) -> bool {
        self.highlight
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotGeometry::Points(self.series.points())
    }

    fn bounds(&self) -> PlotBounds {
        self.series.bounds()
    }
}

/// Text inside the plot.
#[derive(Clone)]
pub struct Text {
    pub(super) text: WidgetText,
    pub(super) position: PlotPoint,
    pub(super) name: String,
    pub(super) highlight: bool,
    pub(super) color: Color32,
    pub(super) anchor: Align2,
}

impl Text {
    pub fn new(position: PlotPoint, text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            position,
            name: Default::default(),
            highlight: false,
            color: Color32::TRANSPARENT,
            anchor: Align2::CENTER_CENTER,
        }
    }

    /// Highlight this text in the plot by drawing a rectangle around it.
    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Text color.
    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        self.color = color.into();
        self
    }

    /// Anchor position of the text. Default is `Align2::CENTER_CENTER`.
    pub fn anchor(mut self, anchor: Align2) -> Self {
        self.anchor = anchor;
        self
    }

    /// Name of this text.
    ///
    /// This name will show up in the plot legend, if legends are turned on.
    ///
    /// Multiple plot items may share the same name, in which case they will also share an entry in
    /// the legend.
    #[allow(clippy::needless_pass_by_value)]
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }
}

impl PlotItem for Text {
    fn shapes(&self, ui: &mut Ui, transform: &ScreenTransform, shapes: &mut Vec<Shape>) {
        let color = if self.color == Color32::TRANSPARENT {
            ui.style().visuals.text_color()
        } else {
            self.color
        };

        let galley =
            self.text
                .clone()
                .into_galley(ui, Some(false), f32::INFINITY, TextStyle::Small);

        let pos = transform.position_from_point(&self.position);
        let rect = self
            .anchor
            .anchor_rect(Rect::from_min_size(pos, galley.size()));

        let mut text_shape = epaint::TextShape::new(rect.min, galley.galley);
        if !galley.galley_has_color {
            text_shape.override_text_color = Some(color);
        }
        shapes.push(text_shape.into());

        if self.highlight {
            shapes.push(Shape::rect_stroke(
                rect.expand(2.0),
                1.0,
                Stroke::new(0.5, color),
            ));
        }
    }

    fn initialize(&mut self, _x_range: RangeInclusive<f64>) {}

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn color(&self) -> Color32 {
        self.color
    }

    fn highlight(&mut self) {
        self.highlight = true;
    }

    fn highlighted(&self) -> bool {
        self.highlight
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotGeometry::None
    }

    fn bounds(&self) -> PlotBounds {
        let mut bounds = PlotBounds::NOTHING;
        bounds.extend_with(&self.position);
        bounds
    }
}

/// A set of points.
pub struct Points {
    pub(super) series: PlotPoints,
    pub(super) shape: MarkerShape,
    /// Color of the marker. `Color32::TRANSPARENT` means that it will be picked automatically.
    pub(super) color: Color32,
    /// Whether to fill the marker. Does not apply to all types.
    pub(super) filled: bool,
    /// The maximum extent of the marker from its center.
    pub(super) radius: f32,
    pub(super) name: String,
    pub(super) highlight: bool,
    pub(super) stems: Option<f32>,
}

impl Points {
    pub fn new(series: impl Into<PlotPoints>) -> Self {
        Self {
            series: series.into(),
            shape: MarkerShape::Circle,
            color: Color32::TRANSPARENT,
            filled: true,
            radius: 1.0,
            name: Default::default(),
            highlight: false,
            stems: None,
        }
    }

    /// Set the shape of the markers.
    pub fn shape(mut self, shape: MarkerShape) -> Self {
        self.shape = shape;
        self
    }

    /// Highlight these points in the plot by scaling up their markers.
    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Set the marker's color.
    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        self.color = color.into();
        self
    }

    /// Whether to fill the marker.
    pub fn filled(mut self, filled: bool) -> Self {
        self.filled = filled;
        self
    }

    /// Whether to add stems between the markers and a horizontal reference line.
    pub fn stems(mut self, y_reference: impl Into<f32>) -> Self {
        self.stems = Some(y_reference.into());
        self
    }

    /// Set the maximum extent of the marker around its position.
    pub fn radius(mut self, radius: impl Into<f32>) -> Self {
        self.radius = radius.into();
        self
    }

    /// Name of this set of points.
    ///
    /// This name will show up in the plot legend, if legends are turned on.
    ///
    /// Multiple plot items may share the same name, in which case they will also share an entry in
    /// the legend.
    #[allow(clippy::needless_pass_by_value)]
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }
}

impl PlotItem for Points {
    fn shapes(&self, _ui: &mut Ui, transform: &ScreenTransform, shapes: &mut Vec<Shape>) {
        let sqrt_3 = 3_f32.sqrt();
        let frac_sqrt_3_2 = 3_f32.sqrt() / 2.0;
        let frac_1_sqrt_2 = 1.0 / 2_f32.sqrt();

        let Self {
            series,
            shape,
            color,
            filled,
            mut radius,
            highlight,
            stems,
            ..
        } = self;

        let stroke_size = radius / 5.0;

        let default_stroke = Stroke::new(stroke_size, *color);
        let mut stem_stroke = default_stroke;
        let (fill, stroke) = if *filled {
            (*color, Stroke::NONE)
        } else {
            (Color32::TRANSPARENT, default_stroke)
        };

        if *highlight {
            radius *= 2f32.sqrt();
            stem_stroke.width *= 2.0;
        }

        let y_reference = stems.map(|y| transform.position_from_point(&PlotPoint::new(0.0, y)).y);

        series
            .points()
            .iter()
            .map(|value| transform.position_from_point(value))
            .for_each(|center| {
                let tf = |dx: f32, dy: f32| -> Pos2 { center + radius * vec2(dx, dy) };

                if let Some(y) = y_reference {
                    let stem = Shape::line_segment([center, pos2(center.x, y)], stem_stroke);
                    shapes.push(stem);
                }

                match shape {
                    MarkerShape::Circle => {
                        shapes.push(Shape::Circle(epaint::CircleShape {
                            center,
                            radius,
                            fill,
                            stroke,
                        }));
                    }
                    MarkerShape::Diamond => {
                        let points = vec![
                            tf(0.0, 1.0),  // bottom
                            tf(-1.0, 0.0), // left
                            tf(0.0, -1.0), // top
                            tf(1.0, 0.0),  // right
                        ];
                        shapes.push(Shape::convex_polygon(points, fill, stroke));
                    }
                    MarkerShape::Square => {
                        let points = vec![
                            tf(-frac_1_sqrt_2, frac_1_sqrt_2),
                            tf(-frac_1_sqrt_2, -frac_1_sqrt_2),
                            tf(frac_1_sqrt_2, -frac_1_sqrt_2),
                            tf(frac_1_sqrt_2, frac_1_sqrt_2),
                        ];
                        shapes.push(Shape::convex_polygon(points, fill, stroke));
                    }
                    MarkerShape::Cross => {
                        let diagonal1 = [
                            tf(-frac_1_sqrt_2, -frac_1_sqrt_2),
                            tf(frac_1_sqrt_2, frac_1_sqrt_2),
                        ];
                        let diagonal2 = [
                            tf(frac_1_sqrt_2, -frac_1_sqrt_2),
                            tf(-frac_1_sqrt_2, frac_1_sqrt_2),
                        ];
                        shapes.push(Shape::line_segment(diagonal1, default_stroke));
                        shapes.push(Shape::line_segment(diagonal2, default_stroke));
                    }
                    MarkerShape::Plus => {
                        let horizontal = [tf(-1.0, 0.0), tf(1.0, 0.0)];
                        let vertical = [tf(0.0, -1.0), tf(0.0, 1.0)];
                        shapes.push(Shape::line_segment(horizontal, default_stroke));
                        shapes.push(Shape::line_segment(vertical, default_stroke));
                    }
                    MarkerShape::Up => {
                        let points =
                            vec![tf(0.0, -1.0), tf(0.5 * sqrt_3, 0.5), tf(-0.5 * sqrt_3, 0.5)];
                        shapes.push(Shape::convex_polygon(points, fill, stroke));
                    }
                    MarkerShape::Down => {
                        let points = vec![
                            tf(0.0, 1.0),
                            tf(-0.5 * sqrt_3, -0.5),
                            tf(0.5 * sqrt_3, -0.5),
                        ];
                        shapes.push(Shape::convex_polygon(points, fill, stroke));
                    }
                    MarkerShape::Left => {
                        let points =
                            vec![tf(-1.0, 0.0), tf(0.5, -0.5 * sqrt_3), tf(0.5, 0.5 * sqrt_3)];
                        shapes.push(Shape::convex_polygon(points, fill, stroke));
                    }
                    MarkerShape::Right => {
                        let points = vec![
                            tf(1.0, 0.0),
                            tf(-0.5, 0.5 * sqrt_3),
                            tf(-0.5, -0.5 * sqrt_3),
                        ];
                        shapes.push(Shape::convex_polygon(points, fill, stroke));
                    }
                    MarkerShape::Asterisk => {
                        let vertical = [tf(0.0, -1.0), tf(0.0, 1.0)];
                        let diagonal1 = [tf(-frac_sqrt_3_2, 0.5), tf(frac_sqrt_3_2, -0.5)];
                        let diagonal2 = [tf(-frac_sqrt_3_2, -0.5), tf(frac_sqrt_3_2, 0.5)];
                        shapes.push(Shape::line_segment(vertical, default_stroke));
                        shapes.push(Shape::line_segment(diagonal1, default_stroke));
                        shapes.push(Shape::line_segment(diagonal2, default_stroke));
                    }
                }
            });
    }

    fn initialize(&mut self, x_range: RangeInclusive<f64>) {
        self.series.generate_points(x_range);
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn color(&self) -> Color32 {
        self.color
    }

    fn highlight(&mut self) {
        self.highlight = true;
    }

    fn highlighted(&self) -> bool {
        self.highlight
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotGeometry::Points(self.series.points())
    }

    fn bounds(&self) -> PlotBounds {
        self.series.bounds()
    }
}

/// A set of arrows.
pub struct Arrows {
    pub(super) origins: PlotPoints,
    pub(super) tips: PlotPoints,
    pub(super) color: Color32,
    pub(super) name: String,
    pub(super) highlight: bool,
}

impl Arrows {
    pub fn new(origins: impl Into<PlotPoints>, tips: impl Into<PlotPoints>) -> Self {
        Self {
            origins: origins.into(),
            tips: tips.into(),
            color: Color32::TRANSPARENT,
            name: Default::default(),
            highlight: false,
        }
    }

    /// Highlight these arrows in the plot.
    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Set the arrows' color.
    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        self.color = color.into();
        self
    }

    /// Name of this set of arrows.
    ///
    /// This name will show up in the plot legend, if legends are turned on.
    ///
    /// Multiple plot items may share the same name, in which case they will also share an entry in
    /// the legend.
    #[allow(clippy::needless_pass_by_value)]
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }
}

impl PlotItem for Arrows {
    fn shapes(&self, _ui: &mut Ui, transform: &ScreenTransform, shapes: &mut Vec<Shape>) {
        use crate::emath::*;
        let Self {
            origins,
            tips,
            color,
            highlight,
            ..
        } = self;
        let stroke = Stroke::new(if *highlight { 2.0 } else { 1.0 }, *color);
        origins
            .points()
            .iter()
            .zip(tips.points().iter())
            .map(|(origin, tip)| {
                (
                    transform.position_from_point(origin),
                    transform.position_from_point(tip),
                )
            })
            .for_each(|(origin, tip)| {
                let vector = tip - origin;
                let rot = Rot2::from_angle(std::f32::consts::TAU / 10.0);
                let tip_length = vector.length() / 4.0;
                let tip = origin + vector;
                let dir = vector.normalized();
                shapes.push(Shape::line_segment([origin, tip], stroke));
                shapes.push(Shape::line(
                    vec![
                        tip - tip_length * (rot.inverse() * dir),
                        tip,
                        tip - tip_length * (rot * dir),
                    ],
                    stroke,
                ));
            });
    }

    fn initialize(&mut self, _x_range: RangeInclusive<f64>) {
        self.origins
            .generate_points(f64::NEG_INFINITY..=f64::INFINITY);
        self.tips.generate_points(f64::NEG_INFINITY..=f64::INFINITY);
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn color(&self) -> Color32 {
        self.color
    }

    fn highlight(&mut self) {
        self.highlight = true;
    }

    fn highlighted(&self) -> bool {
        self.highlight
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotGeometry::Points(self.origins.points())
    }

    fn bounds(&self) -> PlotBounds {
        self.origins.bounds()
    }
}

/// An image in the plot.
#[derive(Clone)]
pub struct PlotImage {
    pub(super) position: PlotPoint,
    pub(super) texture_id: TextureId,
    pub(super) uv: Rect,
    pub(super) size: Vec2,
    pub(super) bg_fill: Color32,
    pub(super) tint: Color32,
    pub(super) highlight: bool,
    pub(super) name: String,
}

impl PlotImage {
    /// Create a new image with position and size in plot coordinates.
    pub fn new(
        texture_id: impl Into<TextureId>,
        center_position: PlotPoint,
        size: impl Into<Vec2>,
    ) -> Self {
        Self {
            position: center_position,
            name: Default::default(),
            highlight: false,
            texture_id: texture_id.into(),
            uv: Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            size: size.into(),
            bg_fill: Default::default(),
            tint: Color32::WHITE,
        }
    }

    /// Highlight this image in the plot.
    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Select UV range. Default is (0,0) in top-left, (1,1) bottom right.
    pub fn uv(mut self, uv: impl Into<Rect>) -> Self {
        self.uv = uv.into();
        self
    }

    /// A solid color to put behind the image. Useful for transparent images.
    pub fn bg_fill(mut self, bg_fill: impl Into<Color32>) -> Self {
        self.bg_fill = bg_fill.into();
        self
    }

    /// Multiply image color with this. Default is WHITE (no tint).
    pub fn tint(mut self, tint: impl Into<Color32>) -> Self {
        self.tint = tint.into();
        self
    }

    /// Name of this image.
    ///
    /// This name will show up in the plot legend, if legends are turned on.
    ///
    /// Multiple plot items may share the same name, in which case they will also share an entry in
    /// the legend.
    #[allow(clippy::needless_pass_by_value)]
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }
}

impl PlotItem for PlotImage {
    fn shapes(&self, ui: &mut Ui, transform: &ScreenTransform, shapes: &mut Vec<Shape>) {
        let Self {
            position,
            texture_id,
            uv,
            size,
            bg_fill,
            tint,
            highlight,
            ..
        } = self;
        let rect = {
            let left_top = PlotPoint::new(
                position.x as f32 - size.x / 2.0,
                position.y as f32 - size.y / 2.0,
            );
            let right_bottom = PlotPoint::new(
                position.x as f32 + size.x / 2.0,
                position.y as f32 + size.y / 2.0,
            );
            let left_top_tf = transform.position_from_point(&left_top);
            let right_bottom_tf = transform.position_from_point(&right_bottom);
            Rect::from_two_pos(left_top_tf, right_bottom_tf)
        };
        Image::new(*texture_id, *size)
            .bg_fill(*bg_fill)
            .tint(*tint)
            .uv(*uv)
            .paint_at(ui, rect);
        if *highlight {
            shapes.push(Shape::rect_stroke(
                rect,
                0.0,
                Stroke::new(1.0, ui.visuals().strong_text_color()),
            ));
        }
    }

    fn initialize(&mut self, _x_range: RangeInclusive<f64>) {}

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn color(&self) -> Color32 {
        Color32::TRANSPARENT
    }

    fn highlight(&mut self) {
        self.highlight = true;
    }

    fn highlighted(&self) -> bool {
        self.highlight
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotGeometry::None
    }

    fn bounds(&self) -> PlotBounds {
        let mut bounds = PlotBounds::NOTHING;
        let left_top = PlotPoint::new(
            self.position.x as f32 - self.size.x / 2.0,
            self.position.y as f32 - self.size.y / 2.0,
        );
        let right_bottom = PlotPoint::new(
            self.position.x as f32 + self.size.x / 2.0,
            self.position.y as f32 + self.size.y / 2.0,
        );
        bounds.extend_with(&left_top);
        bounds.extend_with(&right_bottom);
        bounds
    }
}

// ----------------------------------------------------------------------------

/// A bar chart.
pub struct BarChart {
    pub(super) bars: Vec<Bar>,
    pub(super) default_color: Color32,
    pub(super) name: String,
    /// A custom element formatter
    pub(super) element_formatter: Option<Box<dyn Fn(&Bar, &BarChart) -> String>>,
    highlight: bool,
}

impl BarChart {
    /// Create a bar chart. It defaults to vertically oriented elements.
    pub fn new(bars: Vec<Bar>) -> BarChart {
        BarChart {
            bars,
            default_color: Color32::TRANSPARENT,
            name: String::new(),
            element_formatter: None,
            highlight: false,
        }
    }

    /// Set the default color. It is set on all elements that do not already have a specific color.
    /// This is the color that shows up in the legend.
    /// It can be overridden at the bar level (see [[`Bar`]]).
    /// Default is `Color32::TRANSPARENT` which means a color will be auto-assigned.
    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        let plot_color = color.into();
        self.default_color = plot_color;
        for b in &mut self.bars {
            if b.fill == Color32::TRANSPARENT && b.stroke.color == Color32::TRANSPARENT {
                b.fill = plot_color.linear_multiply(0.2);
                b.stroke.color = plot_color;
            }
        }
        self
    }

    /// Name of this chart.
    ///
    /// This name will show up in the plot legend, if legends are turned on. Multiple charts may
    /// share the same name, in which case they will also share an entry in the legend.
    #[allow(clippy::needless_pass_by_value)]
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }

    /// Set all elements to be in a vertical orientation.
    /// Argument axis will be X and bar values will be on the Y axis.
    pub fn vertical(mut self) -> Self {
        for b in &mut self.bars {
            b.orientation = Orientation::Vertical;
        }
        self
    }

    /// Set all elements to be in a horizontal orientation.
    /// Argument axis will be Y and bar values will be on the X axis.
    pub fn horizontal(mut self) -> Self {
        for b in &mut self.bars {
            b.orientation = Orientation::Horizontal;
        }
        self
    }

    /// Set the width (thickness) of all its elements.
    pub fn width(mut self, width: f64) -> Self {
        for b in &mut self.bars {
            b.bar_width = width;
        }
        self
    }

    /// Highlight all plot elements.
    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Add a custom way to format an element.
    /// Can be used to display a set number of decimals or custom labels.
    pub fn element_formatter(mut self, formatter: Box<dyn Fn(&Bar, &BarChart) -> String>) -> Self {
        self.element_formatter = Some(formatter);
        self
    }

    /// Stacks the bars on top of another chart.
    /// Positive values are stacked on top of other positive values.
    /// Negative values are stacked below other negative values.
    pub fn stack_on(mut self, others: &[&BarChart]) -> Self {
        for (index, bar) in self.bars.iter_mut().enumerate() {
            let new_base_offset = if bar.value.is_sign_positive() {
                others
                    .iter()
                    .filter_map(|other_chart| other_chart.bars.get(index).map(|bar| bar.upper()))
                    .max_by_key(|value| value.ord())
            } else {
                others
                    .iter()
                    .filter_map(|other_chart| other_chart.bars.get(index).map(|bar| bar.lower()))
                    .min_by_key(|value| value.ord())
            };

            if let Some(value) = new_base_offset {
                bar.base_offset = Some(value);
            }
        }
        self
    }
}

impl PlotItem for BarChart {
    fn shapes(&self, _ui: &mut Ui, transform: &ScreenTransform, shapes: &mut Vec<Shape>) {
        for b in &self.bars {
            b.add_shapes(transform, self.highlight, shapes);
        }
    }

    fn initialize(&mut self, _x_range: RangeInclusive<f64>) {
        // nothing to do
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn color(&self) -> Color32 {
        self.default_color
    }

    fn highlight(&mut self) {
        self.highlight = true;
    }

    fn highlighted(&self) -> bool {
        self.highlight
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotGeometry::Rects
    }

    fn bounds(&self) -> PlotBounds {
        let mut bounds = PlotBounds::NOTHING;
        for b in &self.bars {
            bounds.merge(&b.bounds());
        }
        bounds
    }

    fn find_closest(&self, point: Pos2, transform: &ScreenTransform) -> Option<ClosestElem> {
        find_closest_rect(&self.bars, point, transform)
    }

    fn on_hover(
        &self,
        elem: ClosestElem,
        shapes: &mut Vec<Shape>,
        cursors: &mut Vec<Cursor>,
        plot: &PlotConfig<'_>,
        _: &LabelFormatter,
    ) {
        let bar = &self.bars[elem.index];

        bar.add_shapes(plot.transform, true, shapes);
        bar.add_rulers_and_text(self, plot, shapes, cursors);
    }
}

/// A diagram containing a series of [`BoxElem`] elements.
pub struct BoxPlot {
    pub(super) boxes: Vec<BoxElem>,
    pub(super) default_color: Color32,
    pub(super) name: String,
    /// A custom element formatter
    pub(super) element_formatter: Option<Box<dyn Fn(&BoxElem, &BoxPlot) -> String>>,
    highlight: bool,
}

impl BoxPlot {
    /// Create a plot containing multiple `boxes`. It defaults to vertically oriented elements.
    pub fn new(boxes: Vec<BoxElem>) -> Self {
        Self {
            boxes,
            default_color: Color32::TRANSPARENT,
            name: String::new(),
            element_formatter: None,
            highlight: false,
        }
    }

    /// Set the default color. It is set on all elements that do not already have a specific color.
    /// This is the color that shows up in the legend.
    /// It can be overridden at the element level (see [`BoxElem`]).
    /// Default is `Color32::TRANSPARENT` which means a color will be auto-assigned.
    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        let plot_color = color.into();
        self.default_color = plot_color;
        for box_elem in &mut self.boxes {
            if box_elem.fill == Color32::TRANSPARENT
                && box_elem.stroke.color == Color32::TRANSPARENT
            {
                box_elem.fill = plot_color.linear_multiply(0.2);
                box_elem.stroke.color = plot_color;
            }
        }
        self
    }

    /// Name of this box plot diagram.
    ///
    /// This name will show up in the plot legend, if legends are turned on. Multiple series may
    /// share the same name, in which case they will also share an entry in the legend.
    #[allow(clippy::needless_pass_by_value)]
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }

    /// Set all elements to be in a vertical orientation.
    /// Argument axis will be X and values will be on the Y axis.
    pub fn vertical(mut self) -> Self {
        for box_elem in &mut self.boxes {
            box_elem.orientation = Orientation::Vertical;
        }
        self
    }

    /// Set all elements to be in a horizontal orientation.
    /// Argument axis will be Y and values will be on the X axis.
    pub fn horizontal(mut self) -> Self {
        for box_elem in &mut self.boxes {
            box_elem.orientation = Orientation::Horizontal;
        }
        self
    }

    /// Highlight all plot elements.
    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    /// Add a custom way to format an element.
    /// Can be used to display a set number of decimals or custom labels.
    pub fn element_formatter(
        mut self,
        formatter: Box<dyn Fn(&BoxElem, &BoxPlot) -> String>,
    ) -> Self {
        self.element_formatter = Some(formatter);
        self
    }
}

impl PlotItem for BoxPlot {
    fn shapes(&self, _ui: &mut Ui, transform: &ScreenTransform, shapes: &mut Vec<Shape>) {
        for b in &self.boxes {
            b.add_shapes(transform, self.highlight, shapes);
        }
    }

    fn initialize(&mut self, _x_range: RangeInclusive<f64>) {
        // nothing to do
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn color(&self) -> Color32 {
        self.default_color
    }

    fn highlight(&mut self) {
        self.highlight = true;
    }

    fn highlighted(&self) -> bool {
        self.highlight
    }

    fn geometry(&self) -> PlotGeometry<'_> {
        PlotGeometry::Rects
    }

    fn bounds(&self) -> PlotBounds {
        let mut bounds = PlotBounds::NOTHING;
        for b in &self.boxes {
            bounds.merge(&b.bounds());
        }
        bounds
    }

    fn find_closest(&self, point: Pos2, transform: &ScreenTransform) -> Option<ClosestElem> {
        find_closest_rect(&self.boxes, point, transform)
    }

    fn on_hover(
        &self,
        elem: ClosestElem,
        shapes: &mut Vec<Shape>,
        cursors: &mut Vec<Cursor>,
        plot: &PlotConfig<'_>,
        _: &LabelFormatter,
    ) {
        let box_plot = &self.boxes[elem.index];

        box_plot.add_shapes(plot.transform, true, shapes);
        box_plot.add_rulers_and_text(self, plot, shapes, cursors);
    }
}

// ----------------------------------------------------------------------------
// Helper functions

pub(crate) fn rulers_color(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_gray(100).additive()
    } else {
        Color32::from_black_alpha(180)
    }
}

pub(crate) fn vertical_line(
    pointer: Pos2,
    transform: &ScreenTransform,
    line_color: Color32,
) -> Shape {
    let frame = transform.frame();
    Shape::line_segment(
        [
            pos2(pointer.x, frame.top()),
            pos2(pointer.x, frame.bottom()),
        ],
        (1.0, line_color),
    )
}

pub(crate) fn horizontal_line(
    pointer: Pos2,
    transform: &ScreenTransform,
    line_color: Color32,
) -> Shape {
    let frame = transform.frame();
    Shape::line_segment(
        [
            pos2(frame.left(), pointer.y),
            pos2(frame.right(), pointer.y),
        ],
        (1.0, line_color),
    )
}

fn add_rulers_and_text(
    elem: &dyn RectElement,
    plot: &PlotConfig<'_>,
    text: Option<String>,
    shapes: &mut Vec<Shape>,
    cursors: &mut Vec<Cursor>,
) {
    let orientation = elem.orientation();
    let show_argument = plot.show_x && orientation == Orientation::Vertical
        || plot.show_y && orientation == Orientation::Horizontal;
    let show_values = plot.show_y && orientation == Orientation::Vertical
        || plot.show_x && orientation == Orientation::Horizontal;

    // Rulers for argument (usually vertical)
    if show_argument {
        for pos in elem.arguments_with_ruler() {
            cursors.push(match orientation {
                Orientation::Horizontal => Cursor::Horizontal { y: pos.y },
                Orientation::Vertical => Cursor::Vertical { x: pos.x },
            });
        }
    }

    // Rulers for values (usually horizontal)
    if show_values {
        for pos in elem.values_with_ruler() {
            cursors.push(match orientation {
                Orientation::Horizontal => Cursor::Vertical { x: pos.x },
                Orientation::Vertical => Cursor::Horizontal { y: pos.y },
            });
        }
    }

    // Text
    let text = text.unwrap_or({
        let mut text = elem.name().to_owned(); // could be empty

        if show_values {
            text.push('\n');
            text.push_str(&elem.default_values_format(plot.transform));
        }

        text
    });

    let font_id = TextStyle::Body.resolve(plot.ui.style());

    let corner_value = elem.corner_value();
    plot.ui.fonts(|f| {
        shapes.push(Shape::text(
            f,
            plot.transform.position_from_point(&corner_value) + vec2(3.0, -2.0),
            Align2::LEFT_BOTTOM,
            text,
            font_id,
            plot.ui.visuals().text_color(),
        ));
    });
}

/// Draws a cross of horizontal and vertical ruler at the `pointer` position.
/// `value` is used to for text displaying X/Y coordinates.
#[allow(clippy::too_many_arguments)]
pub(super) fn rulers_at_value(
    pointer: Pos2,
    value: PlotPoint,
    name: &str,
    plot: &PlotConfig<'_>,
    shapes: &mut Vec<Shape>,
    cursors: &mut Vec<Cursor>,
    label_formatter: &LabelFormatter,
) {
    if plot.show_x {
        cursors.push(Cursor::Vertical { x: value.x });
    }
    if plot.show_y {
        cursors.push(Cursor::Horizontal { y: value.y });
    }

    let mut prefix = String::new();

    if !name.is_empty() {
        prefix = format!("{}\n", name);
    }

    let text = {
        let scale = plot.transform.dvalue_dpos();
        let x_decimals = ((-scale[0].abs().log10()).ceil().at_least(0.0) as usize).clamp(1, 6);
        let y_decimals = ((-scale[1].abs().log10()).ceil().at_least(0.0) as usize).clamp(1, 6);
        if let Some(custom_label) = label_formatter {
            custom_label(name, &value)
        } else if plot.show_x && plot.show_y {
            format!(
                "{}x = {:.*}\ny = {:.*}",
                prefix, x_decimals, value.x, y_decimals, value.y
            )
        } else if plot.show_x {
            format!("{}x = {:.*}", prefix, x_decimals, value.x)
        } else if plot.show_y {
            format!("{}y = {:.*}", prefix, y_decimals, value.y)
        } else {
            unreachable!()
        }
    };

    let font_id = TextStyle::Body.resolve(plot.ui.style());
    plot.ui.fonts(|f| {
        shapes.push(Shape::text(
            f,
            pointer + vec2(3.0, -2.0),
            Align2::LEFT_BOTTOM,
            text,
            font_id,
            plot.ui.visuals().text_color(),
        ));
    });
}

fn find_closest_rect<'a, T>(
    rects: impl IntoIterator<Item = &'a T>,
    point: Pos2,
    transform: &ScreenTransform,
) -> Option<ClosestElem>
where
    T: 'a + RectElement,
{
    rects
        .into_iter()
        .enumerate()
        .map(|(index, bar)| {
            let bar_rect: Rect = transform.rect_from_values(&bar.bounds_min(), &bar.bounds_max());
            let dist_sq = bar_rect.distance_sq_to_pos(point);

            ClosestElem { index, dist_sq }
        })
        .min_by_key(|e| e.dist_sq.ord())
}
