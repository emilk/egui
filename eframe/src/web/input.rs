use super::{canvas_element, canvas_origin, AppRunner};

pub fn pos_from_mouse_event(canvas_id: &str, event: &web_sys::MouseEvent) -> egui::Pos2 {
    let canvas = canvas_element(canvas_id).unwrap();
    let rect = canvas.get_bounding_client_rect();
    egui::Pos2 {
        x: event.client_x() as f32 - rect.left() as f32,
        y: event.client_y() as f32 - rect.top() as f32,
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
    canvas_id: &str,
    event: &web_sys::TouchEvent,
    touch_id_for_pos: &mut Option<egui::TouchId>,
) -> egui::Pos2 {
    let touch_for_pos = if let Some(touch_id_for_pos) = touch_id_for_pos {
        // search for the touch we previously used for the position
        // (unfortunately, `event.touches()` is not a rust collection):
        (0..event.touches().length())
            .into_iter()
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
            pos_from_touch(canvas_origin(canvas_id), &touch)
        })
}

fn pos_from_touch(canvas_origin: egui::Pos2, touch: &web_sys::Touch) -> egui::Pos2 {
    egui::Pos2 {
        x: touch.page_x() as f32 - canvas_origin.x as f32,
        y: touch.page_y() as f32 - canvas_origin.y as f32,
    }
}

pub fn push_touches(runner: &mut AppRunner, phase: egui::TouchPhase, event: &web_sys::TouchEvent) {
    let canvas_origin = canvas_origin(runner.canvas_id());
    for touch_idx in 0..event.changed_touches().length() {
        if let Some(touch) = event.changed_touches().item(touch_idx) {
            runner.input.raw.events.push(egui::Event::Touch {
                device_id: egui::TouchDeviceId(0),
                id: egui::TouchId::from(touch.identifier()),
                phase,
                pos: pos_from_touch(canvas_origin, &touch),
                force: touch.force(),
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

/// Web sends all all keys as strings, so it is up to us to figure out if it is
/// a real text input or the name of a key.
pub fn translate_key(key: &str) -> Option<egui::Key> {
    match key {
        "ArrowDown" => Some(egui::Key::ArrowDown),
        "ArrowLeft" => Some(egui::Key::ArrowLeft),
        "ArrowRight" => Some(egui::Key::ArrowRight),
        "ArrowUp" => Some(egui::Key::ArrowUp),

        "Esc" | "Escape" => Some(egui::Key::Escape),
        "Tab" => Some(egui::Key::Tab),
        "Backspace" => Some(egui::Key::Backspace),
        "Enter" => Some(egui::Key::Enter),
        "Space" | " " => Some(egui::Key::Space),

        "Help" | "Insert" => Some(egui::Key::Insert),
        "Delete" => Some(egui::Key::Delete),
        "Home" => Some(egui::Key::Home),
        "End" => Some(egui::Key::End),
        "PageUp" => Some(egui::Key::PageUp),
        "PageDown" => Some(egui::Key::PageDown),

        "0" => Some(egui::Key::Num0),
        "1" => Some(egui::Key::Num1),
        "2" => Some(egui::Key::Num2),
        "3" => Some(egui::Key::Num3),
        "4" => Some(egui::Key::Num4),
        "5" => Some(egui::Key::Num5),
        "6" => Some(egui::Key::Num6),
        "7" => Some(egui::Key::Num7),
        "8" => Some(egui::Key::Num8),
        "9" => Some(egui::Key::Num9),

        "a" | "A" => Some(egui::Key::A),
        "b" | "B" => Some(egui::Key::B),
        "c" | "C" => Some(egui::Key::C),
        "d" | "D" => Some(egui::Key::D),
        "e" | "E" => Some(egui::Key::E),
        "f" | "F" => Some(egui::Key::F),
        "g" | "G" => Some(egui::Key::G),
        "h" | "H" => Some(egui::Key::H),
        "i" | "I" => Some(egui::Key::I),
        "j" | "J" => Some(egui::Key::J),
        "k" | "K" => Some(egui::Key::K),
        "l" | "L" => Some(egui::Key::L),
        "m" | "M" => Some(egui::Key::M),
        "n" | "N" => Some(egui::Key::N),
        "o" | "O" => Some(egui::Key::O),
        "p" | "P" => Some(egui::Key::P),
        "q" | "Q" => Some(egui::Key::Q),
        "r" | "R" => Some(egui::Key::R),
        "s" | "S" => Some(egui::Key::S),
        "t" | "T" => Some(egui::Key::T),
        "u" | "U" => Some(egui::Key::U),
        "v" | "V" => Some(egui::Key::V),
        "w" | "W" => Some(egui::Key::W),
        "x" | "X" => Some(egui::Key::X),
        "y" | "Y" => Some(egui::Key::Y),
        "z" | "Z" => Some(egui::Key::Z),

        "F1" => Some(egui::Key::F1),
        "F2" => Some(egui::Key::F2),
        "F3" => Some(egui::Key::F3),
        "F4" => Some(egui::Key::F4),
        "F5" => Some(egui::Key::F5),
        "F6" => Some(egui::Key::F6),
        "F7" => Some(egui::Key::F7),
        "F8" => Some(egui::Key::F8),
        "F9" => Some(egui::Key::F9),
        "F10" => Some(egui::Key::F10),
        "F11" => Some(egui::Key::F11),
        "F12" => Some(egui::Key::F12),
        "F13" => Some(egui::Key::F13),
        "F14" => Some(egui::Key::F14),
        "F15" => Some(egui::Key::F15),
        "F16" => Some(egui::Key::F16),
        "F17" => Some(egui::Key::F17),
        "F18" => Some(egui::Key::F18),
        "F19" => Some(egui::Key::F19),
        "F20" => Some(egui::Key::F20),

        _ => None,
    }
}

pub fn modifiers_from_event(event: &web_sys::KeyboardEvent) -> egui::Modifiers {
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
