//! The text agent is a hidden `<input>` element used to capture
//! IME and mobile keyboard input events.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use wasm_bindgen::prelude::*;
use web_sys::{Document, Node};

use super::{AppRunner, WebRunner};

pub struct TextAgent {
    input: web_sys::HtmlInputElement,
    input_state: Rc<RefCell<InputState>>,
    prev_ime_output: Cell<Option<egui::output::IMEOutput>>,
}

struct InputState {
    input: web_sys::HtmlInputElement,
    last_text: String,
}

impl InputState {
    fn new(input: web_sys::HtmlInputElement) -> Self {
        Self {
            input,
            last_text: String::new(),
        }
    }

    fn clear(&mut self) {
        self.input.set_value("");
        self.last_text.clear();
    }
}

impl TextAgent {
    /// Attach the agent to the document.
    pub fn attach(runner_ref: &WebRunner, root: Node) -> Result<Self, JsValue> {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();

        // create an `<input>` element
        let input = document
            .create_element("input")?
            .dyn_into::<web_sys::HtmlElement>()?;
        input.set_autofocus(true)?;
        let input = input.dyn_into::<web_sys::HtmlInputElement>()?;
        input.set_type("text");
        input.set_attribute("autocapitalize", "off")?;
        let input_state = Rc::new(RefCell::new(InputState::new(input.clone())));

        // append it to `<body>` and hide it outside of the viewport
        let style = input.style();
        style.set_property("background-color", "transparent")?;
        style.set_property("border", "none")?;
        style.set_property("outline", "none")?;
        style.set_property("width", "1px")?;
        style.set_property("height", "1px")?;
        style.set_property("caret-color", "transparent")?;
        style.set_property("position", "absolute")?;
        style.set_property("top", "0")?;
        style.set_property("left", "0")?;

        if root.has_type::<Document>() {
            // root object is a document, append to its body
            root.dyn_into::<Document>()?
                .body()
                .unwrap()
                .append_child(&input)?;
        } else {
            // append input into root directly
            root.append_child(&input)?;
        }

        // attach event listeners

        let on_input = {
            let input = input.clone();
            let input_state = Rc::clone(&input_state);
            move |event: web_sys::InputEvent, runner: &mut AppRunner| {
                let mut input_state_ref = input_state.borrow_mut();
                if !event.is_composing() && event.input_type() != "insertText" {
                    input_state_ref.clear();

                    return;
                }

                let text = input.value();

                let prefix_len = longest_common_prefix_length(&text, &input_state_ref.last_text);
                let last_text_len = input_state_ref.last_text.chars().count();
                if prefix_len < last_text_len {
                    let out_event = egui::Event::Ime(egui::ImeEvent::DeleteSurrounding {
                        before_chars: last_text_len - prefix_len,
                        after_chars: 0,
                    });
                    runner.input.raw.events.push(out_event);
                }

                let preedit_text = text.chars().skip(prefix_len).collect();
                let out_event = if event.is_composing() {
                    egui::Event::Ime(egui::ImeEvent::Preedit(preedit_text))
                } else {
                    egui::Event::Text(preedit_text)
                };
                runner.input.raw.events.push(out_event);

                if event.is_composing() {
                    input_state_ref.last_text = text.chars().take(prefix_len).collect();
                } else {
                    input_state_ref.last_text = text;
                }

                runner.needs_repaint.repaint_asap();
            }
        };

        let on_composition_start = {
            move |_: web_sys::CompositionEvent, runner: &mut AppRunner| {
                // Repaint moves the text agent into place,
                // see `move_to` in `AppRunner::handle_platform_output`.
                runner.needs_repaint.repaint_asap();
            }
        };

        let on_composition_end = {
            let input = input.clone();
            let input_state = Rc::clone(&input_state);
            move |_event: web_sys::CompositionEvent, runner: &mut AppRunner| {
                let mut input_state_ref = input_state.borrow_mut();

                let text = input.value();

                let commit_text = {
                    let prefix_len = input_state_ref.last_text.chars().count();
                    text.chars().skip(prefix_len).collect::<String>()
                };
                let out_event = egui::Event::Ime(egui::ImeEvent::Commit(commit_text));
                runner.input.raw.events.push(out_event);

                input_state_ref.last_text = text;

                runner.needs_repaint.repaint_asap();
            }
        };

        let on_keydown = {
            let input_state = Rc::clone(&input_state);
            move |event: web_sys::KeyboardEvent, runner: &mut AppRunner| {
                let mut input_state_ref = input_state.borrow_mut();

                // https://web.archive.org/web/20200526195704/https://www.fxsitecompat.dev/en-CA/docs/2018/keydown-and-keyup-events-are-now-fired-during-ime-composition/
                if event.is_composing() || event.key_code() == 229 {
                    return;
                }

                if event.key().chars().count() > 1
                    || event.ctrl_key()
                    || event.alt_key()
                    || event.meta_key()
                {
                    input_state_ref.clear();
                }

                // The canvas doesn't get keydown/keyup events when the text agent is focused,
                // so we need to forward them to the runner:
                super::events::on_keydown(event, runner);
            }
        };
        let on_keyup = {
            move |event: web_sys::KeyboardEvent, runner: &mut AppRunner| {
                // https://web.archive.org/web/20200526195704/https://www.fxsitecompat.dev/en-CA/docs/2018/keydown-and-keyup-events-are-now-fired-during-ime-composition/
                if event.is_composing() || event.key_code() == 229 {
                    return;
                }

                // The canvas doesn't get keydown/keyup events when the text agent is focused,
                // so we need to forward them to the runner:
                super::events::on_keyup(event, runner);
            }
        };

        runner_ref.add_event_listener(&input, "input", on_input)?;
        runner_ref.add_event_listener(&input, "compositionstart", on_composition_start)?;
        runner_ref.add_event_listener(&input, "compositionend", on_composition_end)?;

        runner_ref.add_event_listener(&input, "keyup", on_keyup)?;
        runner_ref.add_event_listener(&input, "keydown", on_keydown)?;

        Ok(Self {
            input,
            input_state,
            prev_ime_output: Default::default(),
        })
    }

    pub fn move_to(
        &self,
        ime: Option<egui::output::IMEOutput>,
        canvas: &web_sys::HtmlCanvasElement,
        zoom_factor: f32,
    ) -> Result<(), JsValue> {
        // Don't move the text agent unless the position actually changed:
        if self.prev_ime_output.get() == ime {
            return Ok(());
        }
        self.prev_ime_output.set(ime);

        let Some(ime) = ime else { return Ok(()) };

        if ime.should_interrupt_composition {
            // no-op for now: currently, the text agent is sizeless, so any
            // click shifts focus to the canvas, which naturally interrupts the
            // composition.
        }

        let mut canvas_rect = super::canvas_content_rect(canvas);
        // Fix for safari with virtual keyboard flapping position
        if is_mobile_safari() {
            canvas_rect.min.y = canvas.offset_top() as f32;
        }
        let cursor_rect = ime.cursor_rect.translate(canvas_rect.min.to_vec2());

        let style = self.input.style();

        // This is where the IME input will point to:
        style.set_property(
            "left",
            &format!("{}px", cursor_rect.center().x * zoom_factor),
        )?;
        style.set_property(
            "top",
            &format!("{}px", cursor_rect.center().y * zoom_factor),
        )?;

        Ok(())
    }

    pub fn set_focus(&self, on: bool) {
        if on {
            self.focus();
        } else {
            self.blur();
        }
    }

    pub fn has_focus(&self) -> bool {
        super::has_focus(&self.input)
    }

    pub fn focus(&self) {
        if self.has_focus() {
            return;
        }

        log::trace!("Focusing text agent");

        if let Err(err) = self.input.focus() {
            log::error!("failed to set focus: {}", super::string_from_js_value(&err));
        }
    }

    pub fn blur(&self) {
        if !self.has_focus() {
            return;
        }

        log::trace!("Blurring text agent");

        if let Err(err) = self.input.blur() {
            log::error!("failed to set focus: {}", super::string_from_js_value(&err));
        }
        self.input_state.borrow_mut().clear();
    }
}

impl Drop for TextAgent {
    fn drop(&mut self) {
        self.input.remove();
    }
}

/// Returns `true` if the app is likely running on a mobile device on navigator Safari.
fn is_mobile_safari() -> bool {
    (|| {
        let user_agent = web_sys::window()?.navigator().user_agent().ok()?;
        let is_ios = user_agent.contains("iPhone")
            || user_agent.contains("iPad")
            || user_agent.contains("iPod");
        let is_safari = user_agent.contains("Safari");
        Some(is_ios && is_safari)
    })()
    .unwrap_or(false)
}

fn longest_common_prefix_length(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars()).take_while(|(a, b)| a == b).count()
}
