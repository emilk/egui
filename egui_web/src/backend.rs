use crate::{glow_wrapping::WrappedGlowPainter, *};

use egui::TexturesDelta;
pub use egui::{pos2, Color32};

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

fn web_location() -> epi::Location {
    let location = web_sys::window().unwrap().location();

    let hash = percent_decode(&location.hash().unwrap_or_default());

    let query = location
        .search()
        .unwrap_or_default()
        .strip_prefix('?')
        .map(percent_decode)
        .unwrap_or_default();

    let query_map = parse_query_map(&query)
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    epi::Location {
        url: percent_decode(&location.href().unwrap_or_default()),
        protocol: percent_decode(&location.protocol().unwrap_or_default()),
        host: percent_decode(&location.host().unwrap_or_default()),
        hostname: percent_decode(&location.hostname().unwrap_or_default()),
        port: percent_decode(&location.port().unwrap_or_default()),
        hash,
        query,
        query_map,
        origin: percent_decode(&location.origin().unwrap_or_default()),
    }
}

fn parse_query_map(query: &str) -> BTreeMap<&str, &str> {
    query
        .split('&')
        .filter_map(|pair| {
            if pair.is_empty() {
                None
            } else {
                Some(if let Some((key, value)) = pair.split_once('=') {
                    (key, value)
                } else {
                    (pair, "")
                })
            }
        })
        .collect()
}

#[test]
fn test_parse_query() {
    assert_eq!(parse_query_map(""), BTreeMap::default());
    assert_eq!(parse_query_map("foo"), BTreeMap::from_iter([("foo", "")]));
    assert_eq!(
        parse_query_map("foo=bar"),
        BTreeMap::from_iter([("foo", "bar")])
    );
    assert_eq!(
        parse_query_map("foo=bar&baz=42"),
        BTreeMap::from_iter([("foo", "bar"), ("baz", "42")])
    );
    assert_eq!(
        parse_query_map("foo&baz=42"),
        BTreeMap::from_iter([("foo", ""), ("baz", "42")])
    );
    assert_eq!(
        parse_query_map("foo&baz&&"),
        BTreeMap::from_iter([("foo", ""), ("baz", "")])
    );
}

// ----------------------------------------------------------------------------

pub struct AppRunner {
    pub(crate) frame: epi::Frame,
    egui_ctx: egui::Context,
    painter: WrappedGlowPainter,
    pub(crate) input: WebInput,
    app: Box<dyn epi::App>,
    pub(crate) needs_repaint: std::sync::Arc<NeedRepaint>,
    storage: LocalStorage,
    last_save_time: f64,
    screen_reader: crate::screen_reader::ScreenReader,
    pub(crate) text_cursor_pos: Option<egui::Pos2>,
    pub(crate) mutable_text_under_cursor: bool,
    textures_delta: TexturesDelta,
}

impl AppRunner {
    pub fn new(canvas_id: &str, app: Box<dyn epi::App>) -> Result<Self, JsValue> {
        let painter = WrappedGlowPainter::new(canvas_id).map_err(JsValue::from)?;

        let prefer_dark_mode = crate::prefer_dark_mode();

        let needs_repaint: std::sync::Arc<NeedRepaint> = Default::default();

        let frame = epi::Frame::new(epi::backend::FrameData {
            info: epi::IntegrationInfo {
                name: "egui_web",
                web_info: Some(epi::WebInfo {
                    location: web_location(),
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
            textures_delta: Default::default(),
        };

        runner.input.raw.max_texture_side = Some(runner.painter.max_texture_side());

        let gl = runner.painter.painter.gl();
        runner
            .app
            .setup(&runner.egui_ctx, &runner.frame, Some(&runner.storage), gl);

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

    /// Returns `true` if egui requests a repaint.
    ///
    /// Call [`Self::paint`] later to paint
    pub fn logic(&mut self) -> Result<(bool, Vec<egui::ClippedPrimitive>), JsValue> {
        let frame_start = now_sec();

        resize_canvas_to_screen_size(self.canvas_id(), self.app.max_size_points());
        let canvas_size = canvas_size_in_points(self.canvas_id());
        let raw_input = self.input.new_frame(canvas_size);

        let full_output = self.egui_ctx.run(raw_input, |egui_ctx| {
            self.app.update(egui_ctx, &self.frame);
        });
        let egui::FullOutput {
            platform_output,
            needs_repaint,
            textures_delta,
            shapes,
        } = full_output;

        self.handle_platform_output(platform_output);
        self.textures_delta.append(textures_delta);
        let clipped_primitives = self.egui_ctx.tessellate(shapes);

        {
            let app_output = self.frame.take_app_output();
            let epi::backend::AppOutput {
                quit: _,         // Can't quit a web page
                window_size: _,  // Can't resize a web page
                window_title: _, // TODO: change title of window
                decorated: _,    // Can't toggle decorations
                drag_window: _,  // Can't be dragged
            } = app_output;
        }

        self.frame.lock().info.cpu_usage = Some((now_sec() - frame_start) as f32);
        Ok((needs_repaint, clipped_primitives))
    }

    /// Paint the results of the last call to [`Self::logic`].
    pub fn paint(&mut self, clipped_primitives: &[egui::ClippedPrimitive]) -> Result<(), JsValue> {
        let textures_delta = std::mem::take(&mut self.textures_delta);

        self.painter.clear(self.app.clear_color());

        self.painter.paint_and_update_textures(
            clipped_primitives,
            self.egui_ctx.pixels_per_point(),
            &textures_delta,
        )?;

        Ok(())
    }

    fn handle_platform_output(&mut self, platform_output: egui::PlatformOutput) {
        if self.egui_ctx.options().screen_reader {
            self.screen_reader
                .speak(&platform_output.events_description());
        }

        let egui::PlatformOutput {
            cursor_icon,
            open_url,
            copied_text,
            events: _, // already handled
            mutable_text_under_cursor,
            text_cursor_pos,
        } = platform_output;

        set_cursor_icon(cursor_icon);
        if let Some(open) = open_url {
            crate::open_url(&open.url, open.new_tab);
        }

        #[cfg(web_sys_unstable_apis)]
        if !copied_text.is_empty() {
            set_clipboard_text(&copied_text);
        }

        #[cfg(not(web_sys_unstable_apis))]
        let _ = copied_text;

        self.mutable_text_under_cursor = mutable_text_under_cursor;

        if self.text_cursor_pos != text_cursor_pos {
            text_agent::move_text_cursor(text_cursor_pos, self.canvas_id());
            self.text_cursor_pos = text_cursor_pos;
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
    let runner_container = AppRunnerContainer {
        runner: Arc::new(Mutex::new(app_runner)),
        panicked: Arc::new(AtomicBool::new(false)),
    };

    install_canvas_events(&runner_container)?;
    install_document_events(&runner_container)?;
    text_agent::install_text_agent(&runner_container)?;
    repaint_every_ms(&runner_container, 1000)?; // just in case. TODO: make it a parameter

    paint_and_schedule(&runner_container.runner, runner_container.panicked.clone())?;

    // Disable all event handlers on panic
    std::panic::set_hook(Box::new({
        let previous_hook = std::panic::take_hook();

        let panicked = runner_container.panicked;

        move |panic_info| {
            tracing::info_span!("egui_panic_handler").in_scope(|| {
                tracing::trace!("setting panicked flag");

                panicked.store(true, SeqCst);

                tracing::info!("egui disabled all event handlers due to panic");
            });

            // Propagate panic info to the previously registered panic hook
            previous_hook(panic_info);
        }
    }));

    Ok(runner_container.runner)
}
