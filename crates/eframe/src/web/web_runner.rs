use core::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;

use crate::{App, epi};

use super::{
    AppRunner, PanicHandler, WebCanvas, WebHost,
    events::{self, ResizeObserverContext},
    text_agent::TextAgent,
};

/// This is how `eframe` runs your web application
///
/// This is cheap to clone.
///
/// See [the crate level docs](crate) for an example.
#[derive(Clone)]
pub struct WebRunner {
    /// Have we ever panicked?
    panic_handler: PanicHandler,

    /// If we ever panic during running, this `RefCell` is poisoned.
    /// So before we use it, we need to check [`Self::panic_handler`].
    app_runner: Rc<RefCell<Option<AppRunner>>>,

    /// In case of a panic, unsubscribe these.
    /// They have to be in a separate `Rc` so that we don't need to pass them to
    /// the panic handler, since they aren't `Send`.
    events_to_unsubscribe: Rc<RefCell<Vec<EventToUnsubscribe>>>,

    /// Current animation frame in flight.
    frame: Rc<RefCell<Option<AnimationFrameRequest>>>,

    /// Channel back to the host page. `Some` only when running in a worker.
    ///
    /// Kept here (not only in [`AppRunner`]) because `request_animation_frame`
    /// needs it without locking the app runner.
    host: Rc<RefCell<Option<WebHost>>>,

    resize_observer: Rc<RefCell<Option<ResizeObserverContext>>>,
}

impl WebRunner {
    /// Will install a panic handler that will catch and log any panics
    #[expect(clippy::new_without_default)]
    pub fn new() -> Self {
        let panic_handler = PanicHandler::install();

        Self {
            panic_handler,
            app_runner: Rc::new(RefCell::new(None)),
            events_to_unsubscribe: Rc::new(RefCell::new(Default::default())),
            frame: Default::default(),
            host: Default::default(),
            resize_observer: Default::default(),
        }
    }

    /// Create the application, install callbacks, and start running the app.
    ///
    /// # Errors
    /// Failing to initialize graphics, or failure to create app.
    pub async fn start(
        &self,
        canvas: web_sys::HtmlCanvasElement,
        web_options: crate::WebOptions,
        app_creator: epi::AppCreator<'static>,
    ) -> Result<(), JsValue> {
        self.destroy();

        {
            // Make sure the canvas can be given focus.
            // https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/tabindex
            canvas.set_tab_index(0);

            // Don't outline the canvas when it has focus:
            canvas.style().set_property("outline", "none")?;
        }

        {
            // First set up the app runner:
            let text_agent = TextAgent::attach(self, &canvas)?;
            let app_runner = AppRunner::new(
                WebCanvas::Html(canvas.clone()),
                web_options,
                app_creator,
                None, // DOM mode: no host-page channel
                Some(text_agent),
            )
            .await?;
            self.app_runner.replace(Some(app_runner));
        }

        {
            let resize_observer = events::ResizeObserverContext::new(self)?;

            // Properly size the canvas. Will also call `self.request_animation_frame()` (eventually)
            resize_observer.observe(&canvas);

            self.resize_observer.replace(Some(resize_observer));
        }

        events::install_event_handlers(self)?;

        log::info!("event handlers installed.");

        Ok(())
    }

    /// Start an egui app on an `OffscreenCanvas` inside a web worker.
    ///
    /// The canvas must already have been transferred from the main thread
    /// (`canvas.transferControlToOffscreen()`), and the host page must forward
    /// messages — call [`Self::on_worker_message`] from the worker's `onmessage`.
    ///
    /// # Errors
    /// Returns an error if the renderer cannot be initialized.
    pub async fn start_offscreen(
        &self,
        canvas: web_sys::OffscreenCanvas,
        web_options: crate::WebOptions,
        app_creator: epi::AppCreator<'static>,
    ) -> Result<(), JsValue> {
        let canvas = WebCanvas::Offscreen(canvas);
        let host =
            WebHost::new().ok_or_else(|| JsValue::from_str("no postMessage on worker global"))?;
        // Also keep a copy here: `request_animation_frame` needs it, and it must
        // not lock the app runner (it is called while the runner is borrowed).
        self.host.borrow_mut().replace(host.clone());
        // The panic hook was installed by [`Self::new`].
        let app_runner = AppRunner::new(
            canvas,
            web_options,
            app_creator,
            Some(host),
            None, // worker mode: no DOM, so no text agent
        )
        .await?;
        self.app_runner.replace(Some(app_runner));
        Ok(())
    }

    /// Feed one message from the host page into the running app.
    ///
    /// Call this from the worker's `onmessage` handler. Unknown messages are ignored.
    pub fn on_worker_message(&self, message: &JsValue) {
        let Some(mut runner) = self.try_lock() else {
            return;
        };
        let Some(kind) = js_sys::Reflect::get(message, &JsValue::from_str("type"))
            .ok()
            .and_then(|value| value.as_string())
        else {
            return;
        };

        match kind.as_str() {
            "frame" => {
                // The host page answered our `request_frame`.
                let _ = self.frame.borrow_mut().take();
                drop(runner);
                events::paint_and_schedule(self).ok();
            }
            "resize" => {
                let width = number(message, "width").unwrap_or(0.0) as u32;
                let height = number(message, "height").unwrap_or(0.0) as u32;
                if width > 0 && height > 0 {
                    runner.on_offscreen_resize(width, height);
                }
                if let Some(dpr) = number(message, "dpr") {
                    super::set_native_pixels_per_point(dpr as f32);
                }
                if let Some(rect) = rect_of(message) {
                    runner
                        .offscreen_events
                        .apply(crate::web_events::WebEvent::CanvasRect(rect));
                }
                drop(runner);
                self.request_animation_frame().ok();
            }
            "focus" => {
                runner.input.focused = bool_value(message, "focused").unwrap_or(true);
            }
            "visibility" => {
                runner.input.occluded = bool_value(message, "hidden").unwrap_or(false);
            }
            _ => {
                if let Some(event) = parse_host_message(message) {
                    let zoom_factor = runner.egui_ctx().zoom_factor();
                    runner.offscreen_events.set_zoom_factor(zoom_factor);
                    let events = runner.offscreen_events.apply(event);
                    runner.input.raw.events.extend(events);
                    runner.needs_repaint.repaint_asap();
                }
            }
        }
    }

    /// Has there been a panic?
    pub fn has_panicked(&self) -> bool {
        self.panic_handler.has_panicked()
    }

    /// What was the panic message and callstack?
    pub fn panic_summary(&self) -> Option<super::PanicSummary> {
        self.panic_handler.panic_summary()
    }

    fn unsubscribe_from_all_events(&self) {
        let events_to_unsubscribe: Vec<_> =
            core::mem::take(&mut *self.events_to_unsubscribe.borrow_mut());

        if !events_to_unsubscribe.is_empty() {
            log::debug!("Unsubscribing from {} events", events_to_unsubscribe.len());
            for x in events_to_unsubscribe {
                if let Err(err) = x.unsubscribe() {
                    log::warn!(
                        "Failed to unsubscribe from event: {}",
                        super::string_from_js_value(&err)
                    );
                }
            }
        }

        self.resize_observer.replace(None);
    }

    /// Shut down eframe and clean up resources.
    pub fn destroy(&self) {
        self.unsubscribe_from_all_events();

        if let Some(frame) = self.frame.take() {
            frame.cancel(&web_sys::window().unwrap());
        }

        if let Some(runner) = self.app_runner.replace(None) {
            runner.destroy();
        }
    }

    /// Returns `None` if there has been a panic, or if we have been destroyed.
    /// In that case, just return to JS.
    pub(crate) fn try_lock(&self) -> Option<core::cell::RefMut<'_, AppRunner>> {
        if self.panic_handler.has_panicked() {
            // Unsubscribe from all events so that we don't get any more callbacks
            // that will try to access the poisoned runner.
            self.unsubscribe_from_all_events();
            None
        } else {
            let lock = self.app_runner.try_borrow_mut().ok()?;
            core::cell::RefMut::filter_map(lock, |lock| -> Option<&mut AppRunner> { lock.as_mut() })
                .ok()
        }
    }

    /// Get mutable access to the concrete [`App`] we enclose.
    ///
    /// This will panic if your app does not implement [`App::as_any_mut`],
    /// and return `None` if this  runner has panicked.
    pub fn app_mut<ConcreteApp: 'static + App>(
        &self,
    ) -> Option<core::cell::RefMut<'_, ConcreteApp>> {
        self.try_lock()
            .map(|lock| core::cell::RefMut::map(lock, |runner| runner.app_mut::<ConcreteApp>()))
    }

    /// Convenience function to reduce boilerplate and ensure that all event handlers
    /// are dealt with in the same way.
    ///
    /// All events added with this method will automatically be unsubscribed on panic,
    /// or when [`Self::destroy`] is called.
    pub fn add_event_listener<E: wasm_bindgen::JsCast>(
        &self,
        target: &web_sys::EventTarget,
        event_name: &'static str,
        mut closure: impl FnMut(E, &mut AppRunner) + 'static,
    ) -> Result<(), wasm_bindgen::JsValue> {
        let options = web_sys::AddEventListenerOptions::default();
        self.add_event_listener_ex(
            target,
            event_name,
            &options,
            move |event, app_runner, _web_runner| closure(event, app_runner),
        )
    }

    /// Convenience function to reduce boilerplate and ensure that all event handlers
    /// are dealt with in the same way.
    ///
    /// All events added with this method will automatically be unsubscribed on panic,
    /// or when [`Self::destroy`] is called.
    pub fn add_event_listener_ex<E: wasm_bindgen::JsCast>(
        &self,
        target: &web_sys::EventTarget,
        event_name: &'static str,
        options: &web_sys::AddEventListenerOptions,
        mut closure: impl FnMut(E, &mut AppRunner, &Self) + 'static,
    ) -> Result<(), wasm_bindgen::JsValue> {
        let web_runner = self.clone();

        // Create a JS closure based on the FnMut provided
        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            // Only call the wrapped closure if the egui code has not panicked
            if let Some(mut runner_lock) = web_runner.try_lock() {
                // Cast the event to the expected event type
                let event = event.unchecked_into::<E>();
                closure(event, &mut runner_lock, &web_runner);
            }
        }) as Box<dyn FnMut(web_sys::Event)>);

        // Add the event listener to the target
        target.add_event_listener_with_callback_and_add_event_listener_options(
            event_name,
            closure.as_ref().unchecked_ref(),
            options,
        )?;

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

    /// Request an animation frame from the browser in which we can perform a paint.
    ///
    /// It is safe to call `request_animation_frame` multiple times in quick succession,
    /// this function guarantees that only one animation frame is scheduled at a time.
    ///
    /// A hidden browser tab (e.g. a backgrounded tab) does not receive
    /// `requestAnimationFrame` callbacks, which would otherwise stop our paint loop
    /// and prevent `App::update` from running in response to `request_repaint`.
    /// To keep running in that case, we fall back to `setTimeout`, which keeps firing
    /// while hidden (the browser throttles it to roughly once per second).
    pub(crate) fn request_animation_frame(&self) -> Result<(), wasm_bindgen::JsValue> {
        if self.frame.borrow().is_some() {
            // there is already an animation frame in flight
            return Ok(());
        }

        // In a worker there is no `window`. ask the host page for a single frame
        // instead (see `WebRunner::on_worker_message`). Requesting one frame at a
        // time is what applies back-pressure: input messages can never queue up
        // behind a backlog of frame messages.
        let Some(window) = web_sys::window() else {
            self.request_animation_frame_from_host();
            return Ok(());
        };

        let closure = Closure::once({
            let web_runner = self.clone();
            move || {
                // We can paint now, so clear the animation frame.
                // This drops the `closure` and allows another
                // animation frame to be scheduled
                let _ = web_runner.frame.take();
                events::paint_and_schedule(&web_runner)
            }
        });

        let hidden = window.document().is_some_and(|document| document.hidden());

        let request = if hidden {
            // The tab is hidden: `requestAnimationFrame` would not fire, so use a timer instead.
            let id = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                // Browsers clamp background timers to ~1s, so the exact value has little effect
                // while hidden, but keeps us responsive right after the tab is hidden:
                10,
            )?;
            AnimationFrameRequest {
                id,
                kind: AnimationFrameKind::Timeout,
                _closure: Some(closure),
            }
        } else {
            let id = window.request_animation_frame(closure.as_ref().unchecked_ref())?;
            AnimationFrameRequest {
                id,
                kind: AnimationFrameKind::AnimationFrame,
                _closure: Some(closure),
            }
        };

        self.frame.borrow_mut().replace(request);

        Ok(())
    }

    fn request_animation_frame_from_host(&self) {
        let Some(host) = self.host.borrow().clone() else {
            return;
        };
        host.request_frame();
        self.frame.borrow_mut().replace(AnimationFrameRequest {
            id: 0,
            kind: AnimationFrameKind::Host,
            _closure: None,
        });
    }

    /// Cancel any in-flight frame request and schedule a fresh one.
    ///
    /// Called on `visibilitychange` so we switch between `requestAnimationFrame` (visible)
    /// and `setTimeout` (hidden) scheduling. This is necessary because an in-flight
    /// `requestAnimationFrame` is paused while the tab is hidden, which would otherwise
    /// stall the paint loop and stop `App::update` from running while hidden.
    pub(crate) fn reschedule_frame(&self) -> Result<(), wasm_bindgen::JsValue> {
        if let Some(frame) = self.frame.borrow_mut().take()
            && let Some(window) = web_sys::window()
        {
            frame.cancel(&window);
        }
        self.request_animation_frame()
    }
}

// ----------------------------------------------------------------------------

// https://rustwasm.github.io/wasm-bindgen/api/wasm_bindgen/closure/struct.Closure.html#using-fnonce-and-closureonce-with-requestanimationframe
struct AnimationFrameRequest {
    /// Represents the ID of a frame in flight.
    id: i32,

    /// How the frame was scheduled, so we know how to cancel it.
    kind: AnimationFrameKind,

    /// The callback given to `request_animation_frame`, stored here both to prevent it
    /// from being canceled, and from having to `.forget()` it.

    /// `None` for [`AnimationFrameKind::Host`], where the host page owns the callback.
    _closure: Option<Closure<dyn FnMut() -> Result<(), JsValue>>>,
}

impl AnimationFrameRequest {
    /// Cancel the in-flight frame request, using the API matching how it was scheduled.
    fn cancel(&self, window: &web_sys::Window) {
        match self.kind {
            AnimationFrameKind::AnimationFrame => {
                window.cancel_animation_frame(self.id).ok();
            }
            AnimationFrameKind::Timeout => {
                window.clear_timeout_with_handle(self.id);
            }
            // Nothing to cancel: the host page owns the callback.
            AnimationFrameKind::Host => {}
        }
    }
}

/// How an [`AnimationFrameRequest`] was scheduled.
enum AnimationFrameKind {
    /// Scheduled with `requestAnimationFrame` (visible tab).
    AnimationFrame,

    /// Scheduled with `setTimeout` (hidden tab).
    Timeout,

    /// Requested from the host page (worker mode).
    Host,
}

struct TargetEvent {
    target: web_sys::EventTarget,
    event_name: String,
    closure: Closure<dyn FnMut(web_sys::Event)>,
}

#[expect(unused)]
struct IntervalHandle {
    handle: i32,
    closure: Closure<dyn FnMut()>,
}

enum EventToUnsubscribe {
    TargetEvent(TargetEvent),

    #[expect(unused)]
    IntervalHandle(IntervalHandle),
}

impl EventToUnsubscribe {
    pub fn unsubscribe(self) -> Result<(), JsValue> {
        match self {
            Self::TargetEvent(handle) => {
                handle.target.remove_event_listener_with_callback(
                    handle.event_name.as_str(),
                    handle.closure.as_ref().unchecked_ref(),
                )?;
                Ok(())
            }
            Self::IntervalHandle(handle) => {
                let window = web_sys::window().unwrap();
                window.clear_interval_with_handle(handle.handle);
                Ok(())
            }
        }
    }
}

/// Read a numeric field from a host message.
fn number(message: &JsValue, key: &str) -> Option<f64> {
    js_sys::Reflect::get(message, &JsValue::from_str(key))
        .ok()?
        .as_f64()
}

/// Read a boolean field from a host message.
fn bool_value(message: &JsValue, key: &str) -> Option<bool> {
    js_sys::Reflect::get(message, &JsValue::from_str(key))
        .ok()?
        .as_bool()
}

/// Read the `{left, top, right, bottom}` rect of a `resize` message.
fn rect_of(message: &JsValue) -> Option<egui::Rect> {
    let rect = js_sys::Reflect::get(message, &JsValue::from_str("rect")).ok()?;
    Some(egui::Rect::from_min_max(
        egui::pos2(number(&rect, "left")? as f32, number(&rect, "top")? as f32),
        egui::pos2(
            number(&rect, "right")? as f32,
            number(&rect, "bottom")? as f32,
        ),
    ))
}

/// Read a pointer or wheel input message posted by `web_demo/offscreen_host.js`.
///
/// Returns `None` for unknown message kinds.
fn parse_host_message(message: &JsValue) -> Option<crate::web_events::WebEvent> {
    use crate::web_events::{PointerButton, WebEvent};

    fn get(message: &JsValue, key: &str) -> Option<JsValue> {
        let value = js_sys::Reflect::get(message, &JsValue::from_str(key)).ok()?;
        if value.is_undefined() || value.is_null() {
            None
        } else {
            Some(value)
        }
    }

    fn modifiers_of(message: &JsValue) -> egui::Modifiers {
        let alt = bool_value(message, "alt").unwrap_or(false);
        let ctrl = bool_value(message, "ctrl").unwrap_or(false);
        let shift = bool_value(message, "shift").unwrap_or(false);
        let meta = bool_value(message, "meta").unwrap_or(false);
        let is_mac = bool_value(message, "isMac").unwrap_or(false);
        egui::Modifiers {
            alt,
            ctrl,
            shift,
            mac_cmd: is_mac && meta,
            command: if is_mac { meta } else { ctrl },
        }
    }

    let kind = get(message, "type")?.as_string()?;
    match kind.as_str() {
        "pointer" => {
            let client = egui::Vec2::new(
                number(message, "x").unwrap_or(0.0) as f32,
                number(message, "y").unwrap_or(0.0) as f32,
            );
            let modifiers = modifiers_of(message);
            let pointer_kind = get(message, "kind")?.as_string()?;
            match pointer_kind.as_str() {
                "move" => Some(WebEvent::PointerMove { client, modifiers }),
                "down" | "up" => Some(WebEvent::PointerButton {
                    client,
                    button: PointerButton::from_dom(number(message, "button").unwrap_or(0.0) as i16),
                    pressed: pointer_kind == "down",
                    modifiers,
                }),
                "cancel" | "leave" => Some(WebEvent::PointerGone),
                _ => None,
            }
        }
        "wheel" => Some(WebEvent::Wheel {
            delta: egui::Vec2::new(
                number(message, "dx").unwrap_or(0.0) as f32,
                number(message, "dy").unwrap_or(0.0) as f32,
            ),
            dom_delta_mode: number(message, "mode").unwrap_or(0.0) as u8,
            modifiers: modifiers_of(message),
        }),
        _ => None,
    }
}
