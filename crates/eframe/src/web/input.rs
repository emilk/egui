use super::{canvas_origin, AppRunner};

pub fn pos_from_mouse_event(
    canvas: &web_sys::HtmlCanvasElement,
    event: &web_sys::MouseEvent,
    ctx: &egui::Context,
) -> egui::Pos2 {
    let rect = canvas.get_bounding_client_rect();
    let zoom_factor = ctx.zoom_factor();
    egui::Pos2 {
        x: (event.client_x() as f32 - rect.left() as f32) / zoom_factor,
        y: (event.client_y() as f32 - rect.top() as f32) / zoom_factor,
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
///
/// `touch_id_for_pos` is the [`TouchId`](egui::TouchId) of the [`Touch`](web_sys::Touch) we previously used to determine the
/// pointer position.
pub fn pos_from_touch_event(
    canvas: &web_sys::HtmlCanvasElement,
    event: &web_sys::TouchEvent,
    touch_id_for_pos: &mut Option<egui::TouchId>,
    egui_ctx: &egui::Context,
) -> egui::Pos2 {
    let touch_for_pos = if let Some(touch_id_for_pos) = touch_id_for_pos {
        // search for the touch we previously used for the position
        // (unfortunately, `event.touches()` is not a rust collection):
        (0..event.touches().length())
            .map(|i| event.touches().get(i).unwrap())
            .find(|touch| egui::TouchId::from(touch.identifier()) == *touch_id_for_pos)
    } else {
        None
    };
    // Use the touch found above or pick the first, or return a default position if there is no
    // touch at all. (The latter is not expected as the current method is only called when there is
    // at least one touch.)
    touch_for_pos
        .or_else(|| event.touches().get(0))
        .map_or(Default::default(), |touch| {
            *touch_id_for_pos = Some(egui::TouchId::from(touch.identifier()));
            pos_from_touch(canvas_origin(canvas), &touch, egui_ctx)
        })
}

fn pos_from_touch(
    canvas_origin: egui::Pos2,
    touch: &web_sys::Touch,
    egui_ctx: &egui::Context,
) -> egui::Pos2 {
    let zoom_factor = egui_ctx.zoom_factor();
    egui::Pos2 {
        x: (touch.page_x() as f32 - canvas_origin.x) / zoom_factor,
        y: (touch.page_y() as f32 - canvas_origin.y) / zoom_factor,
    }
}

pub fn push_touches(runner: &mut AppRunner, phase: egui::TouchPhase, event: &web_sys::TouchEvent) {
    let canvas_origin = canvas_origin(runner.canvas());
    for touch_idx in 0..event.changed_touches().length() {
        if let Some(touch) = event.changed_touches().item(touch_idx) {
            runner.input.raw.events.push(egui::Event::Touch {
                device_id: egui::TouchDeviceId(0),
                id: egui::TouchId::from(touch.identifier()),
                phase,
                pos: pos_from_touch(canvas_origin, &touch, runner.egui_ctx()),
                force: Some(touch.force()),
            });
        }
    }
}

/// Web sends all keys as strings, so it is up to us to figure out if it is
/// a real text input or the name of a key.
pub fn should_ignore_key(key: &str) -> bool {
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
                | "GroupNext" // https://github.com/emilk/egui/issues/510
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

/// Web sends all keys as strings, so it is up to us to figure out if it is
/// a real text input or the name of a key.
pub fn translate_key(key: &str) -> Option<egui::Key> {
    egui::Key::from_name(key)
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
