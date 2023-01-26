use crate::emath::NumExt;
use crate::epaint::{Color32, RectShape, Rounding, Shape, Stroke};

use super::{add_rulers_and_text, highlighted_color, Orientation, PlotConfig, RectElement};
use crate::plot::{BarChart, Cursor, PlotPoint, ScreenTransform};

/// One bar in a [`BarChart`]. Potentially floating, allowing stacked bar charts.
/// Width can be changed to allow variable-width histograms.
#[derive(Clone, Debug, PartialEq)]
pub struct Bar {
    /// Name of plot element in the diagram (annotated by default formatter)
    pub name: String,

    /// Which direction the bar faces in the diagram
    pub orientation: Orientation,

    /// Position on the argument (input) axis -- X if vertical, Y if horizontal
    pub argument: f64,

    /// Position on the value (output) axis -- Y if vertical, X if horizontal
    pub value: f64,

    /// For stacked bars, this denotes where the bar starts. None if base axis
    pub base_offset: Option<f64>,

    /// Thickness of the bar
    pub bar_width: f64,

    /// Line width and color
    pub stroke: Stroke,

    /// Fill color
    pub fill: Color32,
}

impl Bar {
    /// Create a bar. Its `orientation` is set by its [`BarChart`] parent.
    ///
    /// - `argument`: Position on the argument axis (X if vertical, Y if horizontal).
    /// - `value`: Height of the bar (if vertical).
    ///
    /// By default the bar is vertical and its base is at zero.
    pub fn new(argument: f64, height: f64) -> Bar {
        Bar {
            argument,
            value: height,
            orientation: Orientation::default(),
            name: Default::default(),
            base_offset: None,
            bar_width: 0.5,
            stroke: Stroke::new(1.0, Color32::TRANSPARENT),
            fill: Color32::TRANSPARENT,
        }
    }

    /// Name of this bar chart element.
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

    /// Offset the base of the bar.
    /// This offset is on the Y axis for a vertical bar
    /// and on the X axis for a horizontal bar.
    pub fn base_offset(mut self, offset: f64) -> Self {
        self.base_offset = Some(offset);
        self
    }

    /// Set the bar width.
    pub fn width(mut self, width: f64) -> Self {
        self.bar_width = width;
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

    pub(super) fn lower(&self) -> f64 {
        if self.value.is_sign_positive() {
            self.base_offset.unwrap_or(0.0)
        } else {
            self.base_offset.map_or(self.value, |o| o + self.value)
        }
    }

    pub(super) fn upper(&self) -> f64 {
        if self.value.is_sign_positive() {
            self.base_offset.map_or(self.value, |o| o + self.value)
        } else {
            self.base_offset.unwrap_or(0.0)
        }
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

        let rect = transform.rect_from_values(&self.bounds_min(), &self.bounds_max());
        let rect = Shape::Rect(RectShape {
            rect,
            rounding: Rounding::none(),
            fill,
            stroke,
        });

        shapes.push(rect);
    }

    pub(super) fn add_rulers_and_text(
        &self,
        parent: &BarChart,
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

impl RectElement for Bar {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn bounds_min(&self) -> PlotPoint {
        self.point_at(self.argument - self.bar_width / 2.0, self.lower())
    }

    fn bounds_max(&self) -> PlotPoint {
        self.point_at(self.argument + self.bar_width / 2.0, self.upper())
    }

    fn values_with_ruler(&self) -> Vec<PlotPoint> {
        let base = self.base_offset.unwrap_or(0.0);
        let value_center = self.point_at(self.argument, base + self.value);

        let mut ruler_positions = vec![value_center];

        if let Some(offset) = self.base_offset {
            ruler_positions.push(self.point_at(self.argument, offset));
        }

        ruler_positions
    }

    fn orientation(&self) -> Orientation {
        self.orientation
    }

    fn default_values_format(&self, transform: &ScreenTransform) -> String {
        let scale = transform.dvalue_dpos();
        let scale = match self.orientation {
            Orientation::Horizontal => scale[0],
            Orientation::Vertical => scale[1],
        };
        let decimals = ((-scale.abs().log10()).ceil().at_least(0.0) as usize).at_most(6);
        crate::plot::format_number(self.value, decimals)
    }
}
