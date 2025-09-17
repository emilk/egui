mod diff_loader;

use crate::diff_loader::DiffLoader;
use eframe::egui::{Context, Image, SizeHint, Slider};
use eframe::{Frame, NativeOptions, egui};
use egui_extras::install_image_loaders;
use ignore::{WalkBuilder, WalkState};
use ignore::types::TypesBuilder;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "kitdiff",
        NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}


#[derive(Debug)]
struct Snapshot {
    name: String,
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
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        install_image_loaders(&cc.egui_ctx);
        cc.egui_ctx
            .add_image_loader(Arc::new(DiffLoader::default()));
        let snapshots = Self::discover_snapshots(".");
        Self {
            snapshots,
            index: 0,
            new_opacity: 0.5,
            diff_opacity: 0.25,
            mode: ImageMode::Fit,
        }
    }

    fn discover_snapshots(path: &str) -> Vec<Snapshot> {
        let snapshots = Arc::new(Mutex::new(Vec::new()));

        // Create type matcher for .png files
        let mut types_builder = TypesBuilder::new();
        types_builder.add("png", "*.png").unwrap();
        types_builder.select("png");
        let types = types_builder.build().unwrap();

        // Build parallel walker for .png files only
        WalkBuilder::new(path)
            .types(types)
            .build_parallel()
            .run(|| {
                let snapshots = Arc::clone(&snapshots);
                Box::new(move |result| {
                    if let Ok(entry) = result {
                        if entry.file_type().map_or(false, |ft| ft.is_file()) {
                            if let Some(snapshot) = Self::try_create_snapshot(entry.path()) {
                                if let Ok(mut snapshots) = snapshots.lock() {
                                    snapshots.push(snapshot);
                                }
                            }
                        }
                    }
                    WalkState::Continue
                })
            });

        let mut snapshots = Arc::try_unwrap(snapshots).unwrap().into_inner().unwrap();
        snapshots.sort_by(|a, b| a.name.cmp(&b.name));
        snapshots
    }

    fn try_create_snapshot(png_path: &Path) -> Option<Snapshot> {
        let file_name = png_path.file_name()?.to_str()?;

        // Skip files that are already variants (.old.png, .new.png, .diff.png)
        if file_name.ends_with(".old.png") ||
           file_name.ends_with(".new.png") ||
           file_name.ends_with(".diff.png") {
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
        let name = relative_path.display().to_string();

        if old_path.exists() {
            // old.png exists, use original as new and old.png as old
            Some(Snapshot {
                name,
                old: old_path,
                new: png_path.to_path_buf(),
                diff: Some(diff_path),
            })
        } else if new_path.exists() {
            // new.png exists, use original as old and new.png as new
            Some(Snapshot {
                name,
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
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.input_mut(|i| i.key_pressed(egui::Key::ArrowRight)) {
                if self.index + 1 < self.snapshots.len() {
                    self.index += 1;
                }
            }
            if ui.input_mut(|i| i.key_pressed(egui::Key::ArrowLeft)) {
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
            } else {
                ui.label("No snapshots found.");
            }
        });
    }
}
