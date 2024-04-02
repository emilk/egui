//! Simple plotting library for [`egui`](https://github.com/emilk/egui).
//!
//! Check out [`Plot`] for how to get started.
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]
//!

mod axis;
mod items;
mod legend;
mod memory;
mod plot_ui;
mod transform;

use std::{cmp::Ordering, ops::RangeInclusive, sync::Arc};

use egui::ahash::HashMap;
use egui::*;
use epaint::{util::FloatOrd, Hsva};

pub use crate::{
    axis::{Axis, AxisHints, HPlacement, Placement, VPlacement},
    items::{
        Arrows, Bar, BarChart, BoxElem, BoxPlot, BoxSpread, HLine, Line, LineStyle, MarkerShape,
        Orientation, PlotGeometry, PlotImage, PlotItem, PlotPoint, PlotPoints, Points, Polygon,
        Text, VLine,
    },
    legend::{Corner, Legend},
    memory::PlotMemory,
    plot_ui::PlotUi,
    transform::{PlotBounds, PlotTransform},
};

use axis::AxisWidget;
use items::{horizontal_line, rulers_color, vertical_line};
use legend::LegendWidget;

type LabelFormatterFn = dyn Fn(&str, &PlotPoint) -> String;
pub type LabelFormatter = Option<Box<LabelFormatterFn>>;

type GridSpacerFn = dyn Fn(GridInput) -> Vec<GridMark>;
type GridSpacer = Box<GridSpacerFn>;

type CoordinatesFormatterFn = dyn Fn(&PlotPoint, &PlotBounds) -> String;

/// Specifies the coordinates formatting when passed to [`Plot::coordinates_formatter`].
pub struct CoordinatesFormatter {
    function: Box<CoordinatesFormatterFn>,
}

impl CoordinatesFormatter {
    /// Create a new formatter based on the pointer coordinate and the plot bounds.
    pub fn new(function: impl Fn(&PlotPoint, &PlotBounds) -> String + 'static) -> Self {
        Self {
            function: Box::new(function),
        }
    }

    /// Show a fixed number of decimal places.
    pub fn with_decimals(num_decimals: usize) -> Self {
        Self {
            function: Box::new(move |value, _| {
                format!("x: {:.d$}\ny: {:.d$}", value.x, value.y, d = num_decimals)
            }),
        }
    }

    fn format(&self, value: &PlotPoint, bounds: &PlotBounds) -> String {
        (self.function)(value, bounds)
    }
}

impl Default for CoordinatesFormatter {
    fn default() -> Self {
        Self::with_decimals(3)
    }
}

// ----------------------------------------------------------------------------

/// Indicates a vertical or horizontal cursor line in plot coordinates.
#[derive(Copy, Clone, PartialEq)]
pub enum Cursor {
    Horizontal { y: f64 },
    Vertical { x: f64 },
}

/// Contains the cursors drawn for a plot widget in a single frame.
#[derive(PartialEq, Clone)]
struct PlotFrameCursors {
    id: Id,
    cursors: Vec<Cursor>,
}

#[derive(Default, Clone)]
struct CursorLinkGroups(HashMap<Id, Vec<PlotFrameCursors>>);

#[derive(Clone)]
struct LinkedBounds {
    bounds: PlotBounds,
    auto_bounds: Vec2b,
}

#[derive(Default, Clone)]
struct BoundsLinkGroups(HashMap<Id, LinkedBounds>);

// ----------------------------------------------------------------------------

/// What [`Plot::show`] returns.
pub struct PlotResponse<R> {
    /// What the user closure returned.
    pub inner: R,

    /// The response of the plot.
    pub response: Response,

    /// The transform between screen coordinates and plot coordinates.
    pub transform: PlotTransform,

    /// The id of a currently hovered item if any.
    ///
    /// This is `None` if either no item was hovered, or the hovered item didn't provide an id.
    pub hovered_plot_item: Option<Id>,
}

// ----------------------------------------------------------------------------

/// A 2D plot, e.g. a graph of a function.
///
/// [`Plot`] supports multiple lines and points.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// use egui_plot::{Line, Plot, PlotPoints};
///
/// let sin: PlotPoints = (0..1000).map(|i| {
///     let x = i as f64 * 0.01;
///     [x, x.sin()]
/// }).collect();
/// let line = Line::new(sin);
/// Plot::new("my_plot").view_aspect(2.0).show(ui, |plot_ui| plot_ui.line(line));
/// # });
/// ```
pub struct Plot {
    id_source: Id,
    id: Option<Id>,

    center_axis: Vec2b,
    allow_zoom: Vec2b,
    allow_drag: Vec2b,
    allow_scroll: Vec2b,
    allow_double_click_reset: bool,
    allow_boxed_zoom: bool,
    default_auto_bounds: Vec2b,
    min_auto_bounds: PlotBounds,
    margin_fraction: Vec2,
    boxed_zoom_pointer_button: PointerButton,
    linked_axes: Option<(Id, Vec2b)>,
    linked_cursors: Option<(Id, Vec2b)>,

    min_size: Vec2,
    width: Option<f32>,
    height: Option<f32>,
    data_aspect: Option<f32>,
    view_aspect: Option<f32>,

    reset: bool,

    show_x: bool,
    show_y: bool,
    label_formatter: LabelFormatter,
    coordinates_formatter: Option<(Corner, CoordinatesFormatter)>,
    x_axes: Vec<AxisHints>, // default x axes
    y_axes: Vec<AxisHints>, // default y axes
    legend_config: Option<Legend>,
    show_background: bool,
    show_axes: Vec2b,

    show_grid: Vec2b,
    grid_spacing: Rangef,
    grid_spacers: [GridSpacer; 2],
    sharp_grid_lines: bool,
    clamp_grid: bool,

    sense: Sense,
}

impl Plot {
    /// Give a unique id for each plot within the same [`Ui`].
    pub fn new(id_source: impl std::hash::Hash) -> Self {
        Self {
            id_source: Id::new(id_source),
            id: None,

            center_axis: false.into(),
            allow_zoom: true.into(),
            allow_drag: true.into(),
            allow_scroll: true.into(),
            allow_double_click_reset: true,
            allow_boxed_zoom: true,
            default_auto_bounds: true.into(),
            min_auto_bounds: PlotBounds::NOTHING,
            margin_fraction: Vec2::splat(0.05),
            boxed_zoom_pointer_button: PointerButton::Secondary,
            linked_axes: None,
            linked_cursors: None,

            min_size: Vec2::splat(64.0),
            width: None,
            height: None,
            data_aspect: None,
            view_aspect: None,

            reset: false,

            show_x: true,
            show_y: true,
            label_formatter: None,
            coordinates_formatter: None,
            x_axes: vec![AxisHints::new(Axis::X)],
            y_axes: vec![AxisHints::new(Axis::Y)],
            legend_config: None,
            show_background: true,
            show_axes: true.into(),

            show_grid: true.into(),
            grid_spacing: Rangef::new(8.0, 300.0),
            grid_spacers: [log_grid_spacer(10), log_grid_spacer(10)],
            sharp_grid_lines: true,
            clamp_grid: false,

            sense: egui::Sense::click_and_drag(),
        }
    }

    /// Set an explicit (global) id for the plot.
    ///
    /// This will override the id set by [`Self::new`].
    ///
    /// This is the same `Id` that can be used for [`PlotMemory::load`].
    #[inline]
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// width / height ratio of the data.
    /// For instance, it can be useful to set this to `1.0` for when the two axes show the same
    /// unit.
    /// By default the plot window's aspect ratio is used.
    #[inline]
    pub fn data_aspect(mut self, data_aspect: f32) -> Self {
        self.data_aspect = Some(data_aspect);
        self
    }

    /// width / height ratio of the plot region.
    /// By default no fixed aspect ratio is set (and width/height will fill the ui it is in).
    #[inline]
    pub fn view_aspect(mut self, view_aspect: f32) -> Self {
        self.view_aspect = Some(view_aspect);
        self
    }

    /// Width of plot. By default a plot will fill the ui it is in.
    /// If you set [`Self::view_aspect`], the width can be calculated from the height.
    #[inline]
    pub fn width(mut self, width: f32) -> Self {
        self.min_size.x = width;
        self.width = Some(width);
        self
    }

    /// Height of plot. By default a plot will fill the ui it is in.
    /// If you set [`Self::view_aspect`], the height can be calculated from the width.
    #[inline]
    pub fn height(mut self, height: f32) -> Self {
        self.min_size.y = height;
        self.height = Some(height);
        self
    }

    /// Minimum size of the plot view.
    #[inline]
    pub fn min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    /// Show the x-value (e.g. when hovering). Default: `true`.
    #[inline]
    pub fn show_x(mut self, show_x: bool) -> Self {
        self.show_x = show_x;
        self
    }

    /// Show the y-value (e.g. when hovering). Default: `true`.
    #[inline]
    pub fn show_y(mut self, show_y: bool) -> Self {
        self.show_y = show_y;
        self
    }

    /// Always keep the X-axis centered. Default: `false`.
    #[inline]
    pub fn center_x_axis(mut self, on: bool) -> Self {
        self.center_axis.x = on;
        self
    }

    /// Always keep the Y-axis centered. Default: `false`.
    #[inline]
    pub fn center_y_axis(mut self, on: bool) -> Self {
        self.center_axis.y = on;
        self
    }

    /// Whether to allow zooming in the plot. Default: `true`.
    ///
    /// Note: Allowing zoom in one axis but not the other may lead to unexpected results if used in combination with `data_aspect`.
    #[inline]
    pub fn allow_zoom<T>(mut self, on: T) -> Self
    where
        T: Into<Vec2b>,
    {
        self.allow_zoom = on.into();
        self
    }

    /// Whether to allow scrolling in the plot. Default: `true`.
    #[inline]
    pub fn allow_scroll<T>(mut self, on: T) -> Self
    where
        T: Into<Vec2b>,
    {
        self.allow_scroll = on.into();
        self
    }

    /// Whether to allow double clicking to reset the view.
    /// Default: `true`.
    #[inline]
    pub fn allow_double_click_reset(mut self, on: bool) -> Self {
        self.allow_double_click_reset = on;
        self
    }

    /// Set the side margin as a fraction of the plot size. Only used for auto bounds.
    ///
    /// For instance, a value of `0.1` will add 10% space on both sides.
    #[inline]
    pub fn set_margin_fraction(mut self, margin_fraction: Vec2) -> Self {
        self.margin_fraction = margin_fraction;
        self
    }

    /// Whether to allow zooming in the plot by dragging out a box with the secondary mouse button.
    ///
    /// Default: `true`.
    #[inline]
    pub fn allow_boxed_zoom(mut self, on: bool) -> Self {
        self.allow_boxed_zoom = on;
        self
    }

    /// Config the button pointer to use for boxed zooming. Default: [`Secondary`](PointerButton::Secondary)
    #[inline]
    pub fn boxed_zoom_pointer_button(mut self, boxed_zoom_pointer_button: PointerButton) -> Self {
        self.boxed_zoom_pointer_button = boxed_zoom_pointer_button;
        self
    }

    /// Whether to allow dragging in the plot to move the bounds. Default: `true`.
    #[inline]
    pub fn allow_drag<T>(mut self, on: T) -> Self
    where
        T: Into<Vec2b>,
    {
        self.allow_drag = on.into();
        self
    }

    /// Provide a function to customize the on-hover label for the x and y axis
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// use egui_plot::{Line, Plot, PlotPoints};
    /// let sin: PlotPoints = (0..1000).map(|i| {
    ///     let x = i as f64 * 0.01;
    ///     [x, x.sin()]
    /// }).collect();
    /// let line = Line::new(sin);
    /// Plot::new("my_plot").view_aspect(2.0)
    /// .label_formatter(|name, value| {
    ///     if !name.is_empty() {
    ///         format!("{}: {:.*}%", name, 1, value.y)
    ///     } else {
    ///         "".to_owned()
    ///     }
    /// })
    /// .show(ui, |plot_ui| plot_ui.line(line));
    /// # });
    /// ```
    pub fn label_formatter(
        mut self,
        label_formatter: impl Fn(&str, &PlotPoint) -> String + 'static,
    ) -> Self {
        self.label_formatter = Some(Box::new(label_formatter));
        self
    }

    /// Show the pointer coordinates in the plot.
    pub fn coordinates_formatter(
        mut self,
        position: Corner,
        formatter: CoordinatesFormatter,
    ) -> Self {
        self.coordinates_formatter = Some((position, formatter));
        self
    }

    /// Configure how the grid in the background is spaced apart along the X axis.
    ///
    /// Default is a log-10 grid, i.e. every plot unit is divided into 10 other units.
    ///
    /// The function has this signature:
    /// ```ignore
    /// fn step_sizes(input: GridInput) -> Vec<GridMark>;
    /// ```
    ///
    /// This function should return all marks along the visible range of the X axis.
    /// `step_size` also determines how thick/faint each line is drawn.
    /// For example, if x = 80..=230 is visible and you want big marks at steps of
    /// 100 and small ones at 25, you can return:
    /// ```no_run
    /// # use egui_plot::GridMark;
    /// vec![
    ///    // 100s
    ///    GridMark { value: 100.0, step_size: 100.0 },
    ///    GridMark { value: 200.0, step_size: 100.0 },
    ///
    ///    // 25s
    ///    GridMark { value: 125.0, step_size: 25.0 },
    ///    GridMark { value: 150.0, step_size: 25.0 },
    ///    GridMark { value: 175.0, step_size: 25.0 },
    ///    GridMark { value: 225.0, step_size: 25.0 },
    /// ];
    /// # ()
    /// ```
    ///
    /// There are helpers for common cases, see [`log_grid_spacer`] and [`uniform_grid_spacer`].
    #[inline]
    pub fn x_grid_spacer(mut self, spacer: impl Fn(GridInput) -> Vec<GridMark> + 'static) -> Self {
        self.grid_spacers[0] = Box::new(spacer);
        self
    }

    /// Default is a log-10 grid, i.e. every plot unit is divided into 10 other units.
    ///
    /// See [`Self::x_grid_spacer`] for explanation.
    #[inline]
    pub fn y_grid_spacer(mut self, spacer: impl Fn(GridInput) -> Vec<GridMark> + 'static) -> Self {
        self.grid_spacers[1] = Box::new(spacer);
        self
    }

    /// Set when the grid starts showing.
    ///
    /// When grid lines are closer than the given minimum, they will be hidden.
    /// When they get further apart they will fade in, until the reaches the given maximum,
    /// at which point they are fully opaque.
    #[inline]
    pub fn grid_spacing(mut self, grid_spacing: impl Into<Rangef>) -> Self {
        self.grid_spacing = grid_spacing.into();
        self
    }

    /// Clamp the grid to only be visible at the range of data where we have values.
    ///
    /// Default: `false`.
    #[inline]
    pub fn clamp_grid(mut self, clamp_grid: bool) -> Self {
        self.clamp_grid = clamp_grid;
        self
    }

    /// Set the sense for the plot rect.
    ///
    /// Default: `Sense::click_and_drag()`.
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// Expand bounds to include the given x value.
    /// For instance, to always show the y axis, call `plot.include_x(0.0)`.
    #[inline]
    pub fn include_x(mut self, x: impl Into<f64>) -> Self {
        self.min_auto_bounds.extend_with_x(x.into());
        self
    }

    /// Expand bounds to include the given y value.
    /// For instance, to always show the x axis, call `plot.include_y(0.0)`.
    #[inline]
    pub fn include_y(mut self, y: impl Into<f64>) -> Self {
        self.min_auto_bounds.extend_with_y(y.into());
        self
    }

    /// Set whether the bounds should be automatically set based on data by default.
    ///
    /// This is enabled by default.
    #[inline]
    pub fn auto_bounds(mut self, auto_bounds: Vec2b) -> Self {
        self.default_auto_bounds = auto_bounds;
        self
    }

    /// Expand bounds to fit all items across the x axis, including values given by `include_x`.
    #[deprecated = "Use `auto_bounds` instead"]
    #[inline]
    pub fn auto_bounds_x(mut self) -> Self {
        self.default_auto_bounds.x = true;
        self
    }

    /// Expand bounds to fit all items across the y axis, including values given by `include_y`.
    #[deprecated = "Use `auto_bounds` instead"]
    #[inline]
    pub fn auto_bounds_y(mut self) -> Self {
        self.default_auto_bounds.y = true;
        self
    }

    /// Show a legend including all named items.
    #[inline]
    pub fn legend(mut self, legend: Legend) -> Self {
        self.legend_config = Some(legend);
        self
    }

    /// Whether or not to show the background [`Rect`].
    ///
    /// Can be useful to disable if the plot is overlaid over existing content.
    /// Default: `true`.
    #[inline]
    pub fn show_background(mut self, show: bool) -> Self {
        self.show_background = show;
        self
    }

    /// Show axis labels and grid tick values on the side of the plot.
    ///
    /// Default: `true`.
    #[inline]
    pub fn show_axes(mut self, show: impl Into<Vec2b>) -> Self {
        self.show_axes = show.into();
        self
    }

    /// Show a grid overlay on the plot.
    ///
    /// Default: `true`.
    #[inline]
    pub fn show_grid(mut self, show: impl Into<Vec2b>) -> Self {
        self.show_grid = show.into();
        self
    }

    /// Add this plot to an axis link group so that this plot will share the bounds with other plots in the
    /// same group. A plot cannot belong to more than one axis group.
    #[inline]
    pub fn link_axis(mut self, group_id: impl Into<Id>, link_x: bool, link_y: bool) -> Self {
        self.linked_axes = Some((
            group_id.into(),
            Vec2b {
                x: link_x,
                y: link_y,
            },
        ));
        self
    }

    /// Add this plot to a cursor link group so that this plot will share the cursor position with other plots
    /// in the same group. A plot cannot belong to more than one cursor group.
    #[inline]
    pub fn link_cursor(mut self, group_id: impl Into<Id>, link_x: bool, link_y: bool) -> Self {
        self.linked_cursors = Some((
            group_id.into(),
            Vec2b {
                x: link_x,
                y: link_y,
            },
        ));
        self
    }

    /// Round grid positions to full pixels to avoid aliasing. Improves plot appearance but might have an
    /// undesired effect when shifting the plot bounds. Enabled by default.
    #[inline]
    pub fn sharp_grid_lines(mut self, enabled: bool) -> Self {
        self.sharp_grid_lines = enabled;
        self
    }

    /// Resets the plot.
    #[inline]
    pub fn reset(mut self) -> Self {
        self.reset = true;
        self
    }

    /// Set the x axis label of the main X-axis.
    ///
    /// Default: no label.
    #[inline]
    pub fn x_axis_label(mut self, label: impl Into<WidgetText>) -> Self {
        if let Some(main) = self.x_axes.first_mut() {
            main.label = label.into();
        }
        self
    }

    /// Set the y axis label of the main Y-axis.
    ///
    /// Default: no label.
    #[inline]
    pub fn y_axis_label(mut self, label: impl Into<WidgetText>) -> Self {
        if let Some(main) = self.y_axes.first_mut() {
            main.label = label.into();
        }
        self
    }

    /// Set the position of the main X-axis.
    #[inline]
    pub fn x_axis_position(mut self, placement: axis::VPlacement) -> Self {
        if let Some(main) = self.x_axes.first_mut() {
            main.placement = placement.into();
        }
        self
    }

    /// Set the position of the main Y-axis.
    #[inline]
    pub fn y_axis_position(mut self, placement: axis::HPlacement) -> Self {
        if let Some(main) = self.y_axes.first_mut() {
            main.placement = placement.into();
        }
        self
    }

    /// Specify custom formatter for ticks on the main X-axis.
    ///
    /// Arguments of `fmt`:
    /// * the grid mark to format
    /// * maximum requested number of characters per tick label.
    /// * currently shown range on this axis.
    pub fn x_axis_formatter(
        mut self,
        fmt: impl Fn(GridMark, usize, &RangeInclusive<f64>) -> String + 'static,
    ) -> Self {
        if let Some(main) = self.x_axes.first_mut() {
            main.formatter = Arc::new(fmt);
        }
        self
    }

    /// Specify custom formatter for ticks on the main Y-axis.
    ///
    /// Arguments of `fmt`:
    /// * the grid mark to format
    /// * maximum requested number of characters per tick label.
    /// * currently shown range on this axis.
    pub fn y_axis_formatter(
        mut self,
        fmt: impl Fn(GridMark, usize, &RangeInclusive<f64>) -> String + 'static,
    ) -> Self {
        if let Some(main) = self.y_axes.first_mut() {
            main.formatter = Arc::new(fmt);
        }
        self
    }

    /// Set the main Y-axis-width by number of digits
    ///
    /// The default is 5 digits.
    ///
    /// > Todo: This is experimental. Changing the font size might break this.
    #[inline]
    pub fn y_axis_width(mut self, digits: usize) -> Self {
        if let Some(main) = self.y_axes.first_mut() {
            main.digits = digits;
        }
        self
    }

    /// Set custom configuration for X-axis
    ///
    /// More than one axis may be specified. The first specified axis is considered the main axis.
    #[inline]
    pub fn custom_x_axes(mut self, hints: Vec<AxisHints>) -> Self {
        self.x_axes = hints;
        self
    }

    /// Set custom configuration for left Y-axis
    ///
    /// More than one axis may be specified. The first specified axis is considered the main axis.
    #[inline]
    pub fn custom_y_axes(mut self, hints: Vec<AxisHints>) -> Self {
        self.y_axes = hints;
        self
    }

    /// Interact with and add items to the plot and finally draw it.
    pub fn show<R>(self, ui: &mut Ui, build_fn: impl FnOnce(&mut PlotUi) -> R) -> PlotResponse<R> {
        self.show_dyn(ui, Box::new(build_fn))
    }

    fn show_dyn<'a, R>(
        self,
        ui: &mut Ui,
        build_fn: Box<dyn FnOnce(&mut PlotUi) -> R + 'a>,
    ) -> PlotResponse<R> {
        let Self {
            id_source,
            id,
            center_axis,
            allow_zoom,
            allow_drag,
            allow_scroll,
            allow_double_click_reset,
            allow_boxed_zoom,
            boxed_zoom_pointer_button,
            default_auto_bounds,
            min_auto_bounds,
            margin_fraction,
            width,
            height,
            min_size,
            data_aspect,
            view_aspect,
            mut show_x,
            mut show_y,
            label_formatter,
            coordinates_formatter,
            x_axes,
            y_axes,
            legend_config,
            reset,
            show_background,
            show_axes,
            show_grid,
            grid_spacing,
            linked_axes,
            linked_cursors,

            clamp_grid,
            grid_spacers,
            sharp_grid_lines,
            sense,
        } = self;

        // Determine position of widget.
        let pos = ui.available_rect_before_wrap().min;
        // Determine size of widget.
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

        // Determine complete rect of widget.
        let complete_rect = Rect {
            min: pos,
            max: pos + size,
        };

        let plot_id = id.unwrap_or_else(|| ui.make_persistent_id(id_source));

        let ([x_axis_widgets, y_axis_widgets], plot_rect) = axis_widgets(
            PlotMemory::load(ui.ctx(), plot_id).as_ref(), // TODO(emilk): avoid loading plot memory twice
            show_axes,
            complete_rect,
            [&x_axes, &y_axes],
        );

        // Allocate the plot window.
        let response = ui.allocate_rect(plot_rect, sense);

        // Load or initialize the memory.
        ui.ctx().check_for_id_clash(plot_id, plot_rect, "Plot");

        let mut mem = if reset {
            if let Some((name, _)) = linked_axes.as_ref() {
                ui.data_mut(|data| {
                    let link_groups: &mut BoundsLinkGroups = data.get_temp_mut_or_default(Id::NULL);
                    link_groups.0.remove(name);
                });
            };
            None
        } else {
            PlotMemory::load(ui.ctx(), plot_id)
        }
        .unwrap_or_else(|| PlotMemory {
            auto_bounds: default_auto_bounds,
            hovered_legend_item: None,
            hidden_items: Default::default(),
            transform: PlotTransform::new(plot_rect, min_auto_bounds, center_axis.x, center_axis.y),
            last_click_pos_for_zoom: None,
            x_axis_thickness: Default::default(),
            y_axis_thickness: Default::default(),
        });

        let last_plot_transform = mem.transform;

        // Call the plot build function.
        let mut plot_ui = PlotUi {
            ctx: ui.ctx().clone(),
            items: Vec::new(),
            next_auto_color_idx: 0,
            last_plot_transform,
            last_auto_bounds: mem.auto_bounds,
            response,
            bounds_modifications: Vec::new(),
        };
        let inner = build_fn(&mut plot_ui);
        let PlotUi {
            mut items,
            mut response,
            last_plot_transform,
            bounds_modifications,
            ..
        } = plot_ui;

        // Background
        if show_background {
            ui.painter()
                .with_clip_rect(plot_rect)
                .add(epaint::RectShape::new(
                    plot_rect,
                    Rounding::same(2.0),
                    ui.visuals().extreme_bg_color,
                    ui.visuals().widgets.noninteractive.bg_stroke,
                ));
        }

        // --- Legend ---
        let legend = legend_config
            .and_then(|config| LegendWidget::try_new(plot_rect, config, &items, &mem.hidden_items));
        // Don't show hover cursor when hovering over legend.
        if mem.hovered_legend_item.is_some() {
            show_x = false;
            show_y = false;
        }
        // Remove the deselected items.
        items.retain(|item| !mem.hidden_items.contains(item.name()));
        // Highlight the hovered items.
        if let Some(hovered_name) = &mem.hovered_legend_item {
            items
                .iter_mut()
                .filter(|entry| entry.name() == hovered_name)
                .for_each(|entry| entry.highlight());
        }
        // Move highlighted items to front.
        items.sort_by_key(|item| item.highlighted());

        // --- Bound computation ---
        let mut bounds = *last_plot_transform.bounds();

        // Find the cursors from other plots we need to draw
        let draw_cursors: Vec<Cursor> = if let Some((id, _)) = linked_cursors.as_ref() {
            ui.data_mut(|data| {
                let frames: &mut CursorLinkGroups = data.get_temp_mut_or_default(Id::NULL);
                let cursors = frames.0.entry(*id).or_default();

                // Look for our previous frame
                let index = cursors
                    .iter()
                    .enumerate()
                    .find(|(_, frame)| frame.id == plot_id)
                    .map(|(i, _)| i);

                // Remove our previous frame and all older frames as these are no longer displayed. This avoids
                // unbounded growth, as we add an entry each time we draw a plot.
                index.map(|index| cursors.drain(0..=index));

                // Gather all cursors of the remaining frames. This will be all the cursors of the
                // other plots in the group. We want to draw these in the current plot too.
                cursors
                    .iter()
                    .flat_map(|frame| frame.cursors.iter().copied())
                    .collect()
            })
        } else {
            Vec::new()
        };

        // Transfer the bounds from a link group.
        if let Some((id, axes)) = linked_axes.as_ref() {
            ui.data_mut(|data| {
                let link_groups: &mut BoundsLinkGroups = data.get_temp_mut_or_default(Id::NULL);
                if let Some(linked_bounds) = link_groups.0.get(id) {
                    if axes.x {
                        bounds.set_x(&linked_bounds.bounds);
                        mem.auto_bounds.x = linked_bounds.auto_bounds.x;
                    }
                    if axes.y {
                        bounds.set_y(&linked_bounds.bounds);
                        mem.auto_bounds.y = linked_bounds.auto_bounds.y;
                    }
                };
            });
        };

        // Allow double-clicking to reset to the initial bounds.
        if allow_double_click_reset && response.double_clicked() {
            mem.auto_bounds = true.into();
        }

        // Apply bounds modifications.
        for modification in bounds_modifications {
            match modification {
                BoundsModification::Set(new_bounds) => {
                    bounds = new_bounds;
                    mem.auto_bounds = false.into();
                }
                BoundsModification::Translate(delta) => {
                    bounds.translate(delta);
                    mem.auto_bounds = false.into();
                }
                BoundsModification::AutoBounds(new_auto_bounds) => {
                    mem.auto_bounds = new_auto_bounds;
                }
                BoundsModification::Zoom(zoom_factor, center) => {
                    bounds.zoom(zoom_factor, center);
                    mem.auto_bounds = false.into();
                }
            }
        }

        // Reset bounds to initial bounds if they haven't been modified.
        if mem.auto_bounds.x {
            bounds.set_x(&min_auto_bounds);
        }
        if mem.auto_bounds.y {
            bounds.set_y(&min_auto_bounds);
        }

        let auto_x = mem.auto_bounds.x && (!min_auto_bounds.is_valid_x() || default_auto_bounds.x);
        let auto_y = mem.auto_bounds.y && (!min_auto_bounds.is_valid_y() || default_auto_bounds.y);

        // Set bounds automatically based on content.
        if auto_x || auto_y {
            for item in &items {
                let item_bounds = item.bounds();
                if auto_x {
                    bounds.merge_x(&item_bounds);
                }
                if auto_y {
                    bounds.merge_y(&item_bounds);
                }
            }

            if auto_x {
                bounds.add_relative_margin_x(margin_fraction);
            }

            if auto_y {
                bounds.add_relative_margin_y(margin_fraction);
            }
        }

        mem.transform = PlotTransform::new(plot_rect, bounds, center_axis.x, center_axis.y);

        // Enforce aspect ratio
        if let Some(data_aspect) = data_aspect {
            if let Some((_, linked_axes)) = &linked_axes {
                let change_x = linked_axes.y && !linked_axes.x;
                mem.transform.set_aspect_by_changing_axis(
                    data_aspect as f64,
                    if change_x { Axis::X } else { Axis::Y },
                );
            } else if default_auto_bounds.any() {
                mem.transform.set_aspect_by_expanding(data_aspect as f64);
            } else {
                mem.transform
                    .set_aspect_by_changing_axis(data_aspect as f64, Axis::Y);
            }
        }

        // Dragging
        if allow_drag.any() && response.dragged_by(PointerButton::Primary) {
            response = response.on_hover_cursor(CursorIcon::Grabbing);
            let mut delta = -response.drag_delta();
            if !allow_drag.x {
                delta.x = 0.0;
            }
            if !allow_drag.y {
                delta.y = 0.0;
            }
            mem.transform.translate_bounds(delta);
            mem.auto_bounds = mem.auto_bounds.and(!allow_drag);
        }

        // Zooming
        let mut boxed_zoom_rect = None;
        if allow_boxed_zoom {
            // Save last click to allow boxed zooming
            if response.drag_started() && response.dragged_by(boxed_zoom_pointer_button) {
                // it would be best for egui that input has a memory of the last click pos because it's a common pattern
                mem.last_click_pos_for_zoom = response.hover_pos();
            }
            let box_start_pos = mem.last_click_pos_for_zoom;
            let box_end_pos = response.hover_pos();
            if let (Some(box_start_pos), Some(box_end_pos)) = (box_start_pos, box_end_pos) {
                // while dragging prepare a Shape and draw it later on top of the plot
                if response.dragged_by(boxed_zoom_pointer_button) {
                    response = response.on_hover_cursor(CursorIcon::ZoomIn);
                    let rect = epaint::Rect::from_two_pos(box_start_pos, box_end_pos);
                    boxed_zoom_rect = Some((
                        epaint::RectShape::stroke(
                            rect,
                            0.0,
                            epaint::Stroke::new(4., Color32::DARK_BLUE),
                        ), // Outer stroke
                        epaint::RectShape::stroke(
                            rect,
                            0.0,
                            epaint::Stroke::new(2., Color32::WHITE),
                        ), // Inner stroke
                    ));
                }
                // when the click is release perform the zoom
                if response.drag_stopped() {
                    let box_start_pos = mem.transform.value_from_position(box_start_pos);
                    let box_end_pos = mem.transform.value_from_position(box_end_pos);
                    let new_bounds = PlotBounds {
                        min: [
                            box_start_pos.x.min(box_end_pos.x),
                            box_start_pos.y.min(box_end_pos.y),
                        ],
                        max: [
                            box_start_pos.x.max(box_end_pos.x),
                            box_start_pos.y.max(box_end_pos.y),
                        ],
                    };
                    if new_bounds.is_valid() {
                        mem.transform.set_bounds(new_bounds);
                        mem.auto_bounds = false.into();
                    }
                    // reset the boxed zoom state
                    mem.last_click_pos_for_zoom = None;
                }
            }
        }

        // Note: we catch zoom/pan if the response contains the pointer, even if it isn't hovered.
        // For instance: The user is painting another interactive widget on top of the plot
        // but they still want to be able to pan/zoom the plot.
        if let (true, Some(hover_pos)) = (
            response.contains_pointer,
            ui.input(|i| i.pointer.hover_pos()),
        ) {
            if allow_zoom.any() {
                let mut zoom_factor = if data_aspect.is_some() {
                    Vec2::splat(ui.input(|i| i.zoom_delta()))
                } else {
                    ui.input(|i| i.zoom_delta_2d())
                };
                if !allow_zoom.x {
                    zoom_factor.x = 1.0;
                }
                if !allow_zoom.y {
                    zoom_factor.y = 1.0;
                }
                if zoom_factor != Vec2::splat(1.0) {
                    mem.transform.zoom(zoom_factor, hover_pos);
                    mem.auto_bounds = mem.auto_bounds.and(!allow_zoom);
                }
            }
            if allow_scroll.any() {
                let mut scroll_delta = ui.input(|i| i.smooth_scroll_delta);
                if !allow_scroll.x {
                    scroll_delta.x = 0.0;
                }
                if !allow_scroll.y {
                    scroll_delta.y = 0.0;
                }
                if scroll_delta != Vec2::ZERO {
                    mem.transform.translate_bounds(-scroll_delta);
                    mem.auto_bounds = false.into();
                }
            }
        }

        // --- transform initialized

        // Add legend widgets to plot
        let bounds = mem.transform.bounds();
        let x_axis_range = bounds.range_x();
        let x_steps = Arc::new({
            let input = GridInput {
                bounds: (bounds.min[0], bounds.max[0]),
                base_step_size: mem.transform.dvalue_dpos()[0].abs() * grid_spacing.min as f64,
            };
            (grid_spacers[0])(input)
        });
        let y_axis_range = bounds.range_y();
        let y_steps = Arc::new({
            let input = GridInput {
                bounds: (bounds.min[1], bounds.max[1]),
                base_step_size: mem.transform.dvalue_dpos()[1].abs() * grid_spacing.min as f64,
            };
            (grid_spacers[1])(input)
        });
        for (i, mut widget) in x_axis_widgets.into_iter().enumerate() {
            widget.range = x_axis_range.clone();
            widget.transform = Some(mem.transform);
            widget.steps = x_steps.clone();
            let (_response, thickness) = widget.ui(ui, Axis::X);
            mem.x_axis_thickness.insert(i, thickness);
        }
        for (i, mut widget) in y_axis_widgets.into_iter().enumerate() {
            widget.range = y_axis_range.clone();
            widget.transform = Some(mem.transform);
            widget.steps = y_steps.clone();
            let (_response, thickness) = widget.ui(ui, Axis::Y);
            mem.y_axis_thickness.insert(i, thickness);
        }

        // Initialize values from functions.
        for item in &mut items {
            item.initialize(mem.transform.bounds().range_x());
        }

        let prepared = PreparedPlot {
            items,
            show_x,
            show_y,
            label_formatter,
            coordinates_formatter,
            show_grid,
            grid_spacing,
            transform: mem.transform,
            draw_cursor_x: linked_cursors.as_ref().map_or(false, |group| group.1.x),
            draw_cursor_y: linked_cursors.as_ref().map_or(false, |group| group.1.y),
            draw_cursors,
            grid_spacers,
            sharp_grid_lines,
            clamp_grid,
        };

        let (plot_cursors, hovered_plot_item) = prepared.ui(ui, &response);

        if let Some(boxed_zoom_rect) = boxed_zoom_rect {
            ui.painter()
                .with_clip_rect(plot_rect)
                .add(boxed_zoom_rect.0);
            ui.painter()
                .with_clip_rect(plot_rect)
                .add(boxed_zoom_rect.1);
        }

        if let Some(mut legend) = legend {
            ui.add(&mut legend);
            mem.hidden_items = legend.hidden_items();
            mem.hovered_legend_item = legend.hovered_item_name();
        }

        if let Some((id, _)) = linked_cursors.as_ref() {
            // Push the frame we just drew to the list of frames
            ui.data_mut(|data| {
                let frames: &mut CursorLinkGroups = data.get_temp_mut_or_default(Id::NULL);
                let cursors = frames.0.entry(*id).or_default();
                cursors.push(PlotFrameCursors {
                    id: plot_id,
                    cursors: plot_cursors,
                });
            });
        }

        if let Some((id, _)) = linked_axes.as_ref() {
            // Save the linked bounds.
            ui.data_mut(|data| {
                let link_groups: &mut BoundsLinkGroups = data.get_temp_mut_or_default(Id::NULL);
                link_groups.0.insert(
                    *id,
                    LinkedBounds {
                        bounds: *mem.transform.bounds(),
                        auto_bounds: mem.auto_bounds,
                    },
                );
            });
        }

        let transform = mem.transform;
        mem.store(ui.ctx(), plot_id);

        let response = if show_x || show_y {
            response.on_hover_cursor(CursorIcon::Crosshair)
        } else {
            response
        };

        ui.advance_cursor_after_rect(complete_rect);

        PlotResponse {
            inner,
            response,
            transform,
            hovered_plot_item,
        }
    }
}

/// Returns the rect left after adding axes.
fn axis_widgets(
    mem: Option<&PlotMemory>,
    show_axes: Vec2b,
    complete_rect: Rect,
    [x_axes, y_axes]: [&[AxisHints]; 2],
) -> ([Vec<AxisWidget>; 2], Rect) {
    // Next we want to create this layout.
    // Indices are only examples.
    //
    //  left                     right
    //  +---+---------x----------+   +
    //  |   |      X-axis 3      |
    //  |   +--------------------+    top
    //  |   |      X-axis 2      |
    //  +-+-+--------------------+-+-+
    //  |y|y|                    |y|y|
    //  |-|-|                    |-|-|
    //  |A|A|                    |A|A|
    // y|x|x|    Plot Window     |x|x|
    //  |i|i|                    |i|i|
    //  |s|s|                    |s|s|
    //  |1|0|                    |2|3|
    //  +-+-+--------------------+-+-+
    //      |      X-axis 0      |   |
    //      +--------------------+   | bottom
    //      |      X-axis 1      |   |
    //  +   +--------------------+---+
    //

    let mut x_axis_widgets = Vec::<AxisWidget>::new();
    let mut y_axis_widgets = Vec::<AxisWidget>::new();

    // Will shrink as we add more axes.
    let mut rect_left = complete_rect;

    if show_axes.x {
        // We will fix this later, once we know how much space the y axes take up.
        let initial_x_range = complete_rect.x_range();

        for (i, cfg) in x_axes.iter().enumerate().rev() {
            let mut height = cfg.thickness(Axis::X);
            if let Some(mem) = mem {
                // If the labels took up too much space the previous frame, give them more space now:
                height = height.max(mem.x_axis_thickness.get(&i).copied().unwrap_or_default());
            }

            let rect = match VPlacement::from(cfg.placement) {
                VPlacement::Bottom => {
                    let bottom = rect_left.bottom();
                    *rect_left.bottom_mut() -= height;
                    let top = rect_left.bottom();
                    Rect::from_x_y_ranges(initial_x_range, top..=bottom)
                }
                VPlacement::Top => {
                    let top = rect_left.top();
                    *rect_left.top_mut() += height;
                    let bottom = rect_left.top();
                    Rect::from_x_y_ranges(initial_x_range, top..=bottom)
                }
            };
            x_axis_widgets.push(AxisWidget::new(cfg.clone(), rect));
        }
    }
    if show_axes.y {
        // We know this, since we've already allocated space for the x axes.
        let plot_y_range = rect_left.y_range();

        for (i, cfg) in y_axes.iter().enumerate().rev() {
            let mut width = cfg.thickness(Axis::Y);
            if let Some(mem) = mem {
                // If the labels took up too much space the previous frame, give them more space now:
                width = width.max(mem.y_axis_thickness.get(&i).copied().unwrap_or_default());
            }

            let rect = match HPlacement::from(cfg.placement) {
                HPlacement::Left => {
                    let left = rect_left.left();
                    *rect_left.left_mut() += width;
                    let right = rect_left.left();
                    Rect::from_x_y_ranges(left..=right, plot_y_range)
                }
                HPlacement::Right => {
                    let right = rect_left.right();
                    *rect_left.right_mut() -= width;
                    let left = rect_left.right();
                    Rect::from_x_y_ranges(left..=right, plot_y_range)
                }
            };
            y_axis_widgets.push(AxisWidget::new(cfg.clone(), rect));
        }
    }

    let mut plot_rect = rect_left;

    // If too little space, remove axis widgets
    if plot_rect.width() <= 0.0 || plot_rect.height() <= 0.0 {
        y_axis_widgets.clear();
        x_axis_widgets.clear();
        plot_rect = complete_rect;
    }

    // Bow that we know the final x_range of the plot_rect,
    // assign it to the x_axis_widgets (they are currently too wide):
    for widget in &mut x_axis_widgets {
        widget.rect = Rect::from_x_y_ranges(plot_rect.x_range(), widget.rect.y_range());
    }

    ([x_axis_widgets, y_axis_widgets], plot_rect)
}

/// User-requested modifications to the plot bounds. We collect them in the plot build function to later apply
/// them at the right time, as other modifications need to happen first.
enum BoundsModification {
    Set(PlotBounds),
    Translate(Vec2),
    AutoBounds(Vec2b),
    Zoom(Vec2, PlotPoint),
}

// ----------------------------------------------------------------------------
// Grid

/// Input for "grid spacer" functions.
///
/// See [`Plot::x_grid_spacer()`] and [`Plot::y_grid_spacer()`].
pub struct GridInput {
    /// Min/max of the visible data range (the values at the two edges of the plot,
    /// for the current axis).
    pub bounds: (f64, f64),

    /// Recommended (but not required) lower-bound on the step size returned by custom grid spacers.
    ///
    /// Computed as the ratio between the diagram's bounds (in plot coordinates) and the viewport
    /// (in frame/window coordinates), scaled up to represent the minimal possible step.
    ///
    /// Always positive.
    pub base_step_size: f64,
}

/// One mark (horizontal or vertical line) in the background grid of a plot.
#[derive(Debug, Clone, Copy)]
pub struct GridMark {
    /// X or Y value in the plot.
    pub value: f64,

    /// The (approximate) distance to the next value of same thickness.
    ///
    /// Determines how thick the grid line is painted. It's not important that `step_size`
    /// matches the difference between two `value`s precisely, but rather that grid marks of
    /// same thickness have same `step_size`. For example, months can have a different number
    /// of days, but consistently using a `step_size` of 30 days is a valid approximation.
    pub step_size: f64,
}

/// Recursively splits the grid into `base` subdivisions (e.g. 100, 10, 1).
///
/// The logarithmic base, expressing how many times each grid unit is subdivided.
/// 10 is a typical value, others are possible though.
pub fn log_grid_spacer(log_base: i64) -> GridSpacer {
    let log_base = log_base as f64;
    let step_sizes = move |input: GridInput| -> Vec<GridMark> {
        // handle degenerate cases
        if input.base_step_size.abs() < f64::EPSILON {
            return Vec::new();
        }

        // The distance between two of the thinnest grid lines is "rounded" up
        // to the next-bigger power of base
        let smallest_visible_unit = next_power(input.base_step_size, log_base);

        let step_sizes = [
            smallest_visible_unit,
            smallest_visible_unit * log_base,
            smallest_visible_unit * log_base * log_base,
        ];

        generate_marks(step_sizes, input.bounds)
    };

    Box::new(step_sizes)
}

/// Splits the grid into uniform-sized spacings (e.g. 100, 25, 1).
///
/// This function should return 3 positive step sizes, designating where the lines in the grid are drawn.
/// Lines are thicker for larger step sizes. Ordering of returned value is irrelevant.
///
/// Why only 3 step sizes? Three is the number of different line thicknesses that egui typically uses in the grid.
/// Ideally, those 3 are not hardcoded values, but depend on the visible range (accessible through `GridInput`).
pub fn uniform_grid_spacer(spacer: impl Fn(GridInput) -> [f64; 3] + 'static) -> GridSpacer {
    let get_marks = move |input: GridInput| -> Vec<GridMark> {
        let bounds = input.bounds;
        let step_sizes = spacer(input);
        generate_marks(step_sizes, bounds)
    };

    Box::new(get_marks)
}

// ----------------------------------------------------------------------------

struct PreparedPlot {
    items: Vec<Box<dyn PlotItem>>,
    show_x: bool,
    show_y: bool,
    label_formatter: LabelFormatter,
    coordinates_formatter: Option<(Corner, CoordinatesFormatter)>,
    // axis_formatters: [AxisFormatter; 2],
    transform: PlotTransform,
    show_grid: Vec2b,
    grid_spacing: Rangef,
    grid_spacers: [GridSpacer; 2],
    draw_cursor_x: bool,
    draw_cursor_y: bool,
    draw_cursors: Vec<Cursor>,

    sharp_grid_lines: bool,
    clamp_grid: bool,
}

impl PreparedPlot {
    fn ui(self, ui: &mut Ui, response: &Response) -> (Vec<Cursor>, Option<Id>) {
        let mut axes_shapes = Vec::new();

        if self.show_grid.x {
            self.paint_grid(ui, &mut axes_shapes, Axis::X, self.grid_spacing);
        }
        if self.show_grid.y {
            self.paint_grid(ui, &mut axes_shapes, Axis::Y, self.grid_spacing);
        }

        // Sort the axes by strength so that those with higher strength are drawn in front.
        axes_shapes.sort_by(|(_, strength1), (_, strength2)| strength1.total_cmp(strength2));

        let mut shapes = axes_shapes.into_iter().map(|(shape, _)| shape).collect();

        let transform = &self.transform;

        let mut plot_ui = ui.child_ui(*transform.frame(), Layout::default());
        plot_ui.set_clip_rect(transform.frame().intersect(ui.clip_rect()));
        for item in &self.items {
            item.shapes(&plot_ui, transform, &mut shapes);
        }

        let hover_pos = response.hover_pos();
        let (cursors, hovered_item_id) = if let Some(pointer) = hover_pos {
            self.hover(ui, pointer, &mut shapes)
        } else {
            (Vec::new(), None)
        };

        // Draw cursors
        let line_color = rulers_color(ui);

        let mut draw_cursor = |cursors: &Vec<Cursor>, always| {
            for &cursor in cursors {
                match cursor {
                    Cursor::Horizontal { y } => {
                        if self.draw_cursor_y || always {
                            shapes.push(horizontal_line(
                                transform.position_from_point(&PlotPoint::new(0.0, y)),
                                &self.transform,
                                line_color,
                            ));
                        }
                    }
                    Cursor::Vertical { x } => {
                        if self.draw_cursor_x || always {
                            shapes.push(vertical_line(
                                transform.position_from_point(&PlotPoint::new(x, 0.0)),
                                &self.transform,
                                line_color,
                            ));
                        }
                    }
                }
            }
        };

        draw_cursor(&self.draw_cursors, false);
        draw_cursor(&cursors, true);

        let painter = ui.painter().with_clip_rect(*transform.frame());
        painter.extend(shapes);

        if let Some((corner, formatter)) = self.coordinates_formatter.as_ref() {
            let hover_pos = response.hover_pos();
            if let Some(pointer) = hover_pos {
                let font_id = TextStyle::Monospace.resolve(ui.style());
                let coordinate = transform.value_from_position(pointer);
                let text = formatter.format(&coordinate, transform.bounds());
                let padded_frame = transform.frame().shrink(4.0);
                let (anchor, position) = match corner {
                    Corner::LeftTop => (Align2::LEFT_TOP, padded_frame.left_top()),
                    Corner::RightTop => (Align2::RIGHT_TOP, padded_frame.right_top()),
                    Corner::LeftBottom => (Align2::LEFT_BOTTOM, padded_frame.left_bottom()),
                    Corner::RightBottom => (Align2::RIGHT_BOTTOM, padded_frame.right_bottom()),
                };
                painter.text(position, anchor, text, font_id, ui.visuals().text_color());
            }
        }

        (cursors, hovered_item_id)
    }

    fn paint_grid(&self, ui: &Ui, shapes: &mut Vec<(Shape, f32)>, axis: Axis, fade_range: Rangef) {
        #![allow(clippy::collapsible_else_if)]
        let Self {
            transform,
            // axis_formatters,
            grid_spacers,
            clamp_grid,
            ..
        } = self;

        let iaxis = usize::from(axis);

        // Where on the cross-dimension to show the label values
        let bounds = transform.bounds();
        let value_cross = 0.0_f64.clamp(bounds.min[1 - iaxis], bounds.max[1 - iaxis]);

        let input = GridInput {
            bounds: (bounds.min[iaxis], bounds.max[iaxis]),
            base_step_size: transform.dvalue_dpos()[iaxis].abs() * fade_range.min as f64,
        };
        let steps = (grid_spacers[iaxis])(input);

        let clamp_range = clamp_grid.then(|| {
            let mut tight_bounds = PlotBounds::NOTHING;
            for item in &self.items {
                let item_bounds = item.bounds();
                tight_bounds.merge_x(&item_bounds);
                tight_bounds.merge_y(&item_bounds);
            }
            tight_bounds
        });

        for step in steps {
            let value_main = step.value;

            if let Some(clamp_range) = clamp_range {
                match axis {
                    Axis::X => {
                        if !clamp_range.range_x().contains(&value_main) {
                            continue;
                        };
                    }
                    Axis::Y => {
                        if !clamp_range.range_y().contains(&value_main) {
                            continue;
                        };
                    }
                }
            }

            let value = match axis {
                Axis::X => PlotPoint::new(value_main, value_cross),
                Axis::Y => PlotPoint::new(value_cross, value_main),
            };

            let pos_in_gui = transform.position_from_point(&value);
            let spacing_in_points = (transform.dpos_dvalue()[iaxis] * step.step_size).abs() as f32;

            if spacing_in_points <= fade_range.min {
                continue; // Too close together
            }

            let line_strength = remap_clamp(spacing_in_points, fade_range, 0.0..=1.0);

            let line_color = color_from_strength(ui, line_strength);

            let mut p0 = pos_in_gui;
            let mut p1 = pos_in_gui;
            p0[1 - iaxis] = transform.frame().min[1 - iaxis];
            p1[1 - iaxis] = transform.frame().max[1 - iaxis];

            if let Some(clamp_range) = clamp_range {
                match axis {
                    Axis::X => {
                        p0.y = transform.position_from_point_y(clamp_range.min[1]);
                        p1.y = transform.position_from_point_y(clamp_range.max[1]);
                    }
                    Axis::Y => {
                        p0.x = transform.position_from_point_x(clamp_range.min[0]);
                        p1.x = transform.position_from_point_x(clamp_range.max[0]);
                    }
                }
            }

            if self.sharp_grid_lines {
                // Round to avoid aliasing
                p0 = ui.painter().round_pos_to_pixels(p0);
                p1 = ui.painter().round_pos_to_pixels(p1);
            }

            shapes.push((
                Shape::line_segment([p0, p1], Stroke::new(1.0, line_color)),
                line_strength,
            ));
        }
    }

    fn hover(&self, ui: &Ui, pointer: Pos2, shapes: &mut Vec<Shape>) -> (Vec<Cursor>, Option<Id>) {
        let Self {
            transform,
            show_x,
            show_y,
            label_formatter,
            items,
            ..
        } = self;

        if !show_x && !show_y {
            return (Vec::new(), None);
        }

        let interact_radius_sq = (16.0_f32).powi(2);

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

        let mut cursors = Vec::new();

        let hovered_plot_item_id = if let Some((item, elem)) = closest {
            item.on_hover(elem, shapes, &mut cursors, &plot, label_formatter);
            item.id()
        } else {
            let value = transform.value_from_position(pointer);
            items::rulers_at_value(
                pointer,
                value,
                "",
                &plot,
                shapes,
                &mut cursors,
                label_formatter,
            );
            None
        };

        (cursors, hovered_plot_item_id)
    }
}

/// Returns next bigger power in given base
/// e.g.
/// ```ignore
/// use egui_plot::next_power;
/// assert_eq!(next_power(0.01, 10.0), 0.01);
/// assert_eq!(next_power(0.02, 10.0), 0.1);
/// assert_eq!(next_power(0.2,  10.0), 1);
/// ```
fn next_power(value: f64, base: f64) -> f64 {
    debug_assert_ne!(value, 0.0); // can be negative (typical for Y axis)
    base.powi(value.abs().log(base).ceil() as i32)
}

/// Fill in all values between [min, max] which are a multiple of `step_size`
fn generate_marks(step_sizes: [f64; 3], bounds: (f64, f64)) -> Vec<GridMark> {
    let mut steps = vec![];
    fill_marks_between(&mut steps, step_sizes[0], bounds);
    fill_marks_between(&mut steps, step_sizes[1], bounds);
    fill_marks_between(&mut steps, step_sizes[2], bounds);

    // Remove duplicates:
    // This can happen because we have overlapping steps, e.g.:
    // step_size[0] =   10  =>  [-10, 0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120]
    // step_size[1] =  100  =>  [     0,                                     100          ]
    // step_size[2] = 1000  =>  [     0                                                   ]

    steps.sort_by(|a, b| match cmp_f64(a.value, b.value) {
        // Keep the largest step size when we dedup later
        Ordering::Equal => cmp_f64(b.step_size, a.step_size),

        ord => ord,
    });
    steps.dedup_by(|a, b| a.value == b.value);

    steps
}

fn cmp_f64(a: f64, b: f64) -> Ordering {
    match a.partial_cmp(&b) {
        Some(ord) => ord,
        None => a.is_nan().cmp(&b.is_nan()),
    }
}

/// Fill in all values between [min, max] which are a multiple of `step_size`
fn fill_marks_between(out: &mut Vec<GridMark>, step_size: f64, (min, max): (f64, f64)) {
    debug_assert!(max > min);
    let first = (min / step_size).ceil() as i64;
    let last = (max / step_size).ceil() as i64;

    let marks_iter = (first..last).map(|i| {
        let value = (i as f64) * step_size;
        GridMark { value, step_size }
    });
    out.extend(marks_iter);
}

/// Helper for formatting a number so that we always show at least a few decimals,
/// unless it is an integer, in which case we never show any decimals.
pub fn format_number(number: f64, num_decimals: usize) -> String {
    let is_integral = number as i64 as f64 == number;
    if is_integral {
        // perfect integer - show it as such:
        format!("{number:.0}")
    } else {
        // make sure we tell the user it is not an integer by always showing a decimal or two:
        format!("{:.*}", num_decimals.at_least(1), number)
    }
}

/// Determine a color from a 0-1 strength value.
pub fn color_from_strength(ui: &Ui, strength: f32) -> Color32 {
    let base_color = ui.visuals().text_color();
    base_color.gamma_multiply(strength.sqrt())
}
