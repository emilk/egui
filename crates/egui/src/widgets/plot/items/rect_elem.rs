use super::{Orientation, PlotPoint};
use crate::plot::transform::{PlotBounds, ScreenTransform};
use epaint::emath::NumExt;
use epaint::{Color32, Rgba, Stroke};

/// Trait that abstracts from rectangular 'Value'-like elements, such as bars or boxes
pub(super) trait RectElement {
    fn name(&self) -> &str;

    fn bounds_min(&self) -> PlotPoint;

    fn bounds_max(&self) -> PlotPoint;

    fn bounds(&self) -> PlotBounds {
        let mut bounds = PlotBounds::NOTHING;
        bounds.extend_with(&self.bounds_min());
        bounds.extend_with(&self.bounds_max());
        bounds
    }

    /// At which argument (input; usually X) there is a ruler (usually vertical)
    fn arguments_with_ruler(&self) -> Vec<PlotPoint> {
        // Default: one at center
        vec![self.bounds().center()]
    }

    /// At which value (output; usually Y) there is a ruler (usually horizontal)
    fn values_with_ruler(&self) -> Vec<PlotPoint>;

    /// The diagram's orientation (vertical/horizontal)
    fn orientation(&self) -> Orientation;

    /// Get X/Y-value for (argument, value) pair, taking into account orientation
    fn point_at(&self, argument: f64, value: f64) -> PlotPoint {
        match self.orientation() {
            Orientation::Horizontal => PlotPoint::new(value, argument),
            Orientation::Vertical => PlotPoint::new(argument, value),
        }
    }

    /// Right top of the rectangle (position of text)
    fn corner_value(&self) -> PlotPoint {
        //self.point_at(self.position + self.width / 2.0, value)
        PlotPoint {
            x: self.bounds_max().x,
            y: self.bounds_max().y,
        }
    }

    /// Debug formatting for hovered-over value, if none is specified by the user
    fn default_values_format(&self, transform: &ScreenTransform) -> String;
}

// ----------------------------------------------------------------------------
// Helper functions

pub(super) fn highlighted_color(mut stroke: Stroke, fill: Color32) -> (Stroke, Color32) {
    stroke.width *= 2.0;
    let fill = Rgba::from(fill);
    let fill_alpha = (2.0 * fill.a()).at_most(1.0);
    let fill = fill.to_opaque().multiply(fill_alpha);
    (stroke, fill.into())
}
