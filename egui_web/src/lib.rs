#![deny(warnings)]
#![warn(clippy::all)]

pub mod webgl;

// ----------------------------------------------------------------------------

use std::sync::Arc;
use wasm_bindgen::JsValue;

pub struct EguiWeb {
    ctx: Arc<egui::Context>,
    webgl_painter: webgl::Painter,
    frame_times: egui::MovementTracker<f32>,
    frame_start: Option<f64>,
}

impl EguiWeb {
    pub fn new(canvas_id: &str) -> Result<EguiWeb, JsValue> {
        let ctx = egui::Context::new();
        load_memory(&ctx);
        Ok(EguiWeb {
            ctx,
            webgl_painter: webgl::Painter::new(canvas_id)?,
            frame_times: egui::MovementTracker::new(1000, 1.0),
            frame_start: None,
        })
    }

    /// id of the canvas html element containing the rendering
    pub fn canvas_id(&self) -> &str {
        self.webgl_painter.canvas_id()
    }

    pub fn begin_frame(&mut self, raw_input: egui::RawInput) -> egui::Ui {
        self.frame_start = Some(now_sec());
        self.ctx.begin_frame(raw_input)
    }

    pub fn end_frame(&mut self) -> Result<egui::Output, JsValue> {
        let frame_start = self
            .frame_start
            .take()
            .expect("unmatched calls to begin_frame/end_frame");

        let bg_color = egui::color::srgba(0, 0, 0, 0); // Use background css color.
        let (output, batches) = self.ctx.end_frame();

        let now = now_sec();
        self.frame_times.add(now, (now - frame_start) as f32);

        self.webgl_painter.paint_batches(
            bg_color,
            batches,
            self.ctx.texture(),
            self.ctx.pixels_per_point(),
        )?;

        save_memory(&self.ctx); // TODO: don't save every frame

        Ok(output)
    }

    pub fn painter_debug_info(&self) -> String {
        self.webgl_painter.debug_info()
    }

    /// excludes painting
    pub fn cpu_usage(&self) -> f32 {
        self.frame_times.average().unwrap_or_default()
    }

    pub fn fps(&self) -> f32 {
        1.0 / self.frame_times.mean_time_interval().unwrap_or_default()
    }
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
