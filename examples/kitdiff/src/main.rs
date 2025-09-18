mod diff_loader;
mod file_diff;

use crate::diff_loader::DiffLoader;
use crate::file_diff::file_discovery;
use eframe::egui::panel::Side;
use eframe::egui::{
    Align, Context, Image, ScrollArea, SizeHint, Slider, TextureFilter, TextureOptions,
};
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
    texture_magnification: TextureFilter,
    use_original_diff: bool,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        install_image_loaders(&cc.egui_ctx);
        cc.egui_ctx
            .add_image_loader(Arc::new(DiffLoader::default()));

        let (sender, receiver) = mpsc::channel();
        let ctx = cc.egui_ctx.clone();

        // Start background discovery
        file_discovery(".", sender, ctx);

        Self {
            snapshots: Vec::new(),
            index: 0,
            new_opacity: 0.5,
            diff_opacity: 0.25,
            mode: ImageMode::Fit,
            receiver: Some(receiver),
            is_loading: true,
            texture_magnification: TextureFilter::Nearest,
            use_original_diff: false,
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

        let mut new_index = None;
        if ctx.input_mut(|i| {
            i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowDown)
        }) {
            if self.index + 1 < self.snapshots.len() {
                new_index = Some(self.index + 1);
            }
        }
        if ctx
            .input_mut(|i| i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowUp))
        {
            if self.index > 0 {
                new_index = Some(self.index - 1);
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
                        let response = ui.selectable_label(
                            i == self.index,
                            snapshot.path.file_name().unwrap().to_str().unwrap(),
                        );

                        if Some(i) == new_index {
                            response.scroll_to_me(Some(Align::Center))
                        }

                        if response.clicked() {
                            self.index = i;
                        }
                    });
                }
            });
        });

        egui::SidePanel::right("options").show(ctx, |ui| {
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

            ui.checkbox(
                &mut self.use_original_diff,
                "Use original diff if available",
            );

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

            if let Some(snapshot) = self.snapshots.get(self.index) {
                let rect = ui.available_rect_before_wrap();

                let ppp = ui.pixels_per_point();
                let make_image = |uri: String| {
                    let mut img = Image::new(uri).texture_options(TextureOptions {
                        magnification: self.texture_magnification,
                        ..TextureOptions::default()
                    });
                    if self.mode == ImageMode::Pixel {
                        img = img.fit_to_original_size(1.0 / ppp);
                    }
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
                    let diff_uri = self
                        .use_original_diff
                        .then_some(snapshot.file_diff_uri())
                        .flatten()
                        .unwrap_or(snapshot.diff_uri());
                    ui.place(rect, make_image(diff_uri));
                }

                ui.set_opacity(1.0);

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
