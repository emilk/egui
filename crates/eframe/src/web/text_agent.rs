//! The text agent is a hidden `<input>` element used to capture
//! IME and mobile keyboard input events.

use std::cell::Cell;

use wasm_bindgen::prelude::*;
use web_sys::{Document, Node};

use super::{AppRunner, WebRunner};

pub struct TextAgent {
    input: web_sys::HtmlInputElement,
    prev_ime_output: Cell<Option<egui::output::IMEOutput>>,
}

impl TextAgent {
    /// Attach the agent to the document.
    pub fn attach(runner_ref: &WebRunner, root: Node) -> Result<Self, JsValue> {
        let document = web_sys::window().unwrap().document().unwrap();

        // create an `<input>` element
        let input = document
            .create_element("input")?
            .dyn_into::<web_sys::HtmlElement>()?;
        input.set_autofocus(true)?;
        let input = input.dyn_into::<web_sys::HtmlInputElement>()?;
        input.set_type("text");
        input.set_attribute("autocapitalize", "off")?;

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
            move |event: web_sys::InputEvent, runner: &mut AppRunner| {
                let text = input.value();
                // Workaround for an Android Gboard issue: after typing a word,
                // the user has to delete invisible characters (whose count
                // matches the length of the current suggestion) before actual
                // characters are deleted, unless the focus has been reset.
                //
                // this issue appears to have been fixed in Gboard sometime
                // between versions 14.7.09 and 17.0.12.
                if !event.is_composing() {
                    input.blur().ok();
                    input.focus().ok();
                }

                if event.is_composing() {
                    // if `is_composing` is true, then user is using IME, for
                    // example: emoji, pinyin, kanji, hangul, etc. In that case,
                    // the browser emits both `input` and `compositionupdate`
                    // events.
                    // We handle the composition update here instead of in the
                    // `compositionupdate` event because the selection range
                    // has not yet been updated when `compositionupdate` fires.

                    let Some(text) = event.data() else { return };
                    let selection_start = input
                        .selection_start()
                        .unwrap_or(None)
                        .map(|pos| pos as usize);
                    let selection_end = input
                        .selection_end()
                        .unwrap_or(None)
                        .map(|pos| pos as usize);
                    let active_range_chars = if let Some(selection_start) = selection_start
                        && let Some(selection_end) = selection_end
                    {
                        let text_utf16 = text.encode_utf16().collect::<Vec<u16>>();
                        let text_before_selection =
                            String::from_utf16_lossy(&text_utf16[..selection_start]);
                        let text_in_selection =
                            String::from_utf16_lossy(&text_utf16[selection_start..selection_end]);
                        let count_before_selection = text_before_selection.chars().count();
                        let count_in_selection = text_in_selection.chars().count();
                        Some(count_before_selection..count_before_selection + count_in_selection)
                    } else {
                        None
                    };
                    let event = egui::Event::Ime(egui::ImeEvent::Preedit {
                        text,
                        active_range_chars,
                    });
                    runner.input.raw.events.push(event);
                    runner.needs_repaint.repaint_asap();
                } else {
                    if text.is_empty() {
                        return;
                    }

                    if !event.is_composing() {
                        input.set_value("");
                        let event = egui::Event::Text(text);
                        runner.input.raw.events.push(event);
                        runner.needs_repaint.repaint_asap();
                    }
                }
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
            move |event: web_sys::CompositionEvent, runner: &mut AppRunner| {
                let Some(text) = event.data() else { return };
                input.set_value("");
                let event = egui::Event::Ime(egui::ImeEvent::Commit(text));
                runner.input.raw.events.push(event);
                runner.needs_repaint.repaint_asap();
            }
        };

        runner_ref.add_event_listener(&input, "input", on_input)?;
        runner_ref.add_event_listener(&input, "compositionstart", on_composition_start)?;
        runner_ref.add_event_listener(&input, "compositionend", on_composition_end)?;

        // The canvas doesn't get keydown/keyup events when the text agent is focused,
        // so we need to forward them to the runner:
        runner_ref.add_event_listener(&input, "keydown", super::events::on_keydown)?;
        runner_ref.add_event_listener(&input, "keyup", super::events::on_keyup)?;

        Ok(Self {
            input,
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
