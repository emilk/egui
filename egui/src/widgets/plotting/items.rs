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

enum Generator {
    /// Describes a function y = f(x) with an optional range for x and a number of points.
    Explicit(Box<dyn Fn(f64) -> f64>, Option<RangeInclusive<f64>>, usize),
    /// Describes a function (x,y) = f(t) with a range for t and a number of points.
    Parametric(Box<dyn Fn(f64) -> (f64, f64)>, RangeInclusive<f64>, usize),
}

// ----------------------------------------------------------------------------

/// A series of values forming a path.
pub struct Curve {
    pub(crate) values: Vec<Value>,
    generator: Option<Generator>,
    pub(crate) bounds: Bounds,
    pub(crate) stroke: Stroke,
    pub(crate) name: String,
}

impl Curve {
    fn empty() -> Self {
        Self {
            values: Vec::new(),
            generator: None,
            bounds: Bounds::NOTHING,
            stroke: Stroke::new(2.0, Color32::TRANSPARENT),
            name: Default::default(),
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

    /// Draw a curve based on a function `y=f(x)`, an optional range for x and the number of points.
    pub fn from_explicit_callback(
        function: impl Fn(f64) -> f64 + 'static,
        range: Option<RangeInclusive<f64>>,
        points: usize,
    ) -> Self {
        let generator = Generator::Explicit(Box::new(function), range, points);
        Self {
            generator: Some(generator),
            ..Self::empty()
        }
    }

    /// Draw a curve based on a function `(x,y)=f(t)`, a range for t and the number of points.
    pub fn from_parametric_callback(
        function: impl Fn(f64) -> (f64, f64) + 'static,
        t_range: RangeInclusive<f64>,
        points: usize,
    ) -> Self {
        let generator = Generator::Parametric(Box::new(function), t_range, points);
        Self {
            generator: Some(generator),
            ..Self::empty()
        }
    }

    /// Returns true if there are no data points available and there is no function to generate any.
    pub(crate) fn no_data(&self) -> bool {
        self.generator.is_none() && self.values.is_empty()
    }

    /// If initialized with a generator function, this will generate `n` evenly spaced points in the
    /// given range.
    pub(crate) fn generate_points(&mut self, mut x_range: RangeInclusive<f64>) {
        let range_union = |range1: &RangeInclusive<f64>, range2: &RangeInclusive<f64>| {
            range1.start().max(*range2.start())..=range1.end().min(*range2.end())
        };
        match &self.generator {
            Some(Generator::Explicit(fun, maybe_range, n)) => {
                if let Some(range) = maybe_range {
                    x_range = range_union(&x_range, range);
                }
                let increment = (x_range.end() - x_range.start()) / (n - 1) as f64;
                self.values = (0..*n)
                    .map(|i| {
                        let x = x_range.start() + i as f64 * increment;
                        Value { x, y: fun(x) }
                    })
                    .collect();
            }
            Some(Generator::Parametric(fun, range, n)) => {
                let increment = (range.end() - range.start()) / (n - 1) as f64;
                self.values = (0..*n)
                    .map(|i| {
                        let t = range.start() + i as f64 * increment;
                        let (x, y) = fun(t);
                        Value { x, y }
                    })
                    .collect();
            }
            None => {}
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

    /// Name of this curve.
    #[allow(clippy::needless_pass_by_value)]
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }
}
