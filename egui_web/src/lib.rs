#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), deny(warnings))] // Forbid warnings in release builds
#![warn(clippy::all)]

pub mod backend;
pub mod fetch;
pub mod webgl;

pub use backend::*;

use egui::mutex::Mutex;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

// ----------------------------------------------------------------------------
// Helpers to hide some of the verbosity of web_sys

/// Log some text to the developer console (`console.log(...)` in JS)
pub fn console_log(s: impl Into<JsValue>) {
    web_sys::console::log_1(&s.into());
}

/// Log a warning to the developer console (`console.warn(...)` in JS)
pub fn console_warn(s: impl Into<JsValue>) {
    web_sys::console::warn_1(&s.into());
}

/// Log an error to the developer console (`console.error(...)` in JS)
pub fn console_error(s: impl Into<JsValue>) {
    web_sys::console::error_1(&s.into());
}

/// Current time in seconds (since undefined point in time)
pub fn now_sec() -> f64 {
    web_sys::window()
        .expect("should have a Window")
        .performance()
        .expect("should have a Performance")
        .now()
        / 1000.0
}

pub fn seconds_since_midnight() -> f64 {
    let d = js_sys::Date::new_0();
    let seconds = (d.get_hours() * 60 + d.get_minutes()) * 60 + d.get_seconds();
    seconds as f64 + 1e-3 * (d.get_milliseconds() as f64)
}

pub fn screen_size_in_native_points() -> Option<egui::Vec2> {
    let window = web_sys::window()?;
    Some(egui::Vec2::new(
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

pub fn pos_from_mouse_event(canvas_id: &str, event: &web_sys::MouseEvent) -> egui::Pos2 {
    let canvas = canvas_element(canvas_id).unwrap();
    let rect = canvas.get_bounding_client_rect();
    egui::Pos2 {
        x: event.client_x() as f32 - rect.left() as f32,
        y: event.client_y() as f32 - rect.top() as f32,
    }
}

pub fn pos_from_touch_event(event: &web_sys::TouchEvent) -> egui::Pos2 {
    let t = event.touches().get(0).unwrap();
    egui::Pos2 {
        x: t.page_x() as f32,
        y: t.page_y() as f32,
    }
}

pub fn canvas_size_in_points(canvas_id: &str) -> egui::Vec2 {
    let canvas = canvas_element(canvas_id).unwrap();
    let pixels_per_point = native_pixels_per_point();
    egui::vec2(
        canvas.width() as f32 / pixels_per_point,
        canvas.height() as f32 / pixels_per_point,
    )
}

pub fn resize_canvas_to_screen_size(canvas_id: &str) -> Option<()> {
    let canvas = canvas_element(canvas_id)?;

    let screen_size_points = screen_size_in_native_points()?;
    let pixels_per_point = native_pixels_per_point();

    let canvas_size_pixels = pixels_per_point * screen_size_points;
    // Some browsers get slow with huge WebGL canvases, so we limit the size:
    let max_size_pixels = egui::vec2(2048.0, 4096.0);
    let canvas_size_pixels = canvas_size_pixels.min(max_size_pixels);
    let canvas_size_points = canvas_size_pixels / pixels_per_point;

    canvas
        .style()
        .set_property("width", &format!("{}px", canvas_size_points.x))
        .ok()?;
    canvas
        .style()
        .set_property("height", &format!("{}px", canvas_size_points.y))
        .ok()?;
    canvas.set_width(canvas_size_pixels.x.round() as u32);
    canvas.set_height(canvas_size_pixels.y.round() as u32);

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

pub fn load_memory(ctx: &egui::Context) {
    if let Some(memory_string) = local_storage_get("egui_memory_json") {
        match serde_json::from_str(&memory_string) {
            Ok(memory) => {
                *ctx.memory() = memory;
            }
            Err(err) => {
                console_error(format!("Failed to parse memory json: {}", err));
            }
        }
    }
}

pub fn save_memory(ctx: &egui::Context) {
    match serde_json::to_string(&*ctx.memory()) {
        Ok(json) => {
            local_storage_set("egui_memory_json", &json);
        }
        Err(err) => {
            console_error(format!("Failed to serialize memory as json: {}", err));
        }
    }
}

#[derive(Default)]
pub struct LocalStorage {}

impl egui::app::Storage for LocalStorage {
    fn get_string(&self, key: &str) -> Option<String> {
        local_storage_get(key)
    }
    fn set_string(&mut self, key: &str, value: String) {
        local_storage_set(key, &value);
    }
    fn flush(&mut self) {}
}

// ----------------------------------------------------------------------------

pub fn handle_output(output: &egui::Output) {
    let egui::Output {
        cursor_icon,
        open_url,
        copied_text,
        needs_repaint: _, // handled elsewhere
    } = output;

    set_cursor_icon(*cursor_icon);
    if let Some(url) = open_url {
        crate::open_url(url);
    }

    if !copied_text.is_empty() {
        set_clipboard_text(copied_text);
    }
}

pub fn set_cursor_icon(cursor: egui::CursorIcon) -> Option<()> {
    let document = web_sys::window()?.document()?;
    document
        .body()?
        .style()
        .set_property("cursor", cursor_web_name(cursor))
        .ok()
}

pub fn set_clipboard_text(s: &str) {
    if let Some(window) = web_sys::window() {
        let clipboard = window.navigator().clipboard();
        let promise = clipboard.write_text(s);
        let future = wasm_bindgen_futures::JsFuture::from(promise);
        let future = async move {
            if let Err(err) = future.await {
                console_error(format!("Copy/cut action denied: {:?}", err));
            }
        };
        wasm_bindgen_futures::spawn_local(future);
    }
}

pub fn spawn_future<F>(future: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

fn cursor_web_name(cursor: egui::CursorIcon) -> &'static str {
    use egui::CursorIcon::*;
    match cursor {
        Default => "default",
        PointingHand => "pointer",
        ResizeHorizontal => "ew-resize",
        ResizeNeSw => "nesw-resize",
        ResizeNwSe => "nwse-resize",
        ResizeVertical => "ns-resize",
        Text => "text",
        Grab => "grab",
        Grabbing => "grabbing",
        // "no-drop"
        // "not-allowed"
        // default, help, pointer, progress, wait, cell, crosshair, text, alias, copy, move
    }
}

pub fn open_url(url: &str) -> Option<()> {
    web_sys::window()?
        .open_with_url_and_target(url, "_self")
        .ok()?;
    Some(())
}

/// e.g. "#fragment" part of "www.example.com/index.html#fragment"
pub fn location_hash() -> Option<String> {
    web_sys::window()?.location().hash().ok()
}

/// Web sends all keys as strings, so it is up to us to figure out if it is
/// a real text input or the name of a key.
fn should_ignore_key(key: &str) -> bool {
    let is_function_key = key.starts_with('F') && key.len() > 1;
    is_function_key
        || matches!(
            key,
            "Alt"
                | "ArrowDown"
                | "ArrowLeft"
                | "ArrowRight"
                | "ArrowUp"
                | "Backspace"
                | "CapsLock"
                | "ContextMenu"
                | "Control"
                | "Delete"
                | "End"
                | "Enter"
                | "Esc"
                | "Escape"
                | "Help"
                | "Home"
                | "Insert"
                | "Meta"
                | "NumLock"
                | "PageDown"
                | "PageUp"
                | "Pause"
                | "ScrollLock"
                | "Shift"
                | "Tab"
        )
}

/// Web sends all all keys as strings, so it is up to us to figure out if it is
/// a real text input or the name of a key.
pub fn translate_key(key: &str) -> Option<egui::Key> {
    match key {
        "ArrowDown" => Some(egui::Key::ArrowDown),
        "ArrowLeft" => Some(egui::Key::ArrowLeft),
        "ArrowRight" => Some(egui::Key::ArrowRight),
        "ArrowUp" => Some(egui::Key::ArrowUp),
        "Backspace" => Some(egui::Key::Backspace),
        "Delete" => Some(egui::Key::Delete),
        "End" => Some(egui::Key::End),
        "Enter" => Some(egui::Key::Enter),
        "Space" => Some(egui::Key::Space),
        "Esc" | "Escape" => Some(egui::Key::Escape),
        "Help" | "Insert" => Some(egui::Key::Insert),
        "Home" => Some(egui::Key::Home),
        "PageDown" => Some(egui::Key::PageDown),
        "PageUp" => Some(egui::Key::PageUp),
        "Tab" => Some(egui::Key::Tab),
        "a" | "A" => Some(egui::Key::A),
        "k" | "K" => Some(egui::Key::K),
        "u" | "U" => Some(egui::Key::U),
        "w" | "W" => Some(egui::Key::W),
        "z" | "Z" => Some(egui::Key::Z),
        _ => None,
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone)]
pub struct AppRunnerRef(Arc<Mutex<AppRunner>>);

fn paint_and_schedule(runner_ref: AppRunnerRef) -> Result<(), JsValue> {
    fn paint_if_needed(runner_ref: &AppRunnerRef) -> Result<(), JsValue> {
        let mut runner_lock = runner_ref.0.lock();
        if runner_lock.needs_repaint.fetch_and_clear() {
            let (output, paint_jobs) = runner_lock.logic()?;
            runner_lock.paint(paint_jobs)?;
            if output.needs_repaint {
                runner_lock.needs_repaint.set_true();
            }
            runner_lock.auto_save();
        }

        Ok(())
    }

    fn request_animation_frame(runner_ref: AppRunnerRef) -> Result<(), JsValue> {
        use wasm_bindgen::JsCast;
        let window = web_sys::window().unwrap();
        let closure = Closure::once(move || paint_and_schedule(runner_ref));
        window.request_animation_frame(closure.as_ref().unchecked_ref())?;
        closure.forget(); // We must forget it, or else the callback is canceled on drop
        Ok(())
    }

    paint_if_needed(&runner_ref)?;
    request_animation_frame(runner_ref)
}

fn install_document_events(runner_ref: &AppRunnerRef) -> Result<(), JsValue> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    {
        // keydown
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            if event.is_composing() || event.key_code() == 229 {
                // https://www.fxsitecompat.dev/en-CA/docs/2018/keydown-and-keyup-events-are-now-fired-during-ime-composition/
                return;
            }

            let mut runner_lock = runner_ref.0.lock();
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
            if !modifiers.ctrl && !modifiers.command && !should_ignore_key(&key) {
                runner_lock.input.raw.events.push(egui::Event::Text(key));
            }
            runner_lock.needs_repaint.set_true();

            // So, shall we call prevent_default?
            // YES:
            // * Tab  (move to next text field)
            //
            // SOMETIMES:
            // * Backspace - when entering text we don't want to go back one page.
            //
            // NO:
            // * F5 / cmd-R (refresh)
            // * cmd-shift-C (debug tools)
            // * ...
            //
            // NOTE: if we call prevent_default for cmd-c/v/x, we will prevent copy/paste/cut events.
            // Let's do things manually for now:
            if matches!(
                event.key().as_str(),
                "Backspace"  // so we don't go back to previous page when deleting text
                | "Tab" // so that e.g. tab doesn't move focus to url bar
            ) {
                event.prevent_default();
            }
        }) as Box<dyn FnMut(_)>);
        document.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        // keyup
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let mut runner_lock = runner_ref.0.lock();
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
        }) as Box<dyn FnMut(_)>);
        document.add_event_listener_with_callback("keyup", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        // paste
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::ClipboardEvent| {
            if let Some(data) = event.clipboard_data() {
                if let Ok(text) = data.get_data("text") {
                    let mut runner_lock = runner_ref.0.lock();
                    runner_lock.input.raw.events.push(egui::Event::Text(text));
                    runner_lock.needs_repaint.set_true();
                }
            }
        }) as Box<dyn FnMut(_)>);
        document.add_event_listener_with_callback("paste", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        // cut
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move |_: web_sys::ClipboardEvent| {
            let mut runner_lock = runner_ref.0.lock();
            runner_lock.input.raw.events.push(egui::Event::Cut);
            runner_lock.needs_repaint.set_true();
        }) as Box<dyn FnMut(_)>);
        document.add_event_listener_with_callback("cut", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        // copy
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move |_: web_sys::ClipboardEvent| {
            let mut runner_lock = runner_ref.0.lock();
            runner_lock.input.raw.events.push(egui::Event::Copy);
            runner_lock.needs_repaint.set_true();
        }) as Box<dyn FnMut(_)>);
        document.add_event_listener_with_callback("copy", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    for event_name in &["load", "pagehide", "pageshow", "resize"] {
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move || {
            runner_ref.0.lock().needs_repaint.set_true();
        }) as Box<dyn FnMut()>);
        window.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    Ok(())
}

/// Repaint at least every `ms` milliseconds.
fn repaint_every_ms(runner_ref: &AppRunnerRef, milliseconds: i32) -> Result<(), JsValue> {
    assert!(milliseconds >= 0);
    use wasm_bindgen::JsCast;
    let window = web_sys::window().unwrap();
    let runner_ref = runner_ref.clone();
    let closure = Closure::wrap(Box::new(move || {
        runner_ref.0.lock().needs_repaint.set_true();
    }) as Box<dyn FnMut()>);
    window.set_interval_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        milliseconds,
    )?;
    closure.forget();
    Ok(())
}

fn modifiers_from_event(event: &web_sys::KeyboardEvent) -> egui::Modifiers {
    egui::Modifiers {
        alt: event.alt_key(),
        ctrl: event.ctrl_key(),
        shift: event.shift_key(),

        // Ideally we should know if we are running or mac or not,
        // but this works good enough for now.
        mac_cmd: event.meta_key(),

        // Ideally we should know if we are running or mac or not,
        // but this works good enough for now.
        command: event.ctrl_key() || event.meta_key(),
    }
}

fn install_canvas_events(runner_ref: &AppRunnerRef) -> Result<(), JsValue> {
    use wasm_bindgen::JsCast;
    let canvas = canvas_element(runner_ref.0.lock().canvas_id()).unwrap();

    {
        let event_name = "mousedown";
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut runner_lock = runner_ref.0.lock();
            if !runner_lock.input.is_touch {
                runner_lock.input.raw.mouse_pos =
                    Some(pos_from_mouse_event(runner_lock.canvas_id(), &event));
                runner_lock.input.raw.mouse_down = true;
                runner_lock.logic().unwrap(); // in case we get "mouseup" the same frame. TODO: handle via events instead
                runner_lock.needs_repaint.set_true();
                event.stop_propagation();
                event.prevent_default();
            }
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let event_name = "mousemove";
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut runner_lock = runner_ref.0.lock();
            if !runner_lock.input.is_touch {
                runner_lock.input.raw.mouse_pos =
                    Some(pos_from_mouse_event(runner_lock.canvas_id(), &event));
                runner_lock.needs_repaint.set_true();
                event.stop_propagation();
                event.prevent_default();
            }
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let event_name = "mouseup";
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut runner_lock = runner_ref.0.lock();
            if !runner_lock.input.is_touch {
                runner_lock.input.raw.mouse_pos =
                    Some(pos_from_mouse_event(runner_lock.canvas_id(), &event));
                runner_lock.input.raw.mouse_down = false;
                runner_lock.needs_repaint.set_true();
                event.stop_propagation();
                event.prevent_default();
            }
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let event_name = "mouseleave";
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut runner_lock = runner_ref.0.lock();
            if !runner_lock.input.is_touch {
                runner_lock.input.raw.mouse_pos = None;
                runner_lock.needs_repaint.set_true();
                event.stop_propagation();
                event.prevent_default();
            }
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let event_name = "touchstart";
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            let mut runner_lock = runner_ref.0.lock();
            runner_lock.input.is_touch = true;
            runner_lock.input.raw.mouse_pos = Some(pos_from_touch_event(&event));
            runner_lock.input.raw.mouse_down = true;
            runner_lock.needs_repaint.set_true();
            event.stop_propagation();
            event.prevent_default();
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let event_name = "touchmove";
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            let mut runner_lock = runner_ref.0.lock();
            runner_lock.input.is_touch = true;
            runner_lock.input.raw.mouse_pos = Some(pos_from_touch_event(&event));
            runner_lock.needs_repaint.set_true();
            event.stop_propagation();
            event.prevent_default();
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let event_name = "touchend";
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::TouchEvent| {
            let mut runner_lock = runner_ref.0.lock();
            runner_lock.input.is_touch = true;
            runner_lock.input.raw.mouse_down = false; // First release mouse to click...
            runner_lock.logic().unwrap(); // ...do the clicking... (TODO: handle via events instead)
            runner_lock.input.raw.mouse_pos = None; // ...remove hover effect
            runner_lock.needs_repaint.set_true();
            event.stop_propagation();
            event.prevent_default();
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        let event_name = "wheel";
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::WheelEvent| {
            let mut runner_lock = runner_ref.0.lock();
            runner_lock.input.raw.scroll_delta.x -= event.delta_x() as f32;
            runner_lock.input.raw.scroll_delta.y -= event.delta_y() as f32;
            runner_lock.needs_repaint.set_true();
            event.stop_propagation();
            event.prevent_default();
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    Ok(())
}
