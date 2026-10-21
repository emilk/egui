use egui::{Context, Id, RawInput, Rect, TextEdit, pos2, vec2};

/// Runs one pass with text fields `a` and `b`, requesting focus for `focus` first,
/// and returns whether the pass asked the integration to interrupt the IME.
fn pass(ctx: &Context, [a, b]: [Id; 2], focus: Id) -> bool {
    let mut text_a = String::new();
    let mut text_b = String::new();
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0))),
            ..Default::default()
        },
        |ui| {
            ui.memory_mut(|mem| mem.request_focus(focus));
            ui.add(TextEdit::singleline(&mut text_a).id(a));
            ui.add(TextEdit::singleline(&mut text_b).id(b));
        },
    );
    output.textures_delta.clear();
    output
        .platform_output
        .ime
        .expect("the focused text field sets IME output")
        .should_interrupt_composition
}

#[test]
fn request_focus_interrupts_ime_only_when_focus_moves() {
    let ctx = Context::default();
    let ids @ [a, b] = [Id::unique("a"), Id::unique("b")];

    assert!(pass(&ctx, ids, a), "focusing a field interrupts");
    assert!(
        !pass(&ctx, ids, a),
        "re-requesting focus each frame keeps composition"
    );
    assert!(!pass(&ctx, ids, a));
    assert!(
        pass(&ctx, ids, b),
        "moving focus to another field interrupts"
    );
    assert!(!pass(&ctx, ids, b));
}
