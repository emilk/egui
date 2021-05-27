//! Contains items that can be added to a plot.

use std::ops::RangeInclusive;

use super::transform::{Bounds, ScreenTransform};
use crate::*;

/// A value in the value-space of the plot.
///
/// Uses f64 for improved accuracy to enable plotting
/// large values (e.g. unix time on x axis).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Value {
    /// This is often something monotonically increasing, such as time, but doesn't have to be.
    /// Goes from left to right.
    pub x: f64,
    /// Goes from bottom to top (inverse of everything else in egui!).
    pub y: f64,
}

impl Value {
    #[inline(always)]
    pub fn new(x: impl Into<f64>, y: impl Into<f64>) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
        }
    }
}

// ----------------------------------------------------------------------------

/// A horizontal line in a plot, filling the full width
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HLine {
    pub(super) y: f64,
    pub(super) stroke: Stroke,
}

impl HLine {
    pub fn new(y: impl Into<f64>, stroke: impl Into<Stroke>) -> Self {
        Self {
            y: y.into(),
            stroke: stroke.into(),
        }
    }
}

/// A vertical line in a plot, filling the full width
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VLine {
    pub(super) x: f64,
    pub(super) stroke: Stroke,
}

impl VLine {
    pub fn new(x: impl Into<f64>, stroke: impl Into<Stroke>) -> Self {
        Self {
            x: x.into(),
            stroke: stroke.into(),
        }
    }
}

pub(super) trait PlotItem {
    fn get_shapes(&self, transform: &ScreenTransform, shapes: &mut Vec<Shape>);
    fn series(&self) -> &Values;
    fn series_mut(&mut self) -> &mut Values;
    fn name(&self) -> &str;
    fn color(&self) -> Color32;
    fn highlight(&mut self);
}

// ----------------------------------------------------------------------------

/// Describes a function y = f(x) with an optional range for x and a number of points.
struct ExplicitGenerator {
    function: Box<dyn Fn(f64) -> f64>,
    x_range: RangeInclusive<f64>,
    points: usize,
}

pub struct Values {
    pub(super) values: Vec<Value>,
    generator: Option<ExplicitGenerator>,
}

impl Default for Values {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            generator: None,
        }
    }
}

impl Values {
    pub fn from_values(values: Vec<Value>) -> Self {
        Self {
            values,
            generator: None,
        }
    }

    pub fn from_values_iter(iter: impl Iterator<Item = Value>) -> Self {
        Self::from_values(iter.collect())
    }

    /// Draw a line based on a function `y=f(x)`, a range (which can be infinite) for x and the number of points.
    pub fn from_explicit_callback(
        function: impl Fn(f64) -> f64 + 'static,
        x_range: RangeInclusive<f64>,
        points: usize,
    ) -> Self {
        let generator = ExplicitGenerator {
            function: Box::new(function),
            x_range,
            points,
        };

        Self {
            values: Vec::new(),
            generator: Some(generator),
        }
    }

    /// Draw a line based on a function `(x,y)=f(t)`, a range for t and the number of points.
    pub fn from_parametric_callback(
        function: impl Fn(f64) -> (f64, f64),
        t_range: RangeInclusive<f64>,
        points: usize,
    ) -> Self {
        let increment = (t_range.end() - t_range.start()) / (points - 1) as f64;
        let values = (0..points).map(|i| {
            let t = t_range.start() + i as f64 * increment;
            let (x, y) = function(t);
            Value { x, y }
        });
        Self::from_values_iter(values)
    }

    /// From a series of y-values.
    /// The x-values will be the indices of these values
    pub fn from_ys_f32(ys: &[f32]) -> Self {
        let values: Vec<Value> = ys
            .iter()
            .enumerate()
            .map(|(i, &y)| Value {
                x: i as f64,
                y: y as f64,
            })
            .collect();
        Self::from_values(values)
    }

    /// Returns true if there are no data points available and there is no function to generate any.
    pub(super) fn is_empty(&self) -> bool {
        self.generator.is_none() && self.values.is_empty()
    }

    /// If initialized with a generator function, this will generate `n` evenly spaced points in the
    /// given range.
    pub(super) fn generate_points(&mut self, x_range: RangeInclusive<f64>) {
        if let Some(generator) = self.generator.take() {
            if let Some(intersection) = Self::range_intersection(&x_range, &generator.x_range) {
                let increment =
                    (intersection.end() - intersection.start()) / (generator.points - 1) as f64;
                self.values = (0..generator.points)
                    .map(|i| {
                        let x = intersection.start() + i as f64 * increment;
                        let y = (generator.function)(x);
                        Value { x, y }
                    })
                    .collect();
            }
        }
    }

    /// Returns the intersection of two ranges if they intersect.
    fn range_intersection(
        range1: &RangeInclusive<f64>,
        range2: &RangeInclusive<f64>,
    ) -> Option<RangeInclusive<f64>> {
        let start = range1.start().max(*range2.start());
        let end = range1.end().min(*range2.end());
        (start < end).then(|| start..=end)
    }

    pub(super) fn get_bounds(&self) -> Bounds {
        let mut bounds = Bounds::NOTHING;
        self.values
            .iter()
            .for_each(|value| bounds.extend_with(value));
        bounds
    }
}

// ----------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MarkerShape {
    Circle,
    Diamond,
    Square,
    Cross,
    Plus,
    Up,
    Down,
    Left,
    Right,
    Asterisk,
}

impl MarkerShape {
    /// Get a vector containing all marker shapes.
    pub fn all() -> Vec<Self> {
        vec![
            Self::Circle,
            Self::Diamond,
            Self::Square,
            Self::Cross,
            Self::Plus,
            Self::Up,
            Self::Down,
            Self::Left,
            Self::Right,
            Self::Asterisk,
        ]
    }
}

/// A series of values forming a path.
pub struct Line {
    pub(super) series: Values,
    pub(super) stroke: Stroke,
    pub(super) name: String,
    pub(super) highlight: bool,
}

impl Line {
    pub fn new(series: Values) -> Self {
        Self {
            series,
            stroke: Stroke::new(1.0, Color32::TRANSPARENT),
            name: Default::default(),
            highlight: false,
        }
    }

    /// Highlight this line in the plot by scaling up the line and marker size.
    pub fn highlight(mut self) -> Self {
        self.highlight = true;
        self
    }

    /// Add a stroke.
    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Stroke width. A high value means the plot thickens.
    pub fn width(mut self, width: f32) -> Self {
        self.stroke.width = width;
        self
    }

    /// Stroke color. Default is `Color32::TRANSPARENT` which means a color will be auto-assigned.
    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        self.stroke.color = color.into();
        self
    }

    /// Name of this line.
    ///
    /// This name will show up in the plot legend, if legends are turned on.
    #[allow(clippy::needless_pass_by_value)]
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }
}

impl PlotItem for Line {
    fn get_shapes(&self, transform: &ScreenTransform, shapes: &mut Vec<Shape>) {
        let Self {
            series,
            mut stroke,
            highlight,
            ..
        } = self;

        if *highlight {
            stroke.width *= 2.0;
        }

        let values_tf: Vec<_> = series
            .values
            .iter()
            .map(|v| transform.position_from_value(v))
            .collect();

        let line_shape = if values_tf.len() > 1 {
            Shape::line(values_tf, stroke)
        } else {
            Shape::circle_filled(values_tf[0], stroke.width / 2.0, stroke.color)
        };
        shapes.push(line_shape);
    }

    fn series(&self) -> &Values {
        &self.series
    }

    fn series_mut(&mut self) -> &mut Values {
        &mut self.series
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
}

/// A set of points.
pub struct Points {
    pub(super) series: Values,
    pub(super) shape: MarkerShape,
    /// Color of the marker. `Color32::TRANSPARENT` means that it will be picked automatically.
    pub(super) color: Color32,
    /// Whether to fill the marker. Does not apply to all types.
    pub(super) filled: bool,
    /// The maximum extent of the marker from its center.
    pub(super) radius: f32,
    pub(super) name: String,
    pub(super) highlight: bool,
}

impl Points {
    pub fn new(series: Values) -> Self {
        Self {
            series,
            shape: MarkerShape::Circle,
            color: Color32::TRANSPARENT,
            filled: true,
            radius: 1.0,
            name: Default::default(),
            highlight: false,
        }
    }

    /// Set the shape of the markers.
    pub fn shape(mut self, shape: MarkerShape) -> Self {
        self.shape = shape;
        self
    }

    /// Highlight these points in the plot by scaling up their markers.
    pub fn highlight(mut self) -> Self {
        self.highlight = true;
        self
    }

    /// Set the marker's color.
    pub fn color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }

    /// Whether to fill the marker.
    pub fn filled(mut self, filled: bool) -> Self {
        self.filled = filled;
        self
    }

    /// Set the maximum extent of the marker around its position.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Name of this series of markers.
    ///
    /// This name will show up in the plot legend, if legends are turned on.
    #[allow(clippy::needless_pass_by_value)]
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }
}

impl PlotItem for Points {
    fn get_shapes(&self, transform: &ScreenTransform, shapes: &mut Vec<Shape>) {
        let sqrt_3 = 3f32.sqrt();
        let frac_sqrt_3_2 = 3f32.sqrt() / 2.0;
        let frac_1_sqrt_2 = 1.0 / 2f32.sqrt();

        let Self {
            series,
            shape,
            color,
            filled,
            mut radius,
            highlight,
            ..
        } = self;

        if *highlight {
            radius *= 2f32.sqrt();
        }

        let stroke_size = radius / 5.0;

        let default_stroke = Stroke::new(stroke_size, *color);
        let stroke = (!filled).then(|| default_stroke).unwrap_or_default();
        let fill = filled.then(|| *color).unwrap_or_default();

        series
            .values
            .iter()
            .map(|value| transform.position_from_value(value))
            .for_each(|center| {
                let tf = |dx: f32, dy: f32| -> Pos2 { center + radius * vec2(dx, dy) };

                match shape {
                    MarkerShape::Circle => {
                        shapes.push(Shape::Circle {
                            center,
                            radius,
                            fill,
                            stroke,
                        });
                    }
                    MarkerShape::Diamond => {
                        let points = vec![tf(1.0, 0.0), tf(0.0, -1.0), tf(-1.0, 0.0), tf(0.0, 1.0)];
                        shapes.push(Shape::Path {
                            points,
                            closed: true,
                            fill,
                            stroke,
                        });
                    }
                    MarkerShape::Square => {
                        let points = vec![
                            tf(frac_1_sqrt_2, frac_1_sqrt_2),
                            tf(frac_1_sqrt_2, -frac_1_sqrt_2),
                            tf(-frac_1_sqrt_2, -frac_1_sqrt_2),
                            tf(-frac_1_sqrt_2, frac_1_sqrt_2),
                        ];
                        shapes.push(Shape::Path {
                            points,
                            closed: true,
                            fill,
                            stroke,
                        });
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
                            vec![tf(0.0, -1.0), tf(-0.5 * sqrt_3, 0.5), tf(0.5 * sqrt_3, 0.5)];
                        shapes.push(Shape::Path {
                            points,
                            closed: true,
                            fill,
                            stroke,
                        });
                    }
                    MarkerShape::Down => {
                        let points = vec![
                            tf(0.0, 1.0),
                            tf(-0.5 * sqrt_3, -0.5),
                            tf(0.5 * sqrt_3, -0.5),
                        ];
                        shapes.push(Shape::Path {
                            points,
                            closed: true,
                            fill,
                            stroke,
                        });
                    }
                    MarkerShape::Left => {
                        let points =
                            vec![tf(-1.0, 0.0), tf(0.5, -0.5 * sqrt_3), tf(0.5, 0.5 * sqrt_3)];
                        shapes.push(Shape::Path {
                            points,
                            closed: true,
                            fill,
                            stroke,
                        });
                    }
                    MarkerShape::Right => {
                        let points = vec![
                            tf(1.0, 0.0),
                            tf(-0.5, -0.5 * sqrt_3),
                            tf(-0.5, 0.5 * sqrt_3),
                        ];
                        shapes.push(Shape::Path {
                            points,
                            closed: true,
                            fill,
                            stroke,
                        });
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

    fn series(&self) -> &Values {
        &self.series
    }

    fn series_mut(&mut self) -> &mut Values {
        &mut self.series
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
}
