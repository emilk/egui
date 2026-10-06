//! Tests for double-click-and-drag (select by words) and
//! triple-click-and-drag (select by lines) text selection.
//! See <https://github.com/emilk/egui/issues/2550>.

use core::cell::RefCell;
use std::rc::Rc;

use egui::text::CCursor;
use egui::{Event, Modifiers, OutputCommand, PointerButton, Pos2, RichText, TextEdit, Vec2, vec2};
use egui_kittest::{Harness, HarnessBuilder};

/// Short enough that a few frames stay well within the double-click window (0.3 s).
const STEP_DT: f32 = 0.01;

fn press_with<S>(harness: &mut Harness<'_, S>, pos: Pos2, modifiers: Modifiers) {
    harness.event(Event::ModifiersChanged(modifiers));
    harness.event(Event::PointerMoved(pos));
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: true,
        modifiers,
    });
    harness.step();
}

fn release_with<S>(harness: &mut Harness<'_, S>, pos: Pos2, modifiers: Modifiers) {
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers,
    });
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.step();
}

fn press<S>(harness: &mut Harness<'_, S>, pos: Pos2) {
    press_with(harness, pos, Modifiers::NONE);
}

fn release<S>(harness: &mut Harness<'_, S>, pos: Pos2) {
    release_with(harness, pos, Modifiers::NONE);
}

fn click<S>(harness: &mut Harness<'_, S>, pos: Pos2) {
    press(harness, pos);
    release(harness, pos);
}

fn shift_click<S>(harness: &mut Harness<'_, S>, pos: Pos2) {
    press_with(harness, pos, Modifiers::SHIFT);
    release_with(harness, pos, Modifiers::SHIFT);
}

fn shift_double_click<S>(harness: &mut Harness<'_, S>, pos: Pos2) {
    shift_click(harness, pos);
    shift_click(harness, pos);
}

/// Let some time pass without any input.
fn wait<S>(harness: &mut Harness<'_, S>, seconds: f32) {
    for _ in 0..(seconds / STEP_DT).round() as usize {
        harness.step();
    }
}

fn drag_to<S>(harness: &mut Harness<'_, S>, pos: Pos2) {
    harness.event(Event::PointerMoved(pos));
    harness.step();
    // Give the drag state a few frames to settle:
    harness.step();
    harness.step();
}

/// Double-click at `from` (keeping the button down), then drag to `to` and release.
fn double_click_drag<S>(harness: &mut Harness<'_, S>, from: Pos2, to: Pos2) {
    press(harness, from);
    release(harness, from);
    press(harness, from);
    drag_to(harness, to);
    release(harness, to);
}

/// Triple-click at `from` (keeping the button down), then drag to `to` and release.
fn triple_click_drag<S>(harness: &mut Harness<'_, S>, from: Pos2, to: Pos2) {
    press(harness, from);
    release(harness, from);
    press(harness, from);
    release(harness, from);
    press(harness, from);
    drag_to(harness, to);
    release(harness, to);
}

/// Send a copy event and return the text that was copied to the clipboard, if any.
fn copied_text<S>(harness: &mut Harness<'_, S>) -> Option<String> {
    harness.event(Event::Copy);
    harness.step();
    harness
        .output()
        .platform_output
        .commands
        .iter()
        .find_map(|cmd| match cmd {
            OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
}

/// A [`TextEdit`] harness, plus the screen position of each character boundary of its text.
fn text_edit_harness(text: &str) -> (Harness<'static, String>, Rc<RefCell<Vec<Pos2>>>) {
    text_edit_harness_with_width(text, 380.0)
}

/// A [`TextEdit`] harness, plus the screen position of each character boundary of its text.
fn text_edit_harness_with_width(
    text: &str,
    desired_width: f32,
) -> (Harness<'static, String>, Rc<RefCell<Vec<Pos2>>>) {
    let char_pos = Rc::new(RefCell::new(Vec::new()));
    let char_pos_clone = Rc::clone(&char_pos);

    let mut harness = HarnessBuilder::default()
        .with_step_dt(STEP_DT)
        .with_size(Vec2::new(400.0, 300.0))
        .build_ui_state(
            move |ui, text: &mut String| {
                let label = ui.label("Text:");
                let output = TextEdit::multiline(text)
                    .desired_width(desired_width)
                    .show(ui);
                output.response.response.clone().labelled_by(label.id);
                *char_pos_clone.borrow_mut() = (0..text.chars().count())
                    .map(|i| {
                        output.galley_pos
                            + output
                                .galley
                                .pos_from_cursor(CCursor::new(i))
                                .center()
                                .to_vec2()
                    })
                    .collect();
            },
            text.to_owned(),
        );
    harness.run();
    (harness, char_pos)
}

#[test]
fn double_click_drag_should_select_words_forward() {
    let (mut harness, char_pos) = text_edit_harness("alpha beta gamma delta");
    let pos = |i: usize| char_pos.borrow()[i];

    // Double-click on "beta", drag into "gamma":
    double_click_drag(&mut harness, pos(8), pos(13));

    assert_eq!(copied_text(&mut harness).as_deref(), Some("beta gamma"));
}

#[test]
fn double_click_drag_should_select_words_backward() {
    let (mut harness, char_pos) = text_edit_harness("alpha beta gamma delta");
    let pos = |i: usize| char_pos.borrow()[i];

    // Double-click on "gamma", drag backward into "alpha":
    double_click_drag(&mut harness, pos(13), pos(2));

    assert_eq!(
        copied_text(&mut harness).as_deref(),
        Some("alpha beta gamma")
    );
}

#[test]
fn triple_click_drag_should_select_lines() {
    let (mut harness, char_pos) = text_edit_harness("alpha beta\ncarrot\ndelta epsilon");
    let pos = |i: usize| char_pos.borrow()[i];

    // Triple-click on "carrot", drag down into "delta epsilon":
    triple_click_drag(&mut harness, pos(13), pos(24));

    assert_eq!(
        copied_text(&mut harness).as_deref(),
        Some("carrot\ndelta epsilon")
    );
}

/// Stacked labels, plus a function mapping (label index, char index) to screen position
/// (the center of that character).
fn labels_harness() -> (Harness<'static>, impl Fn(usize, usize) -> Pos2) {
    labels_harness_with(&["alpha beta gamma", "delta epsilon zeta"])
}

/// Stacked labels, plus a function mapping (label index, char index) to screen position
/// (the center of that character).
fn labels_harness_with(
    texts: &'static [&'static str],
) -> (Harness<'static>, impl Fn(usize, usize) -> Pos2) {
    let label_info = Rc::new(RefCell::new(Vec::new()));
    let label_info_clone = Rc::clone(&label_info);

    let mut harness = HarnessBuilder::default()
        .with_step_dt(STEP_DT)
        .with_size(Vec2::new(400.0, 200.0))
        .build_ui(move |ui| {
            let char_width = ui
                .fonts_mut(|f| f.glyph_width(&egui::TextStyle::Monospace.resolve(ui.style()), 'x'));
            let mut info = label_info_clone.borrow_mut();
            info.clear();
            for text in texts {
                let rect = ui.label(RichText::new(*text).monospace()).rect;
                info.push((rect, char_width));
            }
        });
    harness.run();

    let pos = move |label: usize, char_index: usize| {
        let (rect, char_width) = label_info.borrow()[label];
        rect.left_top() + vec2((char_index as f32 + 0.5) * char_width, rect.height() / 2.0)
    };
    (harness, pos)
}

#[test]
fn double_click_drag_should_select_words_across_labels() {
    let (mut harness, pos) = labels_harness();

    // Double-click on "beta" in the first label, drag into "epsilon" in the second:
    double_click_drag(&mut harness, pos(0, 8), pos(1, 9));

    assert_eq!(
        copied_text(&mut harness).as_deref(),
        Some("beta gamma\ndelta epsilon")
    );
}

#[test]
fn double_click_drag_should_select_words_across_labels_backward() {
    let (mut harness, pos) = labels_harness();

    // Double-click on "epsilon" in the second label, drag up into "beta" in the first:
    double_click_drag(&mut harness, pos(1, 9), pos(0, 8));

    assert_eq!(
        copied_text(&mut harness).as_deref(),
        Some("beta gamma\ndelta epsilon")
    );
}

const WORDS: &str = "alpha beta gamma delta";

#[test]
fn plain_double_click_should_select_word() {
    let (mut harness, char_pos) = text_edit_harness(WORDS);
    let pos = |i: usize| char_pos.borrow()[i];

    click(&mut harness, pos(8));
    click(&mut harness, pos(8));

    assert_eq!(copied_text(&mut harness).as_deref(), Some("beta"));
}

#[test]
fn plain_triple_click_should_select_line() {
    let (mut harness, char_pos) = text_edit_harness("one two\nthree four\nfive");
    let pos = |i: usize| char_pos.borrow()[i];

    click(&mut harness, pos(10));
    click(&mut harness, pos(10));
    click(&mut harness, pos(10));

    assert_eq!(copied_text(&mut harness).as_deref(), Some("three four"));
}

#[test]
fn plain_drag_should_select_chars() {
    let (mut harness, char_pos) = text_edit_harness(WORDS);
    let pos = |i: usize| char_pos.borrow()[i];

    press(&mut harness, pos(8));
    drag_to(&mut harness, pos(13));
    release(&mut harness, pos(13));

    assert_eq!(copied_text(&mut harness).as_deref(), Some("ta ga"));
}

#[test]
fn double_click_drag_should_keep_anchor_when_reversing() {
    let (mut harness, char_pos) = text_edit_harness(WORDS);
    let pos = |i: usize| char_pos.borrow()[i];

    // Double-click on "beta", drag back into "alpha", then forward into "delta":
    click(&mut harness, pos(8));
    press(&mut harness, pos(8));
    drag_to(&mut harness, pos(2));
    assert_eq!(copied_text(&mut harness).as_deref(), Some("alpha beta"));

    drag_to(&mut harness, pos(19));
    release(&mut harness, pos(19));
    assert_eq!(
        copied_text(&mut harness).as_deref(),
        Some("beta gamma delta")
    );
}

#[test]
fn double_click_drag_should_select_words_across_wrapped_rows() {
    let (mut harness, char_pos) =
        text_edit_harness_with_width("alpha beta gamma delta epsilon zeta eta theta", 80.0);
    let pos = |i: usize| char_pos.borrow()[i];
    assert!(pos(8).y < pos(25).y, "The text should wrap");

    // Double-click on "beta", drag down into "epsilon":
    double_click_drag(&mut harness, pos(8), pos(25));

    assert_eq!(
        copied_text(&mut harness).as_deref(),
        Some("beta gamma delta epsilon")
    );
}

#[test]
fn triple_click_drag_should_select_lines_upward() {
    let (mut harness, char_pos) = text_edit_harness("one two\nthree four\nfive six");
    let pos = |i: usize| char_pos.borrow()[i];

    // Triple-click on "five six", drag up into "three four":
    triple_click_drag(&mut harness, pos(21), pos(10));

    assert_eq!(
        copied_text(&mut harness).as_deref(),
        Some("three four\nfive six")
    );
}

/// A press shortly after a double-click, but too late to be part of it,
/// must start a plain character drag, not a line drag.
#[test]
fn slow_press_after_double_click_should_drag_chars() {
    let (mut harness, char_pos) = text_edit_harness(WORDS);
    let pos = |i: usize| char_pos.borrow()[i];

    click(&mut harness, pos(8));
    click(&mut harness, pos(8));
    wait(&mut harness, 0.33); // more than `max_double_click_delay`, less than twice it
    press(&mut harness, pos(8));
    drag_to(&mut harness, pos(13));
    release(&mut harness, pos(13));

    assert_eq!(copied_text(&mut harness).as_deref(), Some("ta ga"));
}

/// Two single clicks that are too far apart in time to be a double-click,
/// followed quickly by a press: that press is the second click of a double-click,
/// so the drag should be by words (not lines).
#[test]
fn press_after_two_slow_clicks_should_drag_words() {
    let (mut harness, char_pos) = text_edit_harness(WORDS);
    let pos = |i: usize| char_pos.borrow()[i];

    click(&mut harness, pos(8));
    wait(&mut harness, 0.4);
    click(&mut harness, pos(8));
    wait(&mut harness, 0.05);
    press(&mut harness, pos(8));
    drag_to(&mut harness, pos(13));
    release(&mut harness, pos(13));

    assert_eq!(copied_text(&mut harness).as_deref(), Some("beta gamma"));
}

#[test]
fn shift_click_after_double_click_should_extend_by_chars() {
    let (mut harness, char_pos) = text_edit_harness(WORDS);
    let pos = |i: usize| char_pos.borrow()[i];

    click(&mut harness, pos(8));
    click(&mut harness, pos(8));
    wait(&mut harness, 1.0);
    shift_click(&mut harness, pos(19));

    assert_eq!(copied_text(&mut harness).as_deref(), Some("beta gamma de"));
}

/// Like on macOS: shift-double-click extends the selection by whole words,
/// keeping the anchor of the existing selection.
#[test]
fn shift_double_click_should_extend_by_words() {
    let (mut harness, char_pos) = text_edit_harness(WORDS);
    let pos = |i: usize| char_pos.borrow()[i];

    // Forward: from the start of "beta" to the end of "delta":
    click(&mut harness, pos(6));
    wait(&mut harness, 1.0);
    shift_double_click(&mut harness, pos(19));
    assert_eq!(
        copied_text(&mut harness).as_deref(),
        Some("beta gamma delta")
    );
}

#[test]
fn shift_double_click_should_extend_by_words_backward() {
    let (mut harness, char_pos) = text_edit_harness(WORDS);
    let pos = |i: usize| char_pos.borrow()[i];

    // Backward: from the middle of "gamma" to the start of "alpha".
    // The anchor stays where it was, in the middle of "gamma":
    click(&mut harness, pos(14));
    wait(&mut harness, 1.0);
    shift_double_click(&mut harness, pos(2));
    assert_eq!(copied_text(&mut harness).as_deref(), Some("alpha beta gam"));
}

#[test]
fn shift_double_click_drag_should_extend_by_words() {
    let (mut harness, char_pos) = text_edit_harness(WORDS);
    let pos = |i: usize| char_pos.borrow()[i];

    click(&mut harness, pos(6));
    wait(&mut harness, 1.0);

    // Shift-double-click on "gamma", keep the button down and drag into "delta":
    shift_click(&mut harness, pos(13));
    press_with(&mut harness, pos(13), Modifiers::SHIFT);
    drag_to(&mut harness, pos(19));
    assert_eq!(
        copied_text(&mut harness).as_deref(),
        Some("beta gamma delta")
    );

    // Drag back before the anchor, into "alpha":
    drag_to(&mut harness, pos(2));
    release_with(&mut harness, pos(2), Modifiers::SHIFT);
    assert_eq!(copied_text(&mut harness).as_deref(), Some("alpha "));
}

#[test]
fn shift_triple_click_should_extend_by_lines() {
    let (mut harness, char_pos) = text_edit_harness("alpha beta\ncarrot\ndelta epsilon");
    let pos = |i: usize| char_pos.borrow()[i];

    click(&mut harness, pos(2));
    wait(&mut harness, 1.0);
    shift_click(&mut harness, pos(13));
    shift_click(&mut harness, pos(13));
    shift_click(&mut harness, pos(13));

    assert_eq!(
        copied_text(&mut harness).as_deref(),
        Some("pha beta\ncarrot")
    );
}

const THREE_LABELS: &[&str] = &["alpha beta gamma", "delta epsilon zeta", "eta theta iota"];

#[test]
fn label_double_click_should_select_word() {
    let (mut harness, pos) = labels_harness_with(THREE_LABELS);

    click(&mut harness, pos(0, 8));
    click(&mut harness, pos(0, 8));

    assert_eq!(copied_text(&mut harness).as_deref(), Some("beta"));
}

#[test]
fn label_double_click_drag_should_select_words_backward() {
    let (mut harness, pos) = labels_harness_with(THREE_LABELS);

    // Double-click on "gamma", drag back into "alpha":
    double_click_drag(&mut harness, pos(0, 13), pos(0, 2));

    assert_eq!(
        copied_text(&mut harness).as_deref(),
        Some("alpha beta gamma")
    );
}

#[test]
fn label_double_click_drag_should_keep_anchor_when_reversing() {
    let (mut harness, pos) = labels_harness_with(THREE_LABELS);

    // Double-click on "epsilon" in the middle label, drag down into the last label...
    click(&mut harness, pos(1, 9));
    press(&mut harness, pos(1, 9));
    drag_to(&mut harness, pos(2, 6));
    assert_eq!(
        copied_text(&mut harness).as_deref(),
        Some("epsilon zeta\neta theta")
    );

    // Then up into the first label:
    drag_to(&mut harness, pos(0, 8));
    release(&mut harness, pos(0, 8));
    assert_eq!(
        copied_text(&mut harness).as_deref(),
        Some("beta gamma\ndelta epsilon")
    );
}

#[test]
fn label_triple_click_drag_should_select_lines_across_labels() {
    let (mut harness, pos) = labels_harness_with(THREE_LABELS);

    triple_click_drag(&mut harness, pos(0, 8), pos(1, 3));

    assert_eq!(
        copied_text(&mut harness).as_deref(),
        Some("alpha beta gamma\ndelta epsilon zeta")
    );
}

/// After a word drag, a shift-click should extend the selection by characters,
/// just like in a [`TextEdit`].
#[test]
fn label_shift_click_after_word_drag_should_extend_by_chars() {
    let (mut harness, pos) = labels_harness_with(THREE_LABELS);

    // Double-click on "beta", drag into "gamma":
    double_click_drag(&mut harness, pos(0, 8), pos(0, 13));
    assert_eq!(copied_text(&mut harness).as_deref(), Some("beta gamma"));

    wait(&mut harness, 1.0);
    shift_click(&mut harness, pos(1, 9));

    let copied = copied_text(&mut harness).unwrap_or_default();
    assert!(
        copied.starts_with("beta gamma\ndelta ep") && !copied.ends_with("epsilon"),
        "Expected a character-based extension, got {copied:?}"
    );
}

#[test]
fn label_shift_double_click_should_extend_by_words() {
    let (mut harness, pos) = labels_harness_with(THREE_LABELS);

    // Within one label, from the start of "beta" to the end of "gamma":
    click(&mut harness, pos(0, 6));
    wait(&mut harness, 1.0);
    shift_double_click(&mut harness, pos(0, 13));
    assert_eq!(copied_text(&mut harness).as_deref(), Some("beta gamma"));
}

#[test]
fn label_shift_double_click_should_extend_by_words_across_labels() {
    let (mut harness, pos) = labels_harness_with(THREE_LABELS);

    // Forward, from the start of "beta" to the end of "epsilon" in the next label:
    click(&mut harness, pos(0, 6));
    wait(&mut harness, 1.0);
    shift_double_click(&mut harness, pos(1, 9));
    assert_eq!(
        copied_text(&mut harness).as_deref(),
        Some("beta gamma\ndelta epsilon")
    );

    // Keep extending by words from the same anchor, with a shift-double-click-and-drag:
    wait(&mut harness, 1.0);
    shift_click(&mut harness, pos(1, 9));
    press_with(&mut harness, pos(1, 9), Modifiers::SHIFT);
    drag_to(&mut harness, pos(2, 6));
    release_with(&mut harness, pos(2, 6), Modifiers::SHIFT);
    assert_eq!(
        copied_text(&mut harness).as_deref(),
        Some("beta gamma\ndelta epsilon zeta\neta theta")
    );
}

#[test]
fn label_shift_double_click_should_extend_by_words_across_labels_backward() {
    let (mut harness, pos) = labels_harness_with(THREE_LABELS);

    // Backward, from the middle of "epsilon" to the start of "beta" in the previous label:
    click(&mut harness, pos(1, 9));
    wait(&mut harness, 1.0);
    shift_double_click(&mut harness, pos(0, 8));

    // The selection should start at a word boundary, but end at the anchor inside "epsilon":
    let copied = copied_text(&mut harness).unwrap_or_default();
    assert!(
        copied.starts_with("beta gamma\ndelta eps") && !copied.ends_with("epsilon"),
        "Expected a word-based extension that keeps the anchor, got {copied:?}"
    );
}
