//! Simple plotting library.

mod items;
mod transform;

pub use items::{Curve, Value};
use items::{HLine, VLine};
use transform::{Bounds, ScreenTransform};

use crate::*;
use color::Hsva;

// ----------------------------------------------------------------------------

/// Information about the plot that has to persist between frames.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone)]
struct PlotMemory {
    bounds: Bounds,
    auto_bounds: bool,
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
pub struct Plot {
    name: String,
    next_auto_color_idx: usize,

    curves: Vec<Curve>,
    hlines: Vec<HLine>,
    vlines: Vec<VLine>,

    center_x_axis: bool,
    center_y_axis: bool,
    allow_zoom: bool,
    allow_drag: bool,
    min_auto_bounds: Bounds,
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
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            next_auto_color_idx: 0,

            curves: Default::default(),
            hlines: Default::default(),
            vlines: Default::default(),

            center_x_axis: false,
            center_y_axis: false,
            allow_zoom: true,
            allow_drag: true,
            min_auto_bounds: Bounds::NOTHING,
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
        if !curve.no_data() {
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

    /// width / height ratio of the data.
    /// For instance, it can be useful to set this to `1.0` for when the two axes show the same
    /// unit.
    /// By default the plot window's aspect ratio is used.
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

    #[deprecated = "Renamed center_x_axis"]
    pub fn symmetrical_x_axis(mut self, on: bool) -> Self {
        self.center_x_axis = on;
        self
    }

    #[deprecated = "Renamed center_y_axis"]
    pub fn symmetrical_y_axis(mut self, on: bool) -> Self {
        self.center_y_axis = on;
        self
    }

    /// Always keep the x-axis centered. Default: `false`.
    pub fn center_x_axis(mut self, on: bool) -> Self {
        self.center_x_axis = on;
        self
    }

    /// Always keep the y-axis centered. Default: `false`.
    pub fn center_y_axis(mut self, on: bool) -> Self {
        self.center_y_axis = on;
        self
    }

    /// Whether to allow zooming in the plot. Default: `true`.
    pub fn allow_zoom(mut self, on: bool) -> Self {
        self.allow_zoom = on;
        self
    }

    /// Whether to allow dragging in the plot to move the bounds. Default: `true`.
    pub fn allow_drag(mut self, on: bool) -> Self {
        self.allow_drag = on;
        self
    }

    /// Expand bounds to include the given x value.
    /// For instance, to always show the y axis, call `plot.include_x(0.0)`.
    pub fn include_x(mut self, x: impl Into<f64>) -> Self {
        self.min_auto_bounds.extend_with_x(x.into());
        self
    }

    /// Expand bounds to include the given y value.
    /// For instance, to always show the x axis, call `plot.include_y(0.0)`.
    pub fn include_y(mut self, y: impl Into<f64>) -> Self {
        self.min_auto_bounds.extend_with_y(y.into());
        self
    }
}

impl Widget for Plot {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            name,
            next_auto_color_idx: _,
            mut curves,
            hlines,
            vlines,
            center_x_axis,
            center_y_axis,
            allow_zoom,
            allow_drag,
            min_auto_bounds,
            margin_fraction,
            width,
            height,
            min_size,
            data_aspect,
            view_aspect,
            show_x,
            show_y,
        } = self;

        let plot_id = ui.make_persistent_id(name);
        let memory = ui
            .memory()
            .id_data
            .get_mut_or_insert_with(plot_id, || PlotMemory {
                bounds: min_auto_bounds,
                auto_bounds: true,
            })
            .clone();

        let PlotMemory {
            mut bounds,
            mut auto_bounds,
        } = memory;

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

        // Background
        ui.painter().add(Shape::Rect {
            rect,
            corner_radius: 2.0,
            fill: ui.visuals().extreme_bg_color,
            stroke: ui.visuals().window_stroke(),
        });

        auto_bounds |= response.double_clicked_by(PointerButton::Primary);

        // Set bounds automatically based on content.
        if auto_bounds || !bounds.is_valid() {
            bounds = min_auto_bounds;
            hlines.iter().for_each(|line| bounds.extend_with_y(line.y));
            vlines.iter().for_each(|line| bounds.extend_with_x(line.x));
            curves.iter().for_each(|curve| bounds.merge(&curve.bounds));
            bounds.add_relative_margin(margin_fraction);
        }
        // Make sure they are not empty.
        if !bounds.is_valid() {
            bounds = Bounds::new_symmetrical(1.0);
        }

        // Scale axes so that the origin is in the center.
        if center_x_axis {
            bounds.make_x_symmetrical();
        };
        if center_y_axis {
            bounds.make_y_symmetrical()
        };

        let mut transform = ScreenTransform::new(rect, bounds, center_x_axis, center_y_axis);

        // Enforce equal aspect ratio.
        if let Some(data_aspect) = data_aspect {
            transform.set_aspect(data_aspect as f64);
        }

        // Dragging
        if allow_drag && response.dragged_by(PointerButton::Primary) {
            transform.translate_bounds(-response.drag_delta());
            auto_bounds = false;
        }

        // Zooming
        if allow_zoom {
            if let Some(hover_pos) = response.hover_pos() {
                let scroll_delta = ui.input().scroll_delta[1];
                if scroll_delta != 0. {
                    transform.zoom(-0.01 * scroll_delta, hover_pos);
                    auto_bounds = false;
                }
            }
        }

        // Initialize values from functions.
        curves
            .iter_mut()
            .for_each(|curve| curve.generate_points(transform.bounds().range_x()));

        ui.memory().id_data.insert(
            plot_id,
            PlotMemory {
                bounds: *transform.bounds(),
                auto_bounds,
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

        response.on_hover_cursor(CursorIcon::Crosshair)
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

        ui.painter().sub_region(*transform.frame()).extend(shapes);
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
                    p0[1 - axis] = transform.frame().min[1 - axis];
                    p1[1 - axis] = transform.frame().max[1 - axis];
                    shapes.push(Shape::line_segment([p0, p1], Stroke::new(1.0, color)));
                }
            }

            let text = emath::round_to_decimals(value_main, 5).to_string(); // hack

            let galley = ui.fonts().layout_multiline(text_style, text, f32::INFINITY);

            let mut text_pos = pos_in_gui + vec2(1.0, -galley.size.y);

            // Make sure we see the labels, even if the axis is off-screen:
            text_pos[1 - axis] = text_pos[1 - axis]
                .at_most(transform.frame().max[1 - axis] - galley.size[1 - axis] - 2.0)
                .at_least(transform.frame().min[1 - axis] + 1.0);

            shapes.push(Shape::Text {
                pos: text_pos,
                galley,
                default_color: ui.visuals().text_color(),
                color_map: epaint::text::TextColorMap::default(),
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

        let line_color = line_color(ui, Strength::Middle);

        let value = if let Some(value) = closest_value {
            let position = transform.position_from_value(value);
            shapes.push(Shape::circle_filled(position, 3.0, line_color));
            *value
        } else {
            transform.value_from_position(pointer)
        };
        let pointer = transform.position_from_value(&value);

        let rect = transform.frame();

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
