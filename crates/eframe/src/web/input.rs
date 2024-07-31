use super::{canvas_content_rect, AppRunner};

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
    let all_touches: Vec<_> = (0..event.touches().length())
        .filter_map(|i| event.touches().get(i))
        // On touchend we don't get anything in `touches`, but we still get `changed_touches`, so include those:
        .chain((0..event.changed_touches().length()).filter_map(|i| event.changed_touches().get(i)))
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
    let key = event.key();

    let is_function_key = key.starts_with('F') && key.len() > 1;
    if is_function_key {
        return None;
    }

    let is_control_key = matches!(
        key.as_str(),
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
    );

    if is_control_key {
        return None;
    }

    Some(key)
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
