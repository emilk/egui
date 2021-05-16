use egui::*;
use plot::{Curve, Legend, LegendPosition, Marker, Plot, Value};
use std::f64::consts::TAU;

#[derive(PartialEq)]
struct CurveDemo {
    animate: bool,
    time: f64,
    circle_radius: f64,
    circle_center: Pos2,
    square: bool,
    proportional: bool,
}

impl Default for CurveDemo {
    fn default() -> Self {
        Self {
            animate: true,
            time: 0.0,
            circle_radius: 1.5,
            circle_center: Pos2::new(0.0, 0.0),
            square: false,
            proportional: true,
        }
    }
}

impl CurveDemo {
    fn options_ui(&mut self, ui: &mut Ui) {
        let Self {
            animate,
            time: _,
            circle_radius,
            circle_center,
            square,
            proportional,
            ..
        } = self;

        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label("Circle:");
                    ui.add(
                        egui::DragValue::new(circle_radius)
                            .speed(0.1)
                            .clamp_range(0.0..=f32::INFINITY)
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
                ui.checkbox(square, "square view");
                ui.checkbox(proportional, "proportional data axes");
            });
        });

        ui.label("Drag to pan, ctrl + scroll to zoom. Double-click to reset view.");
    }

    fn circle(&self) -> Curve {
        let n = 512;
        let circle = (0..=n).map(|i| {
            let t = remap(i as f64, 0.0..=(n as f64), 0.0..=TAU);
            let r = self.circle_radius;
            Value::new(
                r * t.cos() + self.circle_center.x as f64,
                r * t.sin() + self.circle_center.y as f64,
            )
        });
        Curve::from_values_iter(circle)
            .color(Color32::from_rgb(100, 200, 100))
            .name("circle")
    }

    fn sin(&self) -> Curve {
        let time = self.time;
        Curve::from_explicit_callback(
            move |x| 0.5 * (2.0 * x).sin() * time.sin(),
            f64::NEG_INFINITY..=f64::INFINITY,
            512,
        )
        .color(Color32::from_rgb(200, 100, 100))
        .name("wave")
    }

    fn thingy(&self) -> Curve {
        let time = self.time;
        Curve::from_parametric_callback(
            move |t| ((2.0 * t + time).sin(), (3.0 * t).sin()),
            0.0..=TAU,
            100,
        )
        .color(Color32::from_rgb(100, 150, 250))
        .marker(Marker::default())
        .name("x = sin(2t), y = sin(3t)")
    }
}

impl Widget for &mut CurveDemo {
    fn ui(self, ui: &mut Ui) -> Response {
        self.options_ui(ui);
        if self.animate {
            ui.ctx().request_repaint();
            self.time += ui.input().unstable_dt.at_most(1.0 / 30.0) as f64;
        };
        let mut plot = Plot::new("Curves Demo")
            .curve(self.circle())
            .curve(self.sin())
            .curve(self.thingy())
            .height(300.0)
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
    show_markers: bool,
    show_lines: bool,
    fill_markers: bool,
    marker_radius: f32,
    custom_marker_color: bool,
    marker_color: Color32,
}

impl Default for MarkerDemo {
    fn default() -> Self {
        Self {
            show_markers: true,
            show_lines: true,
            fill_markers: true,
            marker_radius: 5.0,
            custom_marker_color: false,
            marker_color: Color32::GRAY,
        }
    }
}

impl MarkerDemo {
    fn markers(&self) -> Vec<Curve> {
        Marker::all()
            .into_iter()
            .enumerate()
            .map(|(i, marker)| {
                let y_offset = i as f32 * 0.5 + 1.0;
                let mut curve = Curve::from_values(vec![
                    Value::new(1.0, 0.0 + y_offset),
                    Value::new(2.0, 0.5 + y_offset),
                    Value::new(3.0, 0.0 + y_offset),
                    Value::new(4.0, 0.5 + y_offset),
                    Value::new(5.0, 0.0 + y_offset),
                    Value::new(6.0, 0.5 + y_offset),
                ])
                .name("Marker Lines");

                if self.show_markers {
                    let mut marker = marker.filled(self.fill_markers).radius(self.marker_radius);
                    if self.custom_marker_color {
                        marker = marker.color(self.marker_color);
                    }
                    curve = curve.marker(marker);
                }
                if !self.show_lines {
                    curve = curve.color(Color32::TRANSPARENT);
                }
                curve
            })
            .collect()
    }
}

impl Widget for &mut MarkerDemo {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_lines, "show lines");
            ui.checkbox(&mut self.show_markers, "show markers");
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.fill_markers, "fill markers");
            ui.add(
                egui::DragValue::new(&mut self.marker_radius)
                    .speed(0.1)
                    .clamp_range(0.0..=f32::INFINITY)
                    .prefix("marker radius: "),
            );
            ui.checkbox(&mut self.custom_marker_color, "custom marker color");
            if self.custom_marker_color {
                ui.color_edit_button_srgba(&mut self.marker_color);
            }
        });

        let markers_plot = Plot::new("Markers Demo")
            .curves(self.markers())
            .height(300.0)
            .legend(Legend::default())
            .data_aspect(1.0);
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
    fn line_with_slope(slope: f64) -> Curve {
        Curve::from_explicit_callback(move |x| slope * x, f64::NEG_INFINITY..=f64::INFINITY, 100)
    }
    fn sin() -> Curve {
        Curve::from_explicit_callback(move |x| x.sin(), f64::NEG_INFINITY..=f64::INFINITY, 100)
    }
    fn cos() -> Curve {
        Curve::from_explicit_callback(move |x| x.cos(), f64::NEG_INFINITY..=f64::INFINITY, 100)
    }
}

impl Widget for &mut LegendDemo {
    fn ui(self, ui: &mut Ui) -> Response {
        let LegendDemo { config } = self;

        ui.label("Text Style:");
        ui.horizontal(|ui| {
            TextStyle::all().for_each(|style| {
                ui.selectable_value(&mut config.text_style, style, format!("{:?}", style));
            });
        });
        ui.label("Position:");
        ui.horizontal(|ui| {
            LegendPosition::all().for_each(|position| {
                ui.selectable_value(&mut config.position, position, format!("{:?}", position));
            });
        });
        let legend_plot = Plot::new("Legend Demo")
            .curve(LegendDemo::line_with_slope(0.5).name("lines"))
            .curve(LegendDemo::line_with_slope(1.0).name("lines"))
            .curve(LegendDemo::line_with_slope(2.0).name("lines"))
            .curve(LegendDemo::sin().name("sin(x)"))
            .curve(LegendDemo::cos().name("cos(x)"))
            .height(300.0)
            .legend(*config)
            .data_aspect(1.0);
        ui.add(legend_plot)
    }
}

#[derive(PartialEq, Default)]
pub struct PlotDemo {
    curve_demo: CurveDemo,
    marker_demo: MarkerDemo,
    legend_demo: LegendDemo,
}

impl super::Demo for PlotDemo {
    fn name(&self) -> &'static str {
        "🗠 Plot"
    }

    fn show(&mut self, ctx: &CtxRef, open: &mut bool) {
        use super::View;
        Window::new(self.name())
            .open(open)
            .default_size(vec2(400.0, 400.0))
            .scroll(false)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for PlotDemo {
    fn ui(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            egui::reset_button(ui, self);
            ui.add(crate::__egui_github_link_file!());
        });
        ui.collapsing("Curves", |ui| ui.add(&mut self.curve_demo));
        ui.collapsing("Markers", |ui| ui.add(&mut self.marker_demo));
        ui.collapsing("Legend", |ui| ui.add(&mut self.legend_demo));
    }
}
