use egui::{Context, Event, PointerButton, Pos2, RawInput, Rect, TextEdit, Vec2, pos2, vec2};

/// Run one pass with a focusable `TextEdit`, and report whether egui asked the integration to
/// interrupt IME composition.
fn pass(ctx: &Context, text: &mut String, events: Vec<Event>) -> bool {
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0))),
            events,
            focused: true,
            ..Default::default()
        },
        |ui| {
            ui.add(TextEdit::singleline(text));
        },
    );
    output.textures_delta.clear(); // no renderer here to consume the font atlas
    output
        .platform_output
        .ime
        .is_some_and(|ime| ime.should_interrupt_composition)
}

fn press(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    }
}

/// Dragging inside an already focused `TextEdit` must not interrupt IME composition on every
/// pass: on Android that hides and re-shows the soft keyboard, making the UI flicker (#8884).
#[test]
fn dragging_in_a_focused_text_edit_does_not_interrupt_ime() {
    let ctx = Context::default();
    let mut text = "hello world".to_owned();
    let inside = pos2(30.0, 15.0);

    pass(&ctx, &mut text, vec![]);

    // Click to focus: this one may interrupt (focus changes).
    pass(
        &ctx,
        &mut text,
        vec![
            Event::PointerMoved(inside),
            press(inside, true),
            press(inside, false),
        ],
    );

    // Now drag to select. The widget keeps focus, so composition must be left alone.
    let mut interrupted = false;
    for dx in 1..6 {
        let to = inside + Vec2::new(dx as f32 * 8.0, 0.0);
        interrupted |= pass(
            &ctx,
            &mut text,
            vec![press(inside, true), Event::PointerMoved(to)],
        );
    }
    assert!(
        !interrupted,
        "IME composition was interrupted while dragging inside the focused TextEdit"
    );
}
