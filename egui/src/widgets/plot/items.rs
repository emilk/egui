//! Contains items that can be added to a plot.

use std::ops::RangeInclusive;

use super::transform::Bounds;
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
    pub(crate) y: f64,
    pub(crate) stroke: Stroke,
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
    pub(crate) x: f64,
    pub(crate) stroke: Stroke,
}

impl VLine {
    pub fn new(x: impl Into<f64>, stroke: impl Into<Stroke>) -> Self {
        Self {
            x: x.into(),
            stroke: stroke.into(),
        }
    }
}

// ----------------------------------------------------------------------------

/// Describes a function y = f(x) with an optional range for x and a number of points.
struct ExplicitGenerator {
    function: Box<dyn Fn(f64) -> f64>,
    x_range: RangeInclusive<f64>,
    points: usize,
}

// ----------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum MarkerShape {
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

#[derive(Debug, Clone, Copy)]
pub struct Marker {
    pub(crate) shape: MarkerShape,
    /// Color of the marker. `Color32::TRANSPARENT` means that it will be picked automatically.
    pub(crate) color: Color32,
    /// Whether to fill the marker. Does not apply to all types.
    pub(crate) filled: bool,
    /// The maximum extent of the marker from its center.
    pub(crate) radius: f32,
}

impl Default for Marker {
    fn default() -> Self {
        Self {
            shape: MarkerShape::Circle,
            color: Color32::TRANSPARENT,
            filled: true,
            radius: 2.0,
        }
    }
}

impl Marker {
    /// Get a vector containing a marker of each shape.
    pub fn all() -> Vec<Self> {
        vec![
            Self::circle(),
            Self::diamond(),
            Self::square(),
            Self::cross(),
            Self::plus(),
            Self::up(),
            Self::down(),
            Self::left(),
            Self::right(),
            Self::asterisk(),
        ]
    }

    pub fn circle() -> Self {
        Self {
            shape: MarkerShape::Circle,
            ..Default::default()
        }
    }

    pub fn diamond() -> Self {
        Self {
            shape: MarkerShape::Diamond,
            ..Default::default()
        }
    }

    pub fn square() -> Self {
        Self {
            shape: MarkerShape::Square,
            ..Default::default()
        }
    }

    pub fn cross() -> Self {
        Self {
            shape: MarkerShape::Cross,
            ..Default::default()
        }
    }

    pub fn plus() -> Self {
        Self {
            shape: MarkerShape::Plus,
            ..Default::default()
        }
    }

    pub fn up() -> Self {
        Self {
            shape: MarkerShape::Up,
            ..Default::default()
        }
    }

    pub fn down() -> Self {
        Self {
            shape: MarkerShape::Down,
            ..Default::default()
        }
    }

    pub fn left() -> Self {
        Self {
            shape: MarkerShape::Left,
            ..Default::default()
        }
    }

    pub fn right() -> Self {
        Self {
            shape: MarkerShape::Right,
            ..Default::default()
        }
    }

    pub fn asterisk() -> Self {
        Self {
            shape: MarkerShape::Asterisk,
            ..Default::default()
        }
    }

    /// Set the marker's color. Defaults to the curve's color.
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

    pub(crate) fn get_shapes(&self, position: &Pos2) -> Vec<Shape> {
        let sqrt_3 = 3f32.sqrt();
        let frac_sqrt_3_2 = 3f32.sqrt() / 2.0;
        let frac_1_sqrt_2 = 1.0 / 2f32.sqrt();

        let Self {
            color,
            filled,
            shape,
            radius,
        } = *self;

        if color == Color32::TRANSPARENT {
            return Vec::new();
        }

        let stroke_size = radius / 5.0;

        let tf = |offset: Vec<Vec2>| -> Vec<Pos2> {
            offset
                .into_iter()
                .map(|offset| *position + radius * offset)
                .collect()
        };

        let default_stroke = Stroke::new(stroke_size, color);
        let stroke = (!filled).then(|| default_stroke).unwrap_or_default();
        let fill = filled.then(|| color).unwrap_or_default();

        match shape {
            MarkerShape::Circle => {
                vec![Shape::Circle {
                    center: *position,
                    radius,
                    fill,
                    stroke,
                }]
            }
            MarkerShape::Diamond => {
                let offsets = vec![
                    vec2(1.0, 0.0),
                    vec2(0.0, -1.0),
                    vec2(-1.0, 0.0),
                    vec2(0.0, 1.0),
                ];
                let points = tf(offsets);
                vec![Shape::Path {
                    points,
                    closed: true,
                    fill,
                    stroke,
                }]
            }
            MarkerShape::Square => {
                let offsets = vec![
                    vec2(frac_1_sqrt_2, frac_1_sqrt_2),
                    vec2(frac_1_sqrt_2, -frac_1_sqrt_2),
                    vec2(-frac_1_sqrt_2, -frac_1_sqrt_2),
                    vec2(-frac_1_sqrt_2, frac_1_sqrt_2),
                ];
                let points = tf(offsets);
                vec![Shape::Path {
                    points,
                    closed: true,
                    fill,
                    stroke,
                }]
            }
            MarkerShape::Cross => {
                let diagonal1 = tf(vec![
                    vec2(-frac_1_sqrt_2, -frac_1_sqrt_2),
                    vec2(frac_1_sqrt_2, frac_1_sqrt_2),
                ]);
                let diagonal2 = tf(vec![
                    vec2(frac_1_sqrt_2, -frac_1_sqrt_2),
                    vec2(-frac_1_sqrt_2, frac_1_sqrt_2),
                ]);
                vec![
                    Shape::line(diagonal1, default_stroke),
                    Shape::line(diagonal2, default_stroke),
                ]
            }
            MarkerShape::Plus => {
                let horizontal = tf(vec![vec2(-1.0, 0.0), vec2(1.0, 0.0)]);
                let vertical = tf(vec![vec2(0.0, -1.0), vec2(0.0, 1.0)]);
                vec![
                    Shape::line(horizontal, default_stroke),
                    Shape::line(vertical, default_stroke),
                ]
            }
            MarkerShape::Up => {
                let offsets = vec![
                    vec2(0.0, -1.0),
                    vec2(-0.5 * sqrt_3, 0.5),
                    vec2(0.5 * sqrt_3, 0.5),
                ];
                let points = tf(offsets);
                vec![Shape::Path {
                    points,
                    closed: true,
                    fill,
                    stroke,
                }]
            }
            MarkerShape::Down => {
                let offsets = vec![
                    vec2(0.0, 1.0),
                    vec2(-0.5 * sqrt_3, -0.5),
                    vec2(0.5 * sqrt_3, -0.5),
                ];
                let points = tf(offsets);
                vec![Shape::Path {
                    points,
                    closed: true,
                    fill,
                    stroke,
                }]
            }
            MarkerShape::Left => {
                let offsets = vec![
                    vec2(-1.0, 0.0),
                    vec2(0.5, -0.5 * sqrt_3),
                    vec2(0.5, 0.5 * sqrt_3),
                ];
                let points = tf(offsets);
                vec![Shape::Path {
                    points,
                    closed: true,
                    fill,
                    stroke,
                }]
            }
            MarkerShape::Right => {
                let offsets = vec![
                    vec2(1.0, 0.0),
                    vec2(-0.5, -0.5 * sqrt_3),
                    vec2(-0.5, 0.5 * sqrt_3),
                ];
                let points = tf(offsets);
                vec![Shape::Path {
                    points,
                    closed: true,
                    fill,
                    stroke,
                }]
            }
            MarkerShape::Asterisk => {
                let vertical = tf(vec![vec2(0.0, -1.0), vec2(0.0, 1.0)]);
                let diagonal1 = tf(vec![vec2(-frac_sqrt_3_2, 0.5), vec2(frac_sqrt_3_2, -0.5)]);
                let diagonal2 = tf(vec![vec2(-frac_sqrt_3_2, -0.5), vec2(frac_sqrt_3_2, 0.5)]);
                vec![
                    Shape::line(vertical, default_stroke),
                    Shape::line(diagonal1, default_stroke),
                    Shape::line(diagonal2, default_stroke),
                ]
            }
        }
    }
}

/// A series of values forming a path.
pub struct Curve {
    pub(crate) values: Vec<Value>,
    generator: Option<ExplicitGenerator>,
    pub(crate) bounds: Bounds,
    pub(crate) marker: Option<Marker>,
    pub(crate) color: Option<Color32>,
    pub(crate) width: f32,
    pub(crate) name: String,
    pub(crate) highlight: bool,
}

impl Curve {
    fn empty() -> Self {
        Self {
            values: Vec::new(),
            generator: None,
            bounds: Bounds::NOTHING,
            marker: None,
            color: None,
            width: 1.0,
            name: Default::default(),
            highlight: false,
        }
    }

    pub fn from_values(values: Vec<Value>) -> Self {
        let mut bounds = Bounds::NOTHING;
        for value in &values {
            bounds.extend_with(value);
        }
        Self {
            values,
            bounds,
            ..Self::empty()
        }
    }

    pub fn from_values_iter(iter: impl Iterator<Item = Value>) -> Self {
        Self::from_values(iter.collect())
    }

    /// Draw a curve based on a function `y=f(x)`, a range (which can be infinite) for x and the number of points.
    pub fn from_explicit_callback(
        function: impl Fn(f64) -> f64 + 'static,
        x_range: RangeInclusive<f64>,
        points: usize,
    ) -> Self {
        let mut bounds = Bounds::NOTHING;
        if x_range.start().is_finite() && x_range.end().is_finite() {
            bounds.min[0] = *x_range.start();
            bounds.max[0] = *x_range.end();
        }

        let generator = ExplicitGenerator {
            function: Box::new(function),
            x_range,
            points,
        };

        Self {
            generator: Some(generator),
            bounds,
            ..Self::empty()
        }
    }

    /// Draw a curve based on a function `(x,y)=f(t)`, a range for t and the number of points.
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

    /// Returns true if there are no data points available and there is no function to generate any.
    pub(crate) fn no_data(&self) -> bool {
        self.generator.is_none() && self.values.is_empty()
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

    /// If initialized with a generator function, this will generate `n` evenly spaced points in the
    /// given range.
    pub(crate) fn generate_points(&mut self, x_range: RangeInclusive<f64>) {
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

    /// Highlight this curve in the plot by scaling up the line and marker size.
    pub fn highlight(mut self) -> Self {
        self.highlight = true;
        self
    }

    /// Add a stroke.
    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        let stroke: Stroke = stroke.into();
        self.color = Some(stroke.color);
        self.width = stroke.width;
        self
    }

    /// Add a marker for all data points.
    pub fn marker(mut self, marker: Marker) -> Self {
        self.marker = Some(marker);
        self
    }

    /// Stroke width. A high value means the plot thickens.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Stroke color.
    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Name of this curve.
    ///
    /// If a curve is given a name it will show up in the plot legend
    /// (if legends are turned on).
    #[allow(clippy::needless_pass_by_value)]
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }

    /// Return the color by which the curve can be identified.
    pub(crate) fn get_color(&self) -> Option<Color32> {
        self.color.filter(|color| color.a() != 0).or_else(|| {
            self.marker
                .map(|marker| marker.color)
                .filter(|color| *color != Color32::TRANSPARENT)
        })
    }
}
