//! [`egui`] bindings for web apps (compiling to WASM).
//!
//! This library is an [`epi`] backend.
//!
//! If you are writing an app, you may want to look at [`eframe`](https://docs.rs/eframe) instead.

#![allow(clippy::missing_errors_doc)] // So many `-> Result<_, JsValue>`

pub mod backend;
mod glow_wrapping;
mod input;
pub mod screen_reader;
mod text_agent;

pub use backend::*;

use egui::mutex::{Mutex, MutexGuard};
pub use wasm_bindgen;
pub use web_sys;

use input::*;
use web_sys::EventTarget;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

// ----------------------------------------------------------------------------

/// Current time in seconds (since undefined point in time)
pub fn now_sec() -> f64 {
    web_sys::window()
        .expect("should have a Window")
        .performance()
        .expect("should have a Performance")
        .now()
        / 1000.0
}

pub fn screen_size_in_native_points() -> Option<egui::Vec2> {
    let window = web_sys::window()?;
    Some(egui::vec2(
        window.inner_width().ok()?.as_f64()? as f32,
        window.inner_height().ok()?.as_f64()? as f32,
    ))
}

pub fn native_pixels_per_point() -> f32 {
    let pixels_per_point = web_sys::window().unwrap().device_pixel_ratio() as f32;
    if pixels_per_point > 0.0 && pixels_per_point.is_finite() {
        pixels_per_point
    } else {
        1.0
    }
}

pub fn prefer_dark_mode() -> Option<bool> {
    Some(
        web_sys::window()?
            .match_media("(prefers-color-scheme: dark)")
            .ok()??
            .matches(),
    )
}

pub fn canvas_element(canvas_id: &str) -> Option<web_sys::HtmlCanvasElement> {
    use wasm_bindgen::JsCast;
    let document = web_sys::window()?.document()?;
    let canvas = document.get_element_by_id(canvas_id)?;
    canvas.dyn_into::<web_sys::HtmlCanvasElement>().ok()
}

pub fn canvas_element_or_die(canvas_id: &str) -> web_sys::HtmlCanvasElement {
    crate::canvas_element(canvas_id)
        .unwrap_or_else(|| panic!("Failed to find canvas with id '{}'", canvas_id))
}

fn canvas_origin(canvas_id: &str) -> egui::Pos2 {
    let rect = canvas_element(canvas_id)
        .unwrap()
        .get_bounding_client_rect();
    egui::Pos2::new(rect.left() as f32, rect.top() as f32)
}

pub fn canvas_size_in_points(canvas_id: &str) -> egui::Vec2 {
    let canvas = canvas_element(canvas_id).unwrap();
    let pixels_per_point = native_pixels_per_point();
    egui::vec2(
        canvas.width() as f32 / pixels_per_point,
        canvas.height() as f32 / pixels_per_point,
    )
}

pub fn resize_canvas_to_screen_size(canvas_id: &str, max_size_points: egui::Vec2) -> Option<()> {
    let canvas = canvas_element(canvas_id)?;

    let screen_size_points = screen_size_in_native_points()?;
    let pixels_per_point = native_pixels_per_point();

    let max_size_pixels = pixels_per_point * max_size_points;

    let canvas_size_pixels = pixels_per_point * screen_size_points;
    let canvas_size_pixels = canvas_size_pixels.min(max_size_pixels);
    let canvas_size_points = canvas_size_pixels / pixels_per_point;

    // Make sure that the height and width are always even numbers.
    // otherwise, the page renders blurry on some platforms.
    // See https://github.com/emilk/egui/issues/103
    fn round_to_even(v: f32) -> f32 {
        (v / 2.0).round() * 2.0
    }

    canvas
        .style()
        .set_property(
            "width",
            &format!("{}px", round_to_even(canvas_size_points.x)),
        )
        .ok()?;
    canvas
        .style()
        .set_property(
            "height",
            &format!("{}px", round_to_even(canvas_size_points.y)),
        )
        .ok()?;
    canvas.set_width(round_to_even(canvas_size_pixels.x) as u32);
    canvas.set_height(round_to_even(canvas_size_pixels.y) as u32);

    Some(())
}

// ----------------------------------------------------------------------------

pub fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

pub fn local_storage_get(key: &str) -> Option<String> {
    local_storage().map(|storage| storage.get_item(key).ok())??
}

pub fn local_storage_set(key: &str, value: &str) {
    local_storage().map(|storage| storage.set_item(key, value));
}

pub fn local_storage_remove(key: &str) {
    local_storage().map(|storage| storage.remove_item(key));
}

#[cfg(feature = "persistence")]
pub fn load_memory(ctx: &egui::Context) {
    if let Some(memory_string) = local_storage_get("egui_memory_ron") {
        match ron::from_str(&memory_string) {
            Ok(memory) => {
                *ctx.memory() = memory;
            }
            Err(err) => {
                tracing::error!("Failed to parse memory RON: {}", err);
            }
        }
    }
}

#[cfg(not(feature = "persistence"))]
pub fn load_memory(_: &egui::Context) {}

#[cfg(feature = "persistence")]
pub fn save_memory(ctx: &egui::Context) {
    match ron::to_string(&*ctx.memory()) {
        Ok(ron) => {
            local_storage_set("egui_memory_ron", &ron);
        }
        Err(err) => {
            tracing::error!("Failed to serialize memory as RON: {}", err);
        }
    }
}

#[cfg(not(feature = "persistence"))]
pub fn save_memory(_: &egui::Context) {}

#[derive(Default)]
pub struct LocalStorage {}

impl epi::Storage for LocalStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        local_storage_get(key)
    }
    fn set_string(&mut self, key: &str, value: String) {
        local_storage_set(key, &value);
    }
    fn flush(&mut self) {}
}

// ----------------------------------------------------------------------------

pub fn set_cursor_icon(cursor: egui::CursorIcon) -> Option<()> {
    let document = web_sys::window()?.document()?;
    document
        .body()?
        .style()
        .set_property("cursor", cursor_web_name(cursor))
        .ok()
}

#[cfg(web_sys_unstable_apis)]
pub fn set_clipboard_text(s: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(clipboard) = window.navigator().clipboard() {
            let promise = clipboard.write_text(s);
            let future = wasm_bindgen_futures::JsFuture::from(promise);
            let future = async move {
                if let Err(err) = future.await {
                    tracing::error!("Copy/cut action denied: {:?}", err);
                }
            };
            wasm_bindgen_futures::spawn_local(future);
        }
    }
}

pub fn spawn_future<F>(future: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

fn cursor_web_name(cursor: egui::CursorIcon) -> &'static str {
    match cursor {
        egui::CursorIcon::Alias => "alias",
        egui::CursorIcon::AllScroll => "all-scroll",
        egui::CursorIcon::Cell => "cell",
        egui::CursorIcon::ContextMenu => "context-menu",
        egui::CursorIcon::Copy => "copy",
        egui::CursorIcon::Crosshair => "crosshair",
        egui::CursorIcon::Default => "default",
        egui::CursorIcon::Grab => "grab",
        egui::CursorIcon::Grabbing => "grabbing",
        egui::CursorIcon::Help => "help",
        egui::CursorIcon::Move => "move",
        egui::CursorIcon::NoDrop => "no-drop",
        egui::CursorIcon::None => "none",
        egui::CursorIcon::NotAllowed => "not-allowed",
        egui::CursorIcon::PointingHand => "pointer",
        egui::CursorIcon::Progress => "progress",
        egui::CursorIcon::ResizeHorizontal => "ew-resize",
        egui::CursorIcon::ResizeNeSw => "nesw-resize",
        egui::CursorIcon::ResizeNwSe => "nwse-resize",
        egui::CursorIcon::ResizeVertical => "ns-resize",
        egui::CursorIcon::Text => "text",
        egui::CursorIcon::VerticalText => "vertical-text",
        egui::CursorIcon::Wait => "wait",
        egui::CursorIcon::ZoomIn => "zoom-in",
        egui::CursorIcon::ZoomOut => "zoom-out",
    }
}

pub fn open_url(url: &str, new_tab: bool) -> Option<()> {
    let name = if new_tab { "_blank" } else { "_self" };

    web_sys::window()?
        .open_with_url_and_target(url, name)
        .ok()?;
    Some(())
}

/// e.g. "#fragment" part of "www.example.com/index.html#fragment",
///
/// Percent decoded
pub fn location_hash() -> String {
    percent_decode(
        &web_sys::window()
            .unwrap()
            .location()
            .hash()
            .unwrap_or_default(),
    )
}

pub fn percent_decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .to_string()
}

// ----------------------------------------------------------------------------

pub type AppRunnerRef = Arc<Mutex<AppRunner>>;

pub struct AppRunnerContainer {
    runner: AppRunnerRef,
    /// Set to `true` if there is a panic.
    /// Used to ignore callbacks after a panic.
    panicked: Arc<AtomicBool>,
}

impl AppRunnerContainer {
    /// Convenience function to reduce boilerplate and ensure that all event handlers
    /// are dealt with in the same way
    pub fn add_event_listener<E: wasm_bindgen::JsCast>(
        &self,
        target: &EventTarget,
        event_name: &'static str,
        mut closure: impl FnMut(E, MutexGuard<'_, AppRunner>) + 'static,
    ) -> Result<(), JsValue> {
        use wasm_bindgen::JsCast;

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
            }) as Box<dyn FnMut(_)>
        });

        // Add the event listener to the target
        target.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;

        // Bypass closure drop so that event handler can call the closure
        closure.forget();

        Ok(())
    }
}

fn paint_and_schedule(runner_ref: &AppRunnerRef, panicked: Arc<AtomicBool>) -> Result<(), JsValue> {
    fn paint_if_needed(runner_ref: &AppRunnerRef) -> Result<(), JsValue> {
        let mut runner_lock = runner_ref.lock();
        if runner_lock.needs_repaint.fetch_and_clear() {
            runner_lock.clear_color_buffer();
            let (needs_repaint, clipped_primitives) = runner_lock.logic()?;
            runner_lock.paint(&clipped_primitives)?;
            if needs_repaint {
                runner_lock.needs_repaint.set_true();
            }
            runner_lock.auto_save();
        }

        Ok(())
    }

    fn request_animation_frame(
        runner_ref: AppRunnerRef,
        panicked: Arc<AtomicBool>,
    ) -> Result<(), JsValue> {
        use wasm_bindgen::JsCast;
        let window = web_sys::window().unwrap();
        let closure = Closure::once(move || paint_and_schedule(&runner_ref, panicked));
        window.request_animation_frame(closure.as_ref().unchecked_ref())?;
        closure.forget(); // We must forget it, or else the callback is canceled on drop
        Ok(())
    }

    // Only paint and schedule if there has been no panic
    if !panicked.load(Ordering::SeqCst) {
        paint_if_needed(runner_ref)?;
        request_animation_frame(runner_ref.clone(), panicked)?;
    }

    Ok(())
}

fn install_document_events(runner_container: &AppRunnerContainer) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    runner_container.add_event_listener(
        &document,
        "keydown",
        |event: web_sys::KeyboardEvent, mut runner_lock| {
            if event.is_composing() || event.key_code() == 229 {
                // https://www.fxsitecompat.dev/en-CA/docs/2018/keydown-and-keyup-events-are-now-fired-during-ime-composition/
                return;
            }

            let modifiers = modifiers_from_event(&event);
            runner_lock.input.raw.modifiers = modifiers;

            let key = event.key();

            if let Some(key) = translate_key(&key) {
                runner_lock.input.raw.events.push(egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                });
            }
            if !modifiers.ctrl
                && !modifiers.command
                && !should_ignore_key(&key)
                // When text agent is shown, it sends text event instead.
                && text_agent::text_agent().hidden()
            {
                runner_lock.input.raw.events.push(egui::Event::Text(key));
            }
            runner_lock.needs_repaint.set_true();

            let egui_wants_keyboard = runner_lock.egui_ctx().wants_keyboard_input();

            let prevent_default = if matches!(event.key().as_str(), "Tab") {
                // Always prevent moving cursor to url bar.
                // egui wants to use tab to move to the next text field.
                true
            } else if egui_wants_keyboard {
                matches!(
                    event.key().as_str(),
                    "Backspace" // so we don't go back to previous page when deleting text
                | "ArrowDown" | "ArrowLeft" | "ArrowRight" | "ArrowUp" // cmd-left is "back" on Mac (https://github.com/emilk/egui/issues/58)
                )
            } else {
                // We never want to prevent:
                // * F5 / cmd-R (refresh)
                // * cmd-shift-C (debug tools)
                // * cmd/ctrl-c/v/x (or we stop copy/past/cut events)
                false
            };

            // tracing::debug!(
            //     "On key-down {:?}, egui_wants_keyboard: {}, prevent_default: {}",
            //     event.key().as_str(),
            //     egui_wants_keyboard,
            //     prevent_default
            // );

            if prevent_default {
                event.prevent_default();
            }
        },
    )?;

    runner_container.add_event_listener(
        &document,
        "keyup",
        |event: web_sys::KeyboardEvent, mut runner_lock| {
            let modifiers = modifiers_from_event(&event);
            runner_lock.input.raw.modifiers = modifiers;
            if let Some(key) = translate_key(&event.key()) {
                runner_lock.input.raw.events.push(egui::Event::Key {
                    key,
                    pressed: false,
                    modifiers,
                });
            }
            runner_lock.needs_repaint.set_true();
        },
    )?;

    #[cfg(web_sys_unstable_apis)]
    runner_container.add_event_listener(
        &document,
        "paste",
        |event: web_sys::ClipboardEvent, mut runner_lock| {
            if let Some(data) = event.clipboard_data() {
                if let Ok(text) = data.get_data("text") {
                    let text = text.replace("\r\n", "\n");
                    if !text.is_empty() {
                        runner_lock.input.raw.events.push(egui::Event::Paste(text));
                        runner_lock.needs_repaint.set_true();
                    }
                    event.stop_propagation();
                    event.prevent_default();
                }
            }
        },
    )?;

    #[cfg(web_sys_unstable_apis)]
    runner_container.add_event_listener(
        &document,
        "cut",
        |_: web_sys::ClipboardEvent, mut runner_lock| {
            runner_lock.input.raw.events.push(egui::Event::Cut);
            runner_lock.needs_repaint.set_true();
        },
    )?;

    #[cfg(web_sys_unstable_apis)]
    runner_container.add_event_listener(
        &document,
        "copy",
        |_: web_sys::ClipboardEvent, mut runner_lock| {
            runner_lock.input.raw.events.push(egui::Event::Copy);
            runner_lock.needs_repaint.set_true();
        },
    )?;

    for event_name in &["load", "pagehide", "pageshow", "resize"] {
        runner_container.add_event_listener(
            &window,
            event_name,
            |_: web_sys::Event, runner_lock| {
                runner_lock.needs_repaint.set_true();
            },
        )?;
    }

    runner_container.add_event_listener(
        &window,
        "hashchange",
        |_: web_sys::Event, mut runner_lock| {
            // `epi::Frame::info(&self)` clones `epi::IntegrationInfo`, but we need to modify the original here
            if let Some(web_info) = &mut runner_lock.frame.info.web_info {
                web_info.location.hash = location_hash();
            }
        },
    )?;

    Ok(())
}

/// Repaint at least every `ms` milliseconds.
pub fn repaint_every_ms(
    runner_container: &AppRunnerContainer,
    milliseconds: i32,
) -> Result<(), JsValue> {
    assert!(milliseconds >= 0);

    use wasm_bindgen::JsCast;

    let window = web_sys::window().unwrap();

    let closure = Closure::wrap(Box::new({
        let runner = runner_container.runner.clone();
        let panicked = runner_container.panicked.clone();

        move || {
            // Do not lock the runner if the code has panicked
            if !panicked.load(Ordering::SeqCst) {
                runner.lock().needs_repaint.set_true();
            }
        }
    }) as Box<dyn FnMut()>);

    window.set_interval_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        milliseconds,
    )?;

    closure.forget();
    Ok(())
}

fn install_canvas_events(runner_container: &AppRunnerContainer) -> Result<(), JsValue> {
    use wasm_bindgen::JsCast;
    let canvas = canvas_element(runner_container.runner.lock().canvas_id()).unwrap();

    {
        // By default, right-clicks open a context menu.
        // We don't want to do that (right clicks is handled by egui):
        let event_name = "contextmenu";
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            event.prevent_default();
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    runner_container.add_event_listener(
        &canvas,
        "mousedown",
        |event: web_sys::MouseEvent, mut runner_lock| {
            if let Some(button) = button_from_mouse_event(&event) {
                let pos = pos_from_mouse_event(runner_lock.canvas_id(), &event);
                let modifiers = runner_lock.input.raw.modifiers;
                runner_lock
                    .input
                    .raw
                    .events
                    .push(egui::Event::PointerButton {
                        pos,
                        button,
                        pressed: true,
                        modifiers,
                    });
                runner_lock.needs_repaint.set_true();
            }
            event.stop_propagation();
            // Note: prevent_default breaks VSCode tab focusing, hence why we don't call it here.
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "mousemove",
        |event: web_sys::MouseEvent, mut runner_lock| {
            let pos = pos_from_mouse_event(runner_lock.canvas_id(), &event);
            runner_lock
                .input
                .raw
                .events
                .push(egui::Event::PointerMoved(pos));
            runner_lock.needs_repaint.set_true();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "mouseup",
        |event: web_sys::MouseEvent, mut runner_lock| {
            if let Some(button) = button_from_mouse_event(&event) {
                let pos = pos_from_mouse_event(runner_lock.canvas_id(), &event);
                let modifiers = runner_lock.input.raw.modifiers;
                runner_lock
                    .input
                    .raw
                    .events
                    .push(egui::Event::PointerButton {
                        pos,
                        button,
                        pressed: false,
                        modifiers,
                    });
                runner_lock.needs_repaint.set_true();

                text_agent::update_text_agent(runner_lock);
            }
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "mouseleave",
        |event: web_sys::MouseEvent, mut runner_lock| {
            runner_lock.input.raw.events.push(egui::Event::PointerGone);
            runner_lock.needs_repaint.set_true();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "touchstart",
        |event: web_sys::TouchEvent, mut runner_lock| {
            let mut latest_touch_pos_id = runner_lock.input.latest_touch_pos_id;
            let pos =
                pos_from_touch_event(runner_lock.canvas_id(), &event, &mut latest_touch_pos_id);
            runner_lock.input.latest_touch_pos_id = latest_touch_pos_id;
            runner_lock.input.latest_touch_pos = Some(pos);
            let modifiers = runner_lock.input.raw.modifiers;
            runner_lock
                .input
                .raw
                .events
                .push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers,
                });

            push_touches(&mut *runner_lock, egui::TouchPhase::Start, &event);
            runner_lock.needs_repaint.set_true();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "touchmove",
        |event: web_sys::TouchEvent, mut runner_lock| {
            let mut latest_touch_pos_id = runner_lock.input.latest_touch_pos_id;
            let pos =
                pos_from_touch_event(runner_lock.canvas_id(), &event, &mut latest_touch_pos_id);
            runner_lock.input.latest_touch_pos_id = latest_touch_pos_id;
            runner_lock.input.latest_touch_pos = Some(pos);
            runner_lock
                .input
                .raw
                .events
                .push(egui::Event::PointerMoved(pos));

            push_touches(&mut *runner_lock, egui::TouchPhase::Move, &event);
            runner_lock.needs_repaint.set_true();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "touchend",
        |event: web_sys::TouchEvent, mut runner_lock| {
            if let Some(pos) = runner_lock.input.latest_touch_pos {
                let modifiers = runner_lock.input.raw.modifiers;
                // First release mouse to click:
                runner_lock
                    .input
                    .raw
                    .events
                    .push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers,
                    });
                // Then remove hover effect:
                runner_lock.input.raw.events.push(egui::Event::PointerGone);

                push_touches(&mut *runner_lock, egui::TouchPhase::End, &event);
                runner_lock.needs_repaint.set_true();
                event.stop_propagation();
                event.prevent_default();
            }

            // Finally, focus or blur text agent to toggle mobile keyboard:
            text_agent::update_text_agent(runner_lock);
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "touchcancel",
        |event: web_sys::TouchEvent, mut runner_lock| {
            push_touches(&mut runner_lock, egui::TouchPhase::Cancel, &event);
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "wheel",
        |event: web_sys::WheelEvent, mut runner_lock| {
            let scroll_multiplier = match event.delta_mode() {
                web_sys::WheelEvent::DOM_DELTA_PAGE => {
                    canvas_size_in_points(runner_lock.canvas_id()).y
                }
                web_sys::WheelEvent::DOM_DELTA_LINE => {
                    #[allow(clippy::let_and_return)]
                    let points_per_scroll_line = 8.0; // Note that this is intentionally different from what we use in egui_glium / winit.
                    points_per_scroll_line
                }
                _ => 1.0, // DOM_DELTA_PIXEL
            };

            let mut delta =
                -scroll_multiplier * egui::vec2(event.delta_x() as f32, event.delta_y() as f32);

            // Report a zoom event in case CTRL (on Windows or Linux) or CMD (on Mac) is pressed.
            // This if-statement is equivalent to how `Modifiers.command` is determined in
            // `modifiers_from_event()`, but we cannot directly use that fn for a [`WheelEvent`].
            if event.ctrl_key() || event.meta_key() {
                let factor = (delta.y / 200.0).exp();
                runner_lock.input.raw.events.push(egui::Event::Zoom(factor));
            } else {
                if event.shift_key() {
                    // Treat as horizontal scrolling.
                    // Note: one Mac we already get horizontal scroll events when shift is down.
                    delta = egui::vec2(delta.x + delta.y, 0.0);
                }

                runner_lock
                    .input
                    .raw
                    .events
                    .push(egui::Event::Scroll(delta));
            }

            runner_lock.needs_repaint.set_true();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "dragover",
        |event: web_sys::DragEvent, mut runner_lock| {
            if let Some(data_transfer) = event.data_transfer() {
                runner_lock.input.raw.hovered_files.clear();
                for i in 0..data_transfer.items().length() {
                    if let Some(item) = data_transfer.items().get(i) {
                        runner_lock.input.raw.hovered_files.push(egui::HoveredFile {
                            mime: item.type_(),
                            ..Default::default()
                        });
                    }
                }
                runner_lock.needs_repaint.set_true();
                event.stop_propagation();
                event.prevent_default();
            }
        },
    )?;

    runner_container.add_event_listener(
        &canvas,
        "dragleave",
        |event: web_sys::DragEvent, mut runner_lock| {
            runner_lock.input.raw.hovered_files.clear();
            runner_lock.needs_repaint.set_true();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    runner_container.add_event_listener(&canvas, "drop", {
        let runner_ref = runner_container.runner.clone();

        move |event: web_sys::DragEvent, mut runner_lock| {
            if let Some(data_transfer) = event.data_transfer() {
                runner_lock.input.raw.hovered_files.clear();
                runner_lock.needs_repaint.set_true();
                // Unlock the runner so it can be locked after a future await point
                drop(runner_lock);

                if let Some(files) = data_transfer.files() {
                    for i in 0..files.length() {
                        if let Some(file) = files.get(i) {
                            let name = file.name();
                            let last_modified = std::time::UNIX_EPOCH
                                + std::time::Duration::from_millis(file.last_modified() as u64);

                            tracing::debug!("Loading {:?} ({} bytes)…", name, file.size());

                            let future = wasm_bindgen_futures::JsFuture::from(file.array_buffer());

                            let runner_ref = runner_ref.clone();
                            let future = async move {
                                match future.await {
                                    Ok(array_buffer) => {
                                        let bytes = js_sys::Uint8Array::new(&array_buffer).to_vec();
                                        tracing::debug!(
                                            "Loaded {:?} ({} bytes).",
                                            name,
                                            bytes.len()
                                        );

                                        // Re-lock the mutex on the other side of the await point
                                        let mut runner_lock = runner_ref.lock();
                                        runner_lock.input.raw.dropped_files.push(
                                            egui::DroppedFile {
                                                name,
                                                last_modified: Some(last_modified),
                                                bytes: Some(bytes.into()),
                                                ..Default::default()
                                            },
                                        );
                                        runner_lock.needs_repaint.set_true();
                                    }
                                    Err(err) => {
                                        tracing::error!("Failed to read file: {:?}", err);
                                    }
                                }
                            };
                            wasm_bindgen_futures::spawn_local(future);
                        }
                    }
                }
                event.stop_propagation();
                event.prevent_default();
            }
        }
    })?;

    Ok(())
}

pub(crate) fn webgl1_requires_brightening(gl: &web_sys::WebGlRenderingContext) -> bool {
    // See https://github.com/emilk/egui/issues/794

    // detect WebKitGTK

    // WebKitGTK use WebKit default unmasked vendor and renderer
    // but safari use same vendor and renderer
    // so exclude "Mac OS X" user-agent.
    let user_agent = web_sys::window().unwrap().navigator().user_agent().unwrap();
    !user_agent.contains("Mac OS X") && crate::is_safari_and_webkit_gtk(gl)
}

/// detecting Safari and `webkitGTK`.
///
/// Safari and `webkitGTK` use unmasked renderer :Apple GPU
///
/// If we detect safari or `webkitGTKs` returns true.
///
/// This function used to avoid displaying linear color with `sRGB` supported systems.
fn is_safari_and_webkit_gtk(gl: &web_sys::WebGlRenderingContext) -> bool {
    // This call produces a warning in Firefox ("WEBGL_debug_renderer_info is deprecated in Firefox and will be removed.")
    // but unless we call it we get errors in Chrome when we call `get_parameter` below.
    // TODO: do something smart based on user agent?
    if gl
        .get_extension("WEBGL_debug_renderer_info")
        .unwrap()
        .is_some()
    {
        if let Ok(renderer) =
            gl.get_parameter(web_sys::WebglDebugRendererInfo::UNMASKED_RENDERER_WEBGL)
        {
            if let Some(renderer) = renderer.as_string() {
                if renderer.contains("Apple") {
                    return true;
                }
            }
        }
    }

    false
}
