//! The focused, editable [`egui::TextEdit`] reports the *content's* resolved
//! reading direction (and its language, when the text carries one) through
//! [`egui::output::IMEOutput::base_direction`] / [`::language`], so platform
//! IME APIs (TSF/UIA, macOS) can shape their candidate windows accordingly.
//!
//! The IME composes content, not UI chrome: a first-strong Arabic buffer
//! reports right-to-left even in a left-to-right interface.

use egui::{CentralPanel, Context, Id, RawInput, TextEdit, text::TextDirection};

fn ime_id() -> Id {
    Id::unique("ime_direction_test")
}

/// Run one frame with a focused, mutable `TextEdit` over `text`.
fn run(ctx: &Context, text: &mut String) -> egui::FullOutput {
    let mut output = ctx.run_ui(RawInput::default(), |ui| {
        CentralPanel::default().show(ui, |ui| {
            ui.memory_mut(|mem| mem.request_focus(ime_id()));
            let out = TextEdit::singleline(text).id(ime_id()).show(ui);
            out.state.store(ui.ctx(), ime_id());
        });
    });
    output.textures_delta.clear();
    output
}

#[test]
fn ime_output_reports_the_resolved_base_direction() {
    let ctx = Context::default();

    // Arabic buffer: first-strong right-to-left, in an LTR interface:
    let mut arabic = String::from("مرحبا");
    let output = run(&ctx, &mut arabic);
    let ime = output
        .platform_output
        .ime
        .expect("a focused editable publishes IMEOutput");
    assert_eq!(
        ime.base_direction,
        Some(TextDirection::RightToLeft),
        "the IME composes Arabic content, so its base direction is RTL"
    );

    // Latin buffer resolves left-to-right:
    let mut latin = String::from("hello");
    let output = run(&ctx, &mut latin);
    let ime = output
        .platform_output
        .ime
        .expect("a focused editable publishes IMEOutput");
    assert_eq!(ime.base_direction, Some(TextDirection::LeftToRight));
}

#[test]
fn ime_output_language_follows_the_laid_out_text() {
    // Nothing on this branch feeds `LayoutJob.language` for text edits yet
    // (per-Ui language plumbing is a separate change), so the field stays
    // `None` — the passthrough itself is what the field pins:
    let ctx = Context::default();
    let mut text = String::from("hello");
    let output = run(&ctx, &mut text);
    let ime = output
        .platform_output
        .ime
        .expect("a focused editable publishes IMEOutput");
    assert_eq!(ime.language, None);
}
