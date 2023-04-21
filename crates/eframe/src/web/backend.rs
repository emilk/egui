use std::{cell::RefCell, rc::Rc};

use egui::{mutex::Mutex, TexturesDelta};

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

    pub fn on_web_page_focus_change(&mut self, focused: bool) {
        self.raw.modifiers = egui::Modifiers::default();
        self.raw.focused = focused;
        self.raw.events.push(egui::Event::WindowFocused(focused));
        self.latest_touch_pos = None;
        self.latest_touch_pos_id = None;
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

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(msg: String);

    type Error;

    #[wasm_bindgen(constructor)]
    fn new() -> Error;

    #[wasm_bindgen(structural, method, getter)]
    fn stack(error: &Error) -> String;
}

#[derive(Clone, Debug)]
pub struct PanicSummary {
    message: String,
    callstack: String,
}

impl PanicSummary {
    pub fn new(info: &std::panic::PanicInfo<'_>) -> Self {
        let message = info.to_string();
        let callstack = Error::new().stack();
        Self { message, callstack }
    }

    pub fn message(&self) -> String {
        self.message.clone()
    }

    pub fn callstack(&self) -> String {
        self.callstack.clone()
    }
}

/// Handle to information about any panic than has occurred
#[derive(Clone, Default)]
pub struct PanicHandler {
    summary: Option<PanicSummary>,
}

impl PanicHandler {
    pub fn has_panicked(&self) -> bool {
        self.summary.is_some()
    }

    pub fn panic_summary(&self) -> Option<PanicSummary> {
        self.summary.clone()
    }

    pub fn on_panic(&mut self, info: &std::panic::PanicInfo<'_>) {
        self.summary = Some(PanicSummary::new(info));
    }
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
}

impl Drop for AppRunner {
    fn drop(&mut self) {
        log::debug!("AppRunner has fully dropped");
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
            egui_ctx.set_request_repaint_callback(move |info| {
                needs_repaint.repaint_after(info.after.as_secs_f64());
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
    pub fn app_mut<ConcreteApp: 'static + App>(&mut self) -> &mut ConcreteApp {
        self.app
            .as_any_mut()
            .expect("Your app must implement `as_any_mut`, but it doesn't")
            .downcast_mut::<ConcreteApp>()
            .unwrap()
    }

    pub fn auto_save_if_needed(&mut self) {
        let time_since_last_save = now_sec() - self.last_save_time;
        if time_since_last_save > self.app.auto_save_interval().as_secs_f64() {
            self.save();
        }
    }

    pub fn save(&mut self) {
        if self.app.persist_egui_memory() {
            save_memory(&self.egui_ctx);
        }
        if let Some(storage) = self.frame.storage_mut() {
            self.app.save(storage);
        }
        self.last_save_time = now_sec();
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

    fn destroy(&mut self) {
        if self.is_destroyed.fetch() {
            log::warn!("App was destroyed already");
        } else {
            log::debug!("Destroying");
            self.painter.destroy();
            self.is_destroyed.set_true();
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

/// This is how we access the [`AppRunner`].
/// This is cheap to clone.
#[derive(Clone)]
pub struct AppRunnerRef {
    /// If we ever panic during running, this mutex is poisoned.
    /// So before we use it, we need to check `panic_handler`.
    runner: Rc<RefCell<AppRunner>>,

    /// Have we ever panicked?
    panic_handler: Arc<Mutex<PanicHandler>>,

    /// In case of a panic, unsubscribe these.
    /// They have to be in a separate `Arc` so that we don't need to pass them to
    /// the panic handler, since they aren't `Send`.
    events_to_unsubscribe: Rc<RefCell<Vec<EventToUnsubscribe>>>,
}

impl AppRunnerRef {
    pub fn new(runner: AppRunner) -> Self {
        Self {
            runner: Rc::new(RefCell::new(runner)),
            panic_handler: Arc::new(Mutex::new(Default::default())),
            events_to_unsubscribe: Rc::new(RefCell::new(Default::default())),
        }
    }

    /// Returns true if there has been a panic.
    fn unsubscribe_if_panicked(&self) {
        if self.panic_handler.lock().has_panicked() {
            // Unsubscribe from all events so that we don't get any more callbacks
            // that will try to access the poisoned runner.
            self.unsubscribe_from_all_events();
        }
    }

    fn unsubscribe_from_all_events(&self) {
        let events_to_unsubscribe: Vec<_> =
            std::mem::take(&mut *self.events_to_unsubscribe.borrow_mut());

        if !events_to_unsubscribe.is_empty() {
            log::debug!("Unsubscribing from {} events", events_to_unsubscribe.len());
            for x in events_to_unsubscribe {
                if let Err(err) = x.unsubscribe() {
                    log::error!("Failed to unsubscribe from event: {err:?}");
                }
            }
        }
    }

    /// Returns true if there has been a panic.
    pub fn has_panicked(&self) -> bool {
        self.unsubscribe_if_panicked();
        self.panic_handler.lock().has_panicked()
    }

    /// Returns `Some` if there has been a panic.
    pub fn panic_summary(&self) -> Option<PanicSummary> {
        self.unsubscribe_if_panicked();
        self.panic_handler.lock().panic_summary()
    }

    pub fn destroy(&self) {
        self.unsubscribe_from_all_events();
        if let Some(mut runner) = self.try_lock() {
            runner.destroy();
        }
    }

    /// Returns `None` if there has been a panic, or if we have been destroyed.
    /// In that case, just return to JS.
    pub fn try_lock(&self) -> Option<std::cell::RefMut<'_, AppRunner>> {
        if self.has_panicked() {
            None
        } else {
            let lock = self.runner.borrow_mut();
            if lock.is_destroyed.fetch() {
                None
            } else {
                Some(lock)
            }
        }
    }

    /// Convenience function to reduce boilerplate and ensure that all event handlers
    /// are dealt with in the same way
    pub fn add_event_listener<E: wasm_bindgen::JsCast>(
        &self,
        target: &EventTarget,
        event_name: &'static str,
        mut closure: impl FnMut(E, &mut AppRunner) + 'static,
    ) -> Result<(), JsValue> {
        let runner_ref = self.clone();

        // Create a JS closure based on the FnMut provided
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            // Only call the wrapped closure if the egui code has not panicked
            if let Some(mut runner_lock) = runner_ref.try_lock() {
                // Cast the event to the expected event type
                let event = event.unchecked_into::<E>();
                closure(event, &mut runner_lock);
            }
        }) as Box<dyn FnMut(web_sys::Event)>);

        // Add the event listener to the target
        target.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;

        let handle = TargetEvent {
            target: target.clone(),
            event_name: event_name.to_owned(),
            closure,
        };

        // Remember it so we unsubscribe on panic.
        // Otherwise we get calls into `self.runner` after it has been poisoned by a panic.
        self.events_to_unsubscribe
            .borrow_mut()
            .push(EventToUnsubscribe::TargetEvent(handle));

        Ok(())
    }
}

// ----------------------------------------------------------------------------

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

// ----------------------------------------------------------------------------

/// Install event listeners to register different input events
/// and start running the given app.
///
/// ``` no_run
/// #[cfg(target_arch = "wasm32")]
/// use wasm_bindgen::prelude::*;
///
/// /// This is the entry-point for all the web-assembly.
/// /// This is called from the HTML.
/// /// It loads the app, installs some callbacks, then returns.
/// /// It returns a handle to the running app that can be stopped calling `AppRunner::stop_web`.
/// /// You can add more callbacks like this if you want to call in to your code.
/// #[cfg(target_arch = "wasm32")]
/// #[wasm_bindgen]
/// pub struct WebHandle {
///     handle: AppRunnerRef,
/// }
/// #[cfg(target_arch = "wasm32")]
/// #[wasm_bindgen]
/// pub async fn start(canvas_id: &str) -> Result<WebHandle, eframe::wasm_bindgen::JsValue> {
///     let web_options = eframe::WebOptions::default();
///     eframe::start_web(
///         canvas_id,
///         web_options,
///         Box::new(|cc| Box::new(MyEguiApp::new(cc))),
///     )
///     .await
///     .map(|handle| WebHandle { handle })
/// }
/// ```
///
/// # Errors
/// Failing to initialize WebGL graphics.
pub async fn start_web(
    canvas_id: &str,
    web_options: crate::WebOptions,
    app_creator: epi::AppCreator,
) -> Result<AppRunnerRef, JsValue> {
    #[cfg(not(web_sys_unstable_apis))]
    log::warn!(
        "eframe compiled without RUSTFLAGS='--cfg=web_sys_unstable_apis'. Copying text won't work."
    );
    let follow_system_theme = web_options.follow_system_theme;

    let mut runner = AppRunner::new(canvas_id, web_options, app_creator).await?;
    runner.warm_up()?;
    let runner_ref = AppRunnerRef::new(runner);

    // Install events:
    {
        super::events::install_canvas_events(&runner_ref)?;
        super::events::install_document_events(&runner_ref)?;
        super::events::install_window_events(&runner_ref)?;
        text_agent::install_text_agent(&runner_ref)?;
        if follow_system_theme {
            super::events::install_color_scheme_change_event(&runner_ref)?;
        }
        super::events::paint_and_schedule(&runner_ref)?;
    }

    // Instal panic handler:
    {
        // Disable all event handlers on panic
        let previous_hook = std::panic::take_hook();
        let panic_handler = runner_ref.panic_handler.clone();

        std::panic::set_hook(Box::new(move |panic_info| {
            log::info!("eframe detected a panic");
            panic_handler.lock().on_panic(panic_info);

            // Propagate panic info to the previously registered panic hook
            previous_hook(panic_info);
        }));
    }

    Ok(runner_ref)
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
