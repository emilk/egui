mod diff_loader;

use crate::diff_loader::DiffLoader;
use eframe::egui::{Context, Image, ImageSource, Slider};
use eframe::{Frame, NativeOptions, egui};
use egui_extras::install_image_loaders;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "kitdiff",
        NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

struct Snapshot {
    name: String,
    old: PathBuf,
    new: PathBuf,
}

struct App {
    snapshots: Vec<Snapshot>,
    index: usize,
    new_opacity: f32,
    diff_opacity: f32,
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
            diff_opacity: 1.0,
        }
    }

    fn discover_snapshots(path: &str) -> Vec<Snapshot> {
        let mut snapshots = Vec::new();
        let mut snapshot_groups: BTreeMap<
            String,
            (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>),
        > = Default::default();

        Self::collect_snapshots_recursive(path, &mut snapshot_groups);

        for (name, (current, old, new)) in snapshot_groups {
            if let Some(current_path) = current {
                if let Some(old) = old {
                    snapshots.push(Snapshot {
                        name,
                        old: old.clone(),
                        new: current_path.clone(),
                    });
                } else if let Some(new) = new {
                    snapshots.push(Snapshot {
                        name,
                        old: current_path.clone(),
                        new: new.clone(),
                    });
                }
            }
        }

        snapshots
    }

    fn collect_snapshots_recursive(
        path: &str,
        snapshot_groups: &mut BTreeMap<String, (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>)>,
    ) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();

                if entry_path.is_dir() {
                    if let Some(dir_name) = entry_path.to_str() {
                        Self::collect_snapshots_recursive(dir_name, snapshot_groups);
                    }
                } else if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.ends_with(".png") {
                        let relative_path = entry_path.strip_prefix(".").unwrap_or(&entry_path);
                        let key = format!("{}", relative_path.display());

                        if file_name.ends_with(".old.png") {
                            let base_key = key.strip_suffix(".old.png").unwrap().to_string();
                            snapshot_groups.entry(base_key).or_default().1 = Some(entry_path);
                        } else if file_name.ends_with(".new.png") {
                            let base_key = key.strip_suffix(".new.png").unwrap().to_string();
                            snapshot_groups.entry(base_key).or_default().2 = Some(entry_path);
                        } else {
                            let base_key = key.strip_suffix(".png").unwrap().to_string();
                            if !file_name.ends_with(".old.png") && !file_name.ends_with(".new.png")
                            {
                                snapshot_groups.entry(base_key).or_default().0 = Some(entry_path);
                            }
                        }
                    }
                }
            }
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

            ui.add(Slider::new(&mut self.new_opacity, 0.0..=1.0).text("New Opacity"));
            ui.add(Slider::new(&mut self.diff_opacity, 0.0..=1.0).text("Diff Opacity"));

            if let Some(snapshot) = self.snapshots.get(self.index) {
                let rect = ui.available_rect_before_wrap();

                ui.place(
                    rect,
                    Image::new(format!("file://{}", snapshot.old.display())),
                );

                ui.set_opacity(self.new_opacity);
                ui.place(
                    rect,
                    Image::new(format!("file://{}", snapshot.new.display())),
                );

                let diff_uri = diff_loader::DiffUri {
                    old: format!("file://{}", snapshot.old.display()),
                    new: format!("file://{}", snapshot.new.display()),
                }
                .to_uri();

                ui.set_opacity(self.diff_opacity);
                ui.place(rect, Image::new(diff_uri));
            } else {
                ui.label("No snapshots found.");
            }
        });
    }
}
