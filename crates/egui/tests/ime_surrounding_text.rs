//! Regression tests for the IME surrounding-text report:
//! [`egui::output::IMEOutput::surrounding_text`] and
//! [`egui::ImeEvent::RequestSurroundingText`].
//!
//! Driven headlessly (no windowing backend, no snapshot/render assertions):
//! a focused, mutable `TextEdit` reports the unmasked buffer text and the
//! byte-indexed cursor/selection while it owns IME events; password fields
//! report nothing; the OS-initiated request event refreshes the report.

use egui::{
    CentralPanel, Context, Event, FullOutput, IMEPurpose, Id, ImeEvent, Key, Modifiers, RawInput,
    TextEdit,
    text::{CCursor, CCursorRange},
};

fn ime_id() -> Id {
    Id::unique("ime_surrounding_text_test")
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
    run_edit(ctx, text, time, events, false)
}

/// Like [`run`], but first runs a frame that stores the caret/selection the
/// report is asserted against (the way the `TextEditState` doc example does).
fn run_with_caret(
    ctx: &Context,
    text: &mut String,
    time: f64,
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
    run(ctx, text, time, vec![])
}

/// Run one frame with a focused `TextEdit` over `text`, optionally masked.
fn run_edit(
    ctx: &Context,
    text: &mut String,
    time: f64,
    events: Vec<Event>,
    password: bool,
) -> FullOutput {
    let mut output = ctx.run_ui(input(time, events), |ui| {
        CentralPanel::default().show(ui, |ui| {
            ui.memory_mut(|mem| mem.request_focus(ime_id()));
            let mut edit = TextEdit::singleline(text).id(ime_id());
            if password {
                edit = edit.password(true);
            }
            let out = edit.show(ui);
            out.state.store(ui.ctx(), ime_id()); // persist the cursor between frames
        });
    });
    output.textures_delta.clear(); // these tests never render
    output
}

#[test]
fn reports_text_and_byte_selection() {
    let ctx = Context::default();
    let mut text = "日本語のテキスト".to_owned(); // 8 chars, 24 bytes

    let ime = run(&ctx, &mut text, 1.0, vec![])
        .platform_output
        .ime
        .expect("focused TextEdit reports IMEOutput");
    let surrounding = ime.surrounding_text.expect("surrounding text reported");
    assert_eq!(surrounding.text, "日本語のテキスト");
    // The caret defaults to the end of the buffer, as a byte offset:
    assert_eq!(surrounding.selection, 24..24);

    // Idle focused frame: the cached report is handed out unchanged.
    let idle_ime = run(&ctx, &mut text, 1.5, vec![])
        .platform_output
        .ime
        .expect("still focused");
    assert_eq!(idle_ime.surrounding_text, Some(surrounding));

    // Move the caret to char 5 (after "日本語のテ") with three ArrowLeft
    // presses: the report follows, byte-indexed (not char-indexed 5..5).
    let mut ime = idle_ime;
    for t in [2.0, 2.1, 2.2] {
        ime = run(
            &ctx,
            &mut text,
            t,
            vec![key_event(Key::ArrowLeft, true, Modifiers::NONE)],
        )
        .platform_output
        .ime
        .expect("still focused");
    }
    let surrounding = ime.surrounding_text.expect("surrounding text reported");
    assert_eq!(surrounding.selection, 15..15);
}

#[test]
fn selection_is_byte_indexed() {
    let ctx = Context::default();
    let mut text = "famille 👨‍👩‍👧‍👦 fin".to_owned();

    // Select the ZWJ family emoji: chars 8..15 (after "famille "),
    // which is bytes 8..33 — a char range would be wrong by 18 bytes.
    // The range is placed explicitly (deterministic, still exercises the
    // report; scripted shift-extend is brittle).
    let ime = run_with_caret(
        &ctx,
        &mut text,
        2.0,
        CCursorRange::two(CCursor::new(8), CCursor::new(15)),
    )
    .platform_output
    .ime
    .expect("focused TextEdit reports IMEOutput");
    let surrounding = ime.surrounding_text.expect("surrounding text reported");
    assert_eq!(surrounding.text, "famille 👨‍👩‍👧‍👦 fin");
    assert_eq!(surrounding.selection, 8..33);
}

#[test]
fn password_fields_report_none() {
    let ctx = Context::default();
    let mut text = "hunter2".to_owned();
    let ime = run_edit(&ctx, &mut text, 1.0, vec![], true)
        .platform_output
        .ime
        .expect("focused TextEdit reports IMEOutput");
    assert_eq!(ime.purpose, IMEPurpose::Password);
    // Never leak plaintext through the IME channel.
    assert!(ime.surrounding_text.is_none());
    // Note: `should_interrupt_composition` is not asserted — `request_focus`
    // interrupts IME composition in the same frame, which is out of scope here.
}

#[test]
fn request_event_refreshes_the_report() {
    let ctx = Context::default();
    let mut text = "abc".to_owned();
    let before = run(&ctx, &mut text, 1.0, vec![])
        .platform_output
        .ime
        .expect("focused TextEdit reports IMEOutput")
        .surrounding_text
        .expect("surrounding text reported");
    assert_eq!(before.text, "abc");

    // Swap the bound buffer externally with a string of the same length and
    // the same first/last bytes, so the (len, first, last) buffer identity
    // check alone cannot refresh the report ("abd" would change the last
    // byte and make this test pass without the request event). Only the
    // pushed `RequestSurroundingText` event can trigger the rebuild.
    text = "aXc".to_owned();
    let surrounding = run(
        &ctx,
        &mut text,
        2.0,
        vec![Event::Ime(ImeEvent::RequestSurroundingText)],
    )
    .platform_output
    .ime
    .expect("focused TextEdit reports IMEOutput")
    .surrounding_text
    .expect("surrounding text reported after the request");
    assert_eq!(surrounding.text, "aXc");
    assert_ne!(surrounding, before);
}

#[test]
fn no_report_after_unfocus() {
    let ctx = Context::default();
    let mut text = "abc".to_owned();
    run(&ctx, &mut text, 1.0, vec![]);

    // A frame where focus is surrendered before the widget is shown: the
    // report block only runs for the widget that owns IME events.
    let mut output = ctx.run_ui(input(2.0, vec![]), |ui| {
        CentralPanel::default().show(ui, |ui| {
            ui.memory_mut(|mem| mem.surrender_focus(ime_id()));
            TextEdit::singleline(&mut text).id(ime_id()).show(ui);
        });
    });
    output.textures_delta.clear();
    assert!(output.platform_output.ime.is_none());
}
