//! The text agent is an `<input>` element used to trigger
//! mobile keyboard and IME input.

use super::{canvas_element, AppRunner, AppRunnerContainer};
use egui::mutex::MutexGuard;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

static AGENT_ID: &str = "egui_text_agent";

pub fn text_agent() -> web_sys::HtmlInputElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(AGENT_ID)
        .unwrap()
        .dyn_into()
        .unwrap()
}

/// Text event handler,
pub fn install_text_agent(runner_container: &mut AppRunnerContainer) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().expect("document should have a body");
    let input = document
        .create_element("input")?
        .dyn_into::<web_sys::HtmlInputElement>()?;
    let input = std::rc::Rc::new(input);
    input.set_id(AGENT_ID);
    let is_composing = Rc::new(Cell::new(false));
    {
        let style = input.style();
        // Transparent
        style.set_property("opacity", "0").unwrap();
        // Hide under canvas
        style.set_property("z-index", "-1").unwrap();
    }
    // Set size as small as possible, in case user may click on it.
    input.set_size(1);
    input.set_autofocus(true);
    input.set_hidden(true);

    // When IME is off
    runner_container.add_event_listener(&input, "input", {
        let input_clone = input.clone();
        let is_composing = is_composing.clone();

        move |_event: web_sys::InputEvent, mut runner_lock| {
            let text = input_clone.value();
            if !text.is_empty() && !is_composing.get() {
                input_clone.set_value("");
                runner_lock.input.raw.events.push(egui::Event::Text(text));
                runner_lock.needs_repaint.repaint_asap();
            }
        }
    })?;

    {
        // When IME is on, handle composition event
        runner_container.add_event_listener(&input, "compositionstart", {
            let input_clone = input.clone();
            let is_composing = is_composing.clone();

            move |_event: web_sys::CompositionEvent, mut runner_lock: MutexGuard<'_, AppRunner>| {
                is_composing.set(true);
                input_clone.set_value("");

                runner_lock
                    .input
                    .raw
                    .events
                    .push(egui::Event::CompositionStart);
                runner_lock.needs_repaint.repaint_asap();
            }
        })?;

        runner_container.add_event_listener(
            &input,
            "compositionupdate",
            move |event: web_sys::CompositionEvent, mut runner_lock: MutexGuard<'_, AppRunner>| {
                if let Some(event) = event.data().map(egui::Event::CompositionUpdate) {
                    runner_lock.input.raw.events.push(event);
                    runner_lock.needs_repaint.repaint_asap();
                }
            },
        )?;

        runner_container.add_event_listener(&input, "compositionend", {
            let input_clone = input.clone();

            move |event: web_sys::CompositionEvent, mut runner_lock: MutexGuard<'_, AppRunner>| {
                is_composing.set(false);
                input_clone.set_value("");

                if let Some(event) = event.data().map(egui::Event::CompositionEnd) {
                    runner_lock.input.raw.events.push(event);
                    runner_lock.needs_repaint.repaint_asap();
                }
            }
        })?;
    }

    // When input lost focus, focus on it again.
    // It is useful when user click somewhere outside canvas.
    runner_container.add_event_listener(
        &input,
        "focusout",
        move |_event: web_sys::MouseEvent, _| {
            // Delay 10 ms, and focus again.
            let func = js_sys::Function::new_no_args(&format!(
                "document.getElementById('{}').focus()",
                AGENT_ID
            ));
            window
                .set_timeout_with_callback_and_timeout_and_arguments_0(&func, 10)
                .unwrap();
        },
    )?;

    body.append_child(&input)?;

    Ok(())
}

/// Focus or blur text agent to toggle mobile keyboard.
pub fn update_text_agent(runner: MutexGuard<'_, AppRunner>) -> Option<()> {
    use web_sys::HtmlInputElement;
    let window = web_sys::window()?;
    let document = window.document()?;
    let input: HtmlInputElement = document.get_element_by_id(AGENT_ID)?.dyn_into().unwrap();
    let canvas_style = canvas_element(runner.canvas_id())?.style();

    if runner.mutable_text_under_cursor {
        let is_already_editing = input.hidden();
        if is_already_editing {
            input.set_hidden(false);
            input.focus().ok()?;

            // Move up canvas so that text edit is shown at ~30% of screen height.
            // Only on touch screens, when keyboard popups.
            if let Some(latest_touch_pos) = runner.input.latest_touch_pos {
                let window_height = window.inner_height().ok()?.as_f64()? as f32;
                let current_rel = latest_touch_pos.y / window_height;

                // estimated amount of screen covered by keyboard
                let keyboard_fraction = 0.5;

                if current_rel > keyboard_fraction {
                    // below the keyboard

                    let target_rel = 0.3;

                    // Note: `delta` is negative, since we are moving the canvas UP
                    let delta = target_rel - current_rel;

                    let delta = delta.max(-keyboard_fraction); // Don't move it crazy much

                    let new_pos_percent = format!("{}%", (delta * 100.0).round());

                    canvas_style.set_property("position", "absolute").ok()?;
                    canvas_style.set_property("top", &new_pos_percent).ok()?;
                }
            }
        }
    } else {
        // Drop runner lock
        drop(runner);

        // Holding the runner lock while calling input.blur() causes a panic.
        // This is most probably caused by the browser running the event handler
        // for the triggered blur event synchronously, meaning that the mutex
        // lock does not get dropped by the time another event handler is called.
        //
        // Why this didn't exist before #1290 is a mystery to me, but it exists now
        // and this apparently is the fix for it
        //
        // ¯\_(ツ)_/¯ - @DusterTheFirst
        input.blur().ok()?;

        input.set_hidden(true);
        canvas_style.set_property("position", "absolute").ok()?;
        canvas_style.set_property("top", "0%").ok()?; // move back to normal position
    }
    Some(())
}

/// If context is running under mobile device?
fn is_mobile() -> Option<bool> {
    const MOBILE_DEVICE: [&str; 6] = ["Android", "iPhone", "iPad", "iPod", "webOS", "BlackBerry"];

    let user_agent = web_sys::window()?.navigator().user_agent().ok()?;
    let is_mobile = MOBILE_DEVICE.iter().any(|&name| user_agent.contains(name));
    Some(is_mobile)
}

// Move text agent to text cursor's position, on desktop/laptop,
// candidate window moves following text element (agent),
// so it appears that the IME candidate window moves with text cursor.
// On mobile devices, there is no need to do that.
pub fn move_text_cursor(cursor: Option<egui::Pos2>, canvas_id: &str) -> Option<()> {
    let style = text_agent().style();
    // Note: movint agent on mobile devices will lead to unpredictable scroll.
    if is_mobile() == Some(false) {
        cursor.as_ref().and_then(|&egui::Pos2 { x, y }| {
            let canvas = canvas_element(canvas_id)?;
            let bounding_rect = text_agent().get_bounding_client_rect();
            let y = (y + (canvas.scroll_top() + canvas.offset_top()) as f32)
                .min(canvas.client_height() as f32 - bounding_rect.height() as f32);
            let x = x + (canvas.scroll_left() + canvas.offset_left()) as f32;
            // Canvas is translated 50% horizontally in html.
            let x = (x - canvas.offset_width() as f32 / 2.0)
                .min(canvas.client_width() as f32 - bounding_rect.width() as f32);
            style.set_property("position", "absolute").ok()?;
            style.set_property("top", &format!("{}px", y)).ok()?;
            style.set_property("left", &format!("{}px", x)).ok()
        })
    } else {
        style.set_property("position", "absolute").ok()?;
        style.set_property("top", "0px").ok()?;
        style.set_property("left", "0px").ok()
    }
}
