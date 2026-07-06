#![expect(rustdoc::missing_crate_level_docs)]

use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::PathBuf,
    sync::Arc,
    time::Instant,
};

use eframe::egui;

fn main() -> eframe::Result {
    let log_path = log_path();
    let app_log_path = log_path.clone();

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(
            "wgpu_recovery_minimal=info,eframe=info,egui_wgpu=info,wgpu_core=info",
        ),
    )
    .target(env_logger::Target::Pipe(Box::new(TeeWriter::new(
        log_path.clone(),
    ))))
    .format(|buf, record| {
        writeln!(
            buf,
            "[{} {:>5} {}] {}",
            buf.timestamp_millis(),
            record.level(),
            record.target(),
            record.args()
        )
    })
    .init();

    log::warn!("Log file: {}", log_path.display());

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title("Minimal WGPU recovery test")
            .with_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Minimal WGPU recovery test",
        options,
        Box::new(move |_cc| Ok(Box::new(RecoveryApp::new(app_log_path)))),
    )
}

fn log_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("wgpu_recovery_minimal.log")
}

struct TeeWriter {
    file: std::fs::File,
}

impl TeeWriter {
    fn new(path: PathBuf) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .unwrap_or_else(|err| panic!("Failed to open log file {}: {err}", path.display()));
        Self { file }
    }
}

impl Write for TeeWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write_all(buf)?;
        let _ = io::stderr().write_all(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()?;
        let _ = io::stderr().flush();
        Ok(())
    }
}

struct RecoveryApp {
    log_path: PathBuf,
    started_at: Instant,
    frame_nr: u64,
    clicks: u64,
    slider: f32,
    text: String,
    static_texture: Option<egui::TextureHandle>,
    dynamic_texture: Option<egui::TextureHandle>,
    last_renderer_ptr: Option<usize>,
    renderer_switches: u64,
}

impl RecoveryApp {
    fn new(log_path: PathBuf) -> Self {
        Self {
            log_path,
            started_at: Instant::now(),
            frame_nr: 0,
            clicks: 0,
            slider: 0.5,
            text: "type after reconnect".to_owned(),
            static_texture: None,
            dynamic_texture: None,
            last_renderer_ptr: None,
            renderer_switches: 0,
        }
    }

    fn track_renderer(&mut self, frame: &eframe::Frame) {
        let Some(render_state) = frame.wgpu_render_state() else {
            return;
        };

        let renderer_ptr = Arc::as_ptr(&render_state.renderer) as usize;
        if self.last_renderer_ptr != Some(renderer_ptr) {
            self.renderer_switches += 1;
            self.last_renderer_ptr = Some(renderer_ptr);
            let adapter = render_state.adapter.get_info();
            log::warn!(
                "Renderer changed: switch={} ptr=0x{renderer_ptr:x} backend={:?} adapter={}",
                self.renderer_switches,
                adapter.backend,
                adapter.name
            );
        }
    }

    fn ensure_static_texture(&mut self, ctx: &egui::Context) {
        if self.static_texture.is_none() {
            self.static_texture = Some(ctx.load_texture(
                "minimal_static_texture",
                checker_image(),
                egui::TextureOptions::LINEAR,
            ));
        }
    }

    fn update_dynamic_texture(&mut self, ctx: &egui::Context) {
        let image = dynamic_image(self.frame_nr);
        if let Some(texture) = self.dynamic_texture.as_mut() {
            texture.set(image, egui::TextureOptions::LINEAR);
        } else {
            self.dynamic_texture = Some(ctx.load_texture(
                "minimal_dynamic_texture",
                image,
                egui::TextureOptions::LINEAR,
            ));
        }
    }
}

impl eframe::App for RecoveryApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.frame_nr += 1;
        if self.frame_nr % 60 == 0 {
            log::info!(
                "heartbeat frame={} clicks={} slider={:.3} text={:?}",
                self.frame_nr,
                self.clicks,
                self.slider,
                self.text
            );
        }

        self.track_renderer(frame);
        self.ensure_static_texture(ui.ctx());
        self.update_dynamic_texture(ui.ctx());

        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Minimal WGPU recovery test");
            ui.label("Disconnect and reconnect RDP, then try the controls below.");

            if ui.button("Click me").clicked() {
                self.clicks += 1;
                log::info!("Click button pressed; clicks={}", self.clicks);
            }

            if ui
                .add(egui::Slider::new(&mut self.slider, 0.0..=1.0).text("slider"))
                .changed()
            {
                log::info!("Slider changed; value={:.3}", self.slider);
            }

            if ui.text_edit_singleline(&mut self.text).changed() {
                log::info!("Text changed; text={:?}", self.text);
            }

            ui.separator();
            ui.label(format!("frame: {}", self.frame_nr));
            ui.label(format!("clicks: {}", self.clicks));
            ui.label(format!(
                "elapsed: {:.1}s",
                self.started_at.elapsed().as_secs_f32()
            ));
            ui.label(format!("log file: {}", self.log_path.display()));
            ui.label(format!("renderer switches: {}", self.renderer_switches));

            if let Some(render_state) = frame.wgpu_render_state() {
                let adapter = render_state.adapter.get_info();
                ui.label(format!(
                    "backend: {:?}; adapter: {}; driver: {} {}",
                    adapter.backend, adapter.name, adapter.driver, adapter.driver_info
                ));
            }

            ui.separator();
            ui.horizontal(|ui| {
                if let Some(texture) = self.static_texture.as_ref() {
                    ui.image((texture.id(), egui::vec2(96.0, 96.0)));
                }
                if let Some(texture) = self.dynamic_texture.as_ref() {
                    ui.image((texture.id(), egui::vec2(96.0, 96.0)));
                }
            });

            let moving_char = (b'A' + (self.frame_nr % 26) as u8) as char;
            ui.label(format!(
                "changing text forces font atlas updates: {moving_char} frame {}",
                self.frame_nr
            ));
        });

        ui.ctx().request_repaint();
    }
}

fn checker_image() -> egui::ColorImage {
    let size = [96, 96];
    let mut image = egui::ColorImage::filled(size, egui::Color32::from_rgb(32, 32, 36));
    for y in 0..size[1] {
        for x in 0..size[0] {
            image.pixels[y * size[0] + x] = if ((x / 12) + (y / 12)) % 2 == 0 {
                egui::Color32::from_rgb(240, 180, 60)
            } else {
                egui::Color32::from_rgb(40, 140, 180)
            };
        }
    }
    image
}

fn dynamic_image(frame_nr: u64) -> egui::ColorImage {
    let size = [96, 96];
    let mut image = egui::ColorImage::filled(size, egui::Color32::BLACK);
    for y in 0..size[1] {
        for x in 0..size[0] {
            let r = ((x as u64 * 3 + frame_nr) % 255) as u8;
            let g = ((y as u64 * 5 + frame_nr * 2) % 255) as u8;
            let b = (((x + y) as u64 * 2 + frame_nr * 3) % 255) as u8;
            image.pixels[y * size[0] + x] = egui::Color32::from_rgb(r, g, b);
        }
    }
    image
}
