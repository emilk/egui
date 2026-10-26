//! A click outside a focused [`TextEdit`] must not drop its selection before
//! the widget that was clicked has run.

use egui::text::{CCursor, CCursorRange};
use egui::{Context, Event, Id, PointerButton, Pos2, RawInput, Rect, TextEdit, pos2, vec2};

const BUTTON: Pos2 = pos2(30.0, 130.0);

/// Runs a pass with a text field above a button, and returns the field's
/// selection as the button saw it, if the button was clicked.
fn pass(ctx: &Context, text: &mut String, events: Vec<Event>) -> Option<(usize, usize)> {
    let field = Id::unique("field");
    let mut seen = None;
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))),
            events,
            ..Default::default()
        },
        |ui| {
            ui.add(TextEdit::singleline(text).id(field));
            ui.add_space(100.0);
            if ui.button("Cut").clicked() {
                let range =
                    TextEdit::load_state(ui.ctx(), field).and_then(|s| s.cursor.char_range());
                seen = range.map(|r| (r.secondary.index.0, r.primary.index.0));
            }
        },
    );
    output.textures_delta.clear();
    seen
}

#[test]
fn clicking_away_keeps_the_selection_until_the_clicked_widget_has_run() {
    let ctx = Context::default();
    let field = Id::unique("field");
    let mut text = "hello world".to_owned();
    pass(&ctx, &mut text, vec![]);
    ctx.memory_mut(|m| m.request_focus(field));
    pass(&ctx, &mut text, vec![]);
    let mut state = TextEdit::load_state(&ctx, field).expect("text edit state");
    state
        .cursor
        .set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(11))));
    state.store(&ctx, field);
    pass(&ctx, &mut text, vec![]);

    let button = |pressed| Event::PointerButton {
        pos: BUTTON,
        button: PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    pass(
        &ctx,
        &mut text,
        vec![Event::PointerMoved(BUTTON), button(true)],
    );
    let seen = pass(&ctx, &mut text, vec![button(false)]);

    assert_eq!(seen, Some((0, 11)), "the button saw the selection");
    assert!(
        !ctx.memory(|m| m.has_focus(field)),
        "the click away still took the focus"
    );
}
