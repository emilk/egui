//! Checks `GlowConfiguration::partial_repaint` against a full repaint, pixel for pixel.
//!
//! Runs itself twice, with and without partial repaint, through the same scripted frames: a spinner, a scrolling
//! list, a moving square, a popup, an updated texture and a recolored panel. Every frame is read back after
//! painting, so any pixel a partial repaint leaves stale shows up as a difference.
//!
//! Needs a display and an OpenGL implementation with buffer age, e.g. `xvfb-run cargo run -p test_partial_repaint`
//! with Mesa. Exits with an error when partial repaint never kicked in, since then nothing was compared.

#![expect(rustdoc::missing_crate_level_docs)] // it's a test app
#![expect(clippy::print_stdout, clippy::print_stderr)] // it reports to its parent through stdout

use core::hash::{Hash as _, Hasher as _};
use std::{
    hash::DefaultHasher,
    io::Write as _,
    process::{Command, ExitCode},
    sync::Arc,
};

use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, Stroke, pos2, vec2};

const FRAMES: u64 = 90;
const MODE_VAR: &str = "TEST_PARTIAL_REPAINT_MODE";
const ZOOM_VAR: &str = "TEST_PARTIAL_REPAINT_ZOOM";
const DUMP_VAR: &str = "TEST_PARTIAL_REPAINT_DUMP";

/// A fractional scale puts shape edges between pixels, where a too tight damage area would show.
const ZOOMS: [f32; 2] = [1.0, 1.5];

#[derive(Clone, Copy, Debug)]
enum Mode {
    Full,
    Partial,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Partial => "partial",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        [Self::Full, Self::Partial]
            .into_iter()
            .find(|mode| mode.name() == name)
    }
}

fn main() -> ExitCode {
    let zoom = std::env::var(ZOOM_VAR)
        .ok()
        .and_then(|zoom| zoom.parse().ok());
    match (
        std::env::var(MODE_VAR)
            .ok()
            .as_deref()
            .and_then(Mode::from_name),
        zoom,
    ) {
        (Some(mode), Some(zoom)) => run_scene(mode, zoom),
        _ => {
            if ZOOMS.map(compare_modes).iter().all(|passed| *passed) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
    }
}

struct Run {
    frames: Vec<String>,
    partial_frames: usize,
}

fn run_child(mode: Mode, zoom: f32) -> Result<Run, String> {
    let exe = std::env::current_exe().map_err(|err| err.to_string())?;
    let output = Command::new(exe)
        .env(MODE_VAR, mode.name())
        .env(ZOOM_VAR, zoom.to_string())
        .env("RUST_LOG", "eframe::native::glow_integration=trace")
        .output()
        .map_err(|err| err.to_string())?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "the {} run failed: {}\n{stderr}",
            mode.name(),
            output.status
        ));
    }
    Ok(Run {
        frames: String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| line.starts_with("frame "))
            .map(str::to_owned)
            .collect(),
        partial_frames: stderr
            .lines()
            .filter(|line| line.contains("partial repaint") && line.contains("repaint: Partial"))
            .count(),
    })
}

fn compare_modes(zoom: f32) -> bool {
    let runs =
        run_child(Mode::Full, zoom).and_then(|full| Ok((full, run_child(Mode::Partial, zoom)?)));
    let (full, partial) = match runs {
        Ok(runs) => runs,
        Err(err) => {
            eprintln!("{err}");
            return false;
        }
    };
    let mismatches: Vec<(&String, &String)> = full
        .frames
        .iter()
        .zip(&partial.frames)
        .filter(|(full, partial)| full != partial)
        .collect();
    println!(
        "zoom {zoom}: {} frames compared, {} painted partially, {} differ",
        full.frames.len().min(partial.frames.len()),
        partial.partial_frames,
        mismatches.len()
    );
    for (full, partial) in &mismatches {
        println!("  full    {full}\n  partial {partial}");
    }
    if full.frames.len() != partial.frames.len() || full.frames.len() < FRAMES as usize {
        println!(
            "expected {FRAMES} frames, got {} full and {} partial",
            full.frames.len(),
            partial.frames.len()
        );
        return false;
    }
    if partial.partial_frames == 0 {
        println!("partial repaint was never used - no buffer age on this platform?");
        return false;
    }
    if !mismatches.is_empty() {
        println!("set {DUMP_VAR}=<dir> to write every frame as PPM");
    }
    mismatches.is_empty()
}

fn run_scene(mode: Mode, zoom: f32) -> ExitCode {
    env_logger::init();
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_resizable(false),
        ..Default::default()
    };
    options.glow_options.partial_repaint = matches!(mode, Mode::Partial);
    let result = eframe::run_native(
        "test_partial_repaint",
        options,
        Box::new(move |cc| Ok(Box::new(Scene::new(&cc.egui_ctx, mode, zoom)))),
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

struct Scene {
    mode: Mode,
    frame: u64,
    texture: egui::TextureHandle,
}

impl Scene {
    fn new(ctx: &egui::Context, mode: Mode, zoom: f32) -> Self {
        // the scene is a function of the frame number only
        ctx.global_style_mut(|style| style.animation_time = 0.0);
        ctx.set_zoom_factor(zoom);
        Self {
            mode,
            frame: 0,
            texture: ctx.load_texture("checker", checker(0), egui::TextureOptions::NEAREST),
        }
    }
}

fn checker(seed: u64) -> ColorImage {
    let size = 16;
    let pixels = (0..size * size)
        .map(|i| {
            let on = ((i % size) / 4 + (i / size) / 4 + seed as usize).is_multiple_of(2);
            if on {
                Color32::from_rgb(200, 40, 40)
            } else {
                Color32::from_rgb(40, 40, 200)
            }
        })
        .collect();
    ColorImage::new([size, size], pixels)
}

impl eframe::App for Scene {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frame += 1;
        let n = self.frame;
        let ctx = ui.ctx().clone();
        if n > FRAMES {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        if n.is_multiple_of(5) {
            self.texture
                .set(checker(n / 5), egui::TextureOptions::NEAREST);
        }

        let fill = if (40..45).contains(&n) {
            Color32::from_rgb(30, 60, 30)
        } else {
            Color32::from_gray(27)
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).fill(fill))
            .show(ui, |ui| {
                ui.label(format!("frame {n}"));
                egui::Grid::new("static").show(ui, |ui| {
                    for row in 0..6 {
                        for column in 0..4 {
                            ui.label(format!("cell {row}:{column}"));
                        }
                        ui.end_row();
                    }
                });
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .max_height(80.0)
                    .max_width(150.0)
                    .vertical_scroll_offset((n as f32 * 2.5) % 400.0)
                    .show(ui, |ui| {
                        for row in 0..40 {
                            ui.label(format!("scrolling row {row}"));
                        }
                    });
                ui.image((self.texture.id(), vec2(32.0, 32.0)));
            });

        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::unique("anim"),
        ));
        let spinner = pos2(370.0, 30.0);
        let angle = n as f32 * 0.3;
        painter.circle_stroke(spinner, 14.0, Stroke::new(1.0, Color32::DARK_GRAY));
        painter.line_segment(
            [spinner, spinner + 12.0 * vec2(angle.cos(), angle.sin())],
            Stroke::new(3.0, Color32::LIGHT_BLUE),
        );
        let x = 180.0 + (n * 7 % 200) as f32;
        painter.rect_filled(
            Rect::from_min_size(pos2(x, 240.0), vec2(14.0, 14.0)),
            3.0,
            Color32::YELLOW,
        );

        if (20..30).contains(&n) {
            egui::Area::new(egui::Id::unique("popup"))
                .fixed_pos(Pos2::new(200.0 + n as f32, 100.0))
                .show(&ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label("a popup");
                    });
                });
        }

        let mode = self.mode;
        ctx.request_screenshot(move |image: Arc<ColorImage>| report(mode, n, &image));
        ctx.request_repaint();
    }
}

fn report(mode: Mode, n: u64, image: &ColorImage) {
    let mut hasher = DefaultHasher::new();
    image.pixels().hash(&mut hasher);
    println!("frame {n} {:016x}", hasher.finish());
    if let Ok(dir) = std::env::var(DUMP_VAR) {
        let path = std::path::Path::new(&dir).join(format!("{}_{n:03}.ppm", mode.name()));
        if let Err(err) = write_ppm(&path, image) {
            eprintln!("{}: {err}", path.display());
        }
    }
}

fn write_ppm(path: &std::path::Path, image: &ColorImage) -> std::io::Result<()> {
    let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
    write!(file, "P6\n{} {}\n255\n", image.width(), image.height())?;
    for pixel in image.pixels() {
        file.write_all(&[pixel.r(), pixel.g(), pixel.b()])?;
    }
    file.flush()
}
