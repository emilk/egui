//! Regression tests for [`egui::ImeEvent::DeleteSurroundingBytes`]:
//! byte-exact IME deletion of surrounding text in [`egui::TextEdit`].
//!
//! Driven headlessly with scripted `egui::Event::Ime` events (no windowing
//! backend, no snapshot/render assertions). The char-counted
//! [`egui::ImeEvent::DeleteSurrounding`] keeps its character units; the
//! byte-counts test locks the two variants apart.

use egui::{
    CentralPanel, Context, Event, FullOutput, Id, ImeEvent, Key, Modifiers, RawInput, TextEdit,
    text::{CCursor, CCursorRange},
};

fn ime_id() -> Id {
    Id::unique("ime_delete_surrounding_test")
}

fn input(time: f64, events: Vec<Event>) -> RawInput {
    RawInput {
        time: Some(time),
        events,
        ..Default::default()
    }
}

/// Builds a key event, spelling all five `Event::Key` fields: `Event` derives
/// no `Default`, so functional-record-update cannot be used on the variant.
fn key_event(key: Key, pressed: bool, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        pressed,
        modifiers,
        repeat: false,
        physical_key: None,
    }
}

/// Run one frame with a focused `TextEdit` over `text`.
fn run(ctx: &Context, text: &mut String, time: f64, events: Vec<Event>) -> FullOutput {
    let mut output = ctx.run_ui(input(time, events), |ui| {
        CentralPanel::default().show(ui, |ui| {
            ui.memory_mut(|mem| mem.request_focus(ime_id()));
            let out = TextEdit::singleline(text).id(ime_id()).show(ui);
            out.state.store(ui.ctx(), ime_id()); // persist the cursor between frames
        });
    });
    output.textures_delta.clear(); // these tests never render
    output
}

/// Store the caret/selection the IME events will be anchored at (in a frame
/// that shows and focuses the widget, the way the `TextEditState` doc example
/// does), then run the event frame.
fn run_with_caret(
    ctx: &Context,
    text: &mut String,
    time: f64,
    events: Vec<Event>,
    char_range: CCursorRange,
) -> FullOutput {
    let mut setup = ctx.run_ui(input(time - 1.0, vec![]), |ui| {
        CentralPanel::default().show(ui, |ui| {
            ui.memory_mut(|mem| mem.request_focus(ime_id()));
            let mut out = TextEdit::singleline(text).id(ime_id()).show(ui);
            out.state.cursor.set_char_range(Some(char_range));
            out.state.store(ui.ctx(), ime_id());
        });
    });
    setup.textures_delta.clear(); // these tests never render
    drop(setup);
    run(ctx, text, time, events)
}

#[test]
fn byte_counts_delete_exactly_the_intended_bytes() {
    // A combining accent composes with the preceding letter into one grapheme,
    // but the byte count the IME reports stops right after the base letter.
    let ctx = Context::default();
    let mut text = "e\u{0301}ab".to_owned(); // "é" as e + combining accent
    run_with_caret(
        &ctx,
        &mut text,
        2.0,
        vec![
            Event::Ime(ImeEvent::DeleteSurroundingBytes {
                before_bytes: 0,
                after_bytes: 2,
            }),
            Event::Ime(ImeEvent::Commit("c".into())),
        ],
        CCursorRange::one(CCursor::new(2)), // after "e\u{0301}" (2 chars, 3 bytes)
    );
    // The 2 bytes after the caret are exactly "ab"; an integration that
    // misreads bytes as chars would delete the combining accent too.
    assert_eq!(text, "e\u{0301}c");
}

#[test]
fn multi_byte_graphemes_survive_byte_exact_replacement() {
    let family = "👨‍👩‍👧"; // ZWJ sequence: 3 emoji + 2 ZWJ = 18 bytes
    let ctx = Context::default();

    // Only the first 4 bytes (the man emoji) after the caret are replaced;
    // the rest of the family survives.
    let mut text = format!("a{family}b");
    run_with_caret(
        &ctx,
        &mut text,
        2.0,
        vec![
            Event::Ime(ImeEvent::DeleteSurroundingBytes {
                before_bytes: 0,
                after_bytes: 4,
            }),
            Event::Ime(ImeEvent::Commit("x".into())),
        ],
        CCursorRange::one(CCursor::new(1)), // after "a"
    );
    assert_eq!(text, format!("ax{}b", &family[4..]));

    // A flag is 2 regional indicators = 8 bytes. The before-count is measured
    // from the caret (the selection start), so the "!" after it survives.
    let mut text = "🇩🇪!".to_owned();
    run_with_caret(
        &ctx,
        &mut text,
        2.0,
        vec![
            Event::Ime(ImeEvent::DeleteSurroundingBytes {
                before_bytes: 8,
                after_bytes: 0,
            }),
            Event::Ime(ImeEvent::Commit("X".into())),
        ],
        CCursorRange::one(CCursor::new(2)), // after the flag (2 chars, 8 bytes)
    );
    assert_eq!(text, "X!");

    // An emoji before the caret is deleted byte-exactly too.
    let mut text = "a👨b".to_owned();
    run_with_caret(
        &ctx,
        &mut text,
        2.0,
        vec![
            Event::Ime(ImeEvent::DeleteSurroundingBytes {
                before_bytes: 4,
                after_bytes: 0,
            }),
            Event::Ime(ImeEvent::Commit("x".into())),
        ],
        CCursorRange::one(CCursor::new(2)), // after the emoji (2 chars, 5 bytes)
    );
    assert_eq!(text, "axb");
}

#[test]
fn byte_counts_are_not_char_counts() {
    let ctx = Context::default();

    // Bytes: the 2 bytes after the base letter are the combining accent only.
    // The empty commit is dropped by the guard for non-composing buffers, but
    // the delete still happened.
    let mut text = "x\u{0301}y".to_owned();
    run_with_caret(
        &ctx,
        &mut text,
        2.0,
        vec![
            Event::Ime(ImeEvent::DeleteSurroundingBytes {
                before_bytes: 0,
                after_bytes: 2,
            }),
            Event::Ime(ImeEvent::Commit(String::new())),
        ],
        CCursorRange::one(CCursor::new(1)), // after "x", before the accent
    );
    assert_eq!(text, "xy");

    // Chars: the same counts in the char-counted variant delete the accent
    // and the following letter. The two variants stay distinct units.
    let mut text = "x\u{0301}y".to_owned();
    run_with_caret(
        &ctx,
        &mut text,
        2.0,
        vec![
            Event::Ime(ImeEvent::DeleteSurrounding {
                before_chars: 0,
                after_chars: 2,
            }),
            Event::Ime(ImeEvent::Commit(String::new())),
        ],
        CCursorRange::one(CCursor::new(1)),
    );
    assert_eq!(text, "x");
}

#[test]
fn hostile_byte_counts_saturate_and_snap_to_boundaries() {
    let ctx = Context::default();

    // A count ending mid-character floors to the previous boundary, so the
    // partially covered emoji survives.
    let mut text = "a👨b".to_owned();
    run_with_caret(
        &ctx,
        &mut text,
        2.0,
        vec![
            Event::Ime(ImeEvent::DeleteSurroundingBytes {
                before_bytes: 0,
                after_bytes: 3, // ends inside the 4-byte emoji
            }),
            Event::Ime(ImeEvent::Commit("x".into())),
        ],
        CCursorRange::one(CCursor::new(1)), // after "a"
    );
    assert_eq!(text, "ax👨b");

    // Overshooting counts saturate at the text bounds instead of panicking.
    let mut text = "ab".to_owned();
    run(
        &ctx,
        &mut text,
        2.0,
        vec![
            Event::Ime(ImeEvent::DeleteSurroundingBytes {
                before_bytes: usize::MAX,
                after_bytes: usize::MAX,
            }),
            Event::Ime(ImeEvent::Commit("x".into())),
        ],
    );
    assert_eq!(text, "x");
}

#[test]
fn delete_plus_commit_is_one_undo_step() {
    let ctx = Context::default();
    let mut text = "hi there".to_owned();
    // Warm-up frame so the undoer records an undo point with the original text:
    run(&ctx, &mut text, 1.0, vec![]);
    run(
        &ctx,
        &mut text,
        2.0,
        vec![
            Event::Ime(ImeEvent::DeleteSurroundingBytes {
                before_bytes: 5,
                after_bytes: 0,
            }),
            Event::Ime(ImeEvent::Commit("you".into())),
        ],
    );
    assert_eq!(text, "hi you");

    // One undo step reverts the delete and the commit together. The modifiers
    // must be `COMMAND` (not `MAC_CMD`): the undo arm matches
    // `modifiers.matches_logically(Modifiers::COMMAND)`, and `MAC_CMD` does
    // not set the `command` flag.
    run(
        &ctx,
        &mut text,
        3.0,
        vec![key_event(Key::Z, true, Modifiers::COMMAND)],
    );
    assert_eq!(text, "hi there");
}

#[test]
fn plain_input_still_reaches_the_text_edit() {
    let ctx = Context::default();
    let mut text = String::new();
    // Printable input reaches `events()` only as `Event::Text` — that is how
    // integrations deliver typed text (a bare `Key::A` key event inserts
    // nothing). Harness sanity for the event plumbing above.
    run(&ctx, &mut text, 1.0, vec![Event::Text("a".into())]);
    assert_eq!(text, "a");

    // Plain key events still mutate the buffer on the paths that exist:
    // backspace deletes the char before the caret.
    run(
        &ctx,
        &mut text,
        2.0,
        vec![key_event(Key::Backspace, true, Modifiers::default())],
    );
    assert_eq!(text, "");
}

#[test]
fn counts_around_a_selection() {
    let ctx = Context::default();

    // (a) The before-count saturates at the bytes available before the
    // selection start; the after-count is anchored at the selection end.
    let mut text = "abcde".to_owned();
    run_with_caret(
        &ctx,
        &mut text,
        2.0,
        vec![
            Event::Ime(ImeEvent::DeleteSurroundingBytes {
                before_bytes: 2,
                after_bytes: 2,
            }),
            Event::Ime(ImeEvent::Commit("X".into())),
        ],
        CCursorRange::two(CCursor::new(1), CCursor::new(3)), // select "bc"
    );
    // "a" (saturated from 2 to the 1 available byte) + "bc" (the selection) +
    // "de" (2 bytes after the selection end) are all replaced.
    assert_eq!(text, "X");

    // (b) The anchoring discriminator: with a multi-byte char at the selection
    // start, the after-count must be measured from the selection end.
    let mut text = "aébc".to_owned();
    run_with_caret(
        &ctx,
        &mut text,
        2.0,
        vec![
            Event::Ime(ImeEvent::DeleteSurroundingBytes {
                before_bytes: 0,
                after_bytes: 2,
            }),
            Event::Ime(ImeEvent::Commit("X".into())),
        ],
        CCursorRange::two(CCursor::new(1), CCursor::new(2)), // select "é"
    );
    // after_bytes: 2 from the selection end (byte 3) covers "bc" (2 chars).
    // A start-anchored conversion would measure the 2 bytes of "é" instead,
    // delete only "b" after the selection end, and produce "aXc".
    assert_eq!(text, "aX");
}
