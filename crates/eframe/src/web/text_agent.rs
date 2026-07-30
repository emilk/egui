//! The text agent is a hidden `<input>` element used to capture
//! IME and mobile keyboard input events.

use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::prelude::*;

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

        // Hide the element, and park it over the top-left corner of the canvas
        // so that focusing it can never scroll some other part
        // of the page into view.
        let style = input.style();
        style.set_property("background-color", "transparent")?;
        style.set_property("border", "none")?;
        style.set_property("outline", "none")?;
        style.set_property("width", "1px")?;
        style.set_property("height", "1px")?;
        style.set_property("caret-color", "transparent")?;
        style.set_property("position", "absolute")?;
        style.set_property("top", &format!("{}px", canvas.offset_top()))?;
        style.set_property("left", &format!("{}px", canvas.offset_left()))?;
        // Prevent auto-zoom on mobile browsers (requires at least 16px).
        style.set_property("font-size", "16px")?;

        // Insert the input as a sibling of the canvas, so that its
        // `position: absolute` resolves against the same containing block
        // as the canvas' `offset_top`/`offset_left`.
        // This anchors the input to the canvas regardless of how the page
        // is scrolled or how the canvas is embedded, and also works when
        // the canvas is inside a shadow DOM.
        if let Some(parent) = canvas.parent_node() {
            parent.insert_before(&input, canvas.next_sibling().as_ref())?;
        } else if let Some(body) = document.body() {
            log::warn!("Canvas has no parent element - appending text agent to document body");
            body.append_child(&input)?;
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
    ime_output: Option<egui::output::IMEOutput>,
    keydown_special_case: KeydownSpecialCase,
}

#[derive(Clone, Copy)]
enum KeydownSpecialCase {
    None,

    /// On Android Gboard 14.7.09, when suggestions remain visible while typing
    /// letters without IME composition (e.g., Latin or Cyrillic), pressing
    /// Backspace produces key code 229 instead of the expected Backspace key
    /// code.
    /// Without the workaround, users have to press Backspace twice before text
    /// starts being deleted.
    ///
    /// This workaround is also required for Android Gboard corrections and
    /// completions (e.g., `tex|` -> `Texas`) to work correctly. In these
    /// cases, a `deleteContentBackward` input event fires first (e.g., to
    /// delete `tex`), followed by an `insertText` input event (e.g., to insert
    /// `Texas`).
    ///
    /// Since it is difficult to distinguish between a Backspace press and a
    /// correction or completion (e.g., when the state is `t|`, it is unclear
    /// whether the user wants to delete `t` or replace it with `Texas`), we
    /// send a `DeleteSurrounding` IME event in all cases instead of
    /// synthetically generating Backspace press and release events.
    AndroidKeycode229,

    /// iOS (18.6)'s built-in Korean keyboard uses `deleteContentBackward` to
    /// compose Hangul characters. In these cases, the key code is 0.
    IosKeycode0,
}

impl InputState {
    fn new(input: web_sys::HtmlInputElement) -> Self {
        Self {
            input,
            last_text: String::new(),
            ime_output: None,
            keydown_special_case: KeydownSpecialCase::None,
        }
    }

    fn update(
        &mut self,
        ime: Option<egui::output::IMEOutput>,
        canvas: &web_sys::HtmlCanvasElement,
        zoom_factor: f32,
    ) -> Result<(), JsValue> {
        // Don't move the text agent unless the position actually changed:
        if self.ime_output == ime {
            return Ok(());
        }
        self.ime_output = ime;

        let Some(ime) = ime else { return Ok(()) };

        // NOTE: we don't set the input's `type` to `password` based on
        // `ime.purpose`, because that would confuse some password managers.
        // For example, Chrome's password manager will always think the last
        // letter typed in the password field is the password.

        let style = self.input.style();
        let native_ppp = super::native_pixels_per_point();

        // The input is a sibling of the canvas (see `attach`), so we position
        // it relative to the same containing block using the canvas offset.
        // Unlike `get_bounding_client_rect`, the offset is unaffected by page
        // scrolling, and doesn't flap when the virtual keyboard is shown on
        // mobile Safari.

        // Clamp the input position within the canvas width to prevent unwanted horizontal scrolling.
        let logical_canvas_width = canvas.width() as f32 / native_ppp;
        let visible_x = ime.cursor_rect.center().x * zoom_factor;
        let clamped_x = visible_x.clamp(0.0, logical_canvas_width);

        // Clamp the input position within the canvas height to prevent unwanted vertical scrolling.
        let logical_canvas_height = canvas.height() as f32 / native_ppp;
        let visible_y = ime.cursor_rect.center().y * zoom_factor;
        let clamped_y = visible_y.clamp(0.0, logical_canvas_height);

        // This is where the IME input will point to:
        style.set_property(
            "left",
            &format!("{}px", canvas.offset_left() as f32 + clamped_x),
        )?;
        style.set_property(
            "top",
            &format!("{}px", canvas.offset_top() as f32 + clamped_y),
        )?;

        Ok(())
    }

    fn clear(&mut self) {
        self.input.set_value("");
        self.last_text.clear();
    }

    fn handle_input_event(&mut self, event: &web_sys::InputEvent, runner: &mut AppRunner) {
        if self
            .ime_output
            .as_ref()
            .is_some_and(|ime| ime.purpose == egui::IMEPurpose::Password)
        {
            self.handle_input_event_password(event, runner);
            return;
        }

        let input_type = event.input_type();

        if !event.is_composing()
            && input_type != "insertText"
            // iOS uses this for corrections and completions (e.g., `tex|` ->
            // `Texas`).
            && input_type != "insertReplacementText"
            && (matches!(self.keydown_special_case, KeydownSpecialCase::None)
                || input_type != "deleteContentBackward")
        {
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

    fn handle_input_event_password(&mut self, event: &web_sys::InputEvent, runner: &mut AppRunner) {
        let input_type = event.input_type();

        if input_type != "insertText" {
            return;
        }

        let text = self.input.value();

        runner.input.raw.events.push(egui::Event::Text(text));
        self.clear();
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
        // Platform-sniffing methods are unreliable, so they are not used as
        // guards here.
        let special_case = match event.key_code() {
            229 => KeydownSpecialCase::AndroidKeycode229,
            0 => KeydownSpecialCase::IosKeycode0,
            _ => KeydownSpecialCase::None,
        };
        input_state.borrow_mut().keydown_special_case = special_case;

        // https://web.archive.org/web/20200526195704/https://www.fxsitecompat.dev/en-CA/docs/2018/keydown-and-keyup-events-are-now-fired-during-ime-composition/
        if event.is_composing() || !matches!(special_case, KeydownSpecialCase::None) {
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
        input_state.borrow_mut().keydown_special_case = KeydownSpecialCase::None;

        // https://web.archive.org/web/20200526195704/https://www.fxsitecompat.dev/en-CA/docs/2018/keydown-and-keyup-events-are-now-fired-during-ime-composition/
        event.is_composing() || event.key_code() == 229
    }
}

fn longest_common_prefix_length(a: &str, b: &str) -> usize {
    std::iter::zip(a.chars(), b.chars())
        .take_while(|(a, b)| a == b)
        .count()
}
