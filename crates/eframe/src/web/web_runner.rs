use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::prelude::*;

use crate::{epi, App};

use super::{events, text_agent::TextAgent, AppRunner, PanicHandler};

/// This is how `eframe` runs your wepp application
///
/// This is cheap to clone.
///
/// See [the crate level docs](crate) for an example.
#[derive(Clone)]
pub struct WebRunner {
    /// Have we ever panicked?
    panic_handler: PanicHandler,

    /// If we ever panic during running, this RefCell is poisoned.
    /// So before we use it, we need to check [`Self::panic_handler`].
    runner: Rc<RefCell<Option<AppRunner>>>,

    /// In case of a panic, unsubscribe these.
    /// They have to be in a separate `Rc` so that we don't need to pass them to
    /// the panic handler, since they aren't `Send`.
    events_to_unsubscribe: Rc<RefCell<Vec<EventToUnsubscribe>>>,

    /// Current animation frame in flight.
    frame: Rc<RefCell<Option<AnimationFrameRequest>>>,

    resize_observer: Rc<RefCell<Option<ResizeObserverContext>>>,
}

impl WebRunner {
    /// Will install a panic handler that will catch and log any panics
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        #[cfg(not(web_sys_unstable_apis))]
        log::warn!(
            "eframe compiled without RUSTFLAGS='--cfg=web_sys_unstable_apis'. Copying text won't work."
        );

        let panic_handler = PanicHandler::install();

        Self {
            panic_handler,
            runner: Rc::new(RefCell::new(None)),
            events_to_unsubscribe: Rc::new(RefCell::new(Default::default())),
            frame: Default::default(),
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
        app_creator: epi::AppCreator,
    ) -> Result<(), JsValue> {
        self.destroy();

        let text_agent = TextAgent::attach(self)?;

        let runner = AppRunner::new(canvas, web_options, app_creator, text_agent).await?;

        {
            // Make sure the canvas can be given focus.
            // https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/tabindex
            runner.canvas().set_tab_index(0);

            // Don't outline the canvas when it has focus:
            runner.canvas().style().set_property("outline", "none")?;
        }

        self.runner.replace(Some(runner));

        {
            events::install_event_handlers(self)?;

            // The resize observer handles calling `request_animation_frame` to start the render loop.
            events::install_resize_observer(self)?;
        }

        Ok(())
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
            std::mem::take(&mut *self.events_to_unsubscribe.borrow_mut());

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

        if let Some(context) = self.resize_observer.take() {
            context.resize_observer.disconnect();
            drop(context.closure);
        }
    }

    /// Shut down eframe and clean up resources.
    pub fn destroy(&self) {
        self.unsubscribe_from_all_events();

        if let Some(frame) = self.frame.take() {
            let window = web_sys::window().unwrap();
            window.cancel_animation_frame(frame.id).ok();
        }

        if let Some(runner) = self.runner.replace(None) {
            runner.destroy();
        }
    }

    /// Returns `None` if there has been a panic, or if we have been destroyed.
    /// In that case, just return to JS.
    pub(crate) fn try_lock(&self) -> Option<std::cell::RefMut<'_, AppRunner>> {
        if self.panic_handler.has_panicked() {
            // Unsubscribe from all events so that we don't get any more callbacks
            // that will try to access the poisoned runner.
            self.unsubscribe_from_all_events();
            None
        } else {
            let lock = self.runner.try_borrow_mut().ok()?;
            std::cell::RefMut::filter_map(lock, |lock| -> Option<&mut AppRunner> { lock.as_mut() })
                .ok()
        }
    }

    /// Get mutable access to the concrete [`App`] we enclose.
    ///
    /// This will panic if your app does not implement [`App::as_any_mut`],
    /// and return `None` if this  runner has panicked.
    pub fn app_mut<ConcreteApp: 'static + App>(
        &self,
    ) -> Option<std::cell::RefMut<'_, ConcreteApp>> {
        self.try_lock()
            .map(|lock| std::cell::RefMut::map(lock, |runner| runner.app_mut::<ConcreteApp>()))
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

    /// Request an animation frame from the browser in which we can perform a paint.
    ///
    /// It is safe to call `request_animation_frame` multiple times in quick succession,
    /// this function guarantees that only one animation frame is scheduled at a time.
    pub(crate) fn request_animation_frame(&self) -> Result<(), wasm_bindgen::JsValue> {
        if self.frame.borrow().is_some() {
            // there is already an animation frame in flight
            return Ok(());
        }

        let window = web_sys::window().unwrap();
        let closure = Closure::once({
            let runner_ref = self.clone();
            move || {
                // We can paint now, so clear the animation frame.
                // This drops the `closure` and allows another
                // animation frame to be scheduled
                let _ = runner_ref.frame.take();
                events::paint_and_schedule(&runner_ref)
            }
        });

        let id = window.request_animation_frame(closure.as_ref().unchecked_ref())?;
        self.frame.borrow_mut().replace(AnimationFrameRequest {
            id,
            _closure: closure,
        });

        Ok(())
    }

    pub(crate) fn set_resize_observer(
        &self,
        resize_observer: web_sys::ResizeObserver,
        closure: Closure<dyn FnMut(js_sys::Array)>,
    ) {
        self.resize_observer
            .borrow_mut()
            .replace(ResizeObserverContext {
                resize_observer,
                closure,
            });
    }
}

// ----------------------------------------------------------------------------

// https://rustwasm.github.io/wasm-bindgen/api/wasm_bindgen/closure/struct.Closure.html#using-fnonce-and-closureonce-with-requestanimationframe
struct AnimationFrameRequest {
    /// Represents the ID of a frame in flight.
    id: i32,

    /// The callback given to `request_animation_frame`, stored here both to prevent it
    /// from being canceled, and from having to `.forget()` it.
    _closure: Closure<dyn FnMut() -> Result<(), JsValue>>,
}

struct ResizeObserverContext {
    resize_observer: web_sys::ResizeObserver,
    closure: Closure<dyn FnMut(js_sys::Array)>,
}

struct TargetEvent {
    target: web_sys::EventTarget,
    event_name: String,
    closure: Closure<dyn FnMut(web_sys::Event)>,
}

#[allow(unused)]
struct IntervalHandle {
    handle: i32,
    closure: Closure<dyn FnMut()>,
}

enum EventToUnsubscribe {
    TargetEvent(TargetEvent),

    #[allow(unused)]
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
