use crate::*;

pub use egui::{pos2, Color32};

// ----------------------------------------------------------------------------

fn create_painter(canvas_id: &str) -> Result<Box<dyn Painter>, JsValue> {
    // Glow takes precedence:
    #[cfg(all(feature = "glow"))]
    return Ok(Box::new(crate::glow_wrapping::WrappedGlowPainter::new(
        canvas_id,
    )));

    #[cfg(all(feature = "webgl", not(feature = "glow")))]
    if let Ok(webgl2_painter) = webgl2::WebGl2Painter::new(canvas_id) {
        console_log("Using WebGL2 backend");
        Ok(Box::new(webgl2_painter))
    } else {
        console_log("Falling back to WebGL1 backend");
        let webgl1_painter = webgl1::WebGlPainter::new(canvas_id)?;
        Ok(Box::new(webgl1_painter))
    }

    #[cfg(all(not(feature = "webgl"), not(feature = "glow")))]
    compile_error!("Either the 'glow' or 'webgl' feature of egui_web must be enabled!");
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

impl epi::backend::RepaintSignal for NeedRepaint {
    fn request_repaint(&self) {
        self.0.store(true, SeqCst);
    }
}

// ----------------------------------------------------------------------------

pub struct AppRunner {
    frame: epi::Frame,
    egui_ctx: egui::Context,
    painter: Box<dyn Painter>,
    pub(crate) input: WebInput,
    app: Box<dyn epi::App>,
    pub(crate) needs_repaint: std::sync::Arc<NeedRepaint>,
    storage: LocalStorage,
    last_save_time: f64,
    screen_reader: crate::screen_reader::ScreenReader,
    pub(crate) text_cursor_pos: Option<egui::Pos2>,
    pub(crate) mutable_text_under_cursor: bool,
    pending_texture_destructions: Vec<u64>,
}

impl AppRunner {
    pub fn new(canvas_id: &str, app: Box<dyn epi::App>) -> Result<Self, JsValue> {
        let painter = create_painter(canvas_id)?;

        let prefer_dark_mode = crate::prefer_dark_mode();

        let needs_repaint: std::sync::Arc<NeedRepaint> = Default::default();

        let frame = epi::Frame::new(epi::backend::FrameData {
            info: epi::IntegrationInfo {
                name: painter.name(),
                web_info: Some(epi::WebInfo {
                    web_location_hash: location_hash().unwrap_or_default(),
                }),
                prefer_dark_mode,
                cpu_usage: None,
                native_pixels_per_point: Some(native_pixels_per_point()),
            },
            output: Default::default(),
            repaint_signal: needs_repaint.clone(),
        });

        let egui_ctx = egui::Context::default();
        load_memory(&egui_ctx);
        if prefer_dark_mode == Some(true) {
            egui_ctx.set_visuals(egui::Visuals::dark());
        } else {
            egui_ctx.set_visuals(egui::Visuals::light());
        }

        let storage = LocalStorage::default();

        let mut runner = Self {
            frame,
            egui_ctx,
            painter,
            input: Default::default(),
            app,
            needs_repaint,
            storage,
            last_save_time: now_sec(),
            screen_reader: Default::default(),
            text_cursor_pos: None,
            mutable_text_under_cursor: false,
            pending_texture_destructions: Default::default(),
        };

        {
            runner
                .app
                .setup(&runner.egui_ctx, &runner.frame, Some(&runner.storage));
        }

        Ok(runner)
    }

    pub fn egui_ctx(&self) -> &egui::Context {
        &self.egui_ctx
    }

    pub fn auto_save(&mut self) {
        let now = now_sec();
        let time_since_last_save = now - self.last_save_time;

        if time_since_last_save > self.app.auto_save_interval().as_secs_f64() {
            if self.app.persist_egui_memory() {
                save_memory(&self.egui_ctx);
            }
            self.app.save(&mut self.storage);
            self.last_save_time = now;
        }
    }

    pub fn canvas_id(&self) -> &str {
        self.painter.canvas_id()
    }

    pub fn warm_up(&mut self) -> Result<(), JsValue> {
        if self.app.warm_up_enabled() {
            let saved_memory: egui::Memory = self.egui_ctx.memory().clone();
            self.egui_ctx.memory().set_everything_is_visible(true);
            self.logic()?;
            *self.egui_ctx.memory() = saved_memory; // We don't want to remember that windows were huge.
            self.egui_ctx.clear_animations();
        }
        Ok(())
    }

    pub fn logic(&mut self) -> Result<(egui::Output, Vec<egui::ClippedMesh>), JsValue> {
        let frame_start = now_sec();

        resize_canvas_to_screen_size(self.canvas_id(), self.app.max_size_points());
        let canvas_size = canvas_size_in_points(self.canvas_id());
        let raw_input = self.input.new_frame(canvas_size);

        let (egui_output, shapes) = self.egui_ctx.run(raw_input, |egui_ctx| {
            self.app.update(egui_ctx, &self.frame);
        });
        let clipped_meshes = self.egui_ctx.tessellate(shapes);

        self.handle_egui_output(&egui_output);

        {
            let app_output = self.frame.take_app_output();
            let epi::backend::AppOutput {
                quit: _,         // Can't quit a web page
                window_size: _,  // Can't resize a web page
                window_title: _, // TODO: change title of window
                decorated: _,    // Can't toggle decorations
                drag_window: _,  // Can't be dragged
                tex_allocation_data,
            } = app_output;

            for (id, image) in tex_allocation_data.creations {
                self.painter.set_texture(id, image);
            }
            self.pending_texture_destructions = tex_allocation_data.destructions;
        }

        self.frame.lock().info.cpu_usage = Some((now_sec() - frame_start) as f32);
        Ok((egui_output, clipped_meshes))
    }

    pub fn paint(&mut self, clipped_meshes: Vec<egui::ClippedMesh>) -> Result<(), JsValue> {
        self.painter
            .upload_egui_texture(&self.egui_ctx.font_image());
        self.painter.clear(self.app.clear_color());
        self.painter
            .paint_meshes(clipped_meshes, self.egui_ctx.pixels_per_point())?;
        for id in self.pending_texture_destructions.drain(..) {
            self.painter.free_texture(id);
        }
        Ok(())
    }

    fn handle_egui_output(&mut self, output: &egui::Output) {
        if self.egui_ctx.memory().options.screen_reader {
            self.screen_reader.speak(&output.events_description());
        }

        let egui::Output {
            cursor_icon,
            open_url,
            copied_text,
            needs_repaint: _, // handled elsewhere
            events: _,        // already handled
            mutable_text_under_cursor,
            text_cursor_pos,
        } = output;

        set_cursor_icon(*cursor_icon);
        if let Some(open) = open_url {
            crate::open_url(&open.url, open.new_tab);
        }

        #[cfg(web_sys_unstable_apis)]
        if !copied_text.is_empty() {
            set_clipboard_text(copied_text);
        }

        #[cfg(not(web_sys_unstable_apis))]
        let _ = copied_text;

        self.mutable_text_under_cursor = *mutable_text_under_cursor;

        if &self.text_cursor_pos != text_cursor_pos {
            move_text_cursor(text_cursor_pos, self.canvas_id());
            self.text_cursor_pos = *text_cursor_pos;
        }
    }
}

/// Install event listeners to register different input events
/// and start running the given app.
pub fn start(canvas_id: &str, app: Box<dyn epi::App>) -> Result<AppRunnerRef, JsValue> {
    let mut runner = AppRunner::new(canvas_id, app)?;
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
