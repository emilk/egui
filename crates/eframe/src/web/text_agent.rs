//! The text agent is a hidden `<input>` element used to capture
//! IME and mobile keyboard input events.

use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::prelude::*;
use web_sys::Document;

use super::{AppRunner, WebRunner};

pub struct TextAgent {
    input: web_sys::HtmlInputElement,
    input_state: Rc<RefCell<InputState>>,
}

impl TextAgent {
    /// Attach the agent to the document.
    pub fn attach(
        runner_ref: &WebRunner,
        canvas: &web_sys::HtmlCanvasElement,
    ) -> Result<Self, JsValue> {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();

        // create an `<input>` element
        let input = document
            .create_element("input")?
            .dyn_into::<web_sys::HtmlInputElement>()?;
        input.set_type("text");
        input.set_attribute("autocapitalize", "off")?;
        let input_state = Rc::new(RefCell::new(InputState::new(input.clone())));

        // Hide the element, and park it over the canvas
        // so that focusing it can never scroll some other part
        // of the page into view.
        let canvas_rect = super::canvas_content_rect(canvas);
        let style = input.style();
        style.set_property("background-color", "transparent")?;
        style.set_property("border", "none")?;
        style.set_property("outline", "none")?;
        style.set_property("width", "1px")?;
        style.set_property("height", "1px")?;
        style.set_property("caret-color", "transparent")?;
        style.set_property("position", "absolute")?;
        style.set_property("top", &format!("{}px", canvas_rect.min.y))?;
        style.set_property("left", &format!("{}px", canvas_rect.min.x))?;
        // Prevent auto-zoom on mobile browsers (requires at least 16px).
        style.set_property("font-size", "16px")?;

        let root = canvas.get_root_node();
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

        // Focus the app on startup, without scrolling the page.
        // We do this instead of setting the `autofocus` attribute,
        // since the browser scrolls the focused element into view when
        // honoring `autofocus`, and there is no way to prevent that.
        // See https://github.com/emilk/egui/issues/8295
        super::focus_without_scroll(&input).ok();

        // attach event listeners

        runner_ref.add_event_listener(
            &input,
            "compositionstart",
            move |_: web_sys::CompositionEvent, runner: &mut AppRunner| {
                // Repaint moves the text agent into place,
                // see `AppRunner::handle_platform_output`, which calls
                // `TextAgent::update`.
                runner.needs_repaint.repaint_asap();
            },
        )?;

        runner_ref.add_event_listener(&input, "input", {
            let input_state = Rc::clone(&input_state);
            move |event: web_sys::InputEvent, runner: &mut AppRunner| {
                input_state.borrow_mut().handle_input_event(&event, runner);
            }
        })?;
        runner_ref.add_event_listener(&input, "compositionend", {
            let input_state = Rc::clone(&input_state);
            move |_event: web_sys::CompositionEvent, runner: &mut AppRunner| {
                input_state
                    .borrow_mut()
                    .handle_composition_end_event(runner);
            }
        })?;

        runner_ref.add_event_listener(&input, "keydown", {
            let input_state = Rc::clone(&input_state);
            move |event: web_sys::KeyboardEvent, runner: &mut AppRunner| {
                let is_consumed = InputState::handle_keydown_event(&input_state, &event);
                if !is_consumed {
                    // The canvas doesn't get keydown/keyup events when the text agent is focused,
                    // so we need to forward them to the runner:
                    super::events::on_keydown(event, runner);
                }
            }
        })?;
        runner_ref.add_event_listener(&input, "keyup", {
            let input_state = Rc::clone(&input_state);
            move |event: web_sys::KeyboardEvent, runner: &mut AppRunner| {
                let is_consumed = InputState::handle_keyup_event(&input_state, &event);
                if !is_consumed {
                    // The canvas doesn't get keydown/keyup events when the text agent is focused,
                    // so we need to forward them to the runner:
                    super::events::on_keyup(event, runner);
                }
            }
        })?;

        Ok(Self { input, input_state })
    }

    pub fn update(
        &self,
        ime: Option<egui::output::IMEOutput>,
        canvas: &web_sys::HtmlCanvasElement,
        zoom_factor: f32,
    ) -> Result<(), JsValue> {
        self.input_state
            .borrow_mut()
            .update(ime, canvas, zoom_factor)
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

        if let Err(err) = super::focus_without_scroll(&self.input) {
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

    pub(crate) fn interrupt_ime_composition(&self) {
        self.input_state.borrow_mut().clear();
    }
}

impl Drop for TextAgent {
    fn drop(&mut self) {
        self.input.remove();
    }
}

struct InputState {
    input: web_sys::HtmlInputElement,
    last_text: String,
    prev_ime_output: Option<egui::output::IMEOutput>,
    is_keydown_code_unidentified: bool,
}

impl InputState {
    fn new(input: web_sys::HtmlInputElement) -> Self {
        Self {
            input,
            last_text: String::new(),
            prev_ime_output: None,
            is_keydown_code_unidentified: false,
        }
    }

    fn update(
        &mut self,
        ime: Option<egui::output::IMEOutput>,
        canvas: &web_sys::HtmlCanvasElement,
        zoom_factor: f32,
    ) -> Result<(), JsValue> {
        // Don't move the text agent unless the position actually changed:
        if self.prev_ime_output == ime {
            return Ok(());
        }
        self.prev_ime_output = ime;

        let Some(ime) = ime else { return Ok(()) };

        let mut canvas_rect = super::canvas_content_rect(canvas);
        // Fix for safari with virtual keyboard flapping position
        if is_mobile_safari() {
            canvas_rect.min.y = canvas.offset_top() as f32;
        }
        let cursor_rect = ime.cursor_rect.translate(canvas_rect.min.to_vec2());

        let style = self.input.style();
        let native_ppp = super::native_pixels_per_point();

        // Clamp the input position within the canvas width to prevent unwanted horizontal scrolling.
        let logical_canvas_width = canvas.width() as f32 / native_ppp;
        let visible_x = cursor_rect.center().x * zoom_factor;
        let clamped_x = visible_x.clamp(0.0, logical_canvas_width);

        // Clamp the input position within the canvas height to prevent unwanted vertical scrolling.
        let logical_canvas_height = canvas.height() as f32 / native_ppp;
        let visible_y = cursor_rect.center().y * zoom_factor;
        let clamped_y = visible_y.clamp(0.0, logical_canvas_height);

        // This is where the IME input will point to:
        style.set_property("left", &format!("{clamped_x}px"))?;
        style.set_property("top", &format!("{clamped_y}px"))?;

        Ok(())
    }

    fn clear(&mut self) {
        self.input.set_value("");
        self.last_text.clear();
    }

    fn handle_input_event(&mut self, event: &web_sys::InputEvent, runner: &mut AppRunner) {
        if self.is_keydown_code_unidentified && event.input_type() == "deleteContentBackward" {
            // Work around a bug in certain Android Gboard versions (e.g.,
            // 14.7.09, but not 17.0.12): when suggestions remain visible while
            // typing letters without IME composition (e.g., Latin or Cyrillic),
            // Backspace clears the suggestions instead of deleting text.
            // Without this, users have to press Backspace twice before text
            // starts being deleted.
            for pressed in [true, false] {
                runner.input.raw.events.push(egui::Event::Key {
                    key: egui::Key::Backspace,
                    physical_key: Some(egui::Key::Backspace),
                    pressed,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            self.clear();
            runner.needs_repaint.repaint_asap();

            return;
        }

        if !event.is_composing() && event.input_type() != "insertText" {
            self.clear();

            return;
        }

        let text = self.input.value();

        let prefix_len = longest_common_prefix_length(&text, &self.last_text);
        let last_text_len = self.last_text.chars().count();
        if prefix_len < last_text_len {
            let out_event = egui::Event::Ime(egui::ImeEvent::DeleteSurrounding {
                before_chars: last_text_len - prefix_len,
                after_chars: 0,
            });
            runner.input.raw.events.push(out_event);
        }

        let preedit_text: String = text.chars().skip(prefix_len).collect();
        let out_event = if event.is_composing() {
            // We handle the composition update here instead of in a
            // `compositionupdate` event because the selection range
            // has not yet been updated when `compositionupdate` fires.
            let active_range_chars = self.active_range_chars(&text, prefix_len);
            egui::Event::Ime(egui::ImeEvent::Preedit {
                text: preedit_text,
                active_range_chars,
            })
        } else {
            egui::Event::Text(preedit_text)
        };
        runner.input.raw.events.push(out_event);

        if event.is_composing() {
            self.last_text = text.chars().take(prefix_len).collect();
        } else {
            self.last_text = text;
        }

        runner.needs_repaint.repaint_asap();
    }

    /// Compute the active range (cursor or conversion segment) within the
    /// preedit text, based on the selection in the input element.
    ///
    /// `text` is the full `input.value()`, and `prefix_len_chars` is the
    /// number of chars at the start of `text` that are committed (not part
    /// of the preedit). `selectionStart`/`selectionEnd` are UTF-16 offsets
    /// within the full `input.value()`, so they are adjusted to be relative
    /// to the preedit text.
    fn active_range_chars(
        &self,
        text: &str,
        prefix_len_chars: usize,
    ) -> Option<std::ops::Range<usize>> {
        let selection_start = self.input.selection_start().unwrap_or(None)? as usize;
        let selection_end = self.input.selection_end().unwrap_or(None)? as usize;

        let text_utf16 = text.encode_utf16().collect::<Vec<u16>>();
        if selection_start > text_utf16.len() || selection_end > text_utf16.len() {
            // This can occur on Android Chrome. see discussion in:
            // <https://github.com/emilk/egui/pull/8045>.
            return None;
        }

        let text_before_selection = String::from_utf16_lossy(&text_utf16[..selection_start]);
        let text_in_selection =
            String::from_utf16_lossy(&text_utf16[selection_start..selection_end]);
        let count_before_selection = text_before_selection.chars().count();
        let count_in_selection = text_in_selection.chars().count();

        // Adjust for the committed prefix to get the range within the preedit text.
        let start = count_before_selection.saturating_sub(prefix_len_chars);
        let end = start + count_in_selection;
        Some(start..end)
    }

    fn handle_composition_end_event(&mut self, runner: &mut AppRunner) {
        let text = self.input.value();

        let commit_text = {
            let prefix_len = self.last_text.chars().count();
            text.chars().skip(prefix_len).collect::<String>()
        };
        let out_event = egui::Event::Ime(egui::ImeEvent::Commit(commit_text));
        runner.input.raw.events.push(out_event);

        self.last_text = text;

        runner.needs_repaint.repaint_asap();
    }

    /// ## Returns
    /// Whether the event is consumed. If `true`, the caller should not do
    /// further processing for this event.
    fn handle_keydown_event(input_state: &RefCell<Self>, event: &web_sys::KeyboardEvent) -> bool {
        let is_keydown_code_unidentified = event.key_code() == 229;
        input_state.borrow_mut().is_keydown_code_unidentified = is_keydown_code_unidentified;

        // https://web.archive.org/web/20200526195704/https://www.fxsitecompat.dev/en-CA/docs/2018/keydown-and-keyup-events-are-now-fired-during-ime-composition/
        if event.is_composing() || is_keydown_code_unidentified {
            true
        } else {
            if event.key().chars().count() > 1
                || event.ctrl_key()
                || event.alt_key()
                || event.meta_key()
            {
                input_state.borrow_mut().clear();
            }
            false
        }
    }

    /// ## Returns
    /// Whether the event is consumed. If `true`, the caller should not do
    /// further processing for this event.
    fn handle_keyup_event(input_state: &RefCell<Self>, event: &web_sys::KeyboardEvent) -> bool {
        input_state.borrow_mut().is_keydown_code_unidentified = false;

        // https://web.archive.org/web/20200526195704/https://www.fxsitecompat.dev/en-CA/docs/2018/keydown-and-keyup-events-are-now-fired-during-ime-composition/
        event.is_composing() || event.key_code() == 229
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
    std::iter::zip(a.chars(), b.chars())
        .take_while(|(a, b)| a == b)
        .count()
}
