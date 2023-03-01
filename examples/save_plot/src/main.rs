#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui::plot::{Legend, Line, Plot, PlotPoints};
use eframe::egui::ColorImage;
use eframe::glow::HasContext;
use eframe::{egui, glow};
use image::{ImageResult, RgbaImage};
use std::path::PathBuf;

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
    plot_size: [f32; 4],
    save_plot: bool,
    plot_to_save: Option<ColorImage>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            picked_path_plot: PathBuf::default(),
            plot_size: [0.0; 4],
            save_plot: false,
            plot_to_save: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let height = 200.0;
            let border_x = 11.0;
            let border_y = 18.0;
            let width = 300.0;

            let window_width = ui.available_size().x;
            let window_height = ui.available_size().y;

            ui.heading("My egui Application");

            ui.add_space(border_y); // add some whitespace in y direction

            if ui.button("Save Plot").clicked() {
                if let Some(mut path) = rfd::FileDialog::new().save_file() {
                    path.set_extension("png");
                    self.save_plot = true;
                    self.picked_path_plot = path;
                }
            }

            ui.add_space(border_y); // add some whitespace in y direction

            let plot_location_y = ui.available_size().y; // this needs to be outside of the ui.horizontal()
            ui.horizontal(|ui| {
                ui.add_space(border_x); // add some whitespace in x direction

                let plot_location_x = window_width - ui.available_size().x; // obviously this needs to be after the last ui.add_space

                // lets set the relative plot size and location for plot saving purposes
                self.plot_size[0] = plot_location_x / window_width; // lower bound x
                self.plot_size[1] = (plot_location_y - height) / window_height; // lower bound y
                self.plot_size[2] = width / window_width; // width
                self.plot_size[3] = height / window_height; // height

                let my_plot = Plot::new("My Plot")
                    .height(height)
                    .width(width)
                    .legend(Legend::default());

                let graph: Vec<[f64; 2]> = vec![[0.0, 1.0], [2.0, 3.0], [3.0, 2.0]];  // dummy data
                my_plot.show(ui, |plot_ui| {
                    plot_ui.line(Line::new(PlotPoints::from(graph)).name("curve"));
                });
            });

            ui.add_space(border_y); // add some whitespace in y direction
        });

        if let Some(plot_to_save) = self.plot_to_save.take() {
            // maybe we should put this in a different thread, so that the GUI
            // doesn't lag during saving
            match save_image(&plot_to_save, &self.picked_path_plot) {
                Ok(_) => {
                    println!("saving ok!");
                }
                Err(e) => {
                    println!("failed to plot to {:?}: {:?}", self.picked_path_plot, e);
                }
            }
        }
    }

    #[allow(unsafe_code)]
    fn post_rendering(&mut self, screen_size_px: [u32; 2], frame: &eframe::Frame) {
        // this is inspired by the Egui screenshot example

        if !self.save_plot {
            return;
        }

        self.save_plot = false;
        if let Some(gl) = frame.gl() {
            let [window_width, window_height] = screen_size_px;

            // we needed the relative values here, because we need to have them in relation to the
            // screen_size_px.
            // calculating with absolut px values does not always work (for example with retina
            // display MacBooks we have different absolute values than with external displays)
            // using relative values, we have a working solution for all cases
            let w_lower = self.plot_size[0] * window_width as f32;
            let h_lower = self.plot_size[1] * window_height as f32;
            let w = self.plot_size[2] * window_width as f32;
            let h = self.plot_size[3] * window_height as f32;

            let mut buf = vec![0u8; w as usize * h as usize * 4];
            let pixels = glow::PixelPackData::Slice(&mut buf[..]);
            unsafe {
                gl.read_pixels(
                    w_lower as i32,
                    h_lower as i32,
                    w as i32,
                    h as i32,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    pixels,
                );
            }

            // Flip vertically:
            let mut rows: Vec<Vec<u8>> = buf
                .chunks(w as usize * 4)
                .into_iter()
                .map(|chunk| chunk.to_vec())
                .collect();
            rows.reverse();
            let buf: Vec<u8> = rows.into_iter().flatten().collect();
            self.plot_to_save = Some(ColorImage::from_rgba_unmultiplied(
                [w as usize, h as usize],
                &buf[..],
            ));
        }
    }
}

fn save_image(img: &ColorImage, file_path: &PathBuf) -> ImageResult<()> {
    let height = img.height();
    let width = img.width();
    let mut raw: Vec<u8> = vec![];
    for p in img.pixels.clone().iter() {
        raw.push(p.r());
        raw.push(p.g());
        raw.push(p.b());
        raw.push(p.a());
    }
    let img_to_save = RgbaImage::from_raw(width as u32, height as u32, raw)
        .expect("container should have the right size for the image dimensions");
    img_to_save.save(file_path)
}
