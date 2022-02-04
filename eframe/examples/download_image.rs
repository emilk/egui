#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{egui, epi};
use poll_promise::Promise;

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(MyApp::default()), options);
}

#[derive(Default)]
struct MyApp {
    /// `None` when download hasn't started yet.
    promise: Option<Promise<ehttp::Result<egui::TextureHandle>>>,
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
            let ctx = ctx.clone();
            let frame = frame.clone();
            let (sender, promise) = Promise::new();
            let request = ehttp::Request::get("https://picsum.photos/seed/1.759706314/1024");
            ehttp::fetch(request, move |response| {
                frame.request_repaint(); // wake up UI thread
                let texture = response.and_then(|response| parse_response(&ctx, response));
                sender.send(texture); // send the results back to the UI thread.
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
            Some(Ok(texture)) => {
                let mut size = texture.size_vec2();
                size *= (ui.available_width() / size.x).min(1.0);
                size *= (ui.available_height() / size.y).min(1.0);
                ui.image(texture, size);
            }
        });
    }
}

fn parse_response(
    ctx: &egui::Context,
    response: ehttp::Response,
) -> Result<egui::TextureHandle, String> {
    let content_type = response.content_type().unwrap_or_default();
    if content_type.starts_with("image/") {
        let image = load_image(&response.bytes).map_err(|err| err.to_string())?;
        Ok(ctx.load_texture("my-image", image))
    } else {
        Err(format!(
            "Expected image, found content-type {:?}",
            content_type
        ))
    }
}

fn load_image(image_data: &[u8]) -> Result<egui::ColorImage, image::ImageError> {
    let image = image::load_from_memory(image_data)?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        size,
        pixels.as_slice(),
    ))
}
