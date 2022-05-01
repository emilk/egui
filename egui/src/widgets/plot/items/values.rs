use epaint::{Pos2, Shape, Stroke, Vec2};
use std::ops::{Bound, RangeBounds, RangeInclusive};

use crate::plot::transform::PlotBounds;

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

    #[inline(always)]
    pub fn to_pos2(self) -> Pos2 {
        Pos2::new(self.x as f32, self.y as f32)
    }

    #[inline(always)]
    pub fn to_vec2(self) -> Vec2 {
        Vec2::new(self.x as f32, self.y as f32)
    }
}

// ----------------------------------------------------------------------------

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum LineStyle {
    Solid,
    Dotted { spacing: f32 },
    Dashed { length: f32 },
}

impl LineStyle {
    pub fn dashed_loose() -> Self {
        Self::Dashed { length: 10.0 }
    }

    pub fn dashed_dense() -> Self {
        Self::Dashed { length: 5.0 }
    }

    pub fn dotted_loose() -> Self {
        Self::Dotted { spacing: 10.0 }
    }

    pub fn dotted_dense() -> Self {
        Self::Dotted { spacing: 5.0 }
    }

    pub(super) fn style_line(
        &self,
        line: Vec<Pos2>,
        mut stroke: Stroke,
        highlight: bool,
        shapes: &mut Vec<Shape>,
    ) {
        match line.len() {
            0 => {}
            1 => {
                let mut radius = stroke.width / 2.0;
                if highlight {
                    radius *= 2f32.sqrt();
                }
                shapes.push(Shape::circle_filled(line[0], radius, stroke.color));
            }
            _ => {
                match self {
                    LineStyle::Solid => {
                        if highlight {
                            stroke.width *= 2.0;
                        }
                        shapes.push(Shape::line(line, stroke));
                    }
                    LineStyle::Dotted { spacing } => {
                        // Take the stroke width for the radius even though it's not "correct", otherwise
                        // the dots would become too small.
                        let mut radius = stroke.width;
                        if highlight {
                            radius *= 2f32.sqrt();
                        }
                        shapes.extend(Shape::dotted_line(&line, stroke.color, *spacing, radius));
                    }
                    LineStyle::Dashed { length } => {
                        if highlight {
                            stroke.width *= 2.0;
                        }
                        let golden_ratio = (5.0_f32.sqrt() - 1.0) / 2.0; // 0.61803398875
                        shapes.extend(Shape::dashed_line(
                            &line,
                            stroke,
                            *length,
                            length * golden_ratio,
                        ));
                    }
                }
            }
        }
    }
}

impl ToString for LineStyle {
    fn to_string(&self) -> String {
        match self {
            LineStyle::Solid => "Solid".into(),
            LineStyle::Dotted { spacing } => format!("Dotted{}Px", spacing),
            LineStyle::Dashed { length } => format!("Dashed{}Px", length),
        }
    }
}

// ----------------------------------------------------------------------------

/// Determines whether a plot element is vertically or horizontally oriented.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

impl Default for Orientation {
    fn default() -> Self {
        Self::Vertical
    }
}

// ----------------------------------------------------------------------------

#[derive(Default)]
pub struct Values {
    pub(super) values: Vec<Value>,
    generator: Option<ExplicitGenerator>,
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
        x_range: impl RangeBounds<f64>,
        points: usize,
    ) -> Self {
        let start = match x_range.start_bound() {
            Bound::Included(x) | Bound::Excluded(x) => *x,
            Bound::Unbounded => f64::NEG_INFINITY,
        };
        let end = match x_range.end_bound() {
            Bound::Included(x) | Bound::Excluded(x) => *x,
            Bound::Unbounded => f64::INFINITY,
        };
        let x_range = start..=end;

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
    /// The range may be specified as start..end or as start..=end.
    pub fn from_parametric_callback(
        function: impl Fn(f64) -> (f64, f64),
        t_range: impl RangeBounds<f64>,
        points: usize,
    ) -> Self {
        let start = match t_range.start_bound() {
            Bound::Included(x) => x,
            Bound::Excluded(_) => unreachable!(),
            Bound::Unbounded => panic!("The range for parametric functions must be bounded!"),
        };
        let end = match t_range.end_bound() {
            Bound::Included(x) | Bound::Excluded(x) => x,
            Bound::Unbounded => panic!("The range for parametric functions must be bounded!"),
        };
        let last_point_included = matches!(t_range.end_bound(), Bound::Included(_));
        let increment = if last_point_included {
            (end - start) / (points - 1) as f64
        } else {
            (end - start) / points as f64
        };
        let values = (0..points).map(|i| {
            let t = start + i as f64 * increment;
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
    pub(crate) fn is_empty(&self) -> bool {
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

    pub(super) fn get_bounds(&self) -> PlotBounds {
        if self.values.is_empty() {
            if let Some(generator) = &self.generator {
                generator.estimate_bounds()
            } else {
                PlotBounds::NOTHING
            }
        } else {
            let mut bounds = PlotBounds::NOTHING;
            for value in &self.values {
                bounds.extend_with(value);
            }
            bounds
        }
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
    pub fn all() -> impl ExactSizeIterator<Item = MarkerShape> {
        [
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
        .iter()
        .copied()
    }
}

// ----------------------------------------------------------------------------

/// Query the values of the plot, for geometric relations like closest checks
pub(crate) enum PlotGeometry<'a> {
    /// No geometry based on single elements (examples: text, image, horizontal/vertical line)
    None,

    /// Point values (X-Y graphs)
    Points(&'a [Value]),

    /// Rectangles (examples: boxes or bars)
    // Has currently no data, as it would require copying rects or iterating a list of pointers.
    // Instead, geometry-based functions are directly implemented in the respective PlotItem impl.
    Rects,
}

// ----------------------------------------------------------------------------

/// Describes a function y = f(x) with an optional range for x and a number of points.
struct ExplicitGenerator {
    function: Box<dyn Fn(f64) -> f64>,
    x_range: RangeInclusive<f64>,
    points: usize,
}

impl ExplicitGenerator {
    fn estimate_bounds(&self) -> PlotBounds {
        let min_x = *self.x_range.start();
        let max_x = *self.x_range.end();
        let min_y = (self.function)(min_x);
        let max_y = (self.function)(max_x);
        // TODO: sample some more points
        PlotBounds {
            min: [min_x, min_y],
            max: [max_x, max_y],
        }
    }
}

// ----------------------------------------------------------------------------

/// Result of [`super::PlotItem::find_closest()`] search, identifies an element inside the item for immediate use
pub(crate) struct ClosestElem {
    /// Position of hovered-over value (or bar/box-plot/...) in PlotItem
    pub index: usize,

    /// Squared distance from the mouse cursor (needed to compare against other PlotItems, which might be nearer)
    pub dist_sq: f32,
}
