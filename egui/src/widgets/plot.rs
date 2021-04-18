//! Simple plotting library.

#![allow(clippy::comparison_chain)]

use color::Hsva;
use serde::{Deserialize, Serialize};

use crate::*;

// ----------------------------------------------------------------------------

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

/// 2D bounding box of f64 precision.
/// The range of data values we show.
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
struct Bounds {
    min: [f64; 2],
    max: [f64; 2],
}

impl Bounds {
    pub const NOTHING: Self = Self {
        min: [f64::INFINITY; 2],
        max: [-f64::INFINITY; 2],
    };

    pub fn new_symmetrical(half_extent: f64) -> Self {
        Self {
            min: [-half_extent; 2],
            max: [half_extent; 2],
        }
    }

    pub fn width(&self) -> f64 {
        self.max[0] - self.min[0]
    }

    pub fn height(&self) -> f64 {
        self.max[1] - self.min[1]
    }

    pub fn is_finite(&self) -> bool {
        self.min[0].is_finite()
            && self.min[1].is_finite()
            && self.max[0].is_finite()
            && self.max[1].is_finite()
    }

    pub fn extend_with(&mut self, value: &Value) {
        self.extend_with_x(value.x);
        self.extend_with_y(value.y);
    }

    /// Expand to include the given x coordinate
    pub fn extend_with_x(&mut self, x: f64) {
        self.min[0] = self.min[0].min(x);
        self.max[0] = self.max[0].max(x);
    }

    /// Expand to include the given y coordinate
    pub fn extend_with_y(&mut self, y: f64) {
        self.min[1] = self.min[1].min(y);
        self.max[1] = self.max[1].max(y);
    }

    pub fn expand_x(&mut self, pad: f64) {
        self.min[0] -= pad;
        self.max[0] += pad;
    }

    pub fn expand_y(&mut self, pad: f64) {
        self.min[1] -= pad;
        self.max[1] += pad;
    }

    pub fn merge(&mut self, other: &Bounds) {
        self.min[0] = self.min[0].min(other.min[0]);
        self.min[1] = self.min[1].min(other.min[1]);
        self.max[0] = self.max[0].max(other.max[0]);
        self.max[1] = self.max[1].max(other.max[1]);
    }

    pub fn shift_x(&mut self, delta: f64) {
        self.min[0] += delta;
        self.max[0] += delta;
    }

    pub fn shift_y(&mut self, delta: f64) {
        self.min[1] += delta;
        self.max[1] += delta;
    }

    pub fn shift(&mut self, delta: Vec2) {
        self.shift_x(delta.x as f64);
        self.shift_y(delta.y as f64);
    }

    pub fn add_relative_margin(&mut self, margin_fraction: Vec2) {
        let width = self.width();
        let height = self.height();
        self.expand_x(margin_fraction.x as f64 * width);
        self.expand_y(margin_fraction.y as f64 * height);
    }
}

// ----------------------------------------------------------------------------

/// A horizontal line in a plot, filling the full width
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HLine {
    y: f64,
    stroke: Stroke,
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
    x: f64,
    stroke: Stroke,
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

/// A series of values forming a path.
#[derive(Clone, PartialEq)]
pub struct Curve {
    values: Vec<Value>,
    bounds: Bounds,
    stroke: Stroke,
    name: String,
}

impl Curve {
    pub fn from_values(values: Vec<Value>) -> Self {
        let mut bounds = Bounds::NOTHING;
        for value in &values {
            bounds.extend_with(value);
        }
        Self {
            values,
            bounds,
            stroke: Stroke::new(2.0, Color32::TRANSPARENT),
            name: Default::default(),
        }
    }

    pub fn from_values_iter(iter: impl Iterator<Item = Value>) -> Self {
        Self::from_values(iter.collect())
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
    pub fn name(mut self, name: impl ToString) -> Self {
        self.name = name.to_string();
        self
    }
}

// ----------------------------------------------------------------------------

/// Information about the plot that has to persist between frames.
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
struct PlotMemory {
    bounds: Bounds,
}

impl Default for PlotMemory {
    fn default() -> Self {
        Self {
            bounds: Bounds::new_symmetrical(1.),
        }
    }
}

// ----------------------------------------------------------------------------

/// A 2D plot, e.g. a graph of a function.
///
/// `Plot` supports multiple curves.
///
/// ```
/// # let ui = &mut egui::Ui::__test();
/// use egui::plot::{Curve, Plot, Value};
/// let sin = (0..1000).map(|i| {
///     let x = i as f64 * 0.01;
///     Value::new(x, x.sin())
/// });
/// let curve = Curve::from_values_iter(sin);
/// ui.add(
///     Plot::new("Test Plot").curve(curve).view_aspect(2.0)
/// );
/// ```
#[derive(Clone, PartialEq)]
pub struct Plot {
    name: String,
    next_auto_color_idx: usize,

    curves: Vec<Curve>,
    hlines: Vec<HLine>,
    vlines: Vec<VLine>,

    symmetrical_x_bounds: bool,
    symmetrical_y_bounds: bool,
    automatic_bounds: bool,
    margin_fraction: Vec2,

    min_size: Vec2,
    width: Option<f32>,
    height: Option<f32>,
    data_aspect: Option<f32>,
    view_aspect: Option<f32>,

    show_x: bool,
    show_y: bool,
}

impl Plot {
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            next_auto_color_idx: 0,

            curves: Default::default(),
            hlines: Default::default(),
            vlines: Default::default(),

            symmetrical_x_bounds: false,
            symmetrical_y_bounds: false,
            automatic_bounds: false,
            margin_fraction: Vec2::splat(0.05),

            min_size: Vec2::splat(64.0),
            width: None,
            height: None,
            data_aspect: None,
            view_aspect: None,

            show_x: true,
            show_y: true,
        }
    }

    fn auto_color(&mut self, color: &mut Color32) {
        if *color == Color32::TRANSPARENT {
            let i = self.next_auto_color_idx;
            self.next_auto_color_idx += 1;
            let golden_ratio = (5.0_f32.sqrt() - 1.0) / 2.0; // 0.61803398875
            let h = i as f32 * golden_ratio;
            *color = Hsva::new(h, 0.85, 0.5, 1.0).into(); // TODO: OkLab or some other perspective color space
        }
    }

    /// Add a data curve.
    /// You can add multiple curves.
    pub fn curve(mut self, mut curve: Curve) -> Self {
        if !curve.values.is_empty() {
            self.auto_color(&mut curve.stroke.color);
            self.curves.push(curve);
        }
        self
    }

    /// Add a horizontal line.
    /// Can be useful e.g. to show min/max bounds or similar.
    /// Always fills the full width of the plot.
    pub fn hline(mut self, mut hline: HLine) -> Self {
        self.auto_color(&mut hline.stroke.color);
        self.hlines.push(hline);
        self
    }

    /// Add a vertical line.
    /// Can be useful e.g. to show min/max bounds or similar.
    /// Always fills the full height of the plot.
    pub fn vline(mut self, mut vline: VLine) -> Self {
        self.auto_color(&mut vline.stroke.color);
        self.vlines.push(vline);
        self
    }

    /// If true, the x-bounds will be symmetrical, so that the x=0 zero line
    /// is always in the center.
    pub fn symmetrical_x_bounds(mut self, symmetrical_x_bounds: bool) -> Self {
        self.symmetrical_x_bounds = symmetrical_x_bounds;
        self
    }

    /// If true, the y-bounds will be symmetrical, so that the y=0 zero line
    /// is always in the center.
    pub fn symmetrical_y_bounds(mut self, symmetrical_y_bounds: bool) -> Self {
        self.symmetrical_y_bounds = symmetrical_y_bounds;
        self
    }

    /// If true, the bounds will be set based on the data.
    pub fn automatic_bounds(mut self, enabled: bool) -> Self {
        self.automatic_bounds = enabled;
        self
    }

    /// width / height ratio of the data.
    /// For instance, it can be useful to set this to `1.0` for when the two axes show the same unit.
    pub fn data_aspect(mut self, data_aspect: f32) -> Self {
        self.data_aspect = Some(data_aspect);
        self
    }

    /// width / height ratio of the plot region.
    /// By default no fixed aspect ratio is set (and width/height will fill the ui it is in).
    pub fn view_aspect(mut self, view_aspect: f32) -> Self {
        self.view_aspect = Some(view_aspect);
        self
    }

    /// Width of plot. By default a plot will fill the ui it is in.
    /// If you set [`Self::view_aspect`], the width can be calculated from the height.
    pub fn width(mut self, width: f32) -> Self {
        self.min_size.x = width;
        self.width = Some(width);
        self
    }

    /// Height of plot. By default a plot will fill the ui it is in.
    /// If you set [`Self::view_aspect`], the height can be calculated from the width.
    pub fn height(mut self, height: f32) -> Self {
        self.min_size.y = height;
        self.height = Some(height);
        self
    }

    /// Minimum size of the plot view.
    pub fn min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    /// Show the x-value (e.g. when hovering). Default: `true`.
    pub fn show_x(mut self, show_x: bool) -> Self {
        self.show_x = show_x;
        self
    }

    /// Show the y-value (e.g. when hovering). Default: `true`.
    pub fn show_y(mut self, show_y: bool) -> Self {
        self.show_y = show_y;
        self
    }
}

impl Widget for Plot {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            name,
            next_auto_color_idx: _,
            curves,
            hlines,
            vlines,
            symmetrical_x_bounds,
            symmetrical_y_bounds,
            margin_fraction,
            width,
            height,
            min_size,
            data_aspect,
            view_aspect,
            show_x,
            show_y,
            automatic_bounds,
        } = self;

        let plot_id = ui.make_persistent_id(name);
        let memory = ui
            .memory()
            .id_data
            .get_or_default::<PlotMemory>(plot_id)
            .clone();

        let PlotMemory { mut bounds } = memory;

        let size = {
            let width = width.unwrap_or_else(|| {
                if let (Some(height), Some(aspect)) = (height, view_aspect) {
                    height * aspect
                } else {
                    ui.available_size_before_wrap_finite().x
                }
            });
            let width = width.at_least(min_size.x);

            let height = height.unwrap_or_else(|| {
                if let Some(aspect) = view_aspect {
                    width / aspect
                } else {
                    ui.available_size_before_wrap_finite().y
                }
            });
            let height = height.at_least(min_size.y);
            vec2(width, height)
        };

        let (rect, response) = ui.allocate_exact_size(size, Sense::drag());

        if automatic_bounds || response.double_clicked_by(PointerButton::Primary) {
            bounds = Bounds::NOTHING;
            hlines.iter().for_each(|line| bounds.extend_with_y(line.y));
            vlines.iter().for_each(|line| bounds.extend_with_x(line.x));
            curves.iter().for_each(|curve| bounds.merge(&curve.bounds));
            bounds.add_relative_margin(margin_fraction);
        }

        if symmetrical_x_bounds {
            let x_abs = bounds.min[0].abs().max(bounds.max[0].abs());
            bounds.min[0] = -x_abs;
            bounds.max[0] = x_abs;
        };
        if symmetrical_y_bounds {
            let y_abs = bounds.min[1].abs().max(bounds.max[1].abs());
            bounds.min[1] = -y_abs;
            bounds.max[1] = y_abs;
        };

        if let Some(data_aspect) = data_aspect {
            let data_aspect = data_aspect as f64;
            let rw = rect.width() as f64;
            let rh = rect.height() as f64;
            let current_data_aspect = (bounds.width() / rw) / (bounds.height() / rh);

            let margin = 1e-5;
            if current_data_aspect < data_aspect - margin {
                bounds.expand_x((data_aspect / current_data_aspect - 1.0) * bounds.width() * 0.5);
            } else if current_data_aspect > data_aspect + margin {
                bounds.expand_y((current_data_aspect / data_aspect - 1.0) * bounds.height() * 0.5);
            }
        }

        // Background:
        ui.painter().add(Shape::Rect {
            rect,
            corner_radius: 2.0,
            fill: ui.visuals().extreme_bg_color,
            stroke: ui.visuals().window_stroke(),
        });

        if bounds.is_finite() && bounds.width() > 0.0 && bounds.height() > 0.0 {
            let mut transform = ScreenTransform { bounds, rect };
            if response.dragged_by(PointerButton::Primary) {
                transform.shift_bounds(-response.drag_delta());
            }
            if let Some(hover_pos) = response.hover_pos() {
                transform.zoom(-0.01 * ui.input().scroll_delta[1], hover_pos);
            }

            ui.memory().id_data.insert(
                plot_id,
                PlotMemory {
                    bounds: *transform.bounds(),
                },
            );

            let prepared = Prepared {
                curves,
                hlines,
                vlines,
                show_x,
                show_y,
                transform,
            };
            prepared.ui(ui, &response);
        }

        response.on_hover_cursor(CursorIcon::Crosshair)
    }
}

/// Contains the screen rectangle and the plot bounds and provides methods to transform them.
struct ScreenTransform {
    /// The screen rectangle.
    rect: Rect,
    /// The plot bounds.
    bounds: Bounds,
}

impl ScreenTransform {
    fn rect(&self) -> &Rect {
        &self.rect
    }

    fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    fn shift_bounds(&mut self, mut delta_pos: Vec2) {
        delta_pos.x *= self.dvalue_dpos()[0] as f32;
        delta_pos.y *= self.dvalue_dpos()[1] as f32;
        self.bounds.shift(delta_pos);
    }

    /// Zoom by a relative amount with the given screen position as center.
    fn zoom(&mut self, delta: f32, center: Pos2) {
        let delta = delta.clamp(-1., 1.);
        let rect_width = self.rect.width();
        let rect_height = self.rect.height();
        let bounds_width = self.bounds.width() as f32;
        let bounds_height = self.bounds.height() as f32;
        let t_x = (center.x - self.rect.min[0]) / rect_width;
        let t_y = (self.rect.max[1] - center.y) / rect_height;
        self.bounds.min[0] -= ((t_x * delta) * bounds_width) as f64;
        self.bounds.min[1] -= ((t_y * delta) * bounds_height) as f64;
        self.bounds.max[0] += (((1. - t_x) * delta) * bounds_width) as f64;
        self.bounds.max[1] += (((1. - t_y) * delta) * bounds_height) as f64;
    }

    fn position_from_value(&self, value: &Value) -> Pos2 {
        let x = remap(
            value.x,
            self.bounds.min[0]..=self.bounds.max[0],
            (self.rect.left() as f64)..=(self.rect.right() as f64),
        );
        let y = remap(
            value.y,
            self.bounds.min[1]..=self.bounds.max[1],
            (self.rect.bottom() as f64)..=(self.rect.top() as f64), // negated y axis!
        );
        pos2(x as f32, y as f32)
    }

    fn value_from_position(&self, pos: Pos2) -> Value {
        let x = remap(
            pos.x as f64,
            (self.rect.left() as f64)..=(self.rect.right() as f64),
            self.bounds.min[0]..=self.bounds.max[0],
        );
        let y = remap(
            pos.y as f64,
            (self.rect.bottom() as f64)..=(self.rect.top() as f64), // negated y axis!
            self.bounds.min[1]..=self.bounds.max[1],
        );
        Value::new(x, y)
    }

    /// delta position / delta value
    fn dpos_dvalue_x(&self) -> f64 {
        self.rect.width() as f64 / self.bounds.width()
    }

    /// delta position / delta value
    fn dpos_dvalue_y(&self) -> f64 {
        -self.rect.height() as f64 / self.bounds.height() // negated y axis!
    }

    /// delta position / delta value
    fn dpos_dvalue(&self) -> [f64; 2] {
        [self.dpos_dvalue_x(), self.dpos_dvalue_y()]
    }

    /// delta value / delta position
    fn dvalue_dpos(&self) -> [f64; 2] {
        [1.0 / self.dpos_dvalue_x(), 1.0 / self.dpos_dvalue_y()]
    }
}

struct Prepared {
    curves: Vec<Curve>,
    hlines: Vec<HLine>,
    vlines: Vec<VLine>,
    show_x: bool,
    show_y: bool,
    transform: ScreenTransform,
}

impl Prepared {
    fn ui(&self, ui: &mut Ui, response: &Response) {
        let Self { transform, .. } = self;

        let mut shapes = Vec::new();

        for d in 0..2 {
            self.paint_axis(ui, d, &mut shapes);
        }

        for &hline in &self.hlines {
            let HLine { y, stroke } = hline;
            let points = [
                transform.position_from_value(&Value::new(transform.bounds().min[0], y)),
                transform.position_from_value(&Value::new(transform.bounds().max[0], y)),
            ];
            shapes.push(Shape::line_segment(points, stroke));
        }

        for &vline in &self.vlines {
            let VLine { x, stroke } = vline;
            let points = [
                transform.position_from_value(&Value::new(x, transform.bounds().min[1])),
                transform.position_from_value(&Value::new(x, transform.bounds().max[1])),
            ];
            shapes.push(Shape::line_segment(points, stroke));
        }

        for curve in &self.curves {
            let stroke = curve.stroke;
            let values = &curve.values;
            let shape = if values.len() == 1 {
                let point = transform.position_from_value(&values[0]);
                Shape::circle_filled(point, stroke.width / 2.0, stroke.color)
            } else {
                Shape::line(
                    values
                        .iter()
                        .map(|v| transform.position_from_value(v))
                        .collect(),
                    stroke,
                )
            };
            shapes.push(shape);
        }

        if let Some(pointer) = response.hover_pos() {
            self.hover(ui, pointer, &mut shapes);
        }

        ui.painter().sub_region(*transform.rect()).extend(shapes);
    }

    fn paint_axis(&self, ui: &Ui, axis: usize, shapes: &mut Vec<Shape>) {
        let Self { transform, .. } = self;

        let bounds = transform.bounds();
        let text_style = TextStyle::Body;

        let base: f64 = 10.0;

        let min_label_spacing_in_points = 60.0; // TODO: large enough for a wide label
        let step_size = transform.dvalue_dpos()[axis] * min_label_spacing_in_points;
        let step_size = base.powi(step_size.abs().log(base).ceil() as i32);

        let step_size_in_points = (transform.dpos_dvalue()[axis] * step_size) as f32;

        // Where on the cross-dimension to show the label values
        let value_cross = 0.0_f64.clamp(bounds.min[1 - axis], bounds.max[1 - axis]);

        for i in 0.. {
            let value_main = step_size * (bounds.min[axis] / step_size + i as f64).floor();
            if value_main > bounds.max[axis] {
                break;
            }

            let value = if axis == 0 {
                Value::new(value_main, value_cross)
            } else {
                Value::new(value_cross, value_main)
            };
            let pos_in_gui = transform.position_from_value(&value);

            {
                // Grid: subdivide each label tick in `n` grid lines:
                let n = if step_size_in_points.abs() < 40.0 {
                    2
                } else if step_size_in_points.abs() < 100.0 {
                    5
                } else {
                    10
                };

                for i in 0..n {
                    let strength = if i == 0 && value_main == 0.0 {
                        Strength::Strong
                    } else if i == 0 {
                        Strength::Middle
                    } else {
                        Strength::Weak
                    };
                    let color = line_color(ui, strength);

                    let mut pos_in_gui = pos_in_gui;
                    pos_in_gui[axis] += step_size_in_points * (i as f32) / (n as f32);
                    let mut p0 = pos_in_gui;
                    let mut p1 = pos_in_gui;
                    p0[1 - axis] = transform.rect.min[1 - axis];
                    p1[1 - axis] = transform.rect.max[1 - axis];
                    shapes.push(Shape::line_segment([p0, p1], Stroke::new(1.0, color)));
                }
            }

            let text = emath::round_to_decimals(value_main, 5).to_string(); // hack

            let galley = ui.fonts().layout_multiline(text_style, text, f32::INFINITY);

            let mut text_pos = pos_in_gui + vec2(1.0, -galley.size.y);

            // Make sure we see the labels, even if the axis is off-screen:
            text_pos[1 - axis] = text_pos[1 - axis]
                .at_most(transform.rect.max[1 - axis] - galley.size[1 - axis] - 2.0)
                .at_least(transform.rect.min[1 - axis] + 1.0);

            shapes.push(Shape::Text {
                pos: text_pos,
                galley,
                color: ui.visuals().text_color(),
                fake_italics: false,
            });
        }
    }

    fn hover(&self, ui: &Ui, pointer: Pos2, shapes: &mut Vec<Shape>) {
        let Self {
            transform,
            show_x,
            show_y,
            curves,
            ..
        } = self;

        if !show_x && !show_y {
            return;
        }

        let interact_radius: f32 = 16.0;
        let mut closest_value = None;
        let mut closest_curve = None;
        let mut closest_dist_sq = interact_radius.powi(2);
        for curve in curves {
            for value in &curve.values {
                let pos = transform.position_from_value(value);
                let dist_sq = pointer.distance_sq(pos);
                if dist_sq < closest_dist_sq {
                    closest_dist_sq = dist_sq;
                    closest_value = Some(value);
                    closest_curve = Some(curve);
                }
            }
        }

        let mut prefix = String::new();
        if let Some(curve) = closest_curve {
            if !curve.name.is_empty() {
                prefix = format!("{}\n", curve.name);
            }
        }

        let line_color = line_color(ui, Strength::Strong);

        let value = if let Some(value) = closest_value {
            let position = transform.position_from_value(value);
            shapes.push(Shape::circle_filled(position, 3.0, line_color));
            *value
        } else {
            transform.value_from_position(pointer)
        };
        let pointer = transform.position_from_value(&value);

        let rect = transform.rect();

        if *show_x {
            // vertical line
            shapes.push(Shape::line_segment(
                [pos2(pointer.x, rect.top()), pos2(pointer.x, rect.bottom())],
                (1.0, line_color),
            ));
        }
        if *show_y {
            // horizontal line
            shapes.push(Shape::line_segment(
                [pos2(rect.left(), pointer.y), pos2(rect.right(), pointer.y)],
                (1.0, line_color),
            ));
        }

        let text = {
            let scale = transform.dvalue_dpos();
            let x_decimals = ((-scale[0].abs().log10()).ceil().at_least(0.0) as usize).at_most(6);
            let y_decimals = ((-scale[1].abs().log10()).ceil().at_least(0.0) as usize).at_most(6);
            if *show_x && *show_y {
                format!(
                    "{}x = {:.*}\ny = {:.*}",
                    prefix, x_decimals, value.x, y_decimals, value.y
                )
            } else if *show_x {
                format!("{}x = {:.*}", prefix, x_decimals, value.x)
            } else if *show_y {
                format!("{}y = {:.*}", prefix, y_decimals, value.y)
            } else {
                unreachable!()
            }
        };

        shapes.push(Shape::text(
            ui.fonts(),
            pointer + vec2(3.0, -2.0),
            Align2::LEFT_BOTTOM,
            text,
            TextStyle::Body,
            ui.visuals().text_color(),
        ));
    }
}

#[derive(Clone, Copy)]
enum Strength {
    Strong,
    Middle,
    Weak,
}

fn line_color(ui: &Ui, strength: Strength) -> Color32 {
    if ui.visuals().dark_mode {
        match strength {
            Strength::Strong => Color32::from_gray(130).additive(),
            Strength::Middle => Color32::from_gray(55).additive(),
            Strength::Weak => Color32::from_gray(25).additive(),
        }
    } else {
        match strength {
            Strength::Strong => Color32::from_black_alpha(220),
            Strength::Middle => Color32::from_black_alpha(120),
            Strength::Weak => Color32::from_black_alpha(35),
        }
    }
}
