use crate::emath::NumExt;
use crate::epaint::{Color32, RectShape, Rounding, Shape, Stroke};

use super::{add_rulers_and_text, highlighted_color, Orientation, PlotConfig, RectElement};
use crate::plot::{BoxPlot, Cursor, PlotPoint, ScreenTransform};

/// Contains the values of a single box in a box plot.
#[derive(Clone, Debug, PartialEq)]
pub struct BoxSpread {
    /// Value of lower whisker (typically minimum).
    ///
    /// The whisker is not drawn if `lower_whisker >= quartile1`.
    pub lower_whisker: f64,

    /// Value of lower box threshold (typically 25% quartile)
    pub quartile1: f64,

    /// Value of middle line in box (typically median)
    pub median: f64,

    /// Value of upper box threshold (typically 75% quartile)
    pub quartile3: f64,

    /// Value of upper whisker (typically maximum)
    ///
    /// The whisker is not drawn if `upper_whisker <= quartile3`.
    pub upper_whisker: f64,
}

impl BoxSpread {
    pub fn new(
        lower_whisker: f64,
        quartile1: f64,
        median: f64,
        quartile3: f64,
        upper_whisker: f64,
    ) -> Self {
        Self {
            lower_whisker,
            quartile1,
            median,
            quartile3,
            upper_whisker,
        }
    }
}

/// A box in a [`BoxPlot`] diagram. This is a low level graphical element; it will not compute quartiles and whiskers,
/// letting one use their preferred formula. Use [`Points`][`super::Points`] to draw the outliers.
#[derive(Clone, Debug, PartialEq)]
pub struct BoxElem {
    /// Name of plot element in the diagram (annotated by default formatter).
    pub name: String,

    /// Which direction the box faces in the diagram.
    pub orientation: Orientation,

    /// Position on the argument (input) axis -- X if vertical, Y if horizontal.
    pub argument: f64,

    /// Values of the box
    pub spread: BoxSpread,

    /// Thickness of the box
    pub box_width: f64,

    /// Width of the whisker at minimum/maximum
    pub whisker_width: f64,

    /// Line width and color
    pub stroke: Stroke,

    /// Fill color
    pub fill: Color32,
}

impl BoxElem {
    /// Create a box element. Its `orientation` is set by its [`BoxPlot`] parent.
    ///
    /// Check [`BoxElem`] fields for detailed description.
    pub fn new(argument: f64, spread: BoxSpread) -> Self {
        Self {
            argument,
            orientation: Orientation::default(),
            name: String::default(),
            spread,
            box_width: 0.25,
            whisker_width: 0.15,
            stroke: Stroke::new(1.0, Color32::TRANSPARENT),
            fill: Color32::TRANSPARENT,
        }
    }

    /// Name of this box element.
    #[allow(clippy::needless_pass_by_value)]
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }

    /// Add a custom stroke.
    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = stroke.into();
        self
    }

    /// Add a custom fill color.
    pub fn fill(mut self, color: impl Into<Color32>) -> Self {
        self.fill = color.into();
        self
    }

    /// Set the box width.
    pub fn box_width(mut self, width: f64) -> Self {
        self.box_width = width;
        self
    }

    /// Set the whisker width.
    pub fn whisker_width(mut self, width: f64) -> Self {
        self.whisker_width = width;
        self
    }

    /// Set orientation of the element as vertical. Argument axis is X.
    pub fn vertical(mut self) -> Self {
        self.orientation = Orientation::Vertical;
        self
    }

    /// Set orientation of the element as horizontal. Argument axis is Y.
    pub fn horizontal(mut self) -> Self {
        self.orientation = Orientation::Horizontal;
        self
    }

    pub(super) fn add_shapes(
        &self,
        transform: &ScreenTransform,
        highlighted: bool,
        shapes: &mut Vec<Shape>,
    ) {
        let (stroke, fill) = if highlighted {
            highlighted_color(self.stroke, self.fill)
        } else {
            (self.stroke, self.fill)
        };

        let rect = transform.rect_from_values(
            &self.point_at(self.argument - self.box_width / 2.0, self.spread.quartile1),
            &self.point_at(self.argument + self.box_width / 2.0, self.spread.quartile3),
        );
        let rect = Shape::Rect(RectShape {
            rect,
            rounding: Rounding::none(),
            fill,
            stroke,
        });
        shapes.push(rect);

        let line_between = |v1, v2| {
            Shape::line_segment(
                [
                    transform.position_from_point(&v1),
                    transform.position_from_point(&v2),
                ],
                stroke,
            )
        };
        let median = line_between(
            self.point_at(self.argument - self.box_width / 2.0, self.spread.median),
            self.point_at(self.argument + self.box_width / 2.0, self.spread.median),
        );
        shapes.push(median);

        if self.spread.upper_whisker > self.spread.quartile3 {
            let high_whisker = line_between(
                self.point_at(self.argument, self.spread.quartile3),
                self.point_at(self.argument, self.spread.upper_whisker),
            );
            shapes.push(high_whisker);
            if self.box_width > 0.0 {
                let high_whisker_end = line_between(
                    self.point_at(
                        self.argument - self.whisker_width / 2.0,
                        self.spread.upper_whisker,
                    ),
                    self.point_at(
                        self.argument + self.whisker_width / 2.0,
                        self.spread.upper_whisker,
                    ),
                );
                shapes.push(high_whisker_end);
            }
        }

        if self.spread.lower_whisker < self.spread.quartile1 {
            let low_whisker = line_between(
                self.point_at(self.argument, self.spread.quartile1),
                self.point_at(self.argument, self.spread.lower_whisker),
            );
            shapes.push(low_whisker);
            if self.box_width > 0.0 {
                let low_whisker_end = line_between(
                    self.point_at(
                        self.argument - self.whisker_width / 2.0,
                        self.spread.lower_whisker,
                    ),
                    self.point_at(
                        self.argument + self.whisker_width / 2.0,
                        self.spread.lower_whisker,
                    ),
                );
                shapes.push(low_whisker_end);
            }
        }
    }

    pub(super) fn add_rulers_and_text(
        &self,
        parent: &BoxPlot,
        plot: &PlotConfig<'_>,
        shapes: &mut Vec<Shape>,
        cursors: &mut Vec<Cursor>,
    ) {
        let text: Option<String> = parent
            .element_formatter
            .as_ref()
            .map(|fmt| fmt(self, parent));

        add_rulers_and_text(self, plot, text, shapes, cursors);
    }
}

impl RectElement for BoxElem {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn bounds_min(&self) -> PlotPoint {
        let argument = self.argument - self.box_width.max(self.whisker_width) / 2.0;
        let value = self.spread.lower_whisker;
        self.point_at(argument, value)
    }

    fn bounds_max(&self) -> PlotPoint {
        let argument = self.argument + self.box_width.max(self.whisker_width) / 2.0;
        let value = self.spread.upper_whisker;
        self.point_at(argument, value)
    }

    fn values_with_ruler(&self) -> Vec<PlotPoint> {
        let median = self.point_at(self.argument, self.spread.median);
        let q1 = self.point_at(self.argument, self.spread.quartile1);
        let q3 = self.point_at(self.argument, self.spread.quartile3);
        let upper = self.point_at(self.argument, self.spread.upper_whisker);
        let lower = self.point_at(self.argument, self.spread.lower_whisker);

        vec![median, q1, q3, upper, lower]
    }

    fn orientation(&self) -> Orientation {
        self.orientation
    }

    fn corner_value(&self) -> PlotPoint {
        self.point_at(self.argument, self.spread.upper_whisker)
    }

    fn default_values_format(&self, transform: &ScreenTransform) -> String {
        let scale = transform.dvalue_dpos();
        let scale = match self.orientation {
            Orientation::Horizontal => scale[0],
            Orientation::Vertical => scale[1],
        };
        let y_decimals = ((-scale.abs().log10()).ceil().at_least(0.0) as usize)
            .at_most(6)
            .at_least(1);
        format!(
            "Max = {max:.decimals$}\
             \nQuartile 3 = {q3:.decimals$}\
             \nMedian = {med:.decimals$}\
             \nQuartile 1 = {q1:.decimals$}\
             \nMin = {min:.decimals$}",
            max = self.spread.upper_whisker,
            q3 = self.spread.quartile3,
            med = self.spread.median,
            q1 = self.spread.quartile1,
            min = self.spread.lower_whisker,
            decimals = y_decimals
        )
    }
}
