use crate::*;

pub use egui::{pos2, Color32};

// ----------------------------------------------------------------------------

pub struct WebBackend {
    ctx: egui::CtxRef,
    painter: webgl::Painter,
    previous_frame_time: Option<f32>,
    frame_start: Option<f64>,
}

impl WebBackend {
    pub fn new(canvas_id: &str) -> Result<Self, JsValue> {
        let ctx = egui::CtxRef::default();
        Ok(Self {
            ctx,
            painter: webgl::Painter::new(canvas_id)?,
            previous_frame_time: None,
            frame_start: None,
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

        let (output, shapes) = self.ctx.end_frame();
        let paint_jobs = self.ctx.tessellate(shapes);

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

    pub fn painter_debug_info(&self) -> String {
        self.painter.debug_info()
    }
}

impl epi::TextureAllocator for webgl::Painter {
    fn alloc_srgba_premultiplied(
        &mut self,
        size: (usize, usize),
        srgba_pixels: &[Color32],
    ) -> egui::TextureId {
        let id = self.alloc_user_texture();
        self.set_user_texture(id, size, srgba_pixels);
        id
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

impl epi::RepaintSignal for NeedRepaint {
    fn request_repaint(&self) {
        self.0.store(true, SeqCst);
    }
}

// ----------------------------------------------------------------------------

pub struct AppRunner {
    web_backend: WebBackend,
    pub(crate) input: WebInput,
    app: Box<dyn epi::App>,
    pub(crate) needs_repaint: std::sync::Arc<NeedRepaint>,
    storage: LocalStorage,
    last_save_time: f64,
    #[cfg(feature = "http")]
    http: Arc<http::WebHttp>,
}

impl AppRunner {
    pub fn new(web_backend: WebBackend, mut app: Box<dyn epi::App>) -> Result<Self, JsValue> {
        load_memory(&web_backend.ctx);
        let storage = LocalStorage::default();
        app.load(&storage);
        app.setup(&web_backend.ctx);
        Ok(Self {
            web_backend,
            input: Default::default(),
            app,
            needs_repaint: Default::default(),
            storage,
            last_save_time: now_sec(),
            #[cfg(feature = "http")]
            http: Arc::new(http::WebHttp {}),
        })
    }

    pub fn auto_save(&mut self) {
        let now = now_sec();
        let time_since_last_save = now - self.last_save_time;

        if time_since_last_save > self.app.auto_save_interval().as_secs_f64() {
            save_memory(&self.web_backend.ctx);
            self.app.save(&mut self.storage);
            self.last_save_time = now;
        }
    }

    pub fn canvas_id(&self) -> &str {
        self.web_backend.canvas_id()
    }

    pub fn warm_up(&mut self) -> Result<(), JsValue> {
        if self.app.warm_up_enabled() {
            let saved_memory = self.web_backend.ctx.memory().clone();
            self.web_backend
                .ctx
                .memory()
                .set_everything_is_visible(true);
            self.logic()?;
            *self.web_backend.ctx.memory() = saved_memory; // We don't want to remember that windows were huge.
            self.web_backend.ctx.clear_animations();
        }
        Ok(())
    }

    pub fn logic(&mut self) -> Result<(egui::Output, egui::PaintJobs), JsValue> {
        resize_canvas_to_screen_size(self.web_backend.canvas_id());
        let canvas_size = canvas_size_in_points(self.web_backend.canvas_id());
        let raw_input = self.input.new_frame(canvas_size);
        self.web_backend.begin_frame(raw_input);

        let mut app_output = epi::backend::AppOutput::default();
        let mut frame = epi::backend::FrameBuilder {
            info: epi::IntegrationInfo {
                web_info: Some(epi::WebInfo {
                    web_location_hash: location_hash().unwrap_or_default(),
                }),
                cpu_usage: self.web_backend.previous_frame_time,
                seconds_since_midnight: Some(seconds_since_midnight()),
                native_pixels_per_point: Some(native_pixels_per_point()),
            },
            tex_allocator: Some(&mut self.web_backend.painter),
            #[cfg(feature = "http")]
            http: self.http.clone(),
            output: &mut app_output,
            repaint_signal: self.needs_repaint.clone(),
        }
        .build();

        let egui_ctx = &self.web_backend.ctx;
        self.app.update(egui_ctx, &mut frame);
        let (egui_output, paint_jobs) = self.web_backend.end_frame()?;
        handle_output(&egui_output);

        {
            let epi::backend::AppOutput {
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
/// and start running the given app.
pub fn start(canvas_id: &str, app: Box<dyn epi::App>) -> Result<AppRunnerRef, JsValue> {
    let backend = WebBackend::new(canvas_id)?;
    let mut runner = AppRunner::new(backend, app)?;
    runner.warm_up()?;
    start_runner(runner)
}

/// Install event listeners to register different input events
/// and starts running the given `AppRunner`.
fn start_runner(app_runner: AppRunner) -> Result<AppRunnerRef, JsValue> {
    let runner_ref = AppRunnerRef(Arc::new(Mutex::new(app_runner)));
    install_canvas_events(&runner_ref)?;
    install_document_events(&runner_ref)?;
    repaint_every_ms(&runner_ref, 1000)?; // just in case. TODO: make it a parameter
    paint_and_schedule(runner_ref.clone())?;
    Ok(runner_ref)
}
