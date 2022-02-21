#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{egui, epi};
use egui_extras::RetainedImage;
use poll_promise::Promise;

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(MyApp::default()), options);
}

#[derive(Default)]
struct MyApp {
    /// `None` when download hasn't started yet.
    promise: Option<Promise<ehttp::Result<RetainedImage>>>,
}

impl epi::App for MyApp {
    fn name(&self) -> &str {
        "Download and show an image with eframe/egui"
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        let promise = self.promise.get_or_insert_with(|| {
            // Begin download.
            // We download the image using `ehttp`, a library that works both in WASM and on native.
            // We use the `poll-promise` library to communicate with the UI thread.
            let frame = frame.clone();
            let (sender, promise) = Promise::new();
            let request = ehttp::Request::get("https://picsum.photos/seed/1.759706314/1024");
            ehttp::fetch(request, move |response| {
                let image = response.and_then(parse_response);
                sender.send(image); // send the results back to the UI thread.
                frame.request_repaint(); // wake up UI thread
            });
            promise
        });

        egui::CentralPanel::default().show(ctx, |ui| match promise.ready() {
            None => {
                ui.add(egui::Spinner::new()); // still loading
            }
            Some(Err(err)) => {
                ui.colored_label(egui::Color32::RED, err); // something went wrong
            }
            Some(Ok(image)) => {
                image.show_max_size(ui, ui.available_size());
            }
        });
    }
}

fn parse_response(response: ehttp::Response) -> Result<RetainedImage, String> {
    let content_type = response.content_type().unwrap_or_default();
    if content_type.starts_with("image/") {
        RetainedImage::from_image_bytes(&response.url, &response.bytes)
    } else {
        Err(format!(
            "Expected image, found content-type {:?}",
            content_type
        ))
    }
}
