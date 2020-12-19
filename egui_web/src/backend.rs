use crate::*;

pub use egui::{
    app::{App, WebInfo},
    pos2, Srgba,
};

// ----------------------------------------------------------------------------

pub struct WebBackend {
    ctx: egui::CtxRef,
    painter: webgl::Painter,
    previous_frame_time: Option<f32>,
    frame_start: Option<f64>,
    last_save_time: Option<f64>,
}

impl WebBackend {
    pub fn new(canvas_id: &str) -> Result<Self, JsValue> {
        let ctx = egui::CtxRef::default();
        load_memory(&ctx);
        Ok(Self {
            ctx,
            painter: webgl::Painter::new(canvas_id)?,
            previous_frame_time: None,
            frame_start: None,
            last_save_time: None,
        })
    }

    /// id of the canvas html element containing the rendering
    pub fn canvas_id(&self) -> &str {
        self.painter.canvas_id()
    }

    pub fn begin_frame(&mut self, raw_input: egui::RawInput) {
        self.frame_start = Some(now_sec());
        self.ctx.begin_frame(raw_input)
    }

    pub fn end_frame(&mut self) -> Result<(egui::Output, egui::PaintJobs), JsValue> {
        let frame_start = self
            .frame_start
            .take()
            .expect("unmatched calls to begin_frame/end_frame");

        let (output, paint_commands) = self.ctx.end_frame();
        let paint_jobs = self.ctx.tesselate(paint_commands);

        self.auto_save();

        let now = now_sec();
        self.previous_frame_time = Some((now - frame_start) as f32);

        Ok((output, paint_jobs))
    }

    pub fn paint(
        &mut self,
        clear_color: egui::Rgba,
        paint_jobs: egui::PaintJobs,
    ) -> Result<(), JsValue> {
        self.painter.paint_jobs(
            clear_color,
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

impl egui::app::TextureAllocator for webgl::Painter {
    fn alloc(&mut self) -> egui::TextureId {
        self.alloc_user_texture()
    }

    fn set_srgba_premultiplied(
        &mut self,
        id: egui::TextureId,
        size: (usize, usize),
        srgba_pixels: &[Srgba],
    ) {
        self.set_user_texture(id, size, srgba_pixels);
    }

    fn free(&mut self, id: egui::TextureId) {
        self.free_user_texture(id)
    }
}

// ----------------------------------------------------------------------------

/// Data gathered between frames.
#[derive(Default)]
pub struct WebInput {
    /// Is this a touch screen? If so, we ignore mouse events.
    pub is_touch: bool,

    pub raw: egui::RawInput,
}

impl WebInput {
    pub fn new_frame(&mut self, canvas_size: egui::Vec2) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Default::default(), canvas_size)),
            pixels_per_point: Some(native_pixels_per_point()),
            time: Some(now_sec()),
            ..self.raw.take()
        }
    }
}

// ----------------------------------------------------------------------------

use std::sync::atomic::Ordering::SeqCst;

pub struct NeedRepaint(std::sync::atomic::AtomicBool);

impl Default for NeedRepaint {
    fn default() -> Self {
        Self(true.into())
    }
}

impl NeedRepaint {
    pub fn fetch_and_clear(&self) -> bool {
        self.0.swap(false, SeqCst)
    }

    pub fn set_true(&self) {
        self.0.store(true, SeqCst);
    }
}

impl egui::app::RepaintSignal for NeedRepaint {
    fn request_repaint(&self) {
        self.0.store(true, SeqCst);
    }
}

// ----------------------------------------------------------------------------

pub struct AppRunner {
    pub web_backend: WebBackend,
    pub input: WebInput,
    pub app: Box<dyn App>,
    pub needs_repaint: std::sync::Arc<NeedRepaint>,
}

impl AppRunner {
    pub fn new(web_backend: WebBackend, mut app: Box<dyn App>) -> Result<Self, JsValue> {
        app.setup(&web_backend.ctx);
        Ok(Self {
            web_backend,
            input: Default::default(),
            app,
            needs_repaint: Default::default(),
        })
    }

    pub fn canvas_id(&self) -> &str {
        self.web_backend.canvas_id()
    }

    pub fn logic(&mut self) -> Result<(egui::Output, egui::PaintJobs), JsValue> {
        resize_canvas_to_screen_size(self.web_backend.canvas_id());
        let canvas_size = canvas_size_in_points(self.web_backend.canvas_id());
        let raw_input = self.input.new_frame(canvas_size);
        self.web_backend.begin_frame(raw_input);

        let mut integration_context = egui::app::IntegrationContext {
            info: egui::app::IntegrationInfo {
                web_info: Some(WebInfo {
                    web_location_hash: location_hash().unwrap_or_default(),
                }),
                cpu_usage: self.web_backend.previous_frame_time,
                seconds_since_midnight: Some(seconds_since_midnight()),
                native_pixels_per_point: Some(native_pixels_per_point()),
            },
            tex_allocator: Some(&mut self.web_backend.painter),
            output: Default::default(),
            repaint_signal: self.needs_repaint.clone(),
        };

        let egui_ctx = &self.web_backend.ctx;
        self.app.ui(egui_ctx, &mut integration_context);
        let app_output = integration_context.output;
        let (egui_output, paint_jobs) = self.web_backend.end_frame()?;
        handle_output(&egui_output);

        {
            let egui::app::AppOutput {
                quit: _,             // Can't quit a web page
                window_size: _,      // Can't resize a web page
                pixels_per_point: _, // Can't zoom from within the app (we respect the web browser's zoom level)
            } = app_output;
        }

        Ok((egui_output, paint_jobs))
    }

    pub fn paint(&mut self, paint_jobs: egui::PaintJobs) -> Result<(), JsValue> {
        self.web_backend.paint(self.app.clear_color(), paint_jobs)
    }
}

/// Install event listeners to register different input events
/// and starts running the given `AppRunner`.
pub fn start(app_runner: AppRunner) -> Result<AppRunnerRef, JsValue> {
    let runner_ref = AppRunnerRef(Arc::new(Mutex::new(app_runner)));
    install_canvas_events(&runner_ref)?;
    install_document_events(&runner_ref)?;
    repaint_every_ms(&runner_ref, 1000)?; // just in case. TODO: make it a parameter
    paint_and_schedule(runner_ref.clone())?;
    Ok(runner_ref)
}
