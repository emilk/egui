//! [`egui`] bindings for web apps (compiling to WASM).

#![expect(clippy::missing_errors_doc)] // So many `-> Result<_, JsValue>`
#![expect(clippy::unwrap_used)] // TODO(emilk): remove unwraps

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

#[cfg(not(any(feature = "glow", feature = "wgpu_no_default_features")))]
compile_error!("You must enable either the 'glow' or 'wgpu' feature");

mod web_painter;

#[cfg(feature = "glow")]
mod web_painter_glow;

#[cfg(feature = "wgpu_no_default_features")]
mod web_painter_wgpu;

pub use backend::*;

use egui::Theme;
use wasm_bindgen::prelude::*;
use web_sys::{Document, MediaQueryList, Node};

use input::{
    button_from_mouse_event, modifiers_from_kb_event, modifiers_from_mouse_event,
    modifiers_from_wheel_event, pos_from_mouse_event, primary_touch_pos, push_touches,
    text_from_keyboard_event, translate_key,
};

// ----------------------------------------------------------------------------

/// Debug browser resizing?
const DEBUG_RESIZE: bool = false;

pub(crate) fn string_from_js_value(value: &JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:#?}"))
}

/// Returns the `Element` with active focus.
///
/// Elements can only be focused if they are:
/// - `<a>`/`<area>` with an `href` attribute
/// - `<input>`/`<select>`/`<textarea>`/`<button>` which aren't `disabled`
/// - any other element with a `tabindex` attribute
pub(crate) fn focused_element(root: &Node) -> Option<web_sys::Element> {
    if let Some(document) = root.dyn_ref::<Document>() {
        document.active_element()
    } else if let Some(shadow) = root.dyn_ref::<web_sys::ShadowRoot>() {
        shadow.active_element()
    } else {
        None
    }
}

pub(crate) fn has_focus<T: JsCast>(element: &T) -> bool {
    fn try_has_focus<T: JsCast>(element: &T) -> Option<bool> {
        let element = element.dyn_ref::<web_sys::Element>()?;
        let root = element.get_root_node();

        let focused_element = focused_element(&root)?;
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
    let window = web_sys::window()?;
    if does_prefer_color_scheme(&window, Theme::Dark) == Some(true) {
        Some(Theme::Dark)
    } else if does_prefer_color_scheme(&window, Theme::Light) == Some(true) {
        Some(Theme::Light)
    } else {
        None
    }
}

fn does_prefer_color_scheme(window: &web_sys::Window, theme: Theme) -> Option<bool> {
    Some(prefers_color_scheme(window, theme).ok()??.matches())
}

fn prefers_color_scheme(
    window: &web_sys::Window,
    theme: Theme,
) -> Result<Option<MediaQueryList>, JsValue> {
    let theme = match theme {
        Theme::Dark => "dark",
        Theme::Light => "light",
    };
    window.match_media(format!("(prefers-color-scheme: {theme})").as_str())
}

/// Returns the canvas in client coordinates.
fn canvas_content_rect(canvas: &web_sys::HtmlCanvasElement) -> egui::Rect {
    let bounding_rect = canvas.get_bounding_client_rect();

    let mut rect = egui::Rect::from_min_max(
        egui::pos2(bounding_rect.left() as f32, bounding_rect.top() as f32),
        egui::pos2(bounding_rect.right() as f32, bounding_rect.bottom() as f32),
    );

    // We need to subtract padding and border:
    if let Some(window) = web_sys::window()
        && let Ok(Some(style)) = window.get_computed_style(canvas)
    {
        let get_property = |name: &str| -> Option<f32> {
            let property = style.get_property_value(name).ok()?;
            property.trim_end_matches("px").parse::<f32>().ok()
        };

        rect.min.x += get_property("padding-left").unwrap_or_default();
        rect.min.y += get_property("padding-top").unwrap_or_default();
        rect.max.x -= get_property("padding-right").unwrap_or_default();
        rect.max.y -= get_property("padding-bottom").unwrap_or_default();
    }

    rect
}

fn canvas_size_in_points(canvas: &web_sys::HtmlCanvasElement, ctx: &egui::Context) -> egui::Vec2 {
    // ctx.pixels_per_point can be outdated

    let pixels_per_point = ctx.zoom_factor() * native_pixels_per_point();

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
fn set_clipboard_text(s: &str) {
    if let Some(window) = web_sys::window() {
        if !window.is_secure_context() {
            log::error!(
                "Clipboard is not available because we are not in a secure context. \
                See https://developer.mozilla.org/en-US/docs/Web/Security/Secure_Contexts"
            );
            return;
        }
        let promise = window.navigator().clipboard().write_text(s);
        let future = wasm_bindgen_futures::JsFuture::from(promise);
        let future = async move {
            if let Err(err) = future.await {
                log::error!("Copy/cut action failed: {}", string_from_js_value(&err));
            }
        };
        wasm_bindgen_futures::spawn_local(future);
    }
}

/// Set the clipboard image.
fn set_clipboard_image(image: &egui::ColorImage) {
    if let Some(window) = web_sys::window() {
        if !window.is_secure_context() {
            log::error!(
                "Clipboard is not available because we are not in a secure context. \
                See https://developer.mozilla.org/en-US/docs/Web/Security/Secure_Contexts"
            );
            return;
        }

        let png_bytes = to_image(image).and_then(|image| to_png_bytes(&image));
        let png_bytes = match png_bytes {
            Ok(png_bytes) => png_bytes,
            Err(err) => {
                log::error!("Failed to encode image to png: {err}");
                return;
            }
        };

        let mime = "image/png";

        let item = match create_clipboard_item(mime, &png_bytes) {
            Ok(item) => item,
            Err(err) => {
                log::error!("Failed to copy image: {}", string_from_js_value(&err));
                return;
            }
        };
        let items = js_sys::Array::of1(&item);
        let promise = window.navigator().clipboard().write(&items);
        let future = wasm_bindgen_futures::JsFuture::from(promise);
        let future = async move {
            if let Err(err) = future.await {
                log::error!(
                    "Copy/cut image action failed: {}",
                    string_from_js_value(&err)
                );
            }
        };
        wasm_bindgen_futures::spawn_local(future);
    }
}

fn to_image(image: &egui::ColorImage) -> Result<image::RgbaImage, String> {
    profiling::function_scope!();
    image::RgbaImage::from_raw(
        image.width() as _,
        image.height() as _,
        bytemuck::cast_slice(&image.pixels).to_vec(),
    )
    .ok_or_else(|| "Invalid IconData".to_owned())
}

fn to_png_bytes(image: &image::RgbaImage) -> Result<Vec<u8>, String> {
    profiling::function_scope!();
    let mut png_bytes: Vec<u8> = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .map_err(|err| err.to_string())?;
    Ok(png_bytes)
}

fn create_clipboard_item(mime: &str, bytes: &[u8]) -> Result<web_sys::ClipboardItem, JsValue> {
    let array = js_sys::Uint8Array::from(bytes);
    let blob_parts = js_sys::Array::new();
    blob_parts.push(&array);

    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime);

    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&blob_parts, &options)?;

    let items = js_sys::Object::new();

    #[expect(unsafe_code, unused_unsafe)] // Weird false positive
    // SAFETY: I hope so
    unsafe {
        js_sys::Reflect::set(&items, &JsValue::from_str(mime), &blob)?
    };

    let clipboard_item = web_sys::ClipboardItem::new_with_record_from_str_to_blob_promise(&items)?;

    Ok(clipboard_item)
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

/// Are we running inside the Safari browser?
pub fn is_safari_browser() -> bool {
    web_sys::window().is_some_and(|window| window.has_own_property(&JsValue::from("safari")))
}
