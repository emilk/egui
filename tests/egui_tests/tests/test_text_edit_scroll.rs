//! A [`TextEdit`] in a [`ScrollArea`] keeps its cursor in view.

use egui::text::{CCursor, CCursorRange};
use egui::{Id, ScrollArea, TextEdit, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

const TEXT_EDIT_ID: &str = "text_edit";

#[derive(Default)]
struct State {
    text: String,

    /// Move the cursor to the end from outside the [`TextEdit`], before it is shown.
    move_cursor_to_end: bool,

    /// Discard each pass in which the text changes.
    discard_on_change: bool,

    scroll_offset: Vec2,
}

fn harness(state: State) -> Harness<'static, State> {
    Harness::builder()
        .with_size(Vec2::new(200.0, 100.0))
        .build_ui_state(
            |ui, state: &mut State| {
                let id = Id::unique(TEXT_EDIT_ID);
                if core::mem::take(&mut state.move_cursor_to_end)
                    && let Some(mut text_edit_state) = TextEdit::load_state(ui.ctx(), id)
                {
                    let end = CCursor::new(state.text.chars().count());
                    text_edit_state
                        .cursor
                        .set_char_range(Some(CCursorRange::one(end)));
                    text_edit_state.store(ui.ctx(), id);
                }

                let text_before = state.text.clone();
                let output = ScrollArea::vertical().max_height(60.0).show(ui, |ui| {
                    let text_edit = TextEdit::multiline(&mut state.text)
                        .id(id)
                        .hint_text("Prompt")
                        .cursor_at_end(false);
                    let response = ui.add(text_edit);
                    if !response.has_focus() {
                        response.request_focus();
                    }
                });
                state.scroll_offset = output.state.clamped_offset();

                if state.discard_on_change && state.text != text_before {
                    ui.request_discard("text changed");
                }
            },
            state,
        )
}

fn many_lines() -> String {
    (1..=50)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn scrolls_to_cursor_moved_from_outside() {
    let mut harness = harness(State {
        text: many_lines(),
        ..Default::default()
    });
    harness.run();
    assert_eq!(harness.state().scroll_offset.y, 0.0);

    harness.state_mut().move_cursor_to_end = true;
    harness.run();
    assert!(0.0 < harness.state().scroll_offset.y);
}

#[test]
fn scrolls_to_cursor_after_discarded_pass() {
    let mut harness = harness(State {
        discard_on_change: true,
        ..Default::default()
    });
    harness.run();

    harness
        .get_by_role(egui::accesskit::Role::MultilineTextInput)
        .type_text(&many_lines());
    harness.run();
    assert!(0.0 < harness.state().scroll_offset.y);
}
