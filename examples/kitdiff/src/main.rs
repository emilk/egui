mod diff_loader;

use crate::diff_loader::DiffLoader;
use eframe::egui::panel::Side;
use eframe::egui::{Context, Image, ScrollArea, SizeHint, Slider};
use eframe::{Frame, NativeOptions, egui};
use egui_extras::install_image_loaders;
use ignore::WalkBuilder;
use ignore::types::TypesBuilder;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "kitdiff",
        NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

#[derive(Debug)]
struct Snapshot {
    path: PathBuf,
    old: PathBuf,
    new: PathBuf,
    diff: Option<PathBuf>,
}

impl Snapshot {
    fn old_uri(&self) -> String {
        format!("file://{}", self.old.display())
    }

    fn new_uri(&self) -> String {
        format!("file://{}", self.new.display())
    }

    fn file_diff_uri(&self) -> Option<String> {
        self.diff
            .as_ref()
            .map(|p| format!("file://{}", p.display()))
    }

    fn diff_uri(&self) -> String {
        self.file_diff_uri().unwrap_or_else(|| {
            diff_loader::DiffUri {
                old: self.old_uri(),
                new: self.new_uri(),
            }
            .to_uri()
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ImageMode {
    Pixel,
    Fit,
}

struct App {
    snapshots: Vec<Snapshot>,
    index: usize,
    new_opacity: f32,
    diff_opacity: f32,
    mode: ImageMode,
    receiver: Option<mpsc::Receiver<Snapshot>>,
    is_loading: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        install_image_loaders(&cc.egui_ctx);
        cc.egui_ctx
            .add_image_loader(Arc::new(DiffLoader::default()));

        let (sender, receiver) = mpsc::channel();
        let ctx = cc.egui_ctx.clone();

        // Start background discovery
        Self::start_discovery(".", sender, ctx);

        Self {
            snapshots: Vec::new(),
            index: 0,
            new_opacity: 0.5,
            diff_opacity: 0.25,
            mode: ImageMode::Fit,
            receiver: Some(receiver),
            is_loading: true,
        }
    }

    fn start_discovery(path: &str, sender: mpsc::Sender<Snapshot>, ctx: Context) {
        let path = path.to_string();

        std::thread::spawn(move || {
            // Create type matcher for .png files
            let mut types_builder = TypesBuilder::new();
            types_builder.add("png", "*.png").unwrap();
            types_builder.select("png");
            let types = types_builder.build().unwrap();

            // Build sequential walker for .png files only to maintain order
            for result in WalkBuilder::new(&path).types(types).build() {
                if let Ok(entry) = result {
                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        if let Some(snapshot) = Self::try_create_snapshot(entry.path()) {
                            if sender.send(snapshot).is_ok() {
                                ctx.request_repaint();
                            }
                        }
                    }
                }
            }
        });
    }

    fn try_create_snapshot(png_path: &Path) -> Option<Snapshot> {
        let file_name = png_path.file_name()?.to_str()?;

        // Skip files that are already variants (.old.png, .new.png, .diff.png)
        if file_name.ends_with(".old.png")
            || file_name.ends_with(".new.png")
            || file_name.ends_with(".diff.png")
        {
            return None;
        }

        // Get base path without .png extension
        let base_path = png_path.with_extension("");
        let old_path = base_path.with_extension("old.png");
        let new_path = base_path.with_extension("new.png");
        let diff_path = base_path.with_extension("diff.png");

        // Only create snapshot if diff exists
        if !diff_path.exists() {
            return None;
        }

        // Determine which files exist and create appropriate snapshot
        let relative_path = png_path.strip_prefix(".").unwrap_or(png_path);

        if old_path.exists() {
            // old.png exists, use original as new and old.png as old
            Some(Snapshot {
                path: relative_path.to_path_buf(),
                old: old_path,
                new: png_path.to_path_buf(),
                diff: Some(diff_path),
            })
        } else if new_path.exists() {
            // new.png exists, use original as old and new.png as new
            Some(Snapshot {
                path: relative_path.to_path_buf(),
                old: png_path.to_path_buf(),
                new: new_path,
                diff: Some(diff_path),
            })
        } else {
            // No old or new variant, skip this snapshot
            None
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        // Handle incoming snapshots from background discovery
        if let Some(receiver) = &self.receiver {
            let mut new_snapshots = Vec::new();
            while let Ok(snapshot) = receiver.try_recv() {
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

        egui::SidePanel::new(Side::Left, "files").show(ctx, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);

            ScrollArea::vertical().show(ui, |ui| {
                ui.set_width(ui.available_width());
                let mut current_prefix = None;
                for (i, snapshot) in self.snapshots.iter().enumerate() {
                    let prefix = snapshot.old.parent().and_then(|p| p.to_str());
                    if prefix != current_prefix {
                        if let Some(prefix) = prefix {
                            ui.label(prefix);
                        }
                        current_prefix = prefix;
                    }

                    ui.indent(&snapshot.path, |ui| {
                        if ui
                            .selectable_label(
                                i == self.index,
                                snapshot.path.file_name().unwrap().to_str().unwrap(),
                            )
                            .clicked()
                        {
                            self.index = i;
                        }
                    });
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.input_mut(|i| {
                i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowDown)
            }) {
                if self.index + 1 < self.snapshots.len() {
                    self.index += 1;
                }
            }
            if ui.input_mut(|i| {
                i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowUp)
            }) {
                if self.index > 0 {
                    self.index -= 1;
                }
            }

            ui.horizontal_wrapped(|ui| {
                ui.add(Slider::new(&mut self.new_opacity, 0.0..=1.0).text("New Opacity"));
                ui.add(Slider::new(&mut self.diff_opacity, 0.0..=1.0).text("Diff Opacity"));
                ui.add(
                    Slider::new(&mut self.index, 0..=self.snapshots.len().saturating_sub(1))
                        .text("Snapshot Index"),
                );

                ui.selectable_value(&mut self.mode, ImageMode::Pixel, "1:1");
                ui.selectable_value(&mut self.mode, ImageMode::Fit, "Fit");

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

            if let Some(snapshot) = self.snapshots.get(self.index) {
                let rect = ui.available_rect_before_wrap();

                let ppp = ui.pixels_per_point();
                let make_image = |uri: String| {
                    let mut img = Image::new(uri);
                    if self.mode == ImageMode::Pixel {
                        img = img.fit_to_original_size(1.0 / ppp);
                    }
                    img
                };

                ui.place(rect, make_image(snapshot.old_uri()));

                ui.set_opacity(self.new_opacity);
                ui.place(rect, make_image(snapshot.new_uri()));

                ui.set_opacity(self.diff_opacity);
                ui.place(rect, make_image(snapshot.diff_uri()));

                // Preload surrounding snapshots
                for i in -2..=2 {
                    if let Some(surrounding_snapshot) =
                        self.snapshots.get((self.index as isize + i) as usize)
                    {
                        ui.ctx()
                            .try_load_image(&surrounding_snapshot.old_uri(), SizeHint::default())
                            .ok();
                        ui.ctx()
                            .try_load_image(&surrounding_snapshot.new_uri(), SizeHint::default())
                            .ok();
                        if let Some(diff_uri) = surrounding_snapshot.file_diff_uri() {
                            ui.ctx().try_load_image(&diff_uri, SizeHint::default()).ok();
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
