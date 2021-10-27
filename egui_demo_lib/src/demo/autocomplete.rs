use super::View;
use egui::{
    epaint::text::cursor::{CCursor, Cursor, PCursor, RCursor},
    text_edit::CursorRange,
    Grid, Key, Label, Modifiers, TextBuffer, TextEdit, Window,
};
use std::ops::Range;

pub struct Autocomplete {
    autocomplete_source: String,
    input: String,
    ac_state: AcState,
    cursor_to_end: bool,
    first_frame: bool,
    accepted_string: String,
}

const AUTOCOMPLETE_DEFAULT_SOURCE: &str = "\
apples avocados armageddon appease appropriate
bazillion beetles baron bewitch
carrot castle car canopy cookie cook cow\
";

impl Default for Autocomplete {
    fn default() -> Self {
        Self {
            autocomplete_source: AUTOCOMPLETE_DEFAULT_SOURCE.into(),
            input: Default::default(),
            ac_state: Default::default(),
            cursor_to_end: false,
            first_frame: true,
            accepted_string: String::new(),
        }
    }
}

impl super::Demo for Autocomplete {
    fn name(&self) -> &'static str {
        "✔ Autocomplete"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        Window::new(self.name())
            .default_height(500.)
            .open(open)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl View for Autocomplete {
    fn ui(&mut self, ui: &mut egui::Ui) {
        Grid::new("autocomplete_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Source");
                ui.text_edit_multiline(&mut self.autocomplete_source);
                ui.end_row();
                ui.label("Input");
                let te_id = ui.make_persistent_id("text_edit_ac");
                let up_pressed = ui
                    .input_mut()
                    .consume_key(Modifiers::default(), Key::ArrowUp);
                let down_pressed = ui
                    .input_mut()
                    .consume_key(Modifiers::default(), Key::ArrowDown);
                let te = TextEdit::singleline(&mut self.input)
                    .lock_focus(true)
                    .id(te_id);
                if self.cursor_to_end {
                    text_edit_cursor_set_to_end(ui, te_id);
                }
                let re = ui.add(te);
                self.ac_state.input_changed = re.changed();
                if self.first_frame {
                    re.request_focus();
                    self.first_frame = false;
                }
                ui.end_row();
                let candidates = self
                    .autocomplete_source
                    .split_whitespace()
                    .collect::<Vec<_>>();
                let msg = autocomplete_popup_below(
                    &mut self.input,
                    &mut self.ac_state,
                    &candidates,
                    ui,
                    &re,
                    up_pressed,
                    down_pressed,
                );
                if msg.applied {
                    self.cursor_to_end = true;
                } else {
                    self.cursor_to_end = false;
                }
                if ui.input().key_pressed(Key::Enter) {
                    self.accepted_string = self.input.take();
                    re.request_focus();
                }
                if msg.stole_focus {
                    re.request_focus();
                }
                ui.label("Accepted");
                ui.add(Label::new(&self.accepted_string).wrap(true));
                ui.end_row();
            });
        ui.separator();
        ui.vertical_centered(|ui| {
            ui.label("Up/Down: navigate list");
            ui.label("Tab: autocomplete");
            ui.label("Enter: autocomplete/accept");
        });
    }
}

fn text_edit_cursor_set_to_end(ui: &mut egui::Ui, te_id: egui::Id) {
    let mut state = TextEdit::load_state(ui.ctx(), te_id).unwrap();
    state.set_cursor_range(Some(CursorRange::one(Cursor {
        ccursor: CCursor {
            index: 0,
            prefer_next_row: false,
        },
        rcursor: RCursor { row: 0, column: 0 },
        pcursor: PCursor {
            paragraph: 0,
            offset: 10000,
            prefer_next_row: false,
        },
    })));
    TextEdit::store_state(ui.ctx(), te_id, state);
}

pub struct AcState {
    /// Selection index in the autocomplete list
    select: Option<usize>,
    /// Input changed this frame
    pub input_changed: bool,
}

impl Default for AcState {
    fn default() -> Self {
        Self {
            select: Some(0),
            input_changed: true,
        }
    }
}

#[derive(Default)]
pub struct PopupMsg {
    /// Returns whether a suggestion was applied or not
    pub applied: bool,
    /// Whether the popup stole focus (for example on pressing enter)
    pub stole_focus: bool,
}

/// Popup for autocompleting.
pub(super) fn autocomplete_popup_below(
    string: &mut String,
    state: &mut AcState,
    candidates: &[&str],
    ui: &mut egui::Ui,
    response: &egui::Response,
    up_pressed: bool,
    down_pressed: bool,
) -> PopupMsg {
    let mut ret_msg = PopupMsg::default();
    let popup_id = ui.make_persistent_id("autocomplete_popup");
    let last_char_is_terminating = string.chars().last().map_or(true, |c| !c.is_alphabetic());
    let last = if last_char_is_terminating {
        ""
    } else {
        string.split_ascii_whitespace().last().unwrap_or("")
    };
    if down_pressed {
        match &mut state.select {
            None => state.select = Some(0),
            Some(sel) => *sel += 1,
        }
    }
    if let Some(sel) = &mut state.select {
        if up_pressed {
            if *sel > 0 {
                *sel -= 1;
            } else {
                // Allow selecting "Nothing" by going above first element
                state.select = None;
            }
        }
    } else if state.input_changed {
        // Always select index 0 when input was changed for convenience
        state.select = Some(0);
    }
    if !string.is_empty() && !last.is_empty() {
        let mut exact_match = None;
        // Get length of list and also whether there is an exact match
        let mut i = 0;
        let len = candidates
            .iter()
            .filter(|candidate| {
                if **candidate == last {
                    exact_match = Some(i);
                }
                let predicate = candidate.contains(last);
                if predicate {
                    i += 1;
                }
                predicate
            })
            .count();
        match exact_match {
            Some(idx) if state.input_changed => state.select = Some(idx),
            _ => {}
        }
        if len > 0 {
            if let Some(selection) = &mut state.select {
                if *selection >= len {
                    *selection = len - 1;
                }
            }
            let mut complete = None;
            egui::popup_below_widget(ui, popup_id, response, |ui| {
                for (i, &candidate) in candidates
                    .iter()
                    .filter(|candidate| candidate.contains(last))
                    .enumerate()
                {
                    if ui
                        .selectable_label(state.select == Some(i), candidate)
                        .clicked()
                    {
                        complete = Some(candidate);
                    }
                    let return_pressed = ui.input().key_pressed(Key::Enter);
                    if state.select == Some(i)
                        && (ui.input().key_pressed(Key::Tab) || return_pressed)
                    {
                        complete = Some(candidate);
                        if return_pressed {
                            ret_msg.stole_focus = true;
                        }
                    }
                }
            });
            if let Some(candidate) = complete {
                let range = str_range(string, last);
                string.replace_range(range, candidate);
                state.input_changed = false;
                ret_msg.applied = true;
            }
            if !string.is_empty() {
                ui.memory().open_popup(popup_id);
            } else {
                ui.memory().close_popup();
            }
        }
    }
    ret_msg
}

fn str_range(parent: &str, sub: &str) -> Range<usize> {
    let beg = sub.as_ptr() as usize - parent.as_ptr() as usize;
    let end = beg + sub.len();
    beg..end
}
