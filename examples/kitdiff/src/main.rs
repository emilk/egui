mod diff_loader;
mod file_diff;
mod git_loader;

use crate::diff_loader::{DiffLoader, DiffOptions};
use crate::file_diff::file_discovery;
use crate::git_loader::{git_discovery, pr_git_discovery};
use clap::{Parser, Subcommand};
use eframe::egui::panel::Side;
use eframe::egui::{
    Align, Context, Image, ImageSource, RichText, ScrollArea, SizeHint, Slider, TextEdit,
    TextureFilter, TextureOptions,
};
use eframe::{Frame, NativeOptions, egui};
use egui_extras::install_image_loaders;
use std::path::PathBuf;
use std::sync::{Arc, mpsc};

#[derive(Parser)]
#[command(name = "kitdiff")]
#[command(about = "A viewer for egui kittest snapshot test files")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Compare snapshot test files (.png with .old/.new/.diff variants) (default)
    Files,
    /// Compare images between current branch and default branch
    Git,
    /// Compare images between PR branches from GitHub PR URL (needs to be run from within the repo)
    Pr { url: String },
}

fn main() -> eframe::Result<()> {
    let cli = Cli::parse();
    let mode = match cli.command {
        Some(Commands::Git) => ComparisonMode::Git,
        Some(Commands::Pr { url }) => ComparisonMode::Pr(url),
        Some(Commands::Files) | None => ComparisonMode::Files,
    };

    eframe::run_native(
        "kitdiff",
        NativeOptions::default(),
        Box::new(move |cc| Ok(Box::new(App::new(cc, mode)))),
    )
}

#[derive(Debug, Clone, PartialEq)]
enum ComparisonMode {
    Files,
    Git,
    Pr(String), // Store the PR URL
}

#[derive(Debug, Clone)]
enum FileReference {
    Path(PathBuf),
    Source(ImageSource<'static>),
}

impl FileReference {
    fn to_uri(&self) -> String {
        match self {
            FileReference::Path(path) => format!("file://{}", path.display()),
            FileReference::Source(source) => match source {
                ImageSource::Uri(uri) => uri.to_string(),
                ImageSource::Bytes { uri, .. } => uri.to_string(),
                _ => "unknown://unknown".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone)]
struct Snapshot {
    path: PathBuf,
    old: FileReference,
    new: FileReference,
    diff: Option<PathBuf>,
}

impl Snapshot {
    fn old_uri(&self) -> String {
        self.old.to_uri()
    }

    fn new_uri(&self) -> String {
        self.new.to_uri()
    }

    fn file_diff_uri(&self) -> Option<String> {
        self.diff
            .as_ref()
            .map(|p| format!("file://{}", p.display()))
    }

    fn diff_uri(&self, use_file_if_available: bool, options: DiffOptions) -> String {
        use_file_if_available
            .then(|| self.file_diff_uri())
            .flatten()
            .unwrap_or_else(|| {
                diff_loader::DiffUri {
                    old: self.old_uri(),
                    new: self.new_uri(),
                    options,
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
    diff_loader: Arc<DiffLoader>,
    snapshots: Vec<Snapshot>,
    index: usize,
    new_opacity: f32,
    diff_opacity: f32,
    mode: ImageMode,
    receiver: Option<mpsc::Receiver<Snapshot>>,
    is_loading: bool,
    texture_magnification: TextureFilter,
    use_original_diff: bool,
    options: DiffOptions,
    filter: String,
}

impl App {
    pub fn new(cc: &eframe::CreationContext, comparison_mode: ComparisonMode) -> Self {
        install_image_loaders(&cc.egui_ctx);
        let diff_loader = Arc::new(DiffLoader::default());
        cc.egui_ctx.add_image_loader(diff_loader.clone());

        let (sender, receiver) = mpsc::channel();
        let ctx = cc.egui_ctx.clone();

        // Start background discovery based on mode
        match comparison_mode {
            ComparisonMode::Files => {
                file_discovery(".", sender, ctx);
            }
            ComparisonMode::Git => {
                if let Err(e) = git_discovery(sender, ctx) {
                    eprintln!("Failed to start git discovery: {:?}", e);
                }
            }
            ComparisonMode::Pr(ref pr_url) => {
                if let Err(e) = pr_git_discovery(pr_url.clone(), sender, ctx) {
                    eprintln!("Failed to start PR discovery: {:?}", e);
                }
            }
        }

        Self {
            diff_loader,
            snapshots: Vec::new(),
            index: 0,
            new_opacity: 0.5,
            diff_opacity: 0.25,
            mode: ImageMode::Fit,
            receiver: Some(receiver),
            is_loading: true,
            texture_magnification: TextureFilter::Nearest,
            use_original_diff: comparison_mode == ComparisonMode::Files,
            options: DiffOptions::default(),
            filter: String::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        // Handle incoming snapshots from background discovery
        if let Some(receiver) = &self.receiver {
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
        let current_filtered_index = filtered
            .iter()
            .position(|(i, _)| *i == self.index)
            .unwrap_or(0);


        let mut new_index = None;
        if ctx.input_mut(|i| {
            i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::ArrowDown)
        }) {
            // Find next snapshot that matches filter
            if current_filtered_index + 1 < filtered.len() {
                new_index = Some(filtered[current_filtered_index + 1].0);
            }
        }
        if ctx
            .input_mut(|i| i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::ArrowUp))
        {
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

            ScrollArea::vertical().show(ui, |ui| {
                ui.set_width(ui.available_width());

                TextEdit::singleline(&mut self.filter)
                    .hint_text("Filter")
                    .show(ui);

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
