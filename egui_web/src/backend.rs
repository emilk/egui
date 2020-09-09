use crate::*;

pub use egui::app::{App, Backend, RunMode, WebInfo};

// ----------------------------------------------------------------------------

pub struct WebBackend {
    ctx: Arc<egui::Context>,
    painter: webgl::Painter,
    frame_times: egui::MovementTracker<f32>,
    frame_start: Option<f64>,
    run_mode: RunMode,
    last_save_time: Option<f64>,
}

impl WebBackend {
    pub fn new(canvas_id: &str, run_mode: RunMode) -> Result<Self, JsValue> {
        let ctx = egui::Context::new();
        load_memory(&ctx);
        Ok(Self {
            ctx,
            painter: webgl::Painter::new(canvas_id)?,
            frame_times: egui::MovementTracker::new(1000, 1.0),
            frame_start: None,
            run_mode,
            last_save_time: None,
        })
    }

    /// id of the canvas html element containing the rendering
    pub fn canvas_id(&self) -> &str {
        self.painter.canvas_id()
    }

    /// Returns a master fullscreen UI, covering the entire screen.
    pub fn begin_frame(&mut self, raw_input: egui::RawInput) -> egui::Ui {
        self.frame_start = Some(now_sec());
        self.ctx.begin_frame(raw_input)
    }

    pub fn end_frame(&mut self) -> Result<(egui::Output, egui::PaintJobs), JsValue> {
        let frame_start = self
            .frame_start
            .take()
            .expect("unmatched calls to begin_frame/end_frame");

        let (output, paint_jobs) = self.ctx.end_frame();

        self.auto_save();

        let now = now_sec();
        self.frame_times.add(now, (now - frame_start) as f32);

        Ok((output, paint_jobs))
    }

    pub fn paint(&mut self, paint_jobs: egui::PaintJobs) -> Result<(), JsValue> {
        let bg_color = egui::color::TRANSPARENT; // Use background css color.
        self.painter.paint_jobs(
            bg_color,
            paint_jobs,
            &self.ctx.texture(),
            self.ctx.pixels_per_point(),
        )
    }

    pub fn auto_save(&mut self) {
        let now = now_sec();
        let time_since_last_save = now - self.last_save_time.unwrap_or(std::f64::NEG_INFINITY);
        const AUTO_SAVE_INTERVAL: f64 = 5.0;
        if time_since_last_save > AUTO_SAVE_INTERVAL {
            self.last_save_time = Some(now);
            save_memory(&self.ctx);
        }
    }

    pub fn painter_debug_info(&self) -> String {
        self.painter.debug_info()
    }
}

impl Backend for WebBackend {
    fn run_mode(&self) -> RunMode {
        self.run_mode
    }

    fn set_run_mode(&mut self, run_mode: RunMode) {
        self.run_mode = run_mode;
    }

    fn web_info(&self) -> Option<WebInfo> {
        Some(WebInfo {
            web_location_hash: location_hash().unwrap_or_default(),
        })
    }

    /// excludes painting
    fn cpu_time(&self) -> f32 {
        self.frame_times.average().unwrap_or_default()
    }

    fn fps(&self) -> f32 {
        1.0 / self.frame_times.mean_time_interval().unwrap_or_default()
    }
}

// ----------------------------------------------------------------------------

// TODO: Just use RawInput?
/// Data gathered between frames.
/// Is translated to `egui::RawInput` at the start of each frame.
#[derive(Default)]
pub struct WebInput {
    pub mouse_pos: Option<egui::Pos2>,
    pub mouse_down: bool, // TODO: which button
    pub is_touch: bool,
    pub scroll_delta: egui::Vec2,
    pub events: Vec<egui::Event>,
}

impl WebInput {
    pub fn new_frame(&mut self) -> egui::RawInput {
        egui::RawInput {
            mouse_down: self.mouse_down,
            mouse_pos: self.mouse_pos,
            scroll_delta: std::mem::take(&mut self.scroll_delta),
            screen_size: screen_size().unwrap(),
            pixels_per_point: Some(pixels_per_point()),
            time: now_sec(),
            seconds_since_midnight: Some(seconds_since_midnight()),
            events: std::mem::take(&mut self.events),
        }
    }
}

// ----------------------------------------------------------------------------

pub struct AppRunner {
    pub web_backend: WebBackend,
    pub web_input: WebInput,
    pub app: Box<dyn App>,
    pub needs_repaint: bool, // TODO: move
}

impl AppRunner {
    pub fn new(web_backend: WebBackend, app: Box<dyn App>) -> Result<Self, JsValue> {
        Ok(Self {
            web_backend,
            web_input: Default::default(),
            app,
            needs_repaint: true, // TODO: move
        })
    }

    pub fn canvas_id(&self) -> &str {
        self.web_backend.canvas_id()
    }

    pub fn logic(&mut self) -> Result<(egui::Output, egui::PaintJobs), JsValue> {
        resize_to_screen_size(self.web_backend.canvas_id());

        let raw_input = self.web_input.new_frame();

        let mut ui = self.web_backend.begin_frame(raw_input);
        self.app.ui(&mut ui, &mut self.web_backend);
        let (output, paint_jobs) = self.web_backend.end_frame()?;
        handle_output(&output);
        Ok((output, paint_jobs))
    }

    pub fn paint(&mut self, paint_jobs: egui::PaintJobs) -> Result<(), JsValue> {
        self.web_backend.paint(paint_jobs)
    }
}

/// Install event listeners to register different input events
/// and starts running the given `AppRunner`.
pub fn run(app_runner: AppRunner) -> Result<AppRunnerRef, JsValue> {
    let runner_ref = AppRunnerRef(Arc::new(Mutex::new(app_runner)));
    install_canvas_events(&runner_ref)?;
    install_document_events(&runner_ref)?;
    paint_and_schedule(runner_ref.clone())?;
    Ok(runner_ref)
}
