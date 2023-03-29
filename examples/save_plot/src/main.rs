#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::path::PathBuf;

use eframe::egui;
use eframe::egui::plot::{Legend, Line, Plot, PlotPoints};
use eframe::egui::ColorImage;

fn main() -> Result<(), eframe::Error> {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(350.0, 400.0)),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App with a plot",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    )
}

struct MyApp {
    picked_path_plot: PathBuf,
    plot_location: [f32; 4],
    save_plot: bool,
    screenshot: Option<ColorImage>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            picked_path_plot: PathBuf::default(),
            plot_location: [0.0; 4],
            save_plot: false,
            screenshot: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let height = 200.0;
            let border_x = 11.0;
            let border_y = 18.0;
            let width = 300.0;

            let window_width = ui.available_size().x;
            let window_height = ui.available_size().y;

            ui.heading("My egui Application");

            // add some whitespace in y direction
            ui.add_space(border_y);

            if ui.button("Save Plot").clicked() {
                if let Some(mut path) = rfd::FileDialog::new().save_file() {
                    path.set_extension("png");
                    frame.request_screenshot();
                    self.save_plot = true;
                    self.picked_path_plot = path;
                }
            }

            // add some whitespace in y direction
            ui.add_space(border_y);

            // this needs to be outside of the ui.horizontal()
            let plot_location_y = window_height - ui.available_size().y;
            ui.horizontal(|ui| {
                // add some whitespace in x direction
                ui.add_space(border_x);

                // obviously this needs to be after the last ui.add_space
                let plot_location_x = window_width - ui.available_size().x;

                // lets set the relative plot location for plot saving purposes
                self.plot_location[0] = plot_location_x / window_width; // lower bound x
                self.plot_location[1] = plot_location_y / window_height; // lower bound y
                self.plot_location[2] = (plot_location_x + width) / window_width; // upper bound x
                self.plot_location[3] = (plot_location_y + height) / window_height; // upper bound y

                let my_plot = Plot::new("My Plot")
                    .height(height)
                    .width(width)
                    .legend(Legend::default());

                // let's create a dummy line in the plot
                let graph: Vec<[f64; 2]> = vec![[0.0, 1.0], [2.0, 3.0], [3.0, 2.0]];
                my_plot.show(ui, |plot_ui| {
                    plot_ui.line(Line::new(PlotPoints::from(graph)).name("curve"));
                });
            });

            // add some whitespace in y direction
            ui.add_space(border_y);
        });

        match &self.screenshot {
            None => {}
            Some(screenshot) => {
                // for a full size application, we should put this in a different thread,
                // so that the GUI doesn't lag during saving

                // we need to use relative coordinates since the plot location comes
                // in relative coordinates.
                // since we scale it by screenshot.size, we do not need pixels_per_point,
                // thus i can be set to 1.0
                let screenshot_width = screenshot.size[0] as f32;
                let screenshot_height = screenshot.size[1] as f32;
                let region = egui::Rect::from_two_pos(
                    egui::Pos2 {
                        x: self.plot_location[0] * screenshot_width,
                        y: self.plot_location[1] * screenshot_height,
                    },
                    egui::Pos2 {
                        x: self.plot_location[2] * screenshot_width,
                        y: self.plot_location[3] * screenshot_height,
                    },
                );

                // only select the plot region from the screenshot
                let plot = screenshot.region(&region, Some(1.0));

                // save the plot to png
                image::save_buffer(
                    &self.picked_path_plot,
                    plot.as_raw(),
                    plot.width() as u32,
                    plot.height() as u32,
                    image::ColorType::Rgba8,
                )
                .unwrap();

                self.screenshot = None;
            }
        }
    }

    fn post_rendering(&mut self, _screen_size_px: [u32; 2], frame: &eframe::Frame) {
        // this is inspired by the Egui screenshot example
        if let Some(screenshot) = frame.screenshot() {
            self.screenshot = Some(screenshot);
        }
    }
}
