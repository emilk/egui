use egui::*;
use plot::{
    Arrows, Corner, HLine, Legend, Line, LineStyle, MarkerShape, Plot, PlotImage, Points, Polygon,
    Text, VLine, Value, Values,
};
use std::f64::consts::TAU;

#[derive(PartialEq)]
struct LineDemo {
    animate: bool,
    time: f64,
    circle_radius: f64,
    circle_center: Pos2,
    square: bool,
    proportional: bool,
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
                ui.checkbox(animate, "animate");
                ui.checkbox(square, "square view")
                    .on_hover_text("Always keep the viewport square.");
                ui.checkbox(proportional, "Proportional data axes")
                    .on_hover_text("Tick are the same size on both axes.");

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
        let mut plot = Plot::new("lines_demo")
            .line(self.circle())
            .line(self.sin())
            .line(self.thingy())
            .legend(Legend::default());
        if self.square {
            plot = plot.view_aspect(1.0);
        }
        if self.proportional {
            plot = plot.data_aspect(1.0);
        }
        ui.add(plot)
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

        let mut markers_plot = Plot::new("markers_demo")
            .data_aspect(1.0)
            .legend(Legend::default());
        for marker in self.markers() {
            markers_plot = markers_plot.points(marker);
        }
        ui.add(markers_plot)
    }
}

#[derive(PartialEq)]
struct LegendDemo {
    config: Legend,
}

impl Default for LegendDemo {
    fn default() -> Self {
        Self {
            config: Legend::default(),
        }
    }
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
                TextStyle::all().for_each(|style| {
                    ui.selectable_value(&mut config.text_style, style, format!("{:?}", style));
                });
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
            .line(LegendDemo::line_with_slope(0.5).name("lines"))
            .line(LegendDemo::line_with_slope(1.0).name("lines"))
            .line(LegendDemo::line_with_slope(2.0).name("lines"))
            .line(LegendDemo::sin().name("sin(x)"))
            .line(LegendDemo::cos().name("cos(x)"))
            .legend(*config)
            .data_aspect(1.0);
        ui.add(legend_plot)
    }
}

#[derive(PartialEq, Default)]
struct ItemsDemo {}

impl ItemsDemo {}

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
        let image = PlotImage::new(
            TextureId::Egui,
            Value::new(0.0, 10.0),
            [
                ui.fonts().texture().width as f32 / 100.0,
                ui.fonts().texture().height as f32 / 100.0,
            ],
        );

        let plot = Plot::new("items_demo")
            .hline(HLine::new(9.0).name("Lines horizontal"))
            .hline(HLine::new(-9.0).name("Lines horizontal"))
            .vline(VLine::new(9.0).name("Lines vertical"))
            .vline(VLine::new(-9.0).name("Lines vertical"))
            .line(line.name("Line with fill"))
            .polygon(polygon.name("Convex polygon"))
            .points(points.name("Points with stems"))
            .text(Text::new(Value::new(-3.0, -3.0), "wow").name("Text"))
            .text(Text::new(Value::new(-2.0, 2.5), "so graph").name("Text"))
            .text(Text::new(Value::new(3.0, 3.0), "much color").name("Text"))
            .text(Text::new(Value::new(2.5, -2.0), "such plot").name("Text"))
            .image(image.name("Image"))
            .arrows(arrows.name("Arrows"))
            .legend(Legend::default().position(Corner::RightBottom))
            .show_x(false)
            .show_y(false)
            .data_aspect(1.0);
        ui.add(plot)
    }
}

#[derive(PartialEq, Eq)]
enum Panel {
    Lines,
    Markers,
    Legend,
    Items,
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
    items_demo: ItemsDemo,
    open_panel: Panel,
}

impl super::Demo for PlotDemo {
    fn name(&self) -> &'static str {
        "🗠 Plot"
    }

    fn show(&mut self, ctx: &CtxRef, open: &mut bool) {
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
                if cfg!(target_arch = "wasm32") {
                    ui.label("Zoom with ctrl / ⌘ + mouse wheel, or with pinch gesture.");
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
            ui.selectable_value(&mut self.open_panel, Panel::Items, "Items");
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
            Panel::Items => {
                ui.add(&mut self.items_demo);
            }
        }
    }
}
