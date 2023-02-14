use egui::{
    mutex::{Mutex, MutexGuard},
    TexturesDelta,
};

use crate::{epi, App};

use super::{web_painter::WebPainter, *};

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

/// Stores when to do the next repaint.
pub struct NeedRepaint(Mutex<f64>);

impl Default for NeedRepaint {
    fn default() -> Self {
        Self(Mutex::new(f64::NEG_INFINITY)) // start with a repaint
    }
}

impl NeedRepaint {
    /// Returns the time (in [`now_sec`] scale) when
    /// we should next repaint.
    pub fn when_to_repaint(&self) -> f64 {
        *self.0.lock()
    }

    /// Unschedule repainting.
    pub fn clear(&self) {
        *self.0.lock() = f64::INFINITY;
    }

    pub fn repaint_after(&self, num_seconds: f64) {
        let mut repaint_time = self.0.lock();
        *repaint_time = repaint_time.min(now_sec() + num_seconds);
    }

    pub fn repaint_asap(&self) {
        *self.0.lock() = f64::NEG_INFINITY;
    }
}

pub struct IsDestroyed(std::sync::atomic::AtomicBool);

impl Default for IsDestroyed {
    fn default() -> Self {
        Self(false.into())
    }
}

impl IsDestroyed {
    pub fn fetch(&self) -> bool {
        self.0.load(SeqCst)
    }

    pub fn set_true(&self) {
        self.0.store(true, SeqCst);
    }
}

// ----------------------------------------------------------------------------

fn user_agent() -> Option<String> {
    web_sys::window()?.navigator().user_agent().ok()
}

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
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
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
    painter: ActiveWebPainter,
    pub(crate) input: WebInput,
    app: Box<dyn epi::App>,
    pub(crate) needs_repaint: std::sync::Arc<NeedRepaint>,
    pub(crate) is_destroyed: std::sync::Arc<IsDestroyed>,
    last_save_time: f64,
    screen_reader: super::screen_reader::ScreenReader,
    pub(crate) text_cursor_pos: Option<egui::Pos2>,
    pub(crate) mutable_text_under_cursor: bool,
    textures_delta: TexturesDelta,
    pub events_to_unsubscribe: Vec<EventToUnsubscribe>,
}

impl Drop for AppRunner {
    fn drop(&mut self) {
        tracing::debug!("AppRunner has fully dropped");
    }
}

impl AppRunner {
    /// # Errors
    /// Failure to initialize WebGL renderer.
    pub async fn new(
        canvas_id: &str,
        web_options: crate::WebOptions,
        app_creator: epi::AppCreator,
    ) -> Result<Self, String> {
        let painter = ActiveWebPainter::new(canvas_id, &web_options).await?;

        let system_theme = if web_options.follow_system_theme {
            super::system_theme()
        } else {
            None
        };

        let info = epi::IntegrationInfo {
            web_info: epi::WebInfo {
                user_agent: user_agent().unwrap_or_default(),
                location: web_location(),
            },
            system_theme,
            cpu_usage: None,
            native_pixels_per_point: Some(native_pixels_per_point()),
        };
        let storage = LocalStorage::default();

        let egui_ctx = egui::Context::default();
        egui_ctx.set_os(egui::os::OperatingSystem::from_user_agent(
            &user_agent().unwrap_or_default(),
        ));
        load_memory(&egui_ctx);

        let theme = system_theme.unwrap_or(web_options.default_theme);
        egui_ctx.set_visuals(theme.egui_visuals());

        let app = app_creator(&epi::CreationContext {
            egui_ctx: egui_ctx.clone(),
            integration_info: info.clone(),
            storage: Some(&storage),

            #[cfg(feature = "glow")]
            gl: Some(painter.gl().clone()),

            #[cfg(all(feature = "wgpu", not(feature = "glow")))]
            wgpu_render_state: painter.render_state(),
            #[cfg(all(feature = "wgpu", feature = "glow"))]
            wgpu_render_state: None,
        });

        let frame = epi::Frame {
            info,
            output: Default::default(),
            storage: Some(Box::new(storage)),

            #[cfg(feature = "glow")]
            gl: Some(painter.gl().clone()),

            #[cfg(all(feature = "wgpu", not(feature = "glow")))]
            wgpu_render_state: painter.render_state(),
            #[cfg(all(feature = "wgpu", feature = "glow"))]
            wgpu_render_state: None,
        };

        let needs_repaint: std::sync::Arc<NeedRepaint> = Default::default();
        {
            let needs_repaint = needs_repaint.clone();
            egui_ctx.set_request_repaint_callback(move || {
                needs_repaint.repaint_asap();
            });
        }

        let mut runner = Self {
            frame,
            egui_ctx,
            painter,
            input: Default::default(),
            app,
            needs_repaint,
            is_destroyed: Default::default(),
            last_save_time: now_sec(),
            screen_reader: Default::default(),
            text_cursor_pos: None,
            mutable_text_under_cursor: false,
            textures_delta: Default::default(),
            events_to_unsubscribe: Default::default(),
        };

        runner.input.raw.max_texture_side = Some(runner.painter.max_texture_side());

        Ok(runner)
    }

    pub fn egui_ctx(&self) -> &egui::Context {
        &self.egui_ctx
    }

    /// Get mutable access to the concrete [`App`] we enclose.
    ///
    /// This will panic if your app does not implement [`App::as_any_mut`].
    pub fn app_mut<ConreteApp: 'static + App>(&mut self) -> &mut ConreteApp {
        self.app
            .as_any_mut()
            .expect("Your app must implement `as_any_mut`, but it doesn't")
            .downcast_mut::<ConreteApp>()
            .unwrap()
    }

    pub fn auto_save(&mut self) {
        let now = now_sec();
        let time_since_last_save = now - self.last_save_time;

        if time_since_last_save > self.app.auto_save_interval().as_secs_f64() {
            if self.app.persist_egui_memory() {
                save_memory(&self.egui_ctx);
            }
            if let Some(storage) = self.frame.storage_mut() {
                self.app.save(storage);
            }
            self.last_save_time = now;
        }
    }

    pub fn canvas_id(&self) -> &str {
        self.painter.canvas_id()
    }

    pub fn warm_up(&mut self) -> Result<(), JsValue> {
        if self.app.warm_up_enabled() {
            let saved_memory: egui::Memory = self.egui_ctx.memory(|m| m.clone());
            self.egui_ctx
                .memory_mut(|m| m.set_everything_is_visible(true));
            self.logic()?;
            self.egui_ctx.memory_mut(|m| *m = saved_memory); // We don't want to remember that windows were huge.
            self.egui_ctx.clear_animations();
        }
        Ok(())
    }

    pub fn destroy(&mut self) -> Result<(), JsValue> {
        let is_destroyed_already = self.is_destroyed.fetch();

        if is_destroyed_already {
            tracing::warn!("App was destroyed already");
            Ok(())
        } else {
            tracing::debug!("Destroying");
            for x in self.events_to_unsubscribe.drain(..) {
                x.unsubscribe()?;
            }

            self.painter.destroy();
            self.is_destroyed.set_true();
            Ok(())
        }
    }

    /// Returns how long to wait until the next repaint.
    ///
    /// Call [`Self::paint`] later to paint
    pub fn logic(&mut self) -> Result<(std::time::Duration, Vec<egui::ClippedPrimitive>), JsValue> {
        let frame_start = now_sec();

        resize_canvas_to_screen_size(self.canvas_id(), self.app.max_size_points());
        let canvas_size = canvas_size_in_points(self.canvas_id());
        let raw_input = self.input.new_frame(canvas_size);

        let full_output = self.egui_ctx.run(raw_input, |egui_ctx| {
            self.app.update(egui_ctx, &mut self.frame);
        });
        let egui::FullOutput {
            platform_output,
            repaint_after,
            textures_delta,
            shapes,
        } = full_output;

        self.handle_platform_output(platform_output);
        self.textures_delta.append(textures_delta);
        let clipped_primitives = self.egui_ctx.tessellate(shapes);

        {
            let app_output = self.frame.take_app_output();
            let epi::backend::AppOutput {} = app_output;
        }

        self.frame.info.cpu_usage = Some((now_sec() - frame_start) as f32);
        Ok((repaint_after, clipped_primitives))
    }

    /// Paint the results of the last call to [`Self::logic`].
    pub fn paint(&mut self, clipped_primitives: &[egui::ClippedPrimitive]) -> Result<(), JsValue> {
        let textures_delta = std::mem::take(&mut self.textures_delta);

        self.painter.paint_and_update_textures(
            self.app.clear_color(&self.egui_ctx.style().visuals),
            clipped_primitives,
            self.egui_ctx.pixels_per_point(),
            &textures_delta,
        )?;

        Ok(())
    }

    fn handle_platform_output(&mut self, platform_output: egui::PlatformOutput) {
        if self.egui_ctx.options(|o| o.screen_reader) {
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
            #[cfg(feature = "accesskit")]
                accesskit_update: _, // not currently implemented
        } = platform_output;

        set_cursor_icon(cursor_icon);
        if let Some(open) = open_url {
            super::open_url(&open.url, open.new_tab);
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

// ----------------------------------------------------------------------------

pub type AppRunnerRef = Arc<Mutex<AppRunner>>;

pub struct TargetEvent {
    target: EventTarget,
    event_name: String,
    closure: Closure<dyn FnMut(web_sys::Event)>,
}

pub struct IntervalHandle {
    pub handle: i32,
    pub closure: Closure<dyn FnMut()>,
}

pub enum EventToUnsubscribe {
    TargetEvent(TargetEvent),
    #[allow(dead_code)]
    IntervalHandle(IntervalHandle),
}

impl EventToUnsubscribe {
    pub fn unsubscribe(self) -> Result<(), JsValue> {
        match self {
            EventToUnsubscribe::TargetEvent(handle) => {
                handle.target.remove_event_listener_with_callback(
                    handle.event_name.as_str(),
                    handle.closure.as_ref().unchecked_ref(),
                )?;
                Ok(())
            }
            EventToUnsubscribe::IntervalHandle(handle) => {
                let window = web_sys::window().unwrap();
                window.clear_interval_with_handle(handle.handle);
                Ok(())
            }
        }
    }
}

pub struct AppRunnerContainer {
    pub runner: AppRunnerRef,

    /// Set to `true` if there is a panic.
    /// Used to ignore callbacks after a panic.
    pub panicked: Arc<AtomicBool>,
    pub events: Vec<EventToUnsubscribe>,
}

impl AppRunnerContainer {
    /// Convenience function to reduce boilerplate and ensure that all event handlers
    /// are dealt with in the same way
    pub fn add_event_listener<E: wasm_bindgen::JsCast>(
        &mut self,
        target: &EventTarget,
        event_name: &'static str,
        mut closure: impl FnMut(E, MutexGuard<'_, AppRunner>) + 'static,
    ) -> Result<(), JsValue> {
        // Create a JS closure based on the FnMut provided
        let closure = Closure::wrap({
            // Clone atomics
            let runner_ref = self.runner.clone();
            let panicked = self.panicked.clone();

            Box::new(move |event: web_sys::Event| {
                // Only call the wrapped closure if the egui code has not panicked
                if !panicked.load(Ordering::SeqCst) {
                    // Cast the event to the expected event type
                    let event = event.unchecked_into::<E>();

                    closure(event, runner_ref.lock());
                }
            }) as Box<dyn FnMut(web_sys::Event)>
        });

        // Add the event listener to the target
        target.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;

        let handle = TargetEvent {
            target: target.clone(),
            event_name: event_name.to_owned(),
            closure,
        };

        self.events.push(EventToUnsubscribe::TargetEvent(handle));

        Ok(())
    }
}

// ----------------------------------------------------------------------------

/// Install event listeners to register different input events
/// and start running the given app.
pub async fn start(
    canvas_id: &str,
    web_options: crate::WebOptions,
    app_creator: epi::AppCreator,
) -> Result<AppRunnerRef, JsValue> {
    #[cfg(not(web_sys_unstable_apis))]
    tracing::warn!(
        "eframe compiled without RUSTFLAGS='--cfg=web_sys_unstable_apis'. Copying text won't work."
    );

    let mut runner = AppRunner::new(canvas_id, web_options, app_creator).await?;
    runner.warm_up()?;
    start_runner(runner)
}

/// Install event listeners to register different input events
/// and starts running the given [`AppRunner`].
fn start_runner(app_runner: AppRunner) -> Result<AppRunnerRef, JsValue> {
    let mut runner_container = AppRunnerContainer {
        runner: Arc::new(Mutex::new(app_runner)),
        panicked: Arc::new(AtomicBool::new(false)),
        events: Vec::with_capacity(20),
    };

    super::events::install_canvas_events(&mut runner_container)?;
    super::events::install_document_events(&mut runner_container)?;
    text_agent::install_text_agent(&mut runner_container)?;

    super::events::paint_and_schedule(&runner_container.runner, runner_container.panicked.clone())?;

    // Disable all event handlers on panic
    let previous_hook = std::panic::take_hook();

    runner_container.runner.lock().events_to_unsubscribe = runner_container.events;

    std::panic::set_hook(Box::new(move |panic_info| {
        tracing::info!("egui disabled all event handlers due to panic");
        runner_container.panicked.store(true, SeqCst);

        // Propagate panic info to the previously registered panic hook
        previous_hook(panic_info);
    }));

    Ok(runner_container.runner)
}

// ----------------------------------------------------------------------------

#[derive(Default)]
struct LocalStorage {}

impl epi::Storage for LocalStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        local_storage_get(key)
    }

    fn set_string(&mut self, key: &str, value: String) {
        local_storage_set(key, &value);
    }

    fn flush(&mut self) {}
}
