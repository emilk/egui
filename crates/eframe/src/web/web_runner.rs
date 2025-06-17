use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::prelude::*;

use crate::{epi, App};

use super::{
    events::{self, ResizeObserverContext},
    text_agent::TextAgent,
    AppRunner, PanicHandler,
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
            let text_agent = TextAgent::attach(self, canvas.get_root_node())?;
            let app_runner =
                AppRunner::new(canvas.clone(), web_options, app_creator, text_agent).await?;
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

        self.resize_observer.replace(None);
    }

    /// Shut down eframe and clean up resources.
    pub fn destroy(&self) {
        self.unsubscribe_from_all_events();

        if let Some(frame) = self.frame.take() {
            let window = web_sys::window().unwrap();
            window.cancel_animation_frame(frame.id).ok();
        }

        if let Some(runner) = self.app_runner.replace(None) {
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
            let lock = self.app_runner.try_borrow_mut().ok()?;
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
    pub(crate) fn request_animation_frame(&self) -> Result<(), wasm_bindgen::JsValue> {
        if self.frame.borrow().is_some() {
            // there is already an animation frame in flight
            return Ok(());
        }

        let window = web_sys::window().unwrap();
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

        let id = window.request_animation_frame(closure.as_ref().unchecked_ref())?;
        self.frame.borrow_mut().replace(AnimationFrameRequest {
            id,
            _closure: closure,
        });

        Ok(())
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
