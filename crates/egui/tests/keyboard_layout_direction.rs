//! Regression tests for the keyboard layout direction signal:
//! [`egui::RawInput::keyboard_layout_direction`] lets an integration report the
//! user's primary keyboard layout's script direction, and the [`egui::TextEdit`]
//! default layouter follows the interface's reading direction first and that
//! layout direction second when stamping the base direction of an edit.
//!
//! Every stamped case also asserts its twin, so this file doubles as the
//! LTR/interface-unchanged regression. No snapshot assertions; painted text
//! ink rects and job metadata only.

use egui::epaint::text::TextDirection;
use egui::{Context, Pos2, RawInput, ReadingDirection, Rect, TextEdit, Ui, Vec2, vec2};

const SCREEN: Vec2 = vec2(400.0, 300.0);

/// A mixed string that starts with a left-to-right character: under first-strong
/// inference it would lay out as an LTR paragraph, so the stamp is what makes it
/// read right-to-left.
const MIXED: &str = "Settings: الإعدادات";

fn input(size: Vec2, layout_direction: Option<ReadingDirection>) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        keyboard_layout_direction: layout_direction,
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, input: RawInput, run_ui: impl FnMut(&mut Ui)) -> egui::FullOutput {
    let mut output = ctx.run_ui(input, run_ui);
    output.textures_delta.clear();
    output
}

/// (text, job direction) for every text shape painted this pass.
fn painted_directions(output: &egui::FullOutput) -> Vec<(String, Option<TextDirection>)> {
    fn collect(shape: &egui::epaint::Shape, out: &mut Vec<(String, Option<TextDirection>)>) {
        match shape {
            egui::epaint::Shape::Text(text) => {
                out.push((text.galley.job.text.clone(), text.galley.job.direction));
            }
            egui::epaint::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut out);
    }
    out
}

fn painted_edit_direction(output: &egui::FullOutput) -> Option<TextDirection> {
    painted_directions(output)
        .iter()
        .find(|(painted, _)| painted == MIXED)
        .and_then(|(_, direction)| *direction)
}

/// A right-to-left keyboard layout stamps a right-to-left base direction on
/// the edited text, even in a default (left-to-right) interface.
#[test]
fn text_edit_follows_a_rtl_keyboard_layout() {
    let ctx = Context::default();
    let mut value = MIXED.to_owned();
    let output = run(&ctx, input(SCREEN, Some(ReadingDirection::Rtl)), |ui| {
        ui.add(TextEdit::singleline(&mut value).desired_width(200.0));
    });
    assert_eq!(
        painted_edit_direction(&output),
        Some(TextDirection::RightToLeft),
        "the TextEdit default layouter should follow the RTL keyboard layout"
    );
}

/// Without the signal the edited text stays first-strong (an LTR-first string
/// lays out left-to-right) — today's behavior, unchanged.
#[test]
fn text_edit_stays_first_strong_without_the_signal() {
    let ctx = Context::default();
    let mut value = MIXED.to_owned();
    let output = run(&ctx, input(SCREEN, None), |ui| {
        ui.add(TextEdit::singleline(&mut value).desired_width(200.0));
    });
    assert_eq!(
        painted_edit_direction(&output),
        Some(TextDirection::LeftToRight),
        "without the signal the TextEdit default layouter must not change"
    );
}

/// An explicit right-to-left interface keeps winning over an absent layout
/// signal (and over an explicit LTR one, which is the same stamp).
#[test]
fn interface_direction_wins_over_the_keyboard_layout() {
    for layout_direction in [None, Some(ReadingDirection::Ltr)] {
        let ctx = Context::default();
        ctx.set_right_to_left(true);
        let mut value = MIXED.to_owned();
        let output = run(&ctx, input(SCREEN, layout_direction), |ui| {
            ui.add(TextEdit::singleline(&mut value).desired_width(200.0));
        });
        assert_eq!(
            painted_edit_direction(&output),
            Some(TextDirection::RightToLeft),
            "the interface direction must win (layout = {layout_direction:?})"
        );
    }
}
