//! This example shows how to implement semi-log and log-log plots
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use std::ops::RangeInclusive;

use eframe::egui::{self};
use egui_plot::{GridInput, GridMark, Legend, Line};

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Plot",
        options,
        Box::new(|_cc| Ok(Box::<PlotExample>::default())),
    )
}

struct PlotExample {
    log_x: bool,
    log_y: bool,
    signals: Vec<Signal>,
}

struct Signal {
    name: &'static str,
    points: Vec<[f64; 2]>,
}

impl Default for PlotExample {
    fn default() -> Self {
        let x = (-2000..2000).map(|x| x as f64 / 100.0);
        let signals = vec![
            Signal {
                name: "y=x",
                points: x.clone().map(|x| [x, x]).collect(),
            },
            Signal {
                name: "y=x^2",
                points: x.clone().map(|x| [x, x.powi(2)]).collect(),
            },
            Signal {
                name: "y=exp(x)",
                points: x.clone().map(|x| [x, x.exp()]).collect(),
            },
            Signal {
                name: "y=ln(x)",
                points: x.clone().map(|x| [x, x.ln()]).collect(),
            },
        ];
        Self {
            log_x: true,
            log_y: true,
            signals,
        }
    }
}

impl eframe::App for PlotExample {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::SidePanel::left("options").show(ctx, |ui| {
            ui.checkbox(&mut self.log_x, "X axis log scale");
            ui.checkbox(&mut self.log_y, "Y axis log scale");
        });
        let log_x = self.log_x;
        let log_y = self.log_y;
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut plot = egui_plot::Plot::new("plot")
                .legend(Legend::default())
                .label_formatter(|name, value| {
                    let x = if log_x {
                        10.0f64.powf(value.x)
                    } else {
                        value.x
                    };
                    let y = if log_y {
                        10.0f64.powf(value.y)
                    } else {
                        value.y
                    };
                    if !name.is_empty() {
                        format!("{name}: {x:.3}, {y:.3}")
                    } else {
                        format!("{x:.3}, {y:.3}")
                    }
                });
            if log_x {
                plot = plot
                    .x_grid_spacer(log_axis_spacer)
                    .x_axis_formatter(log_axis_formatter);
            }
            if log_y {
                plot = plot
                    .y_grid_spacer(log_axis_spacer)
                    .y_axis_formatter(log_axis_formatter);
            }
            plot.show(ui, |plot_ui| {
                for signal in &self.signals {
                    let points: Vec<_> = signal
                        .points
                        .iter()
                        .copied()
                        .map(|[x, y]| {
                            let x = if log_x { x.log10() } else { x };
                            let y = if log_y { y.log10() } else { y };
                            [x, y]
                        })
                        .collect();
                    plot_ui.line(Line::new(points).name(signal.name));
                }
            });
        });
    }
}

#[allow(clippy::needless_pass_by_value)]
fn log_axis_spacer(input: GridInput) -> Vec<GridMark> {
    let (min, max) = input.bounds;
    let mut marks = vec![];
    for i in min.floor() as i32..=max.ceil() as i32 {
        marks.extend(
            (10..100)
                .map(|j| {
                    let value = i as f64 + (j as f64).log10() - 1.0;
                    let step_size = if j == 10 {
                        1.0
                    } else if j % 10 == 0 {
                        0.1
                    } else {
                        0.01
                    };
                    GridMark { value, step_size }
                })
                .filter(|gm| (min..=max).contains(&gm.value)),
        );
    }
    marks
}

fn log_axis_formatter(gm: GridMark, max_size: usize, _bounds: &RangeInclusive<f64>) -> String {
    let precision = (-gm.value).clamp(1.0, 10.0) as usize;
    let digits = (gm.value).clamp(0.0, 10.0) as usize;
    let size = digits + precision + 1;
    let value = 10.0f64.powf(gm.value);
    if size < max_size {
        format!("{value:.precision$}")
    } else {
        let precision = max_size.saturating_sub(size);
        format!("{value:.precision$e}")
    }
}
