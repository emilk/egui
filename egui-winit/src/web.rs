use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

///
static AGENT_ID: &str = "egui_text_agent";
///
/// Text event handler,
fn install_text_agent(runner_ref: &AppRunnerRef) -> Result<(), JsValue> {
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
    {
        // When IME is off
        let input_clone = input.clone();
        let runner_ref = runner_ref.clone();
        let is_composing = is_composing.clone();
        let on_input = Closure::wrap(Box::new(move |_event: web_sys::InputEvent| {
            let text = input_clone.value();
            if !text.is_empty() && !is_composing.get() {
                input_clone.set_value("");
                let mut runner_lock = runner_ref.0.lock();
                runner_lock.input.raw.events.push(egui::Event::Text(text));
                runner_lock.needs_repaint.set_true();
            }
        }) as Box<dyn FnMut(_)>);
        input.add_event_listener_with_callback("input", on_input.as_ref().unchecked_ref())?;
        on_input.forget();
    }
    {
        // When IME is on, handle composition event
        let input_clone = input.clone();
        let runner_ref = runner_ref.clone();
        let on_compositionend = Closure::wrap(Box::new(move |event: web_sys::CompositionEvent| {
            let mut runner_lock = runner_ref.0.lock();
            let opt_event = match event.type_().as_ref() {
                "compositionstart" => {
                    is_composing.set(true);
                    input_clone.set_value("");
                    Some(egui::Event::CompositionStart)
                }
                "compositionend" => {
                    is_composing.set(false);
                    input_clone.set_value("");
                    event.data().map(egui::Event::CompositionEnd)
                }
                "compositionupdate" => event.data().map(egui::Event::CompositionUpdate),
                s => {
                    web_sys::console::error_1(
                        &format!("Unknown composition event type: {:?}", s).into(),
                    );
                    None
                }
            };
            if let Some(event) = opt_event {
                runner_lock.input.raw.events.push(event);
                runner_lock.needs_repaint.set_true();
            }
        }) as Box<dyn FnMut(_)>);
        let f = on_compositionend.as_ref().unchecked_ref();
        input.add_event_listener_with_callback("compositionstart", f)?;
        input.add_event_listener_with_callback("compositionupdate", f)?;
        input.add_event_listener_with_callback("compositionend", f)?;
        on_compositionend.forget();
    }
    {
        // When input lost focus, focus on it again.
        // It is useful when user click somewhere outside canvas.
        let on_focusout = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            // Delay 10 ms, and focus again.
            let func = js_sys::Function::new_no_args(&format!(
                "document.getElementById('{}').focus()",
                AGENT_ID
            ));
            window
                .set_timeout_with_callback_and_timeout_and_arguments_0(&func, 10)
                .unwrap();
        }) as Box<dyn FnMut(_)>);
        input.add_event_listener_with_callback("focusout", on_focusout.as_ref().unchecked_ref())?;
        on_focusout.forget();
    }
    body.append_child(&input)?;
    Ok(())
}
