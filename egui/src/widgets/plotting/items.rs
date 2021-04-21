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

/// Contains a function to generate plot points and an optional range for which to generate them.
pub struct ExplicitGenerator {
    function: Box<dyn Fn(f64) -> f64>,
    range: Option<RangeInclusive<f64>>,
}

// ----------------------------------------------------------------------------

/// A series of values forming a path.
pub struct Curve {
    pub(crate) values: Vec<Value>,
    pub(crate) generator: Option<ExplicitGenerator>,
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

    pub fn from_explicit_callback(
        function: impl Fn(f64) -> f64 + 'static,
        range: Option<RangeInclusive<f64>>,
    ) -> Self {
        let generator = ExplicitGenerator {
            function: Box::new(function),
            range,
        };
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
    pub(crate) fn generate_points(&mut self, mut x_range: RangeInclusive<f64>, n: usize) {
        if let Some(function) = self.generator.as_ref() {
            if let Some(range) = &function.range {
                x_range = x_range.start().max(*range.start())..=x_range.end().min(*range.end());
            }

            let increment = (x_range.end() - x_range.start()) / (n - 1) as f64;

            self.values = (0..n)
                .map(|i| {
                    let x = x_range.start() + i as f64 * increment;
                    Value {
                        x,
                        y: (function.function)(x),
                    }
                })
                .collect();
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
