use crate::*;

pub use egui::{pos2, Color32};

// ----------------------------------------------------------------------------

pub struct WebBackend {
    egui_ctx: egui::CtxRef,
    painter: Box<dyn Painter>,
    previous_frame_time: Option<f32>,
    frame_start: Option<f64>,
}

impl WebBackend {
    pub fn new(canvas_id: &str) -> Result<Self, JsValue> {
        let ctx = egui::CtxRef::default();

        let painter: Box<dyn Painter> =
            if let Ok(webgl2_painter) = webgl2::WebGl2Painter::new(canvas_id) {
                console_log("Using WebGL2 backend");
                Box::new(webgl2_painter)
            } else {
                console_log("Falling back to WebGL1 backend");
                Box::new(webgl1::WebGlPainter::new(canvas_id)?)
            };

        Ok(Self {
            egui_ctx: ctx,
            painter,
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
        self.egui_ctx.begin_frame(raw_input)
    }

    pub fn end_frame(&mut self) -> Result<(egui::Output, Vec<egui::ClippedMesh>), JsValue> {
        let frame_start = self
            .frame_start
            .take()
            .expect("unmatched calls to begin_frame/end_frame");

        let (output, shapes) = self.egui_ctx.end_frame();
        let clipped_meshes = self.egui_ctx.tessellate(shapes);

        let now = now_sec();
        self.previous_frame_time = Some((now - frame_start) as f32);

        Ok((output, clipped_meshes))
    }

    pub fn paint(
        &mut self,
        clear_color: egui::Rgba,
        clipped_meshes: Vec<egui::ClippedMesh>,
    ) -> Result<(), JsValue> {
        self.painter.upload_egui_texture(&self.egui_ctx.texture());
        self.painter.clear(clear_color);
        self.painter
            .paint_meshes(clipped_meshes, self.egui_ctx.pixels_per_point())
    }

    pub fn painter_debug_info(&self) -> String {
        self.painter.debug_info()
    }
}

// ----------------------------------------------------------------------------

/// Data gathered between frames.
#[derive(Default)]
pub struct WebInput {
    /// Required because we don't get a position on touched
    pub latest_touch_pos: Option<egui::Pos2>,

    /// Required to maintain a stable touch position for multi-touch gestures.
    pub latest_touch_pos_id: Option<egui::TouchId>,

    pub raw: egui::RawInput,
}

impl WebInput {
    pub fn new_frame(&mut self, canvas_size: egui::Vec2) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Default::default(), canvas_size)),
            pixels_per_point: Some(native_pixels_per_point()), // We ALWAYS use the native pixels-per-point
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
    prefer_dark_mode: Option<bool>,
    last_save_time: f64,
    screen_reader: crate::screen_reader::ScreenReader,
    pub(crate) last_text_cursor_pos: Option<egui::Pos2>,
}

impl AppRunner {
    pub fn new(web_backend: WebBackend, app: Box<dyn epi::App>) -> Result<Self, JsValue> {
        load_memory(&web_backend.egui_ctx);

        let prefer_dark_mode = crate::prefer_dark_mode();

        if prefer_dark_mode == Some(true) {
            web_backend.egui_ctx.set_visuals(egui::Visuals::dark());
        } else {
            web_backend.egui_ctx.set_visuals(egui::Visuals::light());
        }

        let storage = LocalStorage::default();

        let mut runner = Self {
            web_backend,
            input: Default::default(),
            app,
            needs_repaint: Default::default(),
            storage,
            prefer_dark_mode,
            last_save_time: now_sec(),
            screen_reader: Default::default(),
            last_text_cursor_pos: None,
        };

        {
            let mut app_output = epi::backend::AppOutput::default();
            let mut frame = epi::backend::FrameBuilder {
                info: runner.integration_info(),
                tex_allocator: runner.web_backend.painter.as_tex_allocator(),
                output: &mut app_output,
                repaint_signal: runner.needs_repaint.clone(),
            }
            .build();
            runner.app.setup(
                &runner.web_backend.egui_ctx,
                &mut frame,
                Some(&runner.storage),
            );
        }

        Ok(runner)
    }

    pub fn egui_ctx(&self) -> &egui::CtxRef {
        &self.web_backend.egui_ctx
    }

    pub fn auto_save(&mut self) {
        let now = now_sec();
        let time_since_last_save = now - self.last_save_time;

        if time_since_last_save > self.app.auto_save_interval().as_secs_f64() {
            if self.app.persist_egui_memory() {
                save_memory(&self.web_backend.egui_ctx);
            }
            self.app.save(&mut self.storage);
            self.last_save_time = now;
        }
    }

    pub fn canvas_id(&self) -> &str {
        self.web_backend.canvas_id()
    }

    pub fn warm_up(&mut self) -> Result<(), JsValue> {
        if self.app.warm_up_enabled() {
            let saved_memory = self.web_backend.egui_ctx.memory().clone();
            self.web_backend
                .egui_ctx
                .memory()
                .set_everything_is_visible(true);
            self.logic()?;
            *self.web_backend.egui_ctx.memory() = saved_memory; // We don't want to remember that windows were huge.
            self.web_backend.egui_ctx.clear_animations();
        }
        Ok(())
    }

    fn integration_info(&self) -> epi::IntegrationInfo {
        epi::IntegrationInfo {
            web_info: Some(epi::WebInfo {
                web_location_hash: location_hash().unwrap_or_default(),
            }),
            prefer_dark_mode: self.prefer_dark_mode,
            cpu_usage: self.web_backend.previous_frame_time,
            seconds_since_midnight: Some(seconds_since_midnight()),
            native_pixels_per_point: Some(native_pixels_per_point()),
        }
    }

    pub fn logic(&mut self) -> Result<(egui::Output, Vec<egui::ClippedMesh>), JsValue> {
        resize_canvas_to_screen_size(self.web_backend.canvas_id(), self.app.max_size_points());
        let canvas_size = canvas_size_in_points(self.web_backend.canvas_id());
        let raw_input = self.input.new_frame(canvas_size);

        self.web_backend.begin_frame(raw_input);

        let mut app_output = epi::backend::AppOutput::default();
        let mut frame = epi::backend::FrameBuilder {
            info: self.integration_info(),
            tex_allocator: self.web_backend.painter.as_tex_allocator(),
            output: &mut app_output,
            repaint_signal: self.needs_repaint.clone(),
        }
        .build();

        self.app.update(&self.web_backend.egui_ctx, &mut frame);
        let (egui_output, clipped_meshes) = self.web_backend.end_frame()?;

        if self.web_backend.egui_ctx.memory().options.screen_reader {
            self.screen_reader.speak(&egui_output.events_description());
        }
        handle_output(&egui_output, self);

        {
            let epi::backend::AppOutput {
                quit: _,        // Can't quit a web page
                window_size: _, // Can't resize a web page
                decorated: _,   // Can't show decorations
                drag_window: _, // Can't be dragged
            } = app_output;
        }

        Ok((egui_output, clipped_meshes))
    }

    pub fn paint(&mut self, clipped_meshes: Vec<egui::ClippedMesh>) -> Result<(), JsValue> {
        self.web_backend
            .paint(self.app.clear_color(), clipped_meshes)
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
    install_text_agent(&runner_ref)?;
    repaint_every_ms(&runner_ref, 1000)?; // just in case. TODO: make it a parameter
    paint_and_schedule(runner_ref.clone())?;
    Ok(runner_ref)
}
