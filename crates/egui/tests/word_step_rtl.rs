//! Word-wise caret motion (`Ctrl`/`Alt`+arrow) and word deletes under a
//! right-to-left interface.
//!
//! Columns are logical character offsets over the row's visual-order glyphs
//! (0 = reading start, anchored by the row's base direction). The word arrows
//! must step between word-edge stops in the row's *visual* frame — the
//! word-granularity mirror of the character arrows' local-level stepping — so
//! `Ctrl+Left` on a right-to-left row walks toward the visually-left word edge
//! and `Ctrl+Right` toward the visually-right one. The LTR twins pin the
//! unchanged logical walk.
//!
//! Drives the real `Context::set_right_to_left` → [`egui::TextEdit`] → key
//! event path; no snapshot assertions.

use egui::widgets::text_edit::TextEditState;
use egui::{
    Context, FullOutput, Id, Key, Modifiers, Pos2, RawInput, Rect, TextEdit, Ui, Vec2,
    text::{CCursor, CCursorRange},
};

const SCREEN: Vec2 = Vec2::new(400.0, 120.0);

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(input(SCREEN), run_ui);
    output.textures_delta.clear();
    output
}

/// Run one ui pass with explicit events and discard the texture deltas.
fn run_events(ctx: &Context, events: Vec<egui::Event>, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(
        RawInput {
            events,
            ..input(SCREEN)
        },
        run_ui,
    );
    output.textures_delta.clear();
    output
}

/// Builds a key event, spelling all five `Event::Key` fields (`Event` derives
/// no `Default`, so functional-record-update cannot be used on the variant).
fn key_event(key: Key, pressed: bool, modifiers: Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        pressed,
        modifiers,
        repeat: false,
        physical_key: None,
    }
}

fn edit_id() -> Id {
    Id::unique("word_step_rtl_test")
}

/// Put a single caret at `index` on a focused singleline field, deliver `event`
/// in the next frame, and return the caret column the field holds afterwards.
fn caret_after_event(rtl: bool, text: &mut String, index: usize, event: egui::Event) -> usize {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);

    // Prime the caret on the focused field:
    drop(run(&ctx, |ui| {
        ui.memory_mut(|mem| mem.request_focus(edit_id()));
        let mut out = TextEdit::singleline(&mut *text).id(edit_id()).show(ui);
        out.state
            .cursor
            .set_char_range(Some(CCursorRange::one(CCursor::new(index))));
        out.state.store(ui.ctx(), edit_id());
    }));

    // Deliver the key press:
    run_events(&ctx, vec![event], |ui| {
        ui.memory_mut(|mem| mem.request_focus(edit_id()));
        let out = TextEdit::singleline(&mut *text).id(edit_id()).show(ui);
        out.state.store(ui.ctx(), edit_id());
    });

    TextEditState::load(&ctx, edit_id())
        .and_then(|state| state.cursor.char_range())
        .and_then(|range| range.single())
        .map(|cursor| cursor.index.0)
        .unwrap_or(usize::MAX)
}

/// Put a single caret at `index` on a focused singleline field, deliver `event`
/// in the next frame, and return the field's text afterwards.
fn text_after_event(rtl: bool, text: &mut String, index: usize, event: egui::Event) -> String {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);

    drop(run(&ctx, |ui| {
        ui.memory_mut(|mem| mem.request_focus(edit_id()));
        let mut out = TextEdit::singleline(&mut *text).id(edit_id()).show(ui);
        out.state
            .cursor
            .set_char_range(Some(CCursorRange::one(CCursor::new(index))));
        out.state.store(ui.ctx(), edit_id());
    }));

    run_events(&ctx, vec![event], |ui| {
        ui.memory_mut(|mem| mem.request_focus(edit_id()));
        let out = TextEdit::singleline(&mut *text).id(edit_id()).show(ui);
        out.state.store(ui.ctx(), edit_id());
    });

    text.clone()
}

/// `Ctrl+Left` from the reading start (the row's visual right edge) of a
/// uniform right-to-left row must enter the text and land on the first-read
/// word's visually-left edge — the boundary after the reading-last word تقرير
/// (column 5).
#[test]
fn rtl_uniform_row_ctrl_left_walks_visually_left() {
    let mut text = String::from("تقرير جميل");
    let column = caret_after_event(
        true,
        &mut text,
        0,
        key_event(Key::ArrowLeft, true, Modifiers::CTRL),
    );
    assert_eq!(
        column, 5,
        "Ctrl+Left from the reading start must land on the visually-left edge \
         of the first-read word (column 5), not clamp at the reading start"
    );
}

/// `Ctrl+Right` from the reading end (the row's visual left edge) of a uniform
/// right-to-left row must enter the text and land on the visually-right edge
/// of the word that displays leftmost (before جميل, column 6).
#[test]
fn rtl_uniform_row_ctrl_right_walks_visually_right() {
    let mut text = String::from("تقرير جميل");
    let column = caret_after_event(
        true,
        &mut text,
        10,
        key_event(Key::ArrowRight, true, Modifiers::CTRL),
    );
    assert_eq!(
        column, 6,
        "Ctrl+Right from the reading end must land before the visually-left \
         word (column 6), not clamp at the reading end"
    );
}

/// `Ctrl+Left` from the reading start of a mixed right-to-left row (عربي abc)
/// must cross into the Latin run and land on its visually-left edge — between
/// ي and the space (column 4).
#[test]
fn rtl_mixed_row_ctrl_left_walks_visually_left() {
    let mut text = String::from("عربي abc");
    let column = caret_after_event(
        true,
        &mut text,
        0,
        key_event(Key::ArrowLeft, true, Modifiers::CTRL),
    );
    assert_eq!(
        column, 4,
        "Ctrl+Left from the reading start must land on the Arabic word's \
         visually-left edge (column 4), not clamp at 0"
    );
}

/// `Ctrl+Left` at abc's visually-left edge (column 5 of عربي abc, the visually
/// leftmost word stop in the row) must clamp: the single row has nothing
/// visually left of that word edge to land on.
#[test]
fn rtl_mixed_row_ctrl_left_clamps_at_the_visual_left_edge() {
    let mut text = String::from("عربي abc");
    let column = caret_after_event(
        true,
        &mut text,
        5,
        key_event(Key::ArrowLeft, true, Modifiers::CTRL),
    );
    assert_eq!(
        column, 5,
        "Ctrl+Left at the row's visually-leftmost word stop must clamp in \
         place, not teleport to the reading start"
    );
}

/// `Ctrl+Right` from abc's visually-right edge (column 8 of عربي abc, the
/// reading start) must cross back into the Arabic run and land on its
/// visually-right edge (column 0, the row's visual right edge).
#[test]
fn rtl_mixed_row_ctrl_right_walks_visually_right() {
    let mut text = String::from("عربي abc");
    let column = caret_after_event(
        true,
        &mut text,
        8,
        key_event(Key::ArrowRight, true, Modifiers::CTRL),
    );
    assert_eq!(
        column, 0,
        "Ctrl+Right from the reading start must land on the row's visual \
         right edge (column 0)"
    );
}

/// `Ctrl+Backspace` before جميل (column 6 of تقرير جميل) must delete the word
/// visually left of the caret — جميل — leaving تقرير and the space.
#[test]
fn rtl_ctrl_backspace_deletes_the_visually_left_word() {
    let mut text = String::from("تقرير جميل");
    let after = text_after_event(
        true,
        &mut text,
        6,
        key_event(Key::Backspace, true, Modifiers::CTRL),
    );
    assert_eq!(
        after, "تقرير ",
        "Ctrl+Backspace before جميل must delete the visually-left word جميل, \
         leaving the reading-first word and the space intact"
    );
}

/// `Ctrl+Delete` after جميل's visually-right edge (column 6 of تقرير جميل) must
/// delete the word visually right of the caret — تقرير, which displays to the
/// caret's left — together with the separating space, leaving جميل.
#[test]
fn rtl_ctrl_delete_deletes_the_visually_right_word() {
    let mut text = String::from("تقرير جميل");
    let after = text_after_event(
        true,
        &mut text,
        6,
        key_event(Key::Delete, true, Modifiers::CTRL),
    );
    assert_eq!(
        after, "جميل",
        "Ctrl+Delete before جميل must delete the visually-right word تقرير \
         and the space, leaving جميل"
    );
}

/// LTR twin: `Ctrl+Backspace` before a word deletes it exactly as before.
#[test]
fn ltr_ctrl_backspace_deletes_the_visually_left_word() {
    let mut text = String::from("hello world");
    let after = text_after_event(
        false,
        &mut text,
        6,
        key_event(Key::Backspace, true, Modifiers::CTRL),
    );
    assert_eq!(
        after, "world",
        "LTR: Ctrl+Backspace before `world` must delete `hello ` and keep \
         `world`"
    );
}

/// LTR twin: under a left-to-right interface the word arrows keep the historic
/// logical walk — `Ctrl+Left` steps to word starts, `Ctrl+Right` to word ends.
#[test]
fn ltr_chrome_keeps_the_logical_word_walk() {
    let mut text = String::from("hello world");
    let left_from_end = caret_after_event(
        false,
        &mut text,
        11,
        key_event(Key::ArrowLeft, true, Modifiers::CTRL),
    );
    assert_eq!(left_from_end, 6, "LTR: Ctrl+Left lands on the word start");
    let left_to_start = caret_after_event(
        false,
        &mut text,
        6,
        key_event(Key::ArrowLeft, true, Modifiers::CTRL),
    );
    assert_eq!(left_to_start, 0, "LTR: Ctrl+Left lands on the text start");

    let mut text = String::from("hello world");
    let right_from_start = caret_after_event(
        false,
        &mut text,
        0,
        key_event(Key::ArrowRight, true, Modifiers::CTRL),
    );
    assert_eq!(right_from_start, 5, "LTR: Ctrl+Right lands on the word end");
    let right_to_end = caret_after_event(
        false,
        &mut text,
        5,
        key_event(Key::ArrowRight, true, Modifiers::CTRL),
    );
    assert_eq!(right_to_end, 11, "LTR: Ctrl+Right lands on the text end");
}

/// A Latin value under a right-to-left interface (the field stamps the row's
/// base direction, so all glyphs share one even embedding level and the row
/// reads left-to-right) keeps the historic logical walk.
#[test]
fn latin_value_under_rtl_chrome_keeps_the_logical_word_walk() {
    let mut text = String::from("Hello world");
    let left_from_end = caret_after_event(
        true,
        &mut text,
        11,
        key_event(Key::ArrowLeft, true, Modifiers::CTRL),
    );
    assert_eq!(left_from_end, 6);
    let left_to_start = caret_after_event(
        true,
        &mut text,
        6,
        key_event(Key::ArrowLeft, true, Modifiers::CTRL),
    );
    assert_eq!(left_to_start, 0);

    let mut text = String::from("Hello world");
    let right_from_start = caret_after_event(
        true,
        &mut text,
        0,
        key_event(Key::ArrowRight, true, Modifiers::CTRL),
    );
    assert_eq!(right_from_start, 5);
    let right_to_end = caret_after_event(
        true,
        &mut text,
        5,
        key_event(Key::ArrowRight, true, Modifiers::CTRL),
    );
    assert_eq!(right_to_end, 11);
}
