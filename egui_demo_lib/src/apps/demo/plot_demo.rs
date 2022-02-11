use std::f64::consts::TAU;

use egui::*;
use plot::{
    Arrows, Bar, BarChart, BoxElem, BoxPlot, BoxSpread, CoordinatesFormatter, Corner, HLine,
    Legend, Line, LineStyle, MarkerShape, Plot, PlotImage, Points, Polygon, Text, VLine, Value,
    Values,
};

#[derive(PartialEq)]
struct LineDemo {
    animate: bool,
    time: f64,
    circle_radius: f64,
    circle_center: Pos2,
    square: bool,
    proportional: bool,
    coordinates: bool,
    line_style: LineStyle,
}

impl Default for LineDemo {
    fn default() -> Self {
        Self {
            animate: !cfg!(debug_assertions),
            time: 0.0,
            circle_radius: 1.5,
            circle_center: Pos2::new(0.0, 0.0),
            square: false,
            proportional: true,
            coordinates: true,
            line_style: LineStyle::Solid,
        }
    }
}

impl LineDemo {
    fn options_ui(&mut self, ui: &mut Ui) {
        let Self {
            animate,
            time: _,
            circle_radius,
            circle_center,
            square,
            proportional,
            line_style,
            coordinates,
            ..
        } = self;

        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label("Circle:");
                    ui.add(
                        egui::DragValue::new(circle_radius)
                            .speed(0.1)
                            .clamp_range(0.0..=f64::INFINITY)
                            .prefix("r: "),
                    );
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut circle_center.x)
                                .speed(0.1)
                                .prefix("x: "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut circle_center.y)
                                .speed(1.0)
                                .prefix("y: "),
                        );
                    });
                });
            });

            ui.vertical(|ui| {
                ui.style_mut().wrap = Some(false);
                ui.checkbox(animate, "Animate");
                ui.checkbox(square, "Square view")
                    .on_hover_text("Always keep the viewport square.");
                ui.checkbox(proportional, "Proportional data axes")
                    .on_hover_text("Tick are the same size on both axes.");
                ui.checkbox(coordinates, "Show coordinates")
                    .on_hover_text("Can take a custom formatting function.");

                ComboBox::from_label("Line style")
                    .selected_text(line_style.to_string())
                    .show_ui(ui, |ui| {
                        for style in [
                            LineStyle::Solid,
                            LineStyle::dashed_dense(),
                            LineStyle::dashed_loose(),
                            LineStyle::dotted_dense(),
                            LineStyle::dotted_loose(),
                        ]
                        .iter()
                        {
                            ui.selectable_value(line_style, *style, style.to_string());
                        }
                    });
            });
        });
    }

    fn circle(&self) -> Line {
        let n = 512;
        let circle = (0..=n).map(|i| {
            let t = remap(i as f64, 0.0..=(n as f64), 0.0..=TAU);
            let r = self.circle_radius;
            Value::new(
                r * t.cos() + self.circle_center.x as f64,
                r * t.sin() + self.circle_center.y as f64,
            )
        });
        Line::new(Values::from_values_iter(circle))
            .color(Color32::from_rgb(100, 200, 100))
            .style(self.line_style)
            .name("circle")
    }

    fn sin(&self) -> Line {
        let time = self.time;
        Line::new(Values::from_explicit_callback(
            move |x| 0.5 * (2.0 * x).sin() * time.sin(),
            ..,
            512,
        ))
        .color(Color32::from_rgb(200, 100, 100))
        .style(self.line_style)
        .name("wave")
    }

    fn thingy(&self) -> Line {
        let time = self.time;
        Line::new(Values::from_parametric_callback(
            move |t| ((2.0 * t + time).sin(), (3.0 * t).sin()),
            0.0..=TAU,
            256,
        ))
        .color(Color32::from_rgb(100, 150, 250))
        .style(self.line_style)
        .name("x = sin(2t), y = sin(3t)")
    }
}

impl Widget for &mut LineDemo {
    fn ui(self, ui: &mut Ui) -> Response {
        self.options_ui(ui);
        if self.animate {
            ui.ctx().request_repaint();
            self.time += ui.input().unstable_dt.at_most(1.0 / 30.0) as f64;
        };
        let mut plot = Plot::new("lines_demo").legend(Legend::default());
        if self.square {
            plot = plot.view_aspect(1.0);
        }
        if self.proportional {
            plot = plot.data_aspect(1.0);
        }
        if self.coordinates {
            plot = plot.coordinates_formatter(Corner::LeftBottom, CoordinatesFormatter::default());
        }
        plot.show(ui, |plot_ui| {
            plot_ui.line(self.circle());
            plot_ui.line(self.sin());
            plot_ui.line(self.thingy());
        })
        .response
    }
}

#[derive(PartialEq)]
struct MarkerDemo {
    fill_markers: bool,
    marker_radius: f32,
    automatic_colors: bool,
    marker_color: Color32,
}

impl Default for MarkerDemo {
    fn default() -> Self {
        Self {
            fill_markers: true,
            marker_radius: 5.0,
            automatic_colors: true,
            marker_color: Color32::GREEN,
        }
    }
}

impl MarkerDemo {
    fn markers(&self) -> Vec<Points> {
        MarkerShape::all()
            .enumerate()
            .map(|(i, marker)| {
                let y_offset = i as f32 * 0.5 + 1.0;
                let mut points = Points::new(Values::from_values(vec![
                    Value::new(1.0, 0.0 + y_offset),
                    Value::new(2.0, 0.5 + y_offset),
                    Value::new(3.0, 0.0 + y_offset),
                    Value::new(4.0, 0.5 + y_offset),
                    Value::new(5.0, 0.0 + y_offset),
                    Value::new(6.0, 0.5 + y_offset),
                ]))
                .name(format!("{:?}", marker))
                .filled(self.fill_markers)
                .radius(self.marker_radius)
                .shape(marker);

                if !self.automatic_colors {
                    points = points.color(self.marker_color);
                }

                points
            })
            .collect()
    }
}

impl Widget for &mut MarkerDemo {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.fill_markers, "Fill");
            ui.add(
                egui::DragValue::new(&mut self.marker_radius)
                    .speed(0.1)
                    .clamp_range(0.0..=f64::INFINITY)
                    .prefix("Radius: "),
            );
            ui.checkbox(&mut self.automatic_colors, "Automatic colors");
            if !self.automatic_colors {
                ui.color_edit_button_srgba(&mut self.marker_color);
            }
        });

        let markers_plot = Plot::new("markers_demo")
            .data_aspect(1.0)
            .legend(Legend::default());
        markers_plot
            .show(ui, |plot_ui| {
                for marker in self.markers() {
                    plot_ui.points(marker);
                }
            })
            .response
    }
}

#[derive(Default, PartialEq)]
struct LegendDemo {
    config: Legend,
}

impl LegendDemo {
    fn line_with_slope(slope: f64) -> Line {
        Line::new(Values::from_explicit_callback(move |x| slope * x, .., 100))
    }
    fn sin() -> Line {
        Line::new(Values::from_explicit_callback(move |x| x.sin(), .., 100))
    }
    fn cos() -> Line {
        Line::new(Values::from_explicit_callback(move |x| x.cos(), .., 100))
    }
}

impl Widget for &mut LegendDemo {
    fn ui(self, ui: &mut Ui) -> Response {
        let LegendDemo { config } = self;

        egui::Grid::new("settings").show(ui, |ui| {
            ui.label("Text style:");
            ui.horizontal(|ui| {
                let all_text_styles = ui.style().text_styles();
                for style in all_text_styles {
                    ui.selectable_value(&mut config.text_style, style.clone(), style.to_string());
                }
            });
            ui.end_row();

            ui.label("Position:");
            ui.horizontal(|ui| {
                Corner::all().for_each(|position| {
                    ui.selectable_value(&mut config.position, position, format!("{:?}", position));
                });
            });
            ui.end_row();

            ui.label("Opacity:");
            ui.add(
                egui::DragValue::new(&mut config.background_alpha)
                    .speed(0.02)
                    .clamp_range(0.0..=1.0),
            );
            ui.end_row();
        });

        let legend_plot = Plot::new("legend_demo")
            .legend(config.clone())
            .data_aspect(1.0);
        legend_plot
            .show(ui, |plot_ui| {
                plot_ui.line(LegendDemo::line_with_slope(0.5).name("lines"));
                plot_ui.line(LegendDemo::line_with_slope(1.0).name("lines"));
                plot_ui.line(LegendDemo::line_with_slope(2.0).name("lines"));
                plot_ui.line(LegendDemo::sin().name("sin(x)"));
                plot_ui.line(LegendDemo::cos().name("cos(x)"));
            })
            .response
    }
}

#[derive(PartialEq)]
struct LinkedAxisDemo {
    link_x: bool,
    link_y: bool,
    group: plot::LinkedAxisGroup,
}

impl Default for LinkedAxisDemo {
    fn default() -> Self {
        let link_x = true;
        let link_y = false;
        Self {
            link_x,
            link_y,
            group: plot::LinkedAxisGroup::new(link_x, link_y),
        }
    }
}

impl LinkedAxisDemo {
    fn line_with_slope(slope: f64) -> Line {
        Line::new(Values::from_explicit_callback(move |x| slope * x, .., 100))
    }
    fn sin() -> Line {
        Line::new(Values::from_explicit_callback(move |x| x.sin(), .., 100))
    }
    fn cos() -> Line {
        Line::new(Values::from_explicit_callback(move |x| x.cos(), .., 100))
    }

    fn configure_plot(plot_ui: &mut plot::PlotUi) {
        plot_ui.line(LinkedAxisDemo::line_with_slope(0.5));
        plot_ui.line(LinkedAxisDemo::line_with_slope(1.0));
        plot_ui.line(LinkedAxisDemo::line_with_slope(2.0));
        plot_ui.line(LinkedAxisDemo::sin());
        plot_ui.line(LinkedAxisDemo::cos());
    }
}

impl Widget for &mut LinkedAxisDemo {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            ui.label("Linked axes:");
            ui.checkbox(&mut self.link_x, "X");
            ui.checkbox(&mut self.link_y, "Y");
        });
        self.group.set_link_x(self.link_x);
        self.group.set_link_y(self.link_y);
        ui.horizontal(|ui| {
            Plot::new("linked_axis_1")
                .data_aspect(1.0)
                .width(250.0)
                .height(250.0)
                .link_axis(self.group.clone())
                .show(ui, LinkedAxisDemo::configure_plot);
            Plot::new("linked_axis_2")
                .data_aspect(2.0)
                .width(150.0)
                .height(250.0)
                .link_axis(self.group.clone())
                .show(ui, LinkedAxisDemo::configure_plot);
        });
        Plot::new("linked_axis_3")
            .data_aspect(0.5)
            .width(250.0)
            .height(150.0)
            .link_axis(self.group.clone())
            .show(ui, LinkedAxisDemo::configure_plot)
            .response
    }
}

#[derive(PartialEq, Default)]
struct ItemsDemo {
    texture: Option<egui::TextureHandle>,
}

impl Widget for &mut ItemsDemo {
    fn ui(self, ui: &mut Ui) -> Response {
        let n = 100;
        let mut sin_values: Vec<_> = (0..=n)
            .map(|i| remap(i as f64, 0.0..=n as f64, -TAU..=TAU))
            .map(|i| Value::new(i, i.sin()))
            .collect();

        let line = Line::new(Values::from_values(sin_values.split_off(n / 2))).fill(-1.5);
        let polygon = Polygon::new(Values::from_parametric_callback(
            |t| (4.0 * t.sin() + 2.0 * t.cos(), 4.0 * t.cos() + 2.0 * t.sin()),
            0.0..TAU,
            100,
        ));
        let points = Points::new(Values::from_values(sin_values))
            .stems(-1.5)
            .radius(1.0);

        let arrows = {
            let pos_radius = 8.0;
            let tip_radius = 7.0;
            let arrow_origins = Values::from_parametric_callback(
                |t| (pos_radius * t.sin(), pos_radius * t.cos()),
                0.0..TAU,
                36,
            );
            let arrow_tips = Values::from_parametric_callback(
                |t| (tip_radius * t.sin(), tip_radius * t.cos()),
                0.0..TAU,
                36,
            );
            Arrows::new(arrow_origins, arrow_tips)
        };

        let texture: &egui::TextureHandle = self.texture.get_or_insert_with(|| {
            ui.ctx()
                .load_texture("plot_demo", egui::ColorImage::example())
        });
        let image = PlotImage::new(
            texture,
            Value::new(0.0, 10.0),
            5.0 * vec2(texture.aspect_ratio(), 1.0),
        );

        let plot = Plot::new("items_demo")
            .legend(Legend::default().position(Corner::RightBottom))
            .show_x(false)
            .show_y(false)
            .data_aspect(1.0);
        plot.show(ui, |plot_ui| {
            plot_ui.hline(HLine::new(9.0).name("Lines horizontal"));
            plot_ui.hline(HLine::new(-9.0).name("Lines horizontal"));
            plot_ui.vline(VLine::new(9.0).name("Lines vertical"));
            plot_ui.vline(VLine::new(-9.0).name("Lines vertical"));
            plot_ui.line(line.name("Line with fill"));
            plot_ui.polygon(polygon.name("Convex polygon"));
            plot_ui.points(points.name("Points with stems"));
            plot_ui.text(Text::new(Value::new(-3.0, -3.0), "wow").name("Text"));
            plot_ui.text(Text::new(Value::new(-2.0, 2.5), "so graph").name("Text"));
            plot_ui.text(Text::new(Value::new(3.0, 3.0), "much color").name("Text"));
            plot_ui.text(Text::new(Value::new(2.5, -2.0), "such plot").name("Text"));
            plot_ui.image(image.name("Image"));
            plot_ui.arrows(arrows.name("Arrows"));
        })
        .response
    }
}

#[derive(Default, PartialEq)]
struct InteractionDemo {}

impl Widget for &mut InteractionDemo {
    fn ui(self, ui: &mut Ui) -> Response {
        let plot = Plot::new("interaction_demo").height(300.0);

        let InnerResponse {
            response,
            inner: (screen_pos, pointer_coordinate, pointer_coordinate_drag_delta, bounds, hovered),
        } = plot.show(ui, |plot_ui| {
            (
                plot_ui.screen_from_plot(Value::new(0.0, 0.0)),
                plot_ui.pointer_coordinate(),
                plot_ui.pointer_coordinate_drag_delta(),
                plot_ui.plot_bounds(),
                plot_ui.plot_hovered(),
            )
        });

        ui.label(format!(
            "plot bounds: min: {:.02?}, max: {:.02?}",
            bounds.min(),
            bounds.max()
        ));
        ui.label(format!(
            "origin in screen coordinates: x: {:.02}, y: {:.02}",
            screen_pos.x, screen_pos.y
        ));
        ui.label(format!("plot hovered: {}", hovered));
        let coordinate_text = if let Some(coordinate) = pointer_coordinate {
            format!("x: {:.02}, y: {:.02}", coordinate.x, coordinate.y)
        } else {
            "None".to_string()
        };
        ui.label(format!("pointer coordinate: {}", coordinate_text));
        let coordinate_text = format!(
            "x: {:.02}, y: {:.02}",
            pointer_coordinate_drag_delta.x, pointer_coordinate_drag_delta.y
        );
        ui.label(format!(
            "pointer coordinate drag delta: {}",
            coordinate_text
        ));

        response
    }
}

#[derive(PartialEq, Eq)]
enum Chart {
    GaussBars,
    StackedBars,
    BoxPlot,
}

impl Default for Chart {
    fn default() -> Self {
        Self::GaussBars
    }
}

#[derive(PartialEq)]
struct ChartsDemo {
    chart: Chart,
    vertical: bool,
}

impl Default for ChartsDemo {
    fn default() -> Self {
        Self {
            vertical: true,
            chart: Chart::default(),
        }
    }
}

impl ChartsDemo {
    fn bar_gauss(&self, ui: &mut Ui) -> Response {
        let mut chart = BarChart::new(
            (-395..=395)
                .step_by(10)
                .map(|x| x as f64 * 0.01)
                .map(|x| {
                    (
                        x,
                        (-x * x / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt(),
                    )
                })
                // The 10 factor here is purely for a nice 1:1 aspect ratio
                .map(|(x, f)| Bar::new(x, f * 10.0).width(0.095))
                .collect(),
        )
        .color(Color32::LIGHT_BLUE)
        .name("Normal Distribution");
        if !self.vertical {
            chart = chart.horizontal();
        }

        Plot::new("Normal Distribution Demo")
            .legend(Legend::default())
            .data_aspect(1.0)
            .show(ui, |plot_ui| plot_ui.bar_chart(chart))
            .response
    }

    fn bar_stacked(&self, ui: &mut Ui) -> Response {
        let mut chart1 = BarChart::new(vec![
            Bar::new(0.5, 1.0).name("Day 1"),
            Bar::new(1.5, 3.0).name("Day 2"),
            Bar::new(2.5, 1.0).name("Day 3"),
            Bar::new(3.5, 2.0).name("Day 4"),
            Bar::new(4.5, 4.0).name("Day 5"),
        ])
        .width(0.7)
        .name("Set 1");

        let mut chart2 = BarChart::new(vec![
            Bar::new(0.5, 1.0),
            Bar::new(1.5, 1.5),
            Bar::new(2.5, 0.1),
            Bar::new(3.5, 0.7),
            Bar::new(4.5, 0.8),
        ])
        .width(0.7)
        .name("Set 2")
        .stack_on(&[&chart1]);

        let mut chart3 = BarChart::new(vec![
            Bar::new(0.5, -0.5),
            Bar::new(1.5, 1.0),
            Bar::new(2.5, 0.5),
            Bar::new(3.5, -1.0),
            Bar::new(4.5, 0.3),
        ])
        .width(0.7)
        .name("Set 3")
        .stack_on(&[&chart1, &chart2]);

        let mut chart4 = BarChart::new(vec![
            Bar::new(0.5, 0.5),
            Bar::new(1.5, 1.0),
            Bar::new(2.5, 0.5),
            Bar::new(3.5, -0.5),
            Bar::new(4.5, -0.5),
        ])
        .width(0.7)
        .name("Set 4")
        .stack_on(&[&chart1, &chart2, &chart3]);

        let mut x_fmt: fn(f64, &std::ops::RangeInclusive<f64>) -> String = |val, _range| {
            if val >= 0.0 && val <= 4.0 && is_approx_integer(val) {
                // Only label full days from 0 to 4
                format!("Day {}", val)
            } else {
                // Otherwise return empty string (i.e. no label)
                String::new()
            }
        };

        let mut y_fmt: fn(f64, &std::ops::RangeInclusive<f64>) -> String = |val, _range| {
            let percent = 100.0 * val;

            if is_approx_integer(percent) && !is_approx_zero(percent) {
                // Only show integer percentages,
                // and don't show at Y=0 (label overlaps with X axis label)
                format!("{}%", percent)
            } else {
                String::new()
            }
        };

        if !self.vertical {
            chart1 = chart1.horizontal();
            chart2 = chart2.horizontal();
            chart3 = chart3.horizontal();
            chart4 = chart4.horizontal();
            std::mem::swap(&mut x_fmt, &mut y_fmt);
        }

        Plot::new("Stacked Bar Chart Demo")
            .legend(Legend::default())
            .x_axis_formatter(x_fmt)
            .y_axis_formatter(y_fmt)
            .data_aspect(1.0)
            .show(ui, |plot_ui| {
                plot_ui.bar_chart(chart1);
                plot_ui.bar_chart(chart2);
                plot_ui.bar_chart(chart3);
                plot_ui.bar_chart(chart4);
            })
            .response
    }

    fn box_plot(&self, ui: &mut Ui) -> Response {
        let yellow = Color32::from_rgb(248, 252, 168);
        let mut box1 = BoxPlot::new(vec![
            BoxElem::new(0.5, BoxSpread::new(1.5, 2.2, 2.5, 2.6, 3.1)).name("Day 1"),
            BoxElem::new(2.5, BoxSpread::new(0.4, 1.0, 1.1, 1.4, 2.1)).name("Day 2"),
            BoxElem::new(4.5, BoxSpread::new(1.7, 2.0, 2.2, 2.5, 2.9)).name("Day 3"),
        ])
        .name("Experiment A");

        let mut box2 = BoxPlot::new(vec![
            BoxElem::new(1.0, BoxSpread::new(0.2, 0.5, 1.0, 2.0, 2.7)).name("Day 1"),
            BoxElem::new(3.0, BoxSpread::new(1.5, 1.7, 2.1, 2.9, 3.3))
                .name("Day 2: interesting")
                .stroke(Stroke::new(1.5, yellow))
                .fill(yellow.linear_multiply(0.2)),
            BoxElem::new(5.0, BoxSpread::new(1.3, 2.0, 2.3, 2.9, 4.0)).name("Day 3"),
        ])
        .name("Experiment B");

        let mut box3 = BoxPlot::new(vec![
            BoxElem::new(1.5, BoxSpread::new(2.1, 2.2, 2.6, 2.8, 3.0)).name("Day 1"),
            BoxElem::new(3.5, BoxSpread::new(1.3, 1.5, 1.9, 2.2, 2.4)).name("Day 2"),
            BoxElem::new(5.5, BoxSpread::new(0.2, 0.4, 1.0, 1.3, 1.5)).name("Day 3"),
        ])
        .name("Experiment C");

        if !self.vertical {
            box1 = box1.horizontal();
            box2 = box2.horizontal();
            box3 = box3.horizontal();
        }

        Plot::new("Box Plot Demo")
            .legend(Legend::default())
            .show(ui, |plot_ui| {
                plot_ui.box_plot(box1);
                plot_ui.box_plot(box2);
                plot_ui.box_plot(box3);
            })
            .response
    }
}

impl Widget for &mut ChartsDemo {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.label("Type:");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.chart, Chart::GaussBars, "Histogram");
            ui.selectable_value(&mut self.chart, Chart::StackedBars, "Stacked Bar Chart");
            ui.selectable_value(&mut self.chart, Chart::BoxPlot, "Box Plot");
        });
        ui.label("Orientation:");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.vertical, true, "Vertical");
            ui.selectable_value(&mut self.vertical, false, "Horizontal");
        });
        match self.chart {
            Chart::GaussBars => self.bar_gauss(ui),
            Chart::StackedBars => self.bar_stacked(ui),
            Chart::BoxPlot => self.box_plot(ui),
        }
    }
}

#[derive(PartialEq, Eq)]
enum Panel {
    Lines,
    Markers,
    Legend,
    Charts,
    Items,
    Interaction,
    LinkedAxes,
}

impl Default for Panel {
    fn default() -> Self {
        Self::Lines
    }
}

#[derive(PartialEq, Default)]
pub struct PlotDemo {
    line_demo: LineDemo,
    marker_demo: MarkerDemo,
    legend_demo: LegendDemo,
    charts_demo: ChartsDemo,
    items_demo: ItemsDemo,
    interaction_demo: InteractionDemo,
    linked_axes_demo: LinkedAxisDemo,
    open_panel: Panel,
}

impl super::Demo for PlotDemo {
    fn name(&self) -> &'static str {
        "🗠 Plot"
    }

    fn show(&mut self, ctx: &Context, open: &mut bool) {
        use super::View as _;
        Window::new(self.name())
            .open(open)
            .default_size(vec2(400.0, 400.0))
            .vscroll(false)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for PlotDemo {
    fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            egui::reset_button(ui, self);
            ui.collapsing("Instructions", |ui| {
                ui.label("Pan by dragging, or scroll (+ shift = horizontal).");
                ui.label("Box zooming: Right click to zoom in and zoom out using a selection.");
                if cfg!(target_arch = "wasm32") {
                    ui.label("Zoom with ctrl / ⌘ + pointer wheel, or with pinch gesture.");
                } else if cfg!(target_os = "macos") {
                    ui.label("Zoom with ctrl / ⌘ + scroll.");
                } else {
                    ui.label("Zoom with ctrl + scroll.");
                }
                ui.label("Reset view with double-click.");
                ui.add(crate::__egui_github_link_file!());
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.open_panel, Panel::Lines, "Lines");
            ui.selectable_value(&mut self.open_panel, Panel::Markers, "Markers");
            ui.selectable_value(&mut self.open_panel, Panel::Legend, "Legend");
            ui.selectable_value(&mut self.open_panel, Panel::Charts, "Charts");
            ui.selectable_value(&mut self.open_panel, Panel::Items, "Items");
            ui.selectable_value(&mut self.open_panel, Panel::Interaction, "Interaction");
            ui.selectable_value(&mut self.open_panel, Panel::LinkedAxes, "Linked Axes");
        });
        ui.separator();

        match self.open_panel {
            Panel::Lines => {
                ui.add(&mut self.line_demo);
            }
            Panel::Markers => {
                ui.add(&mut self.marker_demo);
            }
            Panel::Legend => {
                ui.add(&mut self.legend_demo);
            }
            Panel::Charts => {
                ui.add(&mut self.charts_demo);
            }
            Panel::Items => {
                ui.add(&mut self.items_demo);
            }
            Panel::Interaction => {
                ui.add(&mut self.interaction_demo);
            }
            Panel::LinkedAxes => {
                ui.add(&mut self.linked_axes_demo);
            }
        }
    }
}

fn is_approx_zero(val: f64) -> bool {
    val.abs() < 1e-6
}

fn is_approx_integer(val: f64) -> bool {
    val.fract().abs() < 1e-6
}
