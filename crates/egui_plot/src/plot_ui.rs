use egui::{epaint::Hsva, Color32, Context, Pos2, Response, Vec2, Vec2b};

use crate::{
    items::{GenericPlotPoints, PlotItem},
    Arrows, BarChart, BoundsModification, BoxPlot, HLine, Line, PlotBounds, PlotImage, PlotPoint,
    PlotPoints, PlotTransform, Points, Polygon, Text, VLine,
};

pub struct PlotUiBuilder {
    pub(crate) next_auto_color_idx: usize,
    pub(crate) last_plot_transform: PlotTransform,
    pub(crate) last_auto_bounds: Vec2b,
    pub(crate) response: Response,
    pub(crate) bounds_modifications: Vec<BoundsModification>,
    pub(crate) ctx: Context,
}

impl PlotUiBuilder {
    pub fn into_plot_ui<'a>(self) -> PlotUi<'a> {
        let Self {
            next_auto_color_idx,
            last_plot_transform,
            last_auto_bounds,
            response,
            bounds_modifications,
            ctx,
        } = self;

        PlotUi {
            items: Vec::new(),
            next_auto_color_idx,
            last_plot_transform,
            last_auto_bounds,
            response,
            bounds_modifications,
            ctx,
        }
    }
}

impl From<PlotUiBuilder> for PlotUi<'_> {
    fn from(value: PlotUiBuilder) -> Self {
        let PlotUiBuilder {
            next_auto_color_idx,
            last_plot_transform,
            last_auto_bounds,
            response,
            bounds_modifications,
            ctx,
        } = value;

        PlotUi {
            items: Vec::new(),
            next_auto_color_idx,
            last_plot_transform,
            last_auto_bounds,
            response,
            bounds_modifications,
            ctx,
        }
    }
}

/// Provides methods to interact with a plot while building it. It is the single argument of the closure
/// provided to [`Plot::show`]. See [`Plot`] for an example of how to use it.
pub struct PlotUi<'a> {
    pub(crate) items: Vec<Box<dyn PlotItem + 'a>>,
    pub(crate) next_auto_color_idx: usize,
    pub(crate) last_plot_transform: PlotTransform,
    pub(crate) last_auto_bounds: Vec2b,
    pub(crate) response: Response,
    pub(crate) bounds_modifications: Vec<BoundsModification>,
    pub(crate) ctx: Context,
}

impl<'a> PlotUi<'a> {
    fn auto_color(&mut self) -> Color32 {
        let i = self.next_auto_color_idx;
        self.next_auto_color_idx += 1;
        let golden_ratio = (5.0_f32.sqrt() - 1.0) / 2.0; // 0.61803398875
        let h = i as f32 * golden_ratio;
        Hsva::new(h, 0.85, 0.5, 1.0).into() // TODO(emilk): OkLab or some other perspective color space
    }

    pub fn ctx(&self) -> &Context {
        &self.ctx
    }

    /// The plot bounds as they were in the last frame. If called on the first frame and the bounds were not
    /// further specified in the plot builder, this will return bounds centered on the origin. The bounds do
    /// not change until the plot is drawn.
    pub fn plot_bounds(&self) -> PlotBounds {
        *self.last_plot_transform.bounds()
    }

    /// Set the plot bounds. Can be useful for implementing alternative plot navigation methods.
    pub fn set_plot_bounds(&mut self, plot_bounds: PlotBounds) -> &mut Self {
        self.bounds_modifications
            .push(BoundsModification::Set(plot_bounds));

        self
    }

    /// Move the plot bounds. Can be useful for implementing alternative plot navigation methods.
    pub fn translate_bounds(&mut self, delta_pos: Vec2) -> &mut Self {
        self.bounds_modifications
            .push(BoundsModification::Translate(delta_pos));

        self
    }

    /// Whether the plot axes were in auto-bounds mode in the last frame. If called on the first
    /// frame, this is the [`Plot`]'s default auto-bounds mode.
    pub fn auto_bounds(&self) -> Vec2b {
        self.last_auto_bounds
    }

    /// Set the auto-bounds mode for the plot axes.
    pub fn set_auto_bounds(&mut self, auto_bounds: Vec2b) -> &mut Self {
        self.bounds_modifications
            .push(BoundsModification::AutoBounds(auto_bounds));

        self
    }

    /// Can be used to check if the plot was hovered or clicked.
    pub fn response(&self) -> &Response {
        &self.response
    }

    /// Scale the plot bounds around a position in screen coordinates.
    ///
    /// Can be useful for implementing alternative plot navigation methods.
    ///
    /// The plot bounds are divided by `zoom_factor`, therefore:
    /// - `zoom_factor < 1.0` zooms out, i.e., increases the visible range to show more data.
    /// - `zoom_factor > 1.0` zooms in, i.e., reduces the visible range to show more detail.
    pub fn zoom_bounds(&mut self, zoom_factor: Vec2, center: PlotPoint) -> &mut Self {
        self.bounds_modifications
            .push(BoundsModification::Zoom(zoom_factor, center));

        self
    }

    /// Scale the plot bounds around the hovered position, if any.
    ///
    /// Can be useful for implementing alternative plot navigation methods.
    ///
    /// The plot bounds are divided by `zoom_factor`, therefore:
    /// - `zoom_factor < 1.0` zooms out, i.e., increases the visible range to show more data.
    /// - `zoom_factor > 1.0` zooms in, i.e., reduces the visible range to show more detail.
    pub fn zoom_bounds_around_hovered(&mut self, zoom_factor: Vec2) -> &mut Self {
        if let Some(hover_pos) = self.pointer_coordinate() {
            self.zoom_bounds(zoom_factor, hover_pos)
        } else {
            self
        }
    }

    /// The pointer position in plot coordinates. Independent of whether the pointer is in the plot area.
    pub fn pointer_coordinate(&self) -> Option<PlotPoint> {
        // We need to subtract the drag delta to keep in sync with the frame-delayed screen transform:
        let last_pos = self.ctx().input(|i| i.pointer.latest_pos())? - self.response.drag_delta();
        let value = self.plot_from_screen(last_pos);
        Some(value)
    }

    /// The pointer drag delta in plot coordinates.
    pub fn pointer_coordinate_drag_delta(&self) -> Vec2 {
        let delta = self.response.drag_delta();
        let dp_dv = self.last_plot_transform.dpos_dvalue();
        Vec2::new(delta.x / dp_dv[0] as f32, delta.y / dp_dv[1] as f32)
    }

    /// Read the transform between plot coordinates and screen coordinates.
    pub fn transform(&self) -> &PlotTransform {
        &self.last_plot_transform
    }

    /// Transform the plot coordinates to screen coordinates.
    pub fn screen_from_plot(&self, position: PlotPoint) -> Pos2 {
        self.last_plot_transform.position_from_point(&position)
    }

    /// Transform the screen coordinates to plot coordinates.
    pub fn plot_from_screen(&self, position: Pos2) -> PlotPoint {
        self.last_plot_transform.value_from_position(position)
    }

    /// Add a data line.
    pub fn line(&mut self, mut line: Line) -> &mut Self {
        if line.series.is_empty() {
            return self;
        };

        // Give the stroke an automatic color if no color has been assigned.
        if line.stroke.color == Color32::TRANSPARENT {
            line.stroke.color = self.auto_color();
        }
        self.items.push(Box::new(line));
        self
    }

    /// Add a polygon. The polygon has to be convex.
    pub fn polygon(&mut self, mut polygon: Polygon) -> &mut Self {
        if polygon.series.is_empty() {
            return self;
        };

        // Give the stroke an automatic color if no color has been assigned.
        if polygon.stroke.color == Color32::TRANSPARENT {
            polygon.stroke.color = self.auto_color();
        }
        self.items.push(Box::new(polygon));
        self
    }

    /// Add a text.
    pub fn text(&mut self, text: Text) -> &mut Self {
        if text.text.is_empty() {
            return self;
        };

        self.items.push(Box::new(text));
        self
    }

    /// Add data points.
    pub fn owned_points(&mut self, mut points: Points<PlotPoints>) -> &mut Self {
        // Give the points an automatic color if no color has been assigned.
        if points.color == Color32::TRANSPARENT {
            points.color = self.auto_color();
        }
        self.items.push(Box::new(points));

        self
    }

    /// Add data points.
    pub fn borrowed_points<'b, T: 'a>(&mut self, mut points: Points<T>) -> &mut Self
    where
        T: GenericPlotPoints<Item = &'b PlotPoint>,
    {
        // Give the points an automatic color if no color has been assigned.
        if points.color == Color32::TRANSPARENT {
            points.color = self.auto_color();
        }
        self.items.push(Box::new(points));

        self
    }

    /// Add arrows.
    pub fn arrows(&mut self, mut arrows: Arrows) -> &mut Self {
        if arrows.origins.is_empty() || arrows.tips.is_empty() {
            return self;
        };

        // Give the arrows an automatic color if no color has been assigned.
        if arrows.color == Color32::TRANSPARENT {
            arrows.color = self.auto_color();
        }
        self.items.push(Box::new(arrows));

        self
    }

    /// Add an image.
    pub fn image(&mut self, image: PlotImage) -> &mut Self {
        self.items.push(Box::new(image));

        self
    }

    /// Add a horizontal line.
    /// Can be useful e.g. to show min/max bounds or similar.
    /// Always fills the full width of the plot.
    pub fn hline(&mut self, mut hline: HLine) -> &mut Self {
        if hline.stroke.color == Color32::TRANSPARENT {
            hline.stroke.color = self.auto_color();
        }
        self.items.push(Box::new(hline));

        self
    }

    /// Add a vertical line.
    /// Can be useful e.g. to show min/max bounds or similar.
    /// Always fills the full height of the plot.
    pub fn vline(&mut self, mut vline: VLine) -> &mut Self {
        if vline.stroke.color == Color32::TRANSPARENT {
            vline.stroke.color = self.auto_color();
        }
        self.items.push(Box::new(vline));

        self
    }

    /// Add a box plot diagram.
    pub fn box_plot(&mut self, mut box_plot: BoxPlot) -> &mut Self {
        if box_plot.boxes.is_empty() {
            return self;
        }

        // Give the elements an automatic color if no color has been assigned.
        if box_plot.default_color == Color32::TRANSPARENT {
            box_plot = box_plot.color(self.auto_color());
        }
        self.items.push(Box::new(box_plot));

        self
    }

    /// Add a bar chart.
    pub fn bar_chart(&mut self, mut chart: BarChart) -> &mut Self {
        if chart.bars.is_empty() {
            return self;
        }

        // Give the elements an automatic color if no color has been assigned.
        if chart.default_color == Color32::TRANSPARENT {
            chart = chart.color(self.auto_color());
        }
        self.items.push(Box::new(chart));

        self
    }
}
