#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use eframe::egui::load::{SizedTexture, TexturePoll};
use eframe::egui::{
    Color32, ColorImage, Context, Image, ImageData, ImageSource, SizeHint, TextureHandle,
    TextureOptions,
};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use std::{mem, thread};
use url::Url;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Image loading and creating",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::<ImageApp>::default())
        }),
    )
}

struct ImageApp {
    /// URI and Handle - need URI in order to forget the texture
    texture: Option<(String, TextureHandle)>,
    path: String,
    worker: Option<thread::JoinHandle<Option<(String, TextureHandle)>>>,
}

impl Default for ImageApp {
    fn default() -> Self {
        Self {
            texture: None,
            path: "crates/egui/assets/ferris.png".to_string(),
            worker: None,
        }
    }
}

impl ImageApp {
    fn load_image(&mut self, ctx: &Context) {
        self.forget_existing_image(ctx);

        let ctx = ctx.clone();
        let path = PathBuf::from(self.path.as_str());

        let handle = std::thread::Builder::new()
            .name("load_image".to_string())
            .spawn(move || {
                fn load_image_from_file_using_egui_extras(
                    ctx: &Context,
                    path: &Path,
                ) -> Option<(String, TextureHandle)> {
                    // Attempt to load the image
                    let absolute_path = path.canonicalize().ok()?;
                    println!("Loading image from {:?}", absolute_path);
                    let url = Url::from_file_path(absolute_path).unwrap();
                    println!("uri: {}", url);

                    let texture = loop {
                        let poll = ctx
                            .try_load_texture(
                                url.as_str(),
                                TextureOptions::default(),
                                SizeHint::default(),
                            )
                            .ok()?;
                        match poll {
                            TexturePoll::Pending { .. } => {
                                println!("Waiting for image load");
                                thread::sleep(Duration::from_millis(100));
                            }
                            TexturePoll::Ready { texture } => {
                                println!("Loaded");
                                break texture;
                            }
                        }
                    };

                    ctx.tex_manager().write().retain(texture.id);
                    Some((
                        url.to_string(),
                        TextureHandle::new(ctx.tex_manager(), texture.id),
                    ))
                }

                let result = load_image_from_file_using_egui_extras(&ctx.clone(), &path);

                ctx.request_repaint();

                result
            });

        self.worker = Some(handle.unwrap())
    }

    fn generate_image(&mut self, ctx: &Context) {
        self.forget_existing_image(ctx);

        let image_data: ImageData =
            ImageData::Color(Arc::new(ColorImage::new([100, 100], Color32::RED)));

        let uri = "generated-image".to_string();
        let texture_handle = ctx.load_texture(uri.clone(), image_data, Default::default());

        self.texture = Some((uri, texture_handle));
    }

    fn forget_existing_image(&mut self, ctx: &Context) {
        if let Some((uri, _existing_texture)) = self.texture.take() {
            // forget the image so that the image is loaded from disk again.
            ctx.forget_image(uri.as_str());
        }
    }
}

impl eframe::App for ImageApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.worker.is_some() && self.worker.as_ref().unwrap().is_finished() {
            let worker = self.worker.take().unwrap();

            match worker.join() {
                Ok(result) => {
                    self.texture = result;
                }
                Err(_) => {
                    println!("Failed to load image")
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {

            ui.label("this example demonstrates how to render an image that is either loaded from the filesystem or generated programmatically");

            ui.horizontal(|ui| {
                ui.label("path");
                ui.text_edit_singleline(&mut self.path);
                if ui.button("Load").clicked() {
                    self.load_image(ctx);
                }

            });

            ui.horizontal(|ui| {
                if ui.button("Generate").clicked() {
                    self.generate_image(ctx);
                }
                ui.label("(no files are touched when generating image)");
            });

            egui::Frame::new().show(ui, |ui| {
                match &self.texture {
                    Some((_uri, texture_handle)) => {
                        let image_source = ImageSource::Texture(SizedTexture::from_handle(&texture_handle));
                        let image = Image::new(image_source);

                        ui.add_sized(ui.available_size(), image);
                    }
                    None => {
                        ui.label("Click 'Load' or 'Generate'");
                    }
                }
            });
        });
    }
}
