//! [`egui`] bindings for web apps (compiling to WASM).

#![allow(clippy::missing_errors_doc)] // So many `-> Result<_, JsValue>`

mod app_runner;
mod backend;
mod events;
mod input;
mod panic_handler;
mod text_agent;
mod web_logger;
mod web_runner;

/// Access to the browser screen reader.
#[cfg(feature = "web_screen_reader")]
pub mod screen_reader;

/// Access to local browser storage.
pub mod storage;

pub(crate) use app_runner::AppRunner;
pub use panic_handler::{PanicHandler, PanicSummary};
pub use web_logger::WebLogger;
pub use web_runner::WebRunner;

#[cfg(not(any(feature = "glow", feature = "wgpu")))]
compile_error!("You must enable either the 'glow' or 'wgpu' feature");

mod web_painter;

#[cfg(feature = "glow")]
mod web_painter_glow;
#[cfg(feature = "glow")]
pub(crate) type ActiveWebPainter = web_painter_glow::WebPainterGlow;

#[cfg(feature = "wgpu")]
mod web_painter_wgpu;
#[cfg(all(feature = "wgpu", not(feature = "glow")))]
pub(crate) type ActiveWebPainter = web_painter_wgpu::WebPainterWgpu;

pub use backend::*;

use wasm_bindgen::prelude::*;
use web_sys::MediaQueryList;

use input::*;

// ----------------------------------------------------------------------------

pub(crate) fn string_from_js_value(value: &JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:#?}"))
}

/// Returns the `Element` with active focus.
///
/// Elements can only be focused if they are:
/// - `<a>`/`<area>` with an `href` attribute
/// - `<input>`/`<select>`/`<textarea>`/`<button>` which aren't `disabled`
/// - any other element with a `tabindex` attribute
pub(crate) fn focused_element() -> Option<web_sys::Element> {
    web_sys::window()?
        .document()?
        .active_element()?
        .dyn_into()
        .ok()
}

pub(crate) fn has_focus<T: JsCast>(element: &T) -> bool {
    fn try_has_focus<T: JsCast>(element: &T) -> Option<bool> {
        let element = element.dyn_ref::<web_sys::Element>()?;
        let focused_element = focused_element()?;
        Some(element == &focused_element)
    }
    try_has_focus(element).unwrap_or(false)
}

/// Current time in seconds (since undefined point in time).
///
/// Monotonically increasing.
pub fn now_sec() -> f64 {
    web_sys::window()
        .expect("should have a Window")
        .performance()
        .expect("should have a Performance")
        .now()
        / 1000.0
}

/// The native GUI scale factor, taking into account the browser zoom.
///
/// Corresponds to [`window.devicePixelRatio`](https://developer.mozilla.org/en-US/docs/Web/API/Window/devicePixelRatio) in JavaScript.
pub fn native_pixels_per_point() -> f32 {
    let pixels_per_point = web_sys::window().unwrap().device_pixel_ratio() as f32;
    if pixels_per_point > 0.0 && pixels_per_point.is_finite() {
        pixels_per_point
    } else {
        1.0
    }
}

/// Ask the browser about the preferred system theme.
///
/// `None` means unknown.
pub fn system_theme() -> Option<egui::Theme> {
    let dark_mode = prefers_color_scheme_dark(&web_sys::window()?)
        .ok()??
        .matches();
    Some(theme_from_dark_mode(dark_mode))
}

fn prefers_color_scheme_dark(window: &web_sys::Window) -> Result<Option<MediaQueryList>, JsValue> {
    window.match_media("(prefers-color-scheme: dark)")
}

fn theme_from_dark_mode(dark_mode: bool) -> egui::Theme {
    if dark_mode {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    }
}

/// Returns the canvas in client coordinates.
fn canvas_content_rect(canvas: &web_sys::HtmlCanvasElement) -> egui::Rect {
    let bounding_rect = canvas.get_bounding_client_rect();

    let mut rect = egui::Rect::from_min_max(
        egui::pos2(bounding_rect.left() as f32, bounding_rect.top() as f32),
        egui::pos2(bounding_rect.right() as f32, bounding_rect.bottom() as f32),
    );

    // We need to subtract padding and border:
    if let Some(window) = web_sys::window() {
        if let Ok(Some(style)) = window.get_computed_style(canvas) {
            let get_property = |name: &str| -> Option<f32> {
                let property = style.get_property_value(name).ok()?;
                property.trim_end_matches("px").parse::<f32>().ok()
            };

            rect.min.x += get_property("padding-left").unwrap_or_default();
            rect.min.y += get_property("padding-top").unwrap_or_default();
            rect.max.x -= get_property("padding-right").unwrap_or_default();
            rect.max.y -= get_property("padding-bottom").unwrap_or_default();
        }
    }

    rect
}

fn canvas_size_in_points(canvas: &web_sys::HtmlCanvasElement, ctx: &egui::Context) -> egui::Vec2 {
    let pixels_per_point = ctx.pixels_per_point();
    egui::vec2(
        canvas.width() as f32 / pixels_per_point,
        canvas.height() as f32 / pixels_per_point,
    )
}

// ----------------------------------------------------------------------------

/// Set the cursor icon.
fn set_cursor_icon(cursor: egui::CursorIcon) -> Option<()> {
    let document = web_sys::window()?.document()?;
    document
        .body()?
        .style()
        .set_property("cursor", cursor_web_name(cursor))
        .ok()
}

/// Set the clipboard text.
#[cfg(web_sys_unstable_apis)]
fn set_clipboard_text(s: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(clipboard) = window.navigator().clipboard() {
            let promise = clipboard.write_text(s);
            let future = wasm_bindgen_futures::JsFuture::from(promise);
            let future = async move {
                if let Err(err) = future.await {
                    log::error!("Copy/cut action failed: {}", string_from_js_value(&err));
                }
            };
            wasm_bindgen_futures::spawn_local(future);
        } else {
            let is_secure_context = window.is_secure_context();
            if is_secure_context {
                log::warn!("window.navigator.clipboard is null; can't copy text");
            } else {
                log::warn!("window.navigator.clipboard is null; can't copy text, probably because we're not in a secure context. See https://developer.mozilla.org/en-US/docs/Web/Security/Secure_Contexts");
            }
        }
    }
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

        egui::CursorIcon::ResizeEast => "e-resize",
        egui::CursorIcon::ResizeSouthEast => "se-resize",
        egui::CursorIcon::ResizeSouth => "s-resize",
        egui::CursorIcon::ResizeSouthWest => "sw-resize",
        egui::CursorIcon::ResizeWest => "w-resize",
        egui::CursorIcon::ResizeNorthWest => "nw-resize",
        egui::CursorIcon::ResizeNorth => "n-resize",
        egui::CursorIcon::ResizeNorthEast => "ne-resize",
        egui::CursorIcon::ResizeColumn => "col-resize",
        egui::CursorIcon::ResizeRow => "row-resize",

        egui::CursorIcon::Text => "text",
        egui::CursorIcon::VerticalText => "vertical-text",
        egui::CursorIcon::Wait => "wait",
        egui::CursorIcon::ZoomIn => "zoom-in",
        egui::CursorIcon::ZoomOut => "zoom-out",
    }
}

/// Open the given url in the browser.
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

/// Percent-decodes a string.
pub fn percent_decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .to_string()
}
