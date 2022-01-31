//! Simple plotting library.

use crate::*;
use epaint::ahash::AHashSet;
use epaint::color::Hsva;
use epaint::util::FloatOrd;
use items::PlotItem;
use legend::LegendWidget;
use transform::{PlotBounds, ScreenTransform};

pub use items::{
    Arrows, Bar, BarChart, BoxElem, BoxPlot, BoxSpread, HLine, Line, LineStyle, MarkerShape,
    Orientation, PlotImage, Points, Polygon, Text, VLine, Value, Values,
};
pub use legend::{Corner, Legend};

mod items;
mod legend;
mod transform;

type CustomLabelFunc = dyn Fn(&str, &Value) -> String;
type CustomLabelFuncRef = Option<Box<CustomLabelFunc>>;

type AxisFormatterFn = dyn Fn(f64) -> String;
type AxisFormatter = Option<Box<AxisFormatterFn>>;

// ----------------------------------------------------------------------------

/// Information about the plot that has to persist between frames.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone)]
struct PlotMemory {
    auto_bounds: bool,
    hovered_entry: Option<String>,
    hidden_items: AHashSet<String>,
    min_auto_bounds: PlotBounds,
    last_screen_transform: ScreenTransform,
}

impl PlotMemory {
    pub fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data().get_persisted(id)
    }

    pub fn store(self, ctx: &Context, id: Id) {
        ctx.data().insert_persisted(id, self);
    }
}

// ----------------------------------------------------------------------------

/// A 2D plot, e.g. a graph of a function.
///
/// `Plot` supports multiple lines and points.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use egui::plot::{Line, Plot, Value, Values};
/// let sin = (0..1000).map(|i| {
///     let x = i as f64 * 0.01;
///     Value::new(x, x.sin())
/// });
/// let line = Line::new(Values::from_values_iter(sin));
/// Plot::new("my_plot").view_aspect(2.0).show(ui, |plot_ui| plot_ui.line(line));
/// # });
/// ```
pub struct Plot {
    id_source: Id,

    center_x_axis: bool,
    center_y_axis: bool,
    allow_zoom: bool,
    allow_drag: bool,
    min_auto_bounds: PlotBounds,
    margin_fraction: Vec2,

    min_size: Vec2,
    width: Option<f32>,
    height: Option<f32>,
    data_aspect: Option<f32>,
    view_aspect: Option<f32>,

    show_x: bool,
    show_y: bool,
    custom_label_func: CustomLabelFuncRef,
    axis_formatters: [AxisFormatter; 2],
    legend_config: Option<Legend>,
    show_background: bool,
    show_axes: [bool; 2],
}

impl Plot {
    /// Give a unique id for each plot within the same `Ui`.
    pub fn new(id_source: impl std::hash::Hash) -> Self {
        Self {
            id_source: Id::new(id_source),

            center_x_axis: false,
            center_y_axis: false,
            allow_zoom: true,
            allow_drag: true,
            min_auto_bounds: PlotBounds::NOTHING,
            margin_fraction: Vec2::splat(0.05),

            min_size: Vec2::splat(64.0),
            width: None,
            height: None,
            data_aspect: None,
            view_aspect: None,

            show_x: true,
            show_y: true,
            custom_label_func: None,
            axis_formatters: [None, None], // [None; 2] requires Copy
            legend_config: None,
            show_background: true,
            show_axes: [true; 2],
        }
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

    /// Provide a function to customize the on-hovel label for the x and y axis
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// use egui::plot::{Line, Plot, Value, Values};
    /// let sin = (0..1000).map(|i| {
    ///     let x = i as f64 * 0.01;
    ///     Value::new(x, x.sin())
    /// });
    /// let line = Line::new(Values::from_values_iter(sin));
    /// Plot::new("my_plot").view_aspect(2.0)
    /// .custom_label_func(|name, value| {
    ///     if !name.is_empty() {
    ///         format!("{}: {:.*}%", name, 1, value.y).to_string()
    ///     } else {
    ///         "".to_string()
    ///     }
    /// })
    /// .show(ui, |plot_ui| plot_ui.line(line));
    /// # });
    /// ```
    pub fn custom_label_func(
        mut self,
        custom_label_func: impl Fn(&str, &Value) -> String + 'static,
    ) -> Self {
        self.custom_label_func = Some(Box::new(custom_label_func));
        self
    }

    /// Provide a function to customize the labels for the X axis.
    ///
    /// This is useful for custom input domains, e.g. date/time.
    ///
    /// If axis labels should not appear for certain values or beyond a certain zoom/resolution,
    /// the formatter function can return empty strings. This is also useful if your domain is
    /// discrete (e.g. only full days in a calendar).
    pub fn x_axis_formatter(mut self, func: impl Fn(f64) -> String + 'static) -> Self {
        self.axis_formatters[0] = Some(Box::new(func));
        self
    }

    /// Provide a function to customize the labels for the Y axis.
    ///
    /// This is useful for custom value representation, e.g. percentage or units.
    ///
    /// If axis labels should not appear for certain values or beyond a certain zoom/resolution,
    /// the formatter function can return empty strings. This is also useful if your Y values are
    /// discrete (e.g. only integers).
    pub fn y_axis_formatter(mut self, func: impl Fn(f64) -> String + 'static) -> Self {
        self.axis_formatters[1] = Some(Box::new(func));
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

    /// Show a legend including all named items.
    pub fn legend(mut self, legend: Legend) -> Self {
        self.legend_config = Some(legend);
        self
    }

    /// Whether or not to show the background `Rect`.
    /// Can be useful to disable if the plot is overlaid over existing content.
    /// Default: `true`.
    pub fn show_background(mut self, show: bool) -> Self {
        self.show_background = show;
        self
    }

    /// Show the axes.
    /// Can be useful to disable if the plot is overlaid over an existing grid or content.
    /// Default: `[true; 2]`.
    pub fn show_axes(mut self, show: [bool; 2]) -> Self {
        self.show_axes = show;
        self
    }

    /// Interact with and add items to the plot and finally draw it.
    pub fn show<R>(self, ui: &mut Ui, build_fn: impl FnOnce(&mut PlotUi) -> R) -> InnerResponse<R> {
        let Self {
            id_source,
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
            mut show_x,
            mut show_y,
            custom_label_func,
            axis_formatters,
            legend_config,
            show_background,
            show_axes,
        } = self;

        // Determine the size of the plot in the UI
        let size = {
            let width = width
                .unwrap_or_else(|| {
                    if let (Some(height), Some(aspect)) = (height, view_aspect) {
                        height * aspect
                    } else {
                        ui.available_size_before_wrap().x
                    }
                })
                .at_least(min_size.x);

            let height = height
                .unwrap_or_else(|| {
                    if let Some(aspect) = view_aspect {
                        width / aspect
                    } else {
                        ui.available_size_before_wrap().y
                    }
                })
                .at_least(min_size.y);
            vec2(width, height)
        };

        // Allocate the space.
        let (rect, response) = ui.allocate_exact_size(size, Sense::drag());

        // Load or initialize the memory.
        let plot_id = ui.make_persistent_id(id_source);
        let mut memory = PlotMemory::load(ui.ctx(), plot_id).unwrap_or_else(|| PlotMemory {
            auto_bounds: !min_auto_bounds.is_valid(),
            hovered_entry: None,
            hidden_items: Default::default(),
            min_auto_bounds,
            last_screen_transform: ScreenTransform::new(
                rect,
                min_auto_bounds,
                center_x_axis,
                center_y_axis,
            ),
        });

        // If the min bounds changed, recalculate everything.
        if min_auto_bounds != memory.min_auto_bounds {
            memory = PlotMemory {
                auto_bounds: !min_auto_bounds.is_valid(),
                hovered_entry: None,
                min_auto_bounds,
                ..memory
            };
            memory.clone().store(ui.ctx(), plot_id);
        }

        let PlotMemory {
            mut auto_bounds,
            mut hovered_entry,
            mut hidden_items,
            last_screen_transform,
            ..
        } = memory;

        // Call the plot build function.
        let mut plot_ui = PlotUi {
            items: Vec::new(),
            next_auto_color_idx: 0,
            last_screen_transform,
            response,
            ctx: ui.ctx().clone(),
        };
        let inner = build_fn(&mut plot_ui);
        let PlotUi {
            mut items,
            mut response,
            last_screen_transform,
            ..
        } = plot_ui;

        // Background
        if show_background {
            ui.painter().sub_region(rect).add(epaint::RectShape {
                rect,
                corner_radius: 2.0,
                fill: ui.visuals().extreme_bg_color,
                stroke: ui.visuals().widgets.noninteractive.bg_stroke,
            });
        }

        // --- Legend ---
        let legend = legend_config
            .and_then(|config| LegendWidget::try_new(rect, config, &items, &hidden_items));
        // Don't show hover cursor when hovering over legend.
        if hovered_entry.is_some() {
            show_x = false;
            show_y = false;
        }
        // Remove the deselected items.
        items.retain(|item| !hidden_items.contains(item.name()));
        // Highlight the hovered items.
        if let Some(hovered_name) = &hovered_entry {
            items
                .iter_mut()
                .filter(|entry| entry.name() == hovered_name)
                .for_each(|entry| entry.highlight());
        }
        // Move highlighted items to front.
        items.sort_by_key(|item| item.highlighted());

        // --- Bound computation ---
        let mut bounds = *last_screen_transform.bounds();

        // Allow double clicking to reset to automatic bounds.
        auto_bounds |= response.double_clicked_by(PointerButton::Primary);

        // Set bounds automatically based on content.
        if auto_bounds || !bounds.is_valid() {
            bounds = min_auto_bounds;
            items
                .iter()
                .for_each(|item| bounds.merge(&item.get_bounds()));
            bounds.add_relative_margin(margin_fraction);
        }

        let mut transform = ScreenTransform::new(rect, bounds, center_x_axis, center_y_axis);

        // Enforce equal aspect ratio.
        if let Some(data_aspect) = data_aspect {
            transform.set_aspect(data_aspect as f64);
        }

        // Dragging
        if allow_drag && response.dragged_by(PointerButton::Primary) {
            response = response.on_hover_cursor(CursorIcon::Grabbing);
            transform.translate_bounds(-response.drag_delta());
            auto_bounds = false;
        }

        // Zooming
        if allow_zoom {
            if let Some(hover_pos) = response.hover_pos() {
                let zoom_factor = if data_aspect.is_some() {
                    Vec2::splat(ui.input().zoom_delta())
                } else {
                    ui.input().zoom_delta_2d()
                };
                if zoom_factor != Vec2::splat(1.0) {
                    transform.zoom(zoom_factor, hover_pos);
                    auto_bounds = false;
                }

                let scroll_delta = ui.input().scroll_delta;
                if scroll_delta != Vec2::ZERO {
                    transform.translate_bounds(-scroll_delta);
                    auto_bounds = false;
                }
            }
        }

        // Initialize values from functions.
        items
            .iter_mut()
            .for_each(|item| item.initialize(transform.bounds().range_x()));

        let prepared = PreparedPlot {
            items,
            show_x,
            show_y,
            custom_label_func,
            axis_formatters,
            show_axes,
            transform: transform.clone(),
        };
        prepared.ui(ui, &response);

        if let Some(mut legend) = legend {
            ui.add(&mut legend);
            hidden_items = legend.get_hidden_items();
            hovered_entry = legend.get_hovered_entry_name();
        }

        let memory = PlotMemory {
            auto_bounds,
            hovered_entry,
            hidden_items,
            min_auto_bounds,
            last_screen_transform: transform,
        };
        memory.store(ui.ctx(), plot_id);

        let response = if show_x || show_y {
            response.on_hover_cursor(CursorIcon::Crosshair)
        } else {
            response
        };

        InnerResponse { inner, response }
    }
}

/// Provides methods to interact with a plot while building it. It is the single argument of the closure
/// provided to `Plot::show`. See [`Plot`] for an example of how to use it.
pub struct PlotUi {
    items: Vec<Box<dyn PlotItem>>,
    next_auto_color_idx: usize,
    last_screen_transform: ScreenTransform,
    response: Response,
    ctx: Context,
}

impl PlotUi {
    fn auto_color(&mut self) -> Color32 {
        let i = self.next_auto_color_idx;
        self.next_auto_color_idx += 1;
        let golden_ratio = (5.0_f32.sqrt() - 1.0) / 2.0; // 0.61803398875
        let h = i as f32 * golden_ratio;
        Hsva::new(h, 0.85, 0.5, 1.0).into() // TODO: OkLab or some other perspective color space
    }

    pub fn ctx(&self) -> &Context {
        &self.ctx
    }

    /// The plot bounds as they were in the last frame. If called on the first frame and the bounds were not
    /// further specified in the plot builder, this will return bounds centered on the origin. The bounds do
    /// not change until the plot is drawn.
    pub fn plot_bounds(&self) -> PlotBounds {
        *self.last_screen_transform.bounds()
    }

    /// Returns `true` if the plot area is currently hovered.
    pub fn plot_hovered(&self) -> bool {
        self.response.hovered()
    }

    /// The pointer position in plot coordinates. Independent of whether the pointer is in the plot area.
    pub fn pointer_coordinate(&self) -> Option<Value> {
        // We need to subtract the drag delta to keep in sync with the frame-delayed screen transform:
        let last_pos = self.ctx().input().pointer.latest_pos()? - self.response.drag_delta();
        let value = self.plot_from_screen(last_pos);
        Some(value)
    }

    /// The pointer drag delta in plot coordinates.
    pub fn pointer_coordinate_drag_delta(&self) -> Vec2 {
        let delta = self.response.drag_delta();
        let dp_dv = self.last_screen_transform.dpos_dvalue();
        Vec2::new(delta.x / dp_dv[0] as f32, delta.y / dp_dv[1] as f32)
    }

    /// Transform the plot coordinates to screen coordinates.
    pub fn screen_from_plot(&self, position: Value) -> Pos2 {
        self.last_screen_transform.position_from_value(&position)
    }

    /// Transform the screen coordinates to plot coordinates.
    pub fn plot_from_screen(&self, position: Pos2) -> Value {
        self.last_screen_transform.value_from_position(position)
    }

    /// Add a data line.
    pub fn line(&mut self, mut line: Line) {
        if line.series.is_empty() {
            return;
        };

        // Give the stroke an automatic color if no color has been assigned.
        if line.stroke.color == Color32::TRANSPARENT {
            line.stroke.color = self.auto_color();
        }
        self.items.push(Box::new(line));
    }

    /// Add a polygon. The polygon has to be convex.
    pub fn polygon(&mut self, mut polygon: Polygon) {
        if polygon.series.is_empty() {
            return;
        };

        // Give the stroke an automatic color if no color has been assigned.
        if polygon.stroke.color == Color32::TRANSPARENT {
            polygon.stroke.color = self.auto_color();
        }
        self.items.push(Box::new(polygon));
    }

    /// Add a text.
    pub fn text(&mut self, text: Text) {
        if text.text.is_empty() {
            return;
        };

        self.items.push(Box::new(text));
    }

    /// Add data points.
    pub fn points(&mut self, mut points: Points) {
        if points.series.is_empty() {
            return;
        };

        // Give the points an automatic color if no color has been assigned.
        if points.color == Color32::TRANSPARENT {
            points.color = self.auto_color();
        }
        self.items.push(Box::new(points));
    }

    /// Add arrows.
    pub fn arrows(&mut self, mut arrows: Arrows) {
        if arrows.origins.is_empty() || arrows.tips.is_empty() {
            return;
        };

        // Give the arrows an automatic color if no color has been assigned.
        if arrows.color == Color32::TRANSPARENT {
            arrows.color = self.auto_color();
        }
        self.items.push(Box::new(arrows));
    }

    /// Add an image.
    pub fn image(&mut self, image: PlotImage) {
        self.items.push(Box::new(image));
    }

    /// Add a horizontal line.
    /// Can be useful e.g. to show min/max bounds or similar.
    /// Always fills the full width of the plot.
    pub fn hline(&mut self, mut hline: HLine) {
        if hline.stroke.color == Color32::TRANSPARENT {
            hline.stroke.color = self.auto_color();
        }
        self.items.push(Box::new(hline));
    }

    /// Add a vertical line.
    /// Can be useful e.g. to show min/max bounds or similar.
    /// Always fills the full height of the plot.
    pub fn vline(&mut self, mut vline: VLine) {
        if vline.stroke.color == Color32::TRANSPARENT {
            vline.stroke.color = self.auto_color();
        }
        self.items.push(Box::new(vline));
    }

    /// Add a box plot diagram.
    pub fn box_plot(&mut self, mut box_plot: BoxPlot) {
        if box_plot.boxes.is_empty() {
            return;
        }

        // Give the elements an automatic color if no color has been assigned.
        if box_plot.default_color == Color32::TRANSPARENT {
            box_plot = box_plot.color(self.auto_color());
        }
        self.items.push(Box::new(box_plot));
    }

    /// Add a bar chart.
    pub fn bar_chart(&mut self, mut chart: BarChart) {
        if chart.bars.is_empty() {
            return;
        }

        // Give the elements an automatic color if no color has been assigned.
        if chart.default_color == Color32::TRANSPARENT {
            chart = chart.color(self.auto_color());
        }
        self.items.push(Box::new(chart));
    }
}

struct PreparedPlot {
    items: Vec<Box<dyn PlotItem>>,
    show_x: bool,
    show_y: bool,
    custom_label_func: CustomLabelFuncRef,
    axis_formatters: [AxisFormatter; 2],
    show_axes: [bool; 2],
    transform: ScreenTransform,
}

impl PreparedPlot {
    fn ui(self, ui: &mut Ui, response: &Response) {
        let mut shapes = Vec::new();

        for d in 0..2 {
            if self.show_axes[d] {
                self.paint_axis(ui, d, &mut shapes);
            }
        }

        let transform = &self.transform;

        let mut plot_ui = ui.child_ui(*transform.frame(), Layout::default());
        plot_ui.set_clip_rect(*transform.frame());
        for item in &self.items {
            item.get_shapes(&mut plot_ui, transform, &mut shapes);
        }

        if let Some(pointer) = response.hover_pos() {
            self.hover(ui, pointer, &mut shapes);
        }

        ui.painter().sub_region(*transform.frame()).extend(shapes);
    }

    fn paint_axis(&self, ui: &Ui, axis: usize, shapes: &mut Vec<Shape>) {
        let Self {
            transform,
            axis_formatters,
            ..
        } = self;

        let bounds = transform.bounds();

        let font_id = TextStyle::Body.resolve(ui.style());

        let base: i64 = 10;
        let basef = base as f64;

        let min_line_spacing_in_points = 6.0; // TODO: large enough for a wide label
        let step_size = transform.dvalue_dpos()[axis] * min_line_spacing_in_points;
        let step_size = basef.powi(step_size.abs().log(basef).ceil() as i32);

        let step_size_in_points = (transform.dpos_dvalue()[axis] * step_size).abs() as f32;

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

            let n = (value_main / step_size).round() as i64;
            let spacing_in_points = if n % (base * base) == 0 {
                step_size_in_points * (basef * basef) as f32 // think line (multiple of 100)
            } else if n % base == 0 {
                step_size_in_points * basef as f32 // medium line (multiple of 10)
            } else {
                step_size_in_points // thin line
            };

            let line_alpha = remap_clamp(
                spacing_in_points,
                (min_line_spacing_in_points as f32)..=300.0,
                0.0..=0.15,
            );

            if line_alpha > 0.0 {
                let line_color = color_from_alpha(ui, line_alpha);

                let mut p0 = pos_in_gui;
                let mut p1 = pos_in_gui;
                p0[1 - axis] = transform.frame().min[1 - axis];
                p1[1 - axis] = transform.frame().max[1 - axis];
                shapes.push(Shape::line_segment([p0, p1], Stroke::new(1.0, line_color)));
            }

            let text_alpha = remap_clamp(spacing_in_points, 40.0..=150.0, 0.0..=0.4);

            if text_alpha > 0.0 {
                let color = color_from_alpha(ui, text_alpha);

                let text: String = if let Some(formatter) = axis_formatters[axis].as_deref() {
                    formatter(value_main)
                } else {
                    emath::round_to_decimals(value_main, 5).to_string() // hack
                };

                // Custom formatters can return empty string to signal "no label at this resolution"
                if !text.is_empty() {
                    let galley = ui.painter().layout_no_wrap(text, font_id.clone(), color);

                    let mut text_pos = pos_in_gui + vec2(1.0, -galley.size().y);

                    // Make sure we see the labels, even if the axis is off-screen:
                    text_pos[1 - axis] = text_pos[1 - axis]
                        .at_most(transform.frame().max[1 - axis] - galley.size()[1 - axis] - 2.0)
                        .at_least(transform.frame().min[1 - axis] + 1.0);

                    shapes.push(Shape::galley(text_pos, galley));
                }
            }
        }

        fn color_from_alpha(ui: &Ui, alpha: f32) -> Color32 {
            if ui.visuals().dark_mode {
                Rgba::from_white_alpha(alpha).into()
            } else {
                Rgba::from_black_alpha((4.0 * alpha).at_most(1.0)).into()
            }
        }
    }

    fn hover(&self, ui: &Ui, pointer: Pos2, shapes: &mut Vec<Shape>) {
        let Self {
            transform,
            show_x,
            show_y,
            custom_label_func,
            items,
            ..
        } = self;

        if !show_x && !show_y {
            return;
        }

        let interact_radius_sq: f32 = (16.0f32).powi(2);

        let candidates = items.iter().filter_map(|item| {
            let item = &**item;
            let closest = item.find_closest(pointer, transform);

            Some(item).zip(closest)
        });

        let closest = candidates
            .min_by_key(|(_, elem)| elem.dist_sq.ord())
            .filter(|(_, elem)| elem.dist_sq <= interact_radius_sq);

        let plot = items::PlotConfig {
            ui,
            transform,
            show_x: *show_x,
            show_y: *show_y,
        };

        if let Some((item, elem)) = closest {
            item.on_hover(elem, shapes, &plot, custom_label_func);
        } else {
            let value = transform.value_from_position(pointer);
            items::rulers_at_value(pointer, value, "", &plot, shapes, custom_label_func);
        }
    }
}
