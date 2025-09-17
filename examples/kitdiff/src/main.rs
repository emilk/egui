mod diff_loader;

use crate::diff_loader::DiffLoader;
use eframe::egui::{Context, Image, ImageSource, SizeHint, Slider};
use eframe::{Frame, NativeOptions, egui};
use egui_extras::install_image_loaders;
use jwalk::WalkDir;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "kitdiff",
        NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

struct SnapshotGroup {
    current: Option<PathBuf>,
    old: Option<PathBuf>,
    new: Option<PathBuf>,
    diff: Option<PathBuf>,
}

impl Default for SnapshotGroup {
    fn default() -> Self {
        Self {
            current: None,
            old: None,
            new: None,
            diff: None,
        }
    }
}

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
        let mut snapshots = Vec::new();
        let mut snapshot_groups: HashMap<String, SnapshotGroup> = Default::default();

        for entry in WalkDir::new(path)
            .skip_hidden(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".png") {
                    let entry_path = entry.path();
                    let relative_path = entry_path.strip_prefix(".").unwrap_or(&entry_path);
                    let key = format!("{}", relative_path.display());

                    if file_name.ends_with(".old.png") {
                        let base_key = key.strip_suffix(".old.png").unwrap().to_string();
                        snapshot_groups.entry(base_key).or_default().old =
                            Some(entry_path.to_path_buf());
                    } else if file_name.ends_with(".new.png") {
                        let base_key = key.strip_suffix(".new.png").unwrap().to_string();
                        snapshot_groups.entry(base_key).or_default().new =
                            Some(entry_path.to_path_buf());
                    } else if file_name.ends_with(".diff.png") {
                        let base_key = key.strip_suffix(".diff.png").unwrap().to_string();
                        snapshot_groups.entry(base_key).or_default().diff =
                            Some(entry_path.to_path_buf());
                    } else {
                        let base_key = key.strip_suffix(".png").unwrap().to_string();
                        if !file_name.ends_with(".old.png")
                            && !file_name.ends_with(".new.png")
                            && !file_name.ends_with(".diff.png")
                        {
                            snapshot_groups.entry(base_key).or_default().current =
                                Some(entry_path.to_path_buf());
                        }
                    }
                }
            }
        }

        for (name, group) in snapshot_groups {
            if let Some(diff) = group.diff {
                if let Some(current_path) = group.current {
                    if let Some(old) = group.old {
                        snapshots.push(Snapshot {
                            name,
                            old: old.clone(),
                            new: current_path.clone(),
                            diff: Some(diff.clone()),
                        });
                    } else if let Some(new) = group.new {
                        snapshots.push(Snapshot {
                            name,
                            old: current_path.clone(),
                            new: new.clone(),
                            diff: Some(diff.clone()),
                        });
                    }
                }
            }
        }

        snapshots.sort_by(|a, b| a.name.cmp(&b.name));

        snapshots
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
