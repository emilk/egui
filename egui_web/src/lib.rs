#![deny(warnings)]
#![warn(clippy::all)]

pub mod webgl;

use parking_lot::Mutex;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

// ----------------------------------------------------------------------------

pub struct WebInfo {
    /// e.g. "#fragment" part of "www.example.com/index.html#fragment"
    pub web_location_hash: String,
}

/// Implement this and use `egui_web::AppRunner` to run your app.
pub trait App {
    fn ui(&mut self, ui: &mut egui::Ui, backend: &mut Backend, info: &WebInfo);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunMode {
    /// Uses `request_animation_frame` to repaint the UI on each display Hz.
    /// This is good for games and stuff where you want to run logic at e.g. 60 FPS.
    Continuous,

    /// Only repaint when there is new input (mouse movement, keyboard input etc).
    Reactive,
}

// ----------------------------------------------------------------------------

pub struct Backend {
    ctx: Arc<egui::Context>,
    painter: webgl::Painter,
    frame_times: egui::MovementTracker<f32>,
    frame_start: Option<f64>,
    /// If true, paint at full framerate always.
    /// If false, only paint on input.
    run_mode: RunMode,
    last_save_time: Option<f64>,
}

impl Backend {
    pub fn new(canvas_id: &str, run_mode: RunMode) -> Result<Backend, JsValue> {
        let ctx = egui::Context::new();
        load_memory(&ctx);
        Ok(Backend {
            ctx,
            painter: webgl::Painter::new(canvas_id)?,
            frame_times: egui::MovementTracker::new(1000, 1.0),
            frame_start: None,
            run_mode,
            last_save_time: None,
        })
    }

    pub fn run_mode(&self) -> RunMode {
        self.run_mode
    }

    pub fn set_run_mode(&mut self, run_mode: RunMode) {
        self.run_mode = run_mode;
    }

    /// id of the canvas html element containing the rendering
    pub fn canvas_id(&self) -> &str {
        self.painter.canvas_id()
    }

    pub fn begin_frame(&mut self, raw_input: egui::RawInput) -> egui::Ui {
        self.frame_start = Some(now_sec());
        self.ctx.begin_frame(raw_input)
    }

    pub fn end_frame(&mut self) -> Result<(egui::Output, egui::PaintJobs), JsValue> {
        let frame_start = self
            .frame_start
            .take()
            .expect("unmatched calls to begin_frame/end_frame");

        let (output, paint_jobs) = self.ctx.end_frame();

        self.auto_save();

        let now = now_sec();
        self.frame_times.add(now, (now - frame_start) as f32);

        Ok((output, paint_jobs))
    }

    pub fn paint(&mut self, paint_jobs: egui::PaintJobs) -> Result<(), JsValue> {
        let bg_color = egui::color::TRANSPARENT; // Use background css color.
        self.painter.paint_jobs(
            bg_color,
            paint_jobs,
            self.ctx.texture(),
            self.ctx.pixels_per_point(),
        )
    }

    pub fn auto_save(&mut self) {
        let now = now_sec();
        let time_since_last_save = now - self.last_save_time.unwrap_or(std::f64::NEG_INFINITY);
        const AUTO_SAVE_INTERVAL: f64 = 5.0;
        if time_since_last_save > AUTO_SAVE_INTERVAL {
            self.last_save_time = Some(now);
            save_memory(&self.ctx);
        }
    }

    pub fn painter_debug_info(&self) -> String {
        self.painter.debug_info()
    }

    /// excludes painting
    pub fn cpu_time(&self) -> f32 {
        self.frame_times.average().unwrap_or_default()
    }

    pub fn fps(&self) -> f32 {
        1.0 / self.frame_times.mean_time_interval().unwrap_or_default()
    }
}

// ----------------------------------------------------------------------------

/// Data gathered between frames.
/// Is translated to `egui::RawInput` at the start of each frame.
#[derive(Default)]
pub struct WebInput {
    pub mouse_pos: Option<egui::Pos2>,
    pub mouse_down: bool, // TODO: which button
    pub is_touch: bool,
    pub scroll_delta: egui::Vec2,
    pub events: Vec<egui::Event>,
}

impl WebInput {
    pub fn new_frame(&mut self) -> egui::RawInput {
        egui::RawInput {
            mouse_down: self.mouse_down,
            mouse_pos: self.mouse_pos,
            scroll_delta: std::mem::take(&mut self.scroll_delta),
            screen_size: screen_size().unwrap(),
            pixels_per_point: Some(pixels_per_point()),
            time: now_sec(),
            seconds_since_midnight: Some(seconds_since_midnight()),
            events: std::mem::take(&mut self.events),
        }
    }
}

// ----------------------------------------------------------------------------

pub struct AppRunner {
    pub backend: Backend,
    pub web_input: WebInput,
    pub app: Box<dyn App>,
    pub needs_repaint: bool, // TODO: move
}

impl AppRunner {
    pub fn new(backend: Backend, app: Box<dyn App>) -> Result<Self, JsValue> {
        Ok(Self {
            backend,
            web_input: Default::default(),
            app,
            needs_repaint: true, // TODO: move
        })
    }

    pub fn canvas_id(&self) -> &str {
        self.backend.canvas_id()
    }

    pub fn logic(&mut self) -> Result<(egui::Output, egui::PaintJobs), JsValue> {
        resize_to_screen_size(self.backend.canvas_id());

        let raw_input = self.web_input.new_frame();

        let info = WebInfo {
            web_location_hash: location_hash().unwrap_or_default(),
        };

        let mut ui = self.backend.begin_frame(raw_input);
        self.app.ui(&mut ui, &mut self.backend, &info);
        let (output, paint_jobs) = self.backend.end_frame()?;
        handle_output(&output);
        Ok((output, paint_jobs))
    }

    pub fn paint(&mut self, paint_jobs: egui::PaintJobs) -> Result<(), JsValue> {
        self.backend.paint(paint_jobs)
    }
}

/// Install event listeners to register different input events
/// and starts running the given `AppRunner`.
pub fn run(app_runner: AppRunner) -> Result<AppRunnerRef, JsValue> {
    let runner_ref = AppRunnerRef(Arc::new(Mutex::new(app_runner)));
    install_canvas_events(&runner_ref)?;
    install_document_events(&runner_ref)?;
    paint_and_schedule(runner_ref.clone())?;
    Ok(runner_ref)
}

// ----------------------------------------------------------------------------
// Helpers to hide some of the verbosity of web_sys

pub fn console_log(s: String) {
    web_sys::console::log_1(&s.into());
}

pub fn screen_size() -> Option<egui::Vec2> {
    let window = web_sys::window()?;
    Some(egui::Vec2::new(
        window.inner_width().ok()?.as_f64()? as f32,
        window.inner_height().ok()?.as_f64()? as f32,
    ))
}

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
    return seconds as f64 + 1e-3 * (d.get_milliseconds() as f64);
}

pub fn pixels_per_point() -> f32 {
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

pub fn resize_to_screen_size(canvas_id: &str) -> Option<()> {
    let canvas = canvas_element(canvas_id)?;

    let screen_size = screen_size()?;
    let pixels_per_point = pixels_per_point();
    canvas
        .style()
        .set_property("width", &format!("{}px", screen_size.x))
        .ok()?;
    canvas
        .style()
        .set_property("height", &format!("{}px", screen_size.y))
        .ok()?;
    canvas.set_width((screen_size.x * pixels_per_point).round() as u32);
    canvas.set_height((screen_size.y * pixels_per_point).round() as u32);

    Some(())
}

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
                console_log(format!("ERROR: Failed to parse memory json: {}", err));
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
            console_log(format!(
                "ERROR: Failed to seriealize memory as json: {}",
                err
            ));
        }
    }
}

pub fn handle_output(output: &egui::Output) {
    set_cursor_icon(output.cursor_icon);
    if let Some(url) = &output.open_url {
        open_url(url);
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
        // "no-drop"
        // "not-allowed"
        // default, help, pointer, progress, wait, cell, crosshair, text, alias, copy, move, grab, grabbing,
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

pub fn translate_key(key: &str) -> Option<egui::Key> {
    match key {
        "Alt" => Some(egui::Key::Alt),
        "Backspace" => Some(egui::Key::Backspace),
        "Control" => Some(egui::Key::Control),
        "Delete" => Some(egui::Key::Delete),
        "ArrowDown" => Some(egui::Key::Down),
        "End" => Some(egui::Key::End),
        "Escape" => Some(egui::Key::Escape),
        "Home" => Some(egui::Key::Home),
        "Help" => Some(egui::Key::Insert),
        "ArrowLeft" => Some(egui::Key::Left),
        "Meta" => Some(egui::Key::Logo),
        "PageDown" => Some(egui::Key::PageDown),
        "PageUp" => Some(egui::Key::PageUp),
        "Enter" => Some(egui::Key::Return),
        "ArrowRight" => Some(egui::Key::Right),
        "Shift" => Some(egui::Key::Shift),
        "Tab" => Some(egui::Key::Tab),
        "ArrowUp" => Some(egui::Key::Up),
        _ => None,
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone)]
pub struct AppRunnerRef(Arc<Mutex<AppRunner>>);

fn paint_and_schedule(runner_ref: AppRunnerRef) -> Result<(), JsValue> {
    fn paint(runner_ref: &AppRunnerRef) -> Result<(), JsValue> {
        let mut runner_lock = runner_ref.0.lock();
        if runner_lock.backend.run_mode() == RunMode::Continuous || runner_lock.needs_repaint {
            runner_lock.needs_repaint = false;
            let (output, paint_jobs) = runner_lock.logic()?;
            runner_lock.paint(paint_jobs)?;
            runner_lock.needs_repaint = output.needs_repaint;
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

    paint(&runner_ref)?;
    request_animation_frame(runner_ref)
}

fn install_document_events(runner_ref: &AppRunnerRef) -> Result<(), JsValue> {
    use wasm_bindgen::JsCast;
    let document = web_sys::window().unwrap().document().unwrap();

    {
        // keydown
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let mut runner_lock = runner_ref.0.lock();
            let key = event.key();
            if let Some(key) = translate_key(&key) {
                runner_lock
                    .web_input
                    .events
                    .push(egui::Event::Key { key, pressed: true });
            } else {
                runner_lock.web_input.events.push(egui::Event::Text(key));
            }
            runner_lock.needs_repaint = true;
        }) as Box<dyn FnMut(_)>);
        document.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    {
        // keyup
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            let mut runner_lock = runner_ref.0.lock();
            let key = event.key();
            if let Some(key) = translate_key(&key) {
                runner_lock.web_input.events.push(egui::Event::Key {
                    key,
                    pressed: false,
                });
                runner_lock.needs_repaint = true;
            }
        }) as Box<dyn FnMut(_)>);
        document.add_event_listener_with_callback("keyup", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    for event_name in &["load", "pagehide", "pageshow", "resize"] {
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move || {
            runner_ref.0.lock().needs_repaint = true;
        }) as Box<dyn FnMut()>);
        document.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    Ok(())
}

fn install_canvas_events(runner_ref: &AppRunnerRef) -> Result<(), JsValue> {
    use wasm_bindgen::JsCast;
    let canvas = canvas_element(runner_ref.0.lock().canvas_id()).unwrap();

    {
        let event_name = "mousedown";
        let runner_ref = runner_ref.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut runner_lock = runner_ref.0.lock();
            if !runner_lock.web_input.is_touch {
                runner_lock.web_input.mouse_pos =
                    Some(pos_from_mouse_event(runner_lock.canvas_id(), &event));
                runner_lock.web_input.mouse_down = true;
                runner_lock.needs_repaint = true;
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
            if !runner_lock.web_input.is_touch {
                runner_lock.web_input.mouse_pos =
                    Some(pos_from_mouse_event(runner_lock.canvas_id(), &event));
                runner_lock.needs_repaint = true;
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
            if !runner_lock.web_input.is_touch {
                runner_lock.web_input.mouse_pos =
                    Some(pos_from_mouse_event(runner_lock.canvas_id(), &event));
                runner_lock.web_input.mouse_down = false;
                runner_lock.needs_repaint = true;
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
            if !runner_lock.web_input.is_touch {
                runner_lock.web_input.mouse_pos = None;
                runner_lock.needs_repaint = true;
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
            runner_lock.web_input.is_touch = true;
            runner_lock.web_input.mouse_pos = Some(pos_from_touch_event(&event));
            runner_lock.web_input.mouse_down = true;
            runner_lock.needs_repaint = true;
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
            runner_lock.web_input.is_touch = true;
            runner_lock.web_input.mouse_pos = Some(pos_from_touch_event(&event));
            runner_lock.needs_repaint = true;
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
            runner_lock.web_input.is_touch = true;
            runner_lock.web_input.mouse_down = false; // First release mouse to click...
            runner_lock.logic().unwrap(); // ...do the clicking...
            runner_lock.web_input.mouse_pos = None; // ...remove hover effect
            runner_lock.needs_repaint = true;
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
            runner_lock.web_input.scroll_delta.x -= event.delta_x() as f32;
            runner_lock.web_input.scroll_delta.y -= event.delta_y() as f32;
            runner_lock.needs_repaint = true;
            event.stop_propagation();
            event.prevent_default();
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    Ok(())
}
