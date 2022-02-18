use std::ops::RangeInclusive;

use super::items::Value;
use crate::*;

/// 2D bounding box of f64 precision.
/// The range of data values we show.
#[derive(Clone, Copy, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PlotBounds {
    pub(crate) min: [f64; 2],
    pub(crate) max: [f64; 2],
}

impl PlotBounds {
    pub const NOTHING: Self = Self {
        min: [f64::INFINITY; 2],
        max: [-f64::INFINITY; 2],
    };

    pub fn min(&self) -> [f64; 2] {
        self.min
    }

    pub fn max(&self) -> [f64; 2] {
        self.max
    }

    pub(crate) fn new_symmetrical(half_extent: f64) -> Self {
        Self {
            min: [-half_extent; 2],
            max: [half_extent; 2],
        }
    }

    pub fn is_finite(&self) -> bool {
        self.min[0].is_finite()
            && self.min[1].is_finite()
            && self.max[0].is_finite()
            && self.max[1].is_finite()
    }

    pub fn is_valid(&self) -> bool {
        self.is_finite() && self.width() > 0.0 && self.height() > 0.0
    }

    pub fn width(&self) -> f64 {
        self.max[0] - self.min[0]
    }

    pub fn height(&self) -> f64 {
        self.max[1] - self.min[1]
    }

    pub fn center(&self) -> Value {
        Value {
            x: (self.min[0] + self.max[0]) / 2.0,
            y: (self.min[1] + self.max[1]) / 2.0,
        }
    }

    /// Expand to include the given (x,y) value
    pub(crate) fn extend_with(&mut self, value: &Value) {
        self.extend_with_x(value.x);
        self.extend_with_y(value.y);
    }

    /// Expand to include the given x coordinate
    pub(crate) fn extend_with_x(&mut self, x: f64) {
        self.min[0] = self.min[0].min(x);
        self.max[0] = self.max[0].max(x);
    }

    /// Expand to include the given y coordinate
    pub(crate) fn extend_with_y(&mut self, y: f64) {
        self.min[1] = self.min[1].min(y);
        self.max[1] = self.max[1].max(y);
    }

    pub(crate) fn expand_x(&mut self, pad: f64) {
        self.min[0] -= pad;
        self.max[0] += pad;
    }

    pub(crate) fn expand_y(&mut self, pad: f64) {
        self.min[1] -= pad;
        self.max[1] += pad;
    }

    pub(crate) fn merge(&mut self, other: &PlotBounds) {
        self.min[0] = self.min[0].min(other.min[0]);
        self.min[1] = self.min[1].min(other.min[1]);
        self.max[0] = self.max[0].max(other.max[0]);
        self.max[1] = self.max[1].max(other.max[1]);
    }

    pub(crate) fn translate_x(&mut self, delta: f64) {
        self.min[0] += delta;
        self.max[0] += delta;
    }

    pub(crate) fn translate_y(&mut self, delta: f64) {
        self.min[1] += delta;
        self.max[1] += delta;
    }

    pub(crate) fn translate(&mut self, delta: Vec2) {
        self.translate_x(delta.x as f64);
        self.translate_y(delta.y as f64);
    }

    pub(crate) fn add_relative_margin(&mut self, margin_fraction: Vec2) {
        let width = self.width().max(0.0);
        let height = self.height().max(0.0);
        self.expand_x(margin_fraction.x as f64 * width);
        self.expand_y(margin_fraction.y as f64 * height);
    }

    pub(crate) fn range_x(&self) -> RangeInclusive<f64> {
        self.min[0]..=self.max[0]
    }

    pub(crate) fn range_y(&self) -> RangeInclusive<f64> {
        self.min[1]..=self.max[1]
    }

    pub(crate) fn make_x_symmetrical(&mut self) {
        let x_abs = self.min[0].abs().max(self.max[0].abs());
        self.min[0] = -x_abs;
        self.max[0] = x_abs;
    }

    pub(crate) fn make_y_symmetrical(&mut self) {
        let y_abs = self.min[1].abs().max(self.max[1].abs());
        self.min[1] = -y_abs;
        self.max[1] = y_abs;
    }
}

/// Contains the screen rectangle and the plot bounds and provides methods to transform them.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone)]
pub(crate) struct ScreenTransform {
    /// The screen rectangle.
    frame: Rect,
    /// The plot bounds.
    bounds: PlotBounds,
    /// Whether to always center the x-range of the bounds.
    x_centered: bool,
    /// Whether to always center the y-range of the bounds.
    y_centered: bool,
}

impl ScreenTransform {
    pub fn new(frame: Rect, mut bounds: PlotBounds, x_centered: bool, y_centered: bool) -> Self {
        // Make sure they are not empty.
        if !bounds.is_valid() {
            bounds = PlotBounds::new_symmetrical(1.0);
        }

        // Scale axes so that the origin is in the center.
        if x_centered {
            bounds.make_x_symmetrical();
        };
        if y_centered {
            bounds.make_y_symmetrical();
        };

        Self {
            frame,
            bounds,
            x_centered,
            y_centered,
        }
    }

    pub fn frame(&self) -> &Rect {
        &self.frame
    }

    pub fn bounds(&self) -> &PlotBounds {
        &self.bounds
    }

    pub fn bounds_mut(&mut self) -> &mut PlotBounds {
        &mut self.bounds
    }

    pub fn translate_bounds(&mut self, mut delta_pos: Vec2) {
        if self.x_centered {
            delta_pos.x = 0.;
        }
        if self.y_centered {
            delta_pos.y = 0.;
        }
        delta_pos.x *= self.dvalue_dpos()[0] as f32;
        delta_pos.y *= self.dvalue_dpos()[1] as f32;
        self.bounds.translate(delta_pos);
    }

    /// Zoom by a relative factor with the given screen position as center.
    pub fn zoom(&mut self, zoom_factor: Vec2, center: Pos2) {
        let center = self.value_from_position(center);

        let mut new_bounds = self.bounds;
        new_bounds.min[0] = center.x + (new_bounds.min[0] - center.x) / (zoom_factor.x as f64);
        new_bounds.max[0] = center.x + (new_bounds.max[0] - center.x) / (zoom_factor.x as f64);
        new_bounds.min[1] = center.y + (new_bounds.min[1] - center.y) / (zoom_factor.y as f64);
        new_bounds.max[1] = center.y + (new_bounds.max[1] - center.y) / (zoom_factor.y as f64);

        if new_bounds.is_valid() {
            self.bounds = new_bounds;
        }
    }

    pub fn position_from_value(&self, value: &Value) -> Pos2 {
        let x = remap(
            value.x,
            self.bounds.min[0]..=self.bounds.max[0],
            (self.frame.left() as f64)..=(self.frame.right() as f64),
        );
        let y = remap(
            value.y,
            self.bounds.min[1]..=self.bounds.max[1],
            (self.frame.bottom() as f64)..=(self.frame.top() as f64), // negated y axis!
        );
        pos2(x as f32, y as f32)
    }

    pub fn value_from_position(&self, pos: Pos2) -> Value {
        let x = remap(
            pos.x as f64,
            (self.frame.left() as f64)..=(self.frame.right() as f64),
            self.bounds.min[0]..=self.bounds.max[0],
        );
        let y = remap(
            pos.y as f64,
            (self.frame.bottom() as f64)..=(self.frame.top() as f64), // negated y axis!
            self.bounds.min[1]..=self.bounds.max[1],
        );
        Value::new(x, y)
    }

    /// Transform a rectangle of plot values to a screen-coordinate rectangle.
    ///
    /// This typically means that the rect is mirrored vertically (top becomes bottom and vice versa),
    /// since the plot's coordinate system has +Y up, while egui has +Y down.
    pub fn rect_from_values(&self, value1: &Value, value2: &Value) -> Rect {
        let pos1 = self.position_from_value(value1);
        let pos2 = self.position_from_value(value2);

        let mut rect = Rect::NOTHING;
        rect.extend_with(pos1);
        rect.extend_with(pos2);
        rect
    }

    /// delta position / delta value
    pub fn dpos_dvalue_x(&self) -> f64 {
        self.frame.width() as f64 / self.bounds.width()
    }

    /// delta position / delta value
    pub fn dpos_dvalue_y(&self) -> f64 {
        -self.frame.height() as f64 / self.bounds.height() // negated y axis!
    }

    /// delta position / delta value
    pub fn dpos_dvalue(&self) -> [f64; 2] {
        [self.dpos_dvalue_x(), self.dpos_dvalue_y()]
    }

    /// delta value / delta position
    pub fn dvalue_dpos(&self) -> [f64; 2] {
        [1.0 / self.dpos_dvalue_x(), 1.0 / self.dpos_dvalue_y()]
    }

    pub fn get_aspect(&self) -> f64 {
        let rw = self.frame.width() as f64;
        let rh = self.frame.height() as f64;
        (self.bounds.width() / rw) / (self.bounds.height() / rh)
    }

    /// Sets the aspect ratio by either expanding the x-axis or contracting the y-axis.
    pub fn set_aspect(&mut self, aspect: f64, preserve_y: bool) {
        let current_aspect = self.get_aspect();

        let epsilon = 1e-5;
        if (current_aspect - aspect).abs() < epsilon {
            // Don't make any changes when the aspect is already almost correct.
            return;
        }

        if preserve_y {
            self.bounds
                .expand_x((aspect / current_aspect - 1.0) * self.bounds.width() * 0.5);
        } else {
            self.bounds
                .expand_y((current_aspect / aspect - 1.0) * self.bounds.height() * 0.5);
        }
    }
}
