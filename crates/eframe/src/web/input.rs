use core::cell::{Cell, RefCell};
use std::rc::Rc;

use js_sys::Reflect;
use wasm_bindgen::{JsCast as _, JsValue};

use crate::web_keys::{KeyDown, KeyResolver};

use super::{AppRunner, canvas_content_rect};

pub fn pos_from_mouse_event(
    canvas: &web_sys::HtmlCanvasElement,
    event: &web_sys::MouseEvent,
    ctx: &egui::Context,
) -> egui::Pos2 {
    let rect = canvas_content_rect(canvas);
    let zoom_factor = ctx.zoom_factor();
    egui::Pos2 {
        x: (event.client_x() as f32 - rect.left()) / zoom_factor,
        y: (event.client_y() as f32 - rect.top()) / zoom_factor,
    }
}

pub fn button_from_mouse_event(event: &web_sys::MouseEvent) -> Option<egui::PointerButton> {
    match event.button() {
        0 => Some(egui::PointerButton::Primary),
        1 => Some(egui::PointerButton::Middle),
        2 => Some(egui::PointerButton::Secondary),
        3 => Some(egui::PointerButton::Extra1),
        4 => Some(egui::PointerButton::Extra2),
        _ => None,
    }
}

/// A single touch is translated to a pointer movement. When a second touch is added, the pointer
/// should not jump to a different position. Therefore, we do not calculate the average position
/// of all touches, but we keep using the same touch as long as it is available.
pub fn primary_touch_pos(
    runner: &mut AppRunner,
    event: &web_sys::TouchEvent,
) -> Option<(egui::Pos2, web_sys::Touch)> {
    // On touchend we don't get anything in `touches`, but we still get `changed_touches`, so include those:
    let all_touches: Vec<_> = core::iter::chain(
        (0..event.touches().length()).filter_map(|i| event.touches().get(i)),
        (0..event.changed_touches().length()).filter_map(|i| event.changed_touches().get(i)),
    )
    .collect();

    if let Some(primary_touch) = runner.input.primary_touch {
        // Is the primary touch is gone?
        if !all_touches
            .iter()
            .any(|touch| primary_touch == egui::TouchId::from(touch.identifier()))
        {
            runner.input.primary_touch = None;
        }
    }

    if runner.input.primary_touch.is_none() {
        runner.input.primary_touch = all_touches
            .first()
            .map(|touch| egui::TouchId::from(touch.identifier()));
    }

    let primary_touch = runner.input.primary_touch;

    if let Some(primary_touch) = primary_touch {
        for touch in all_touches {
            if primary_touch == egui::TouchId::from(touch.identifier()) {
                let canvas_rect = canvas_content_rect(runner.canvas());
                return Some((
                    pos_from_touch(canvas_rect, &touch, runner.egui_ctx()),
                    touch,
                ));
            }
        }
    }

    None
}

fn pos_from_touch(
    canvas_rect: egui::Rect,
    touch: &web_sys::Touch,
    egui_ctx: &egui::Context,
) -> egui::Pos2 {
    let zoom_factor = egui_ctx.zoom_factor();
    egui::Pos2 {
        x: (touch.client_x() as f32 - canvas_rect.left()) / zoom_factor,
        y: (touch.client_y() as f32 - canvas_rect.top()) / zoom_factor,
    }
}

pub fn push_touches(runner: &mut AppRunner, phase: egui::TouchPhase, event: &web_sys::TouchEvent) {
    let canvas_rect = canvas_content_rect(runner.canvas());
    for touch_idx in 0..event.changed_touches().length() {
        if let Some(touch) = event.changed_touches().item(touch_idx) {
            runner.input.raw.events.push(egui::Event::Touch {
                device_id: egui::TouchDeviceId(0),
                id: egui::TouchId::from(touch.identifier()),
                phase,
                pos: pos_from_touch(canvas_rect, &touch, runner.egui_ctx()),
                force: Some(touch.force()),
            });
        }
    }
}

/// The text input from a keyboard event (e.g. `X` when pressing the `X` key).
pub fn text_from_keyboard_event(event: &web_sys::KeyboardEvent) -> Option<String> {
    crate::web_keys::text_from_key(&event.key()).map(ToOwned::to_owned)
}

/// Turns keyboard events into egui keys; see [`KeyResolver`].
///
/// Clones share the resolver, so a pending layout-map read can update it.
#[derive(Clone, Default)]
pub(crate) struct KeyboardKeys {
    resolver: Rc<RefCell<KeyResolver>>,
    layout_map_pending: Rc<Cell<bool>>,
}

impl KeyboardKeys {
    pub fn key_down(&self, event: &web_sys::KeyboardEvent) -> KeyDown {
        let plain = !(event.shift_key()
            || event.ctrl_key()
            || event.alt_key()
            || event.meta_key()
            || event.get_modifier_state("AltGraph"));
        let down = self
            .resolver
            .borrow_mut()
            .key_down(&event.key(), &event.code(), plain);
        if down.query_layout_map {
            self.read_layout_map();
        }
        down
    }

    /// The logical key and the physical key of a `keyup`.
    pub fn key_up(&self, event: &web_sys::KeyboardEvent) -> (Option<egui::Key>, Option<egui::Key>) {
        self.resolver
            .borrow_mut()
            .key_up(&event.key(), &event.code())
    }

    pub fn clear_pressed(&self) {
        self.resolver.borrow_mut().clear_pressed();
    }

    /// Read the keyboard layout with `navigator.keyboard.getLayoutMap()`, where the browser has it
    /// (Chromium). The result arrives asynchronously.
    pub fn read_layout_map(&self) {
        if self.layout_map_pending.get() {
            return;
        }
        let Some(promise) = layout_map_promise() else {
            return;
        };
        self.layout_map_pending.set(true);

        let keys = self.clone();
        js_sys::futures::spawn_local(async move {
            match promise.await {
                Ok(map) => {
                    let letters = layout_map_letters(&map);
                    keys.resolver
                        .borrow_mut()
                        .set_layout_map(letters.iter().map(String::as_str));
                }
                Err(err) => {
                    // E.g. a `SecurityError` in a cross-origin iframe; typed letters decide instead.
                    log::debug!("getLayoutMap failed: {}", super::string_from_js_value(&err));
                }
            }
            keys.layout_map_pending.set(false);
        });
    }
}

fn layout_map_promise() -> Option<js_sys::Promise> {
    let navigator = web_sys::window()?.navigator();
    let keyboard = Reflect::get(&navigator, &JsValue::from_str("keyboard")).ok()?;
    if keyboard.is_undefined() || keyboard.is_null() {
        return None;
    }
    let get_layout_map = Reflect::get(&keyboard, &JsValue::from_str("getLayoutMap"))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    get_layout_map.call0(&keyboard).ok()?.dyn_into().ok()
}

/// What `KeyA` … `KeyZ` type according to a `KeyboardLayoutMap`.
fn layout_map_letters(map: &JsValue) -> Vec<String> {
    let Some(get) = Reflect::get(map, &JsValue::from_str("get"))
        .ok()
        .and_then(|get| get.dyn_into::<js_sys::Function>().ok())
    else {
        return vec![];
    };
    (b'A'..=b'Z')
        .filter_map(|letter| {
            let code = format!("Key{}", char::from(letter));
            get.call1(map, &JsValue::from_str(&code)).ok()?.as_string()
        })
        .collect()
}

pub fn modifiers_from_kb_event(event: &web_sys::KeyboardEvent) -> egui::Modifiers {
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

pub fn modifiers_from_mouse_event(event: &web_sys::MouseEvent) -> egui::Modifiers {
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

pub fn modifiers_from_wheel_event(event: &web_sys::WheelEvent) -> egui::Modifiers {
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
