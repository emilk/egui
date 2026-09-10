//! Tests for double-click-and-drag (select by words) and
//! triple-click-and-drag (select by lines) text selection.
//! See <https://github.com/emilk/egui/issues/2550>.

use std::cell::RefCell;
use std::rc::Rc;

use egui::text::CCursor;
use egui::{Event, Modifiers, OutputCommand, PointerButton, Pos2, RichText, TextEdit, Vec2, vec2};
use egui_kittest::{Harness, HarnessBuilder};

/// Short enough that a few frames stay well within the double-click window (0.3 s).
const STEP_DT: f32 = 0.01;

fn press<S>(harness: &mut Harness<'_, S>, pos: Pos2) {
    harness.event(Event::PointerMoved(pos));
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
}

fn release<S>(harness: &mut Harness<'_, S>, pos: Pos2) {
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.step();
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

/// A [`TextEdit`] harness, plus the screen position of each character of its text.
fn text_edit_harness(text: &str) -> (Harness<'static, String>, Rc<RefCell<Vec<Pos2>>>) {
    let char_pos = Rc::new(RefCell::new(Vec::new()));
    let char_pos_clone = Rc::clone(&char_pos);

    let mut harness = HarnessBuilder::default()
        .with_step_dt(STEP_DT)
        .with_size(Vec2::new(400.0, 200.0))
        .build_ui_state(
            move |ui, text: &mut String| {
                let output = TextEdit::multiline(text).show(ui);
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

/// Two stacked labels, plus a function mapping (label index, char index) to screen position.
fn labels_harness() -> (Harness<'static>, impl Fn(usize, usize) -> Pos2) {
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
            for text in ["alpha beta gamma", "delta epsilon zeta"] {
                let rect = ui.label(RichText::new(text).monospace()).rect;
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
