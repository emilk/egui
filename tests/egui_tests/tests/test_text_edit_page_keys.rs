//! Tests for moving the text cursor by a page with `PageUp`/`PageDown`.

#![expect(clippy::unwrap_used)] // it's a test

use egui::accesskit::Role;
use egui::text_edit::TextEditState;
use egui::{Id, Key, Modifiers, OutputCommand, ScrollArea, TextEdit, TextStyle, Vec2};
use egui_kittest::{Harness, kittest::Queryable as _};

const LINES: usize = 40;

// Each line is `line NN` plus a newline.
const LINE_CHARS: usize = 8;

// Whole rows that fit in the scroll area.
const VISIBLE_ROWS: usize = 5;

fn id() -> Id {
    Id::unique("page_keys")
}

fn text() -> String {
    (0..LINES)
        .map(|i| format!("line {i:02}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn scroll_area_height(ui: &egui::Ui, rows: f32) -> f32 {
    rows * ui.text_style_height(&TextStyle::Monospace)
}

// TextEdit in a scroll area
fn harness() -> Harness<'static, String> {
    harness_with(VISIBLE_ROWS as f32 + 0.5, text())
}

fn harness_with(visible_rows: f32, text: String) -> Harness<'static, String> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 400.0))
        .with_accessibility_check(false) // the text edit is unlabelled
        .build_ui_state(
            move |ui, text| {
                ScrollArea::vertical()
                    .max_height(scroll_area_height(ui, visible_rows))
                    .min_scrolled_height(0.0) // the default minimum is taller than half a row
                    .show(ui, |ui| {
                        ui.add(TextEdit::multiline(text).id(id()).cursor_at_end(false));
                    });
            },
            text,
        );
    harness.run();
    harness.get_by_role(Role::MultilineTextInput).focus();
    harness.run();
    harness
}

fn cursor(harness: &Harness<'_, String>) -> (usize, usize) {
    let range = TextEditState::load(&harness.ctx, id())
        .and_then(|state| state.cursor.char_range())
        .unwrap();
    (range.primary.index.0, range.secondary.index.0)
}

fn start_of_line(line: usize) -> usize {
    line * LINE_CHARS
}

fn press(harness: &mut Harness<'_, String>, key: Key) {
    harness.key_press(key);
    harness.run();
}

fn shift_press(harness: &mut Harness<'_, String>, key: Key) {
    harness.key_press_modifiers(Modifiers::SHIFT, key);
    harness.run();
}

#[test]
fn page_down_should_move_by_the_visible_rows() {
    let mut harness = harness();

    press(&mut harness, Key::PageDown);
    assert_eq!(
        cursor(&harness),
        (start_of_line(VISIBLE_ROWS), start_of_line(VISIBLE_ROWS))
    );

    press(&mut harness, Key::PageDown);
    assert_eq!(
        cursor(&harness),
        (
            start_of_line(2 * VISIBLE_ROWS),
            start_of_line(2 * VISIBLE_ROWS)
        )
    );
}

#[test]
fn page_up_should_move_back_by_the_visible_rows() {
    let mut harness = harness();
    press(&mut harness, Key::PageDown);
    press(&mut harness, Key::PageDown);

    press(&mut harness, Key::PageUp);
    assert_eq!(cursor(&harness).0, start_of_line(VISIBLE_ROWS));

    press(&mut harness, Key::PageUp);
    assert_eq!(cursor(&harness).0, 0);
}

#[test]
fn page_keys_should_keep_the_column() {
    let mut harness = harness();
    for _ in 0..3 {
        press(&mut harness, Key::ArrowRight);
    }
    assert_eq!(cursor(&harness).0, 3);

    press(&mut harness, Key::PageDown);
    assert_eq!(cursor(&harness).0, start_of_line(VISIBLE_ROWS) + 3);

    press(&mut harness, Key::PageUp);
    assert_eq!(cursor(&harness).0, 3);
}

#[test]
fn page_up_on_the_first_row_should_go_to_the_start() {
    let mut harness = harness();
    for _ in 0..3 {
        press(&mut harness, Key::ArrowRight);
    }

    press(&mut harness, Key::PageUp);
    assert_eq!(cursor(&harness).0, 0);
}

#[test]
fn page_down_should_stop_at_the_last_row_and_then_go_to_the_end() {
    let mut harness = harness();
    let pages_to_the_last_full_page = (LINES - 1) / VISIBLE_ROWS;
    for _ in 0..pages_to_the_last_full_page {
        press(&mut harness, Key::PageDown);
    }
    assert_eq!(
        cursor(&harness).0,
        start_of_line(pages_to_the_last_full_page * VISIBLE_ROWS)
    );

    press(&mut harness, Key::PageDown);
    assert_eq!(cursor(&harness).0, start_of_line(LINES - 1));

    press(&mut harness, Key::PageDown);
    assert_eq!(cursor(&harness).0, text().chars().count());
}

#[test]
fn shift_page_down_should_extend_the_selection() {
    let mut harness = harness();

    shift_press(&mut harness, Key::PageDown);
    assert_eq!(cursor(&harness), (start_of_line(VISIBLE_ROWS), 0));

    shift_press(&mut harness, Key::PageDown);
    assert_eq!(cursor(&harness), (start_of_line(2 * VISIBLE_ROWS), 0));

    shift_press(&mut harness, Key::PageUp);
    assert_eq!(cursor(&harness), (start_of_line(VISIBLE_ROWS), 0));
}

// Rows of different heights
#[test]
fn page_keys_should_count_rows_by_their_own_heights() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 400.0))
        .with_accessibility_check(false) // the text edit is unlabelled
        .build_ui_state(
            |ui, text| {
                let small = ui.style().text_styles[&TextStyle::Monospace].size;
                let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                    let mut job = egui::text::LayoutJob::default();
                    job.wrap.max_width = wrap_width;
                    for (i, line) in text.as_str().split_inclusive('\n').enumerate() {
                        let size = if i % 2 == 0 { 2.0 * small } else { small };
                        job.append(
                            line,
                            0.0,
                            egui::TextFormat::simple(
                                egui::FontId::monospace(size),
                                ui.visuals().text_color(),
                            ),
                        );
                    }
                    ui.fonts_mut(|fonts| fonts.layout_job(job))
                };
                ScrollArea::vertical()
                    .max_height(scroll_area_height(ui, VISIBLE_ROWS as f32 + 0.5))
                    .show(ui, |ui| {
                        ui.add(
                            TextEdit::multiline(text)
                                .id(id())
                                .cursor_at_end(false)
                                .layouter(&mut layouter),
                        );
                    });
            },
            text(),
        );
    harness.run();
    harness.get_by_role(Role::MultilineTextInput).focus();
    harness.run();

    press(&mut harness, Key::PageDown);
    assert_eq!(cursor(&harness).0, start_of_line(3));

    press(&mut harness, Key::PageUp);
    assert_eq!(cursor(&harness).0, 0);
}

#[test]
fn page_down_should_count_wrapped_rows() {
    let long_line = "word ".repeat(200); // wraps into many rows, no newlines
    let mut harness = harness_with(VISIBLE_ROWS as f32 + 0.5, long_line.clone());
    for _ in 0..VISIBLE_ROWS {
        press(&mut harness, Key::ArrowDown);
    }
    let after_arrows = cursor(&harness).0;
    assert!(
        0 < after_arrows && after_arrows < long_line.len(),
        "the text should have wrapped: cursor at {after_arrows}"
    );

    let mut harness = harness_with(VISIBLE_ROWS as f32 + 0.5, long_line);
    press(&mut harness, Key::PageDown);
    assert_eq!(cursor(&harness).0, after_arrows);
}

#[test]
fn page_keys_should_move_at_least_one_row() {
    let mut harness = harness_with(0.5, text());

    press(&mut harness, Key::PageDown);
    assert_eq!(cursor(&harness).0, start_of_line(1));

    press(&mut harness, Key::PageDown);
    assert_eq!(cursor(&harness).0, start_of_line(2));

    press(&mut harness, Key::PageUp);
    assert_eq!(cursor(&harness).0, start_of_line(1));
}

#[test]
fn page_keys_with_other_modifiers_should_be_ignored() {
    let mut harness = harness();
    shift_press(&mut harness, Key::PageDown);
    assert_eq!(cursor(&harness), (start_of_line(VISIBLE_ROWS), 0));

    for modifiers in [Modifiers::CTRL, Modifiers::ALT, Modifiers::COMMAND] {
        harness.key_press_modifiers(modifiers, Key::PageDown);
        harness.run();
        assert_eq!(cursor(&harness), (start_of_line(VISIBLE_ROWS), 0));
    }
}

// Row height in points
fn row_height(harness: &Harness<'_, String>) -> f32 {
    let font_id = harness.ctx.global_style().text_styles[&TextStyle::Monospace].clone();
    harness.ctx.fonts_mut(|fonts| fonts.row_height(&font_id))
}

fn text_edit_top(harness: &Harness<'_, String>) -> f32 {
    harness.get_by_role(Role::MultilineTextInput).rect().top()
}

#[test]
fn page_keys_should_scroll_the_view_by_a_page() {
    let mut harness = harness();
    let page = VISIBLE_ROWS as f32 * row_height(&harness);
    let top = text_edit_top(&harness);

    press(&mut harness, Key::PageDown);
    let scrolled = top - text_edit_top(&harness);
    assert!(
        (scrolled - page).abs() < 1.5,
        "should have scrolled one page ({page}) down, scrolled {scrolled}"
    );

    press(&mut harness, Key::PageUp);
    let scrolled = top - text_edit_top(&harness);
    assert!(
        scrolled.abs() < 1.5,
        "should have scrolled back to the top, is {scrolled} down"
    );
}

#[test]
fn page_down_at_the_end_should_still_move_the_cursor() {
    let mut harness = harness();
    // One page per press, one press for the last partial page, and one more to the end:
    for _ in 0..=LINES / VISIBLE_ROWS {
        press(&mut harness, Key::PageDown);
    }
    assert_eq!(cursor(&harness).0, text().chars().count());

    // The whole text edit is scrolled as far as it goes:
    let bottom = harness
        .get_by_role(Role::MultilineTextInput)
        .rect()
        .bottom();
    let scroll_area_bottom = harness.ctx.content_rect().bottom();
    assert!(
        bottom <= scroll_area_bottom,
        "text edit bottom {bottom} should be visible above {scroll_area_bottom}"
    );
}

#[test]
fn shift_page_down_should_extend_a_label_selection() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 400.0))
        .with_step_dt(1.0 / 60.0) // a press should not turn into a drag before the pointer has moved
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, text| {
                ScrollArea::vertical()
                    .max_height(scroll_area_height(ui, VISIBLE_ROWS as f32 + 0.5))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(text.as_str()).monospace());
                    });
            },
            text(),
        );
    harness.run();

    // Click at the start of the first line to put the cursor there:
    let label = harness.get_by_role(Role::Label).rect();
    let pos = egui::pos2(label.left() + 1.0, label.top() + row_height(&harness) / 2.0);
    harness.hover_at(pos);
    harness.step();
    harness.drag_at(pos);
    harness.step();
    harness.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();

    shift_press(&mut harness, Key::PageDown);
    harness.event(egui::Event::Copy);
    harness.step(); // one pass, so that the copy command is in the output we look at

    let copied =
        harness
            .output()
            .platform_output
            .commands
            .iter()
            .find_map(|command| match command {
                OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            });
    let expected: String = text().chars().take(start_of_line(VISIBLE_ROWS)).collect();
    assert_eq!(copied, Some(expected));
}
