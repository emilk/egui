use std::ops::{Bound, RangeBounds, RangeInclusive};

use egui::{Pos2, Shape, Stroke, Vec2};

use crate::transform::PlotBounds;

/// A point coordinate in the plot.
///
/// Uses f64 for improved accuracy to enable plotting
/// large values (e.g. unix time on x axis).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlotPoint {
    /// This is often something monotonically increasing, such as time, but doesn't have to be.
    /// Goes from left to right.
    pub x: f64,

    /// Goes from bottom to top (inverse of everything else in egui!).
    pub y: f64,
}

impl From<[f64; 2]> for PlotPoint {
    #[inline]
    fn from([x, y]: [f64; 2]) -> Self {
        Self { x, y }
    }
}

impl PlotPoint {
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

/// Solid, dotted, dashed, etc.
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
                    Self::Solid => {
                        if highlight {
                            stroke.width *= 2.0;
                        }
                        shapes.push(Shape::line(line, stroke));
                    }
                    Self::Dotted { spacing } => {
                        // Take the stroke width for the radius even though it's not "correct", otherwise
                        // the dots would become too small.
                        let mut radius = stroke.width;
                        if highlight {
                            radius *= 2f32.sqrt();
                        }
                        shapes.extend(Shape::dotted_line(&line, stroke.color, *spacing, radius));
                    }
                    Self::Dashed { length } => {
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
            Self::Solid => "Solid".into(),
            Self::Dotted { spacing } => format!("Dotted{spacing}Px"),
            Self::Dashed { length } => format!("Dashed{length}Px"),
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

/// Represents many [`PlotPoint`]s.
///
/// These can be an owned `Vec` or generated with a function.
pub enum PlotPoints {
    Owned(Vec<PlotPoint>),
    Generator(ExplicitGenerator),
    // Borrowed(&[PlotPoint]), // TODO(EmbersArc): Lifetimes are tricky in this case.
}

impl Default for PlotPoints {
    fn default() -> Self {
        Self::Owned(Vec::new())
    }
}

impl From<[f64; 2]> for PlotPoints {
    fn from(coordinate: [f64; 2]) -> Self {
        Self::new(vec![coordinate])
    }
}

impl From<Vec<[f64; 2]>> for PlotPoints {
    fn from(coordinates: Vec<[f64; 2]>) -> Self {
        Self::new(coordinates)
    }
}

impl FromIterator<[f64; 2]> for PlotPoints {
    fn from_iter<T: IntoIterator<Item = [f64; 2]>>(iter: T) -> Self {
        Self::Owned(iter.into_iter().map(|point| point.into()).collect())
    }
}

impl PlotPoints {
    pub fn new(points: Vec<[f64; 2]>) -> Self {
        Self::from_iter(points)
    }

    pub fn points(&self) -> &[PlotPoint] {
        match self {
            Self::Owned(points) => points.as_slice(),
            Self::Generator(_) => &[],
        }
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

        Self::Generator(generator)
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
        (0..points)
            .map(|i| {
                let t = start + i as f64 * increment;
                function(t).into()
            })
            .collect()
    }

    /// From a series of y-values.
    /// The x-values will be the indices of these values
    pub fn from_ys_f32(ys: &[f32]) -> Self {
        ys.iter()
            .enumerate()
            .map(|(i, &y)| [i as f64, y as f64])
            .collect()
    }

    /// From a series of y-values.
    /// The x-values will be the indices of these values
    pub fn from_ys_f64(ys: &[f64]) -> Self {
        ys.iter().enumerate().map(|(i, &y)| [i as f64, y]).collect()
    }

    /// Returns true if there are no data points available and there is no function to generate any.
    pub(crate) fn is_empty(&self) -> bool {
        match self {
            Self::Owned(points) => points.is_empty(),
            Self::Generator(_) => false,
        }
    }

    /// If initialized with a generator function, this will generate `n` evenly spaced points in the
    /// given range.
    pub(super) fn generate_points(&mut self, x_range: RangeInclusive<f64>) {
        if let Self::Generator(generator) = self {
            *self = Self::range_intersection(&x_range, &generator.x_range)
                .map(|intersection| {
                    let increment =
                        (intersection.end() - intersection.start()) / (generator.points - 1) as f64;
                    (0..generator.points)
                        .map(|i| {
                            let x = intersection.start() + i as f64 * increment;
                            let y = (generator.function)(x);
                            [x, y]
                        })
                        .collect()
                })
                .unwrap_or_default();
        }
    }

    /// Returns the intersection of two ranges if they intersect.
    fn range_intersection(
        range1: &RangeInclusive<f64>,
        range2: &RangeInclusive<f64>,
    ) -> Option<RangeInclusive<f64>> {
        let start = range1.start().max(*range2.start());
        let end = range1.end().min(*range2.end());
        (start < end).then_some(start..=end)
    }

    pub(super) fn bounds(&self) -> PlotBounds {
        match self {
            Self::Owned(points) => {
                let mut bounds = PlotBounds::NOTHING;
                for point in points {
                    bounds.extend_with(point);
                }
                bounds
            }
            Self::Generator(generator) => generator.estimate_bounds(),
        }
    }
}

// ----------------------------------------------------------------------------

/// Circle, Diamond, Square, Cross, …
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
    pub fn all() -> impl ExactSizeIterator<Item = Self> {
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

/// Query the points of the plot, for geometric relations like closest checks
pub enum PlotGeometry<'a> {
    /// No geometry based on single elements (examples: text, image, horizontal/vertical line)
    None,

    /// Point values (X-Y graphs)
    Points(&'a [PlotPoint]),

    /// Rectangles (examples: boxes or bars)
    // Has currently no data, as it would require copying rects or iterating a list of pointers.
    // Instead, geometry-based functions are directly implemented in the respective PlotItem impl.
    Rects,
}

// ----------------------------------------------------------------------------

/// Describes a function y = f(x) with an optional range for x and a number of points.
pub struct ExplicitGenerator {
    function: Box<dyn Fn(f64) -> f64>,
    x_range: RangeInclusive<f64>,
    points: usize,
}

impl ExplicitGenerator {
    fn estimate_bounds(&self) -> PlotBounds {
        let mut bounds = PlotBounds::NOTHING;

        let mut add_x = |x: f64| {
            // avoid infinities, as we cannot auto-bound on them!
            if x.is_finite() {
                bounds.extend_with_x(x);
            }
            let y = (self.function)(x);
            if y.is_finite() {
                bounds.extend_with_y(y);
            }
        };

        let min_x = *self.x_range.start();
        let max_x = *self.x_range.end();

        add_x(min_x);
        add_x(max_x);

        if min_x.is_finite() && max_x.is_finite() {
            // Sample some points in the interval:
            const N: u32 = 8;
            for i in 1..N {
                let t = i as f64 / (N - 1) as f64;
                let x = crate::lerp(min_x..=max_x, t);
                add_x(x);
            }
        } else {
            // Try adding some points anyway:
            for x in [-1, 0, 1] {
                let x = x as f64;
                if min_x <= x && x <= max_x {
                    add_x(x);
                }
            }
        }

        bounds
    }
}

// ----------------------------------------------------------------------------

/// Result of [`super::PlotItem::find_closest()`] search, identifies an element inside the item for immediate use
pub struct ClosestElem {
    /// Position of hovered-over value (or bar/box-plot/...) in PlotItem
    pub index: usize,

    /// Squared distance from the mouse cursor (needed to compare against other PlotItems, which might be nearer)
    pub dist_sq: f32,
}
