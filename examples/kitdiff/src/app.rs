use crate::diff_image_loader::{DiffLoader, DiffOptions};
use crate::snapshot::{FileReference, Snapshot};
use crate::{DiffSource, PathOrBlob};
use eframe::egui::panel::Side;
use eframe::egui::{
    Align, Context, Image, ImageSource, Modifiers, RichText, ScrollArea, SizeHint, Slider,
    TextEdit, TextureFilter, TextureOptions,
};
use eframe::{Frame, egui};
use egui_extras::install_image_loaders;
use std::sync::{Arc, mpsc};

#[derive(Debug, Clone, PartialEq)]
enum ImageMode {
    Pixel,
    Fit,
}

pub struct App {
    diff_loader: Arc<DiffLoader>,
    snapshots: Vec<Snapshot>,
    index: usize,
    new_opacity: f32,
    diff_opacity: f32,
    mode: ImageMode,
    receiver: mpsc::Receiver<Snapshot>,
    sender: mpsc::Sender<Snapshot>,
    is_loading: bool,
    texture_magnification: TextureFilter,
    use_original_diff: bool,
    options: DiffOptions,
    filter: String,
    #[cfg(feature = "zip_support")]
    _temp_dir: Option<TempDir>, // Keep temp directory alive for zip mode
    drag_and_drop_enabled: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext, source: Option<DiffSource>) -> Self {
        install_image_loaders(&cc.egui_ctx);
        let diff_loader = Arc::new(DiffLoader::default());
        cc.egui_ctx.add_image_loader(diff_loader.clone());

        let (sender, receiver) = mpsc::channel();
        let ctx = cc.egui_ctx.clone();

        if let Some(source) = source {
            source.load(sender.clone(), ctx);
        }

        Self {
            diff_loader,
            snapshots: Vec::new(),
            index: 0,
            new_opacity: 0.5,
            diff_opacity: 0.25,
            mode: ImageMode::Fit,
            receiver: receiver,
            sender: sender,
            is_loading: true,
            texture_magnification: TextureFilter::Nearest,
            use_original_diff: true,
            options: DiffOptions::default(),
            filter: String::new(),
            #[cfg(feature = "zip_support")]
            _temp_dir: temp_dir,
            drag_and_drop_enabled: true,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        // Handle incoming snapshots from background discovery
        {
            let receiver = &self.receiver;
            let mut new_snapshots = Vec::new();
            while let Ok(snapshot) = receiver.try_recv() {
                if let FileReference::Source(ImageSource::Bytes { bytes, uri }) = &snapshot.old {
                    ctx.include_bytes(uri.clone(), bytes.clone())
                }
                if let FileReference::Source(ImageSource::Bytes { bytes, uri }) = &snapshot.new {
                    ctx.include_bytes(uri.clone(), bytes.clone())
                }
                new_snapshots.push(snapshot);
            }

            if !new_snapshots.is_empty() {
                // Snapshots should arrive sorted.
                self.snapshots.extend(new_snapshots);
            }

            // Check if the channel is disconnected (discovery finished)
            if receiver.try_recv().is_err() && self.is_loading {
                // Try one more time to ensure we didn't miss any
                if matches!(receiver.try_recv(), Err(mpsc::TryRecvError::Disconnected)) {
                    self.is_loading = false;
                }
            }
        }

        for file in &ctx.input(|i| i.raw.dropped_files.clone()) {
            let data = file
                .bytes.clone()
                .map(|b| PathOrBlob::Blob(b.into()))
                .or(file.path.as_ref().map(|p| PathOrBlob::Path(p.clone())));

            if let Some(data) = data {
                let source = if file.name.ends_with(".tar.gz") || file.name.ends_with(".tgz") {
                    Some(DiffSource::TarGz(data))
                } else if file.name.ends_with(".zip") {
                    Some(DiffSource::Zip(data))
                } else {
                    None
                };

                if let Some(source) = source {
                    // Clear existing snapshots for new file
                    self.snapshots.clear();
                    self.index = 0;
                    self.is_loading = true;

                    source.load(self.sender.clone(), ctx.clone());
                }
            }

            // if let Some(path) = &file.path {
            //     if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            //         if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
            //             // For native, read from file system
            //             #[cfg(not(target_arch = "wasm32"))]
            //             if let Ok(data) = std::fs::read(path) {
            //                 if let Some(sender) = &self.sender {
            //                     // Clear existing snapshots for new file
            //                     self.snapshots.clear();
            //                     self.index = 0;
            //                     self.is_loading = true;
            //
            //                     if let Err(e) =
            //                         extract_and_discover_tar_gz(data, sender.clone(), ctx.clone())
            //                     {
            //                         eprintln!("Failed to extract tar.gz: {:?}", e);
            //                     }
            //                 }
            //             }
            //         }
            //     }
            // }
            //
            // // For wasm, use the bytes directly if available
            // #[cfg(target_arch = "wasm32")]
            // if let Some(bytes) = &file.bytes {
            //     let name = &file.name;
            //     if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
            //         if let Some(sender) = &self.sender {
            //             // Clear existing snapshots for new file
            //             self.snapshots.clear();
            //             self.index = 0;
            //             self.is_loading = true;
            //
            //             if let Err(e) =
            //                 extract_and_discover_tar_gz(bytes.to_vec(), sender.clone(), ctx.clone())
            //             {
            //                 eprintln!("Failed to extract tar.gz: {:?}", e);
            //                 panic!("{e:?}")
            //             }
            //         }
            //     }
            // }
        }

        let filtered = self
            .snapshots
            .iter()
            .enumerate()
            .filter(|(_, snapshot)| {
                self.filter.is_empty()
                    || snapshot
                        .path
                        .to_str()
                        .map(|p| p.contains(&self.filter))
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        let current_filtered_index = filtered.iter().position(|(i, _)| *i == self.index);
        if current_filtered_index.is_none() && !filtered.is_empty() {
            // Current index is filtered out, jump to first filtered
            self.index = filtered[0].0;
        }
        let current_filtered_index = current_filtered_index.unwrap_or(0);

        let mut new_index = None;
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, egui::Key::ArrowDown)) {
            // Find next snapshot that matches filter
            if current_filtered_index + 1 < filtered.len() {
                new_index = Some(filtered[current_filtered_index + 1].0);
            }
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, egui::Key::ArrowUp)) {
            // Find previous snapshot that matches filter
            if current_filtered_index > 0 {
                new_index = Some(filtered[current_filtered_index - 1].0);
            }
        }
        if let Some(new_index) = new_index {
            self.index = new_index;
        }

        let (show_old, show_new, show_diff) = ctx.input(|i| {
            let show_old = i.key_down(egui::Key::Num1);
            let show_new = i.key_down(egui::Key::Num2);
            let show_diff = i.key_down(egui::Key::Num3);
            (show_old, show_new, show_diff)
        });
        let show_all = !show_old && !show_new && !show_diff;

        egui::SidePanel::new(Side::Left, "files").show(ctx, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);

            TextEdit::singleline(&mut self.filter)
                .hint_text("Filter")
                .show(ui);

            ScrollArea::vertical().show(ui, |ui| {
                ui.set_width(ui.available_width());

                let mut current_prefix = None;
                for (i, snapshot) in &filtered {
                    let prefix = snapshot.path.parent().and_then(|p| p.to_str());
                    if prefix != current_prefix {
                        if let Some(prefix) = prefix {
                            ui.label(prefix);
                        }
                        current_prefix = prefix;
                    }

                    ui.indent(&snapshot.path, |ui| {
                        let response = ui.selectable_label(
                            *i == self.index,
                            snapshot.path.file_name().unwrap().to_str().unwrap(),
                        );

                        if Some(*i) == new_index {
                            response.scroll_to_me(Some(Align::Center))
                        }

                        if response.clicked() {
                            self.index = *i;
                        }
                    });
                }
            });
        });

        egui::SidePanel::right("options").show(ctx, |ui| {
            ui.set_width(ui.available_width());
            ui.add(Slider::new(&mut self.new_opacity, 0.0..=1.0).text("New Opacity"));
            ui.add(Slider::new(&mut self.diff_opacity, 0.0..=1.0).text("Diff Opacity"));
            ui.add(
                Slider::new(&mut self.index, 0..=self.snapshots.len().saturating_sub(1))
                    .text("Snapshot Index"),
            );

            ui.horizontal_wrapped(|ui| {
                ui.label("Size:");
                ui.selectable_value(&mut self.mode, ImageMode::Pixel, "1:1");
                ui.selectable_value(&mut self.mode, ImageMode::Fit, "Fit");
            });

            ui.horizontal_wrapped(|ui| {
                ui.label("Filtering:");
                ui.selectable_value(
                    &mut self.texture_magnification,
                    TextureFilter::Nearest,
                    "Nearest",
                );
                ui.selectable_value(
                    &mut self.texture_magnification,
                    TextureFilter::Linear,
                    "Linear",
                );
            });

            ui.group(|ui| {
                ui.heading("Diff Options");
                ui.checkbox(
                    &mut self.use_original_diff,
                    "Use original diff if available",
                );

                ui.add(
                    Slider::new(&mut self.options.threshold, 0.01..=1000.0)
                        .logarithmic(true)
                        .text("Diff Threshold"),
                );
                ui.checkbox(&mut self.options.detect_aa_pixels, "Detect AA Pixels");
            });

            // Show loading status
            if self.is_loading {
                ui.label(format!(
                    "Loading... {} snapshots found",
                    self.snapshots.len()
                ));
            } else {
                ui.label(format!("{} snapshots total", self.snapshots.len()));
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(
                "Use 1/2/3 to only show old / new / diff at 100% opacity. Arrow keys to navigate.",
            );

            #[cfg(feature = "tar_support")]
            if self.drag_and_drop_enabled && self.snapshots.is_empty() && !self.is_loading {
                ui.separator();
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.heading("Drop a .tar.gz or .tgz file here");
                    ui.label("The file should contain PNG snapshot files");
                    ui.add_space(20.0);
                });
            }

            if let Some(snapshot) = self.snapshots.get(self.index) {
                let diff_uri = snapshot.diff_uri(self.use_original_diff, self.options);

                if let Some(info) = self.diff_loader.diff_info(&diff_uri) {
                    if info.diff == 0 {
                        ui.strong("All differences below threshold!");
                    } else {
                        ui.label(
                            RichText::new(format!("Diff pixels: {}", info.diff))
                                .color(ui.visuals().warn_fg_color),
                        );
                    }
                } else {
                    ui.label("No diff info yet...");
                }

                // ui.label(format!("old: {}", snapshot.old_uri()));
                // ui.label(format!("new: {}", snapshot.new_uri()));
                // ui.label(format!("diff: {}", diff_uri));

                let rect = ui.available_rect_before_wrap();

                let ppp = ui.pixels_per_point();
                let mut any_loading = false;
                let mut make_image = |uri: String| {
                    let mut img = Image::new(uri).texture_options(TextureOptions {
                        magnification: self.texture_magnification,
                        ..TextureOptions::default()
                    });
                    if self.mode == ImageMode::Pixel {
                        img = img.fit_to_original_size(1.0 / ppp);
                    }
                    any_loading |= img
                        .load_for_size(ctx, rect.size())
                        .is_ok_and(|poll| poll.is_pending());
                    img
                };

                if show_all || show_old {
                    ui.place(rect, make_image(snapshot.old_uri()));
                }

                if show_all || show_new {
                    if show_all {
                        ui.set_opacity(self.new_opacity);
                    }
                    ui.place(rect, make_image(snapshot.new_uri()));
                }

                if show_all || show_diff {
                    if show_all {
                        ui.set_opacity(self.diff_opacity);
                    }
                    ui.place(rect, make_image(diff_uri));
                }

                ui.set_opacity(1.0);

                // Preload surrounding snapshots once our image is loaded
                if !any_loading {
                    for i in -10..=10 {
                        if let Some(surrounding_snapshot) =
                            self.snapshots.get((self.index as isize + i) as usize)
                        {
                            ui.ctx()
                                .try_load_image(
                                    &surrounding_snapshot.old_uri(),
                                    SizeHint::default(),
                                )
                                .ok();
                            ui.ctx()
                                .try_load_image(
                                    &surrounding_snapshot.new_uri(),
                                    SizeHint::default(),
                                )
                                .ok();
                            ui.ctx()
                                .try_load_image(
                                    &surrounding_snapshot
                                        .diff_uri(self.use_original_diff, self.options),
                                    SizeHint::default(),
                                )
                                .ok();
                        }
                    }
                }
            } else if self.is_loading {
                ui.label("Searching for snapshots...");
            } else {
                ui.label("No snapshots found.");
            }
        });
    }
}
