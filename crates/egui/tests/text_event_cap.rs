//! Regression tests for the text-event size cap ([`egui::MAX_TEXT_EVENT_CHARS`],
//! upstream issue #2150) and for `TextEdit::char_limit` bounding text before it is
//! laid out.
//!
//! Driven headlessly (no windowing backend, no snapshot/render assertions):
//! A1 every text-carrying event carried into a pass is cut to the cap, at a character
//! boundary, before any widget sees it; A2 an IME preedit range past the cut is
//! dropped; A3 short text and non-text events pass through untouched; A4 an explicit
//! `char_limit` finishes the frame with a bounded value and never lays out more;
//! A5 the default `char_limit` is the event cap — both on a pasted event (already
//! cut in `begin_pass`) and on the accesskit `SetValue` path, which the begin-pass
//! cap deliberately leaves uncapped, so there the field's own default limit is the
//! only bound.

use core::cell::Cell;

use egui::{
    CentralPanel, Color32, Context, Event, FullOutput, Id, ImeEvent, RawInput, TextBuffer,
    TextEdit, TextStyle, Ui, accesskit, epaint::text::LayoutJob, pos2,
};

const MAX: usize = egui::MAX_TEXT_EVENT_CHARS;

fn input(time: f64, events: Vec<Event>) -> RawInput {
    RawInput {
        time: Some(time),
        events,
        ..Default::default()
    }
}

/// Run one frame with a focused `TextEdit` over `text`, with an optional explicit
/// `char_limit`, through a layouter that records the largest character count it was
/// ever asked to lay out in `max_seen` (and then lays out exactly what it saw).
fn run_edit(
    ctx: &Context,
    id: Id,
    text: &mut String,
    time: f64,
    events: Vec<Event>,
    char_limit: Option<usize>,
    max_seen: &Cell<usize>,
) -> FullOutput {
    let mut output = ctx.run_ui(input(time, events), |ui| {
        CentralPanel::default().show(ui, |ui| {
            ui.memory_mut(|mem| mem.request_focus(id));
            let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, wrap_width: f32| {
                let text = buf.as_str();
                max_seen.set(max_seen.get().max(text.chars().count()));
                let job = LayoutJob::simple(
                    text.to_owned(),
                    TextStyle::Body.resolve(ui.style()),
                    Color32::WHITE,
                    wrap_width,
                );
                ui.fonts_mut(|f| f.layout_job(job))
            };
            let mut edit = TextEdit::multiline(text).id(id).layouter(&mut layouter);
            if let Some(limit) = char_limit {
                edit = edit.char_limit(limit);
            }
            edit.show(ui);
        });
    });
    output.textures_delta.clear(); // these tests never render
    output
}

/// A1, A2: every text-carrying event entering a pass is cut to exactly the cap
/// (at a character boundary: the test string is multi-byte, so a byte-boundary
/// truncation would panic); the preedit range past the cut is dropped; short
/// text and non-text events pass through untouched.
#[test]
fn oversized_text_events_are_capped_on_begin_pass() {
    let long = "é".repeat(MAX + 10);
    let ctx = Context::default();
    ctx.begin_pass(input(
        1.0,
        vec![
            Event::Paste(long.clone()),
            Event::Text(long.clone()),
            Event::Ime(ImeEvent::Commit(long.clone())),
            Event::Ime(ImeEvent::Preedit {
                text: long.clone(),
                active_range_chars: Some(0..MAX + 2),
            }),
            Event::MiddleClickPaste {
                pos: pos2(5.0, 5.0),
                text: long.clone(),
            },
            Event::Text("ok".into()),
            Event::Copy,
        ],
    ));

    let events = ctx.input(|i| i.events.clone());
    assert_eq!(events.len(), 7);

    for e in &events[..5] {
        let text = match e {
            Event::Paste(t)
            | Event::Text(t)
            | Event::MiddleClickPaste { text: t, .. }
            | Event::Ime(ImeEvent::Commit(t) | ImeEvent::Preedit { text: t, .. }) => t,
            other => panic!("unexpected event: {other:?}"),
        };
        assert_eq!(text.chars().count(), MAX, "{e:?}");
    }

    // A2: the highlighted range indexes the pre-truncation text; past the cut
    // it would point nowhere, so it is dropped.
    assert!(
        matches!(
            &events[3],
            Event::Ime(ImeEvent::Preedit {
                active_range_chars: None,
                ..
            })
        ),
        "the preedit range past the cut is dropped"
    );

    // A3: short text and non-text events pass through untouched.
    assert_eq!(events[5], Event::Text("ok".into()));
    assert_eq!(events[6], Event::Copy);
}

/// A3: a pass carrying only short text and non-text events stores them
/// byte-identically — the cap changes nothing about legitimate input.
#[test]
fn short_text_and_non_text_events_pass_through() {
    let events_in = vec![
        Event::Text("héllo".into()),
        Event::Copy,
        Event::PointerMoved(pos2(1.0, 2.0)),
    ];
    let ctx = Context::default();
    ctx.begin_pass(input(1.0, events_in.clone()));
    ctx.input(|i| assert_eq!(i.events, events_in));
}

/// A4: an oversized paste into a `char_limit(6)` field finishes the frame with a
/// 6-char value, and the layouter never saw more than 6 characters — the layout
/// cost of the untrimmed text is never paid (trim happens before layout).
#[test]
fn text_edit_applies_char_limit_before_layout() {
    let id = Id::unique("char_limit_before_layout");
    let mut s = String::new();
    let max_seen = Cell::new(0usize);
    let ctx = Context::default();

    // Frame 1: add the field and grant it focus (granted at the end of the pass).
    run_edit(&ctx, id, &mut s, 1.0, vec![], Some(6), &max_seen);

    // Frame 2: an oversized paste (bigger than the event cap itself).
    let oversized = "a".repeat(MAX + 1);
    run_edit(
        &ctx,
        id,
        &mut s,
        2.0,
        vec![Event::Paste(oversized)],
        Some(6),
        &max_seen,
    );

    assert_eq!(s, "aaaaaa");
    assert!(
        max_seen.get() <= 6,
        "the layouter saw {} characters; the limit must trim before layout",
        max_seen.get()
    );
}

/// A5: with no explicit limit, `char_limit` defaults to the event cap, so a paste
/// larger than the cap leaves at most `MAX_TEXT_EVENT_CHARS` characters in the
/// field, and the layouter never saw more.
#[test]
fn char_limit_defaults_to_the_event_cap() {
    let id = Id::unique("default_char_limit");
    let mut s = String::new();
    let max_seen = Cell::new(0usize);
    let ctx = Context::default();

    run_edit(&ctx, id, &mut s, 1.0, vec![], None, &max_seen);

    let oversized = "a".repeat(MAX + 1);
    run_edit(
        &ctx,
        id,
        &mut s,
        2.0,
        vec![Event::Paste(oversized)],
        None,
        &max_seen,
    );

    assert_eq!(s.chars().count(), MAX);
    assert!(
        max_seen.get() <= MAX,
        "the layouter saw {} characters; the cap must bound layout",
        max_seen.get()
    );
}

/// A5, mechanism: with no explicit limit, the field's default `char_limit` — not the
/// begin-pass cap — is what bounds an oversized insert on the one text path
/// `cap_text_events` deliberately skips: an accesskit `SetValue` request
/// ([`Event::AccessKitActionRequest`] with [`accesskit::ActionData::Value`]; assistive
/// technologies set the whole value without focusing the field first, so this does not
/// wait for focus). Such an event is stored verbatim — `cap_text_events` matches only
/// the typed/pasted/IME variants — so the value reaches the widget intact and the only
/// thing trimming it is the field's `char_limit`, applied inside `insert_text_at`
/// *before* the post-insert layout.
///
/// Discriminator: this test FAILS if the `char_limit` default were `usize::MAX` —
/// the insert would then not be trimmed at all, leaving `MAX + 1` characters in the
/// field and in the layouter — unlike `char_limit_defaults_to_the_event_cap`, whose
/// oversized paste is already cut to the cap in `InputState::begin_pass` before any
/// widget sees it.
#[test]
fn char_limit_bounds_the_accesskit_set_value_path() {
    let id = Id::unique("accesskit_set_value_default_limit");
    let mut s = String::new();
    let max_seen = Cell::new(0usize);
    let ctx = Context::default();

    // Frame 1: add the field with the default limit (focus is granted as in the
    // other tests, but the SetValue path does not need it).
    run_edit(&ctx, id, &mut s, 1.0, vec![], None, &max_seen);

    // Frame 2: an oversized SetValue, bigger than the event cap itself, delivered
    // the way an integration forwards an accesskit action.
    let oversized = "a".repeat(MAX + 1);
    let request = accesskit::ActionRequest {
        action: accesskit::Action::SetValue,
        target_tree: accesskit::TreeId::ROOT,
        target_node: id.accesskit_id(),
        data: Some(accesskit::ActionData::Value(oversized.into())),
    };
    run_edit(
        &ctx,
        id,
        &mut s,
        2.0,
        vec![Event::AccessKitActionRequest(request)],
        None,
        &max_seen,
    );

    assert_eq!(s.chars().count(), MAX);
    assert!(
        max_seen.get() <= MAX,
        "the layouter saw {} characters; the default char_limit must trim the \
         accesskit insert before layout",
        max_seen.get()
    );
}
