//! Regression tests for the progress bar's inner text anchor under a right-to-left
//! reading direction: the text inside the bar sits at the reading start (right under
//! the interface flag, left under the default), mirroring the bar's fill, which
//! already hugs the reading-start edge.
//!
//! The progress bar's chrome follows the interface reading direction only (the
//! [`egui::Context::set_right_to_left`] flag, like its fill), so only that mechanism
//! is exercised here. Every mirrored case also asserts its left-to-right counterpart,
//! so this file doubles as the LTR-unchanged regression. No snapshot assertions;
//! painted text layout-rect geometry only.

use egui::epaint::Shape;
use egui::{Context, FullOutput, Pos2, ProgressBar, RawInput, Rect, Ui, Vec2, vec2};

const SCREEN: Vec2 = vec2(400.0, 120.0);

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, input: RawInput, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(input, run_ui);
    output.textures_delta.clear();
    output
}

/// (text, absolute layout rect) for every text shape painted this pass.
///
/// `Galley::rect` is anchored on its halign edge (`rect.left() == 0.0` for
/// `Align::LEFT`, `rect.right() == 0.0` for `Align::RIGHT`), so the translated rect
/// shows exactly where the anchor put the text.
fn painted_texts(output: &FullOutput) -> Vec<(String, Rect)> {
    fn collect(shape: &Shape, out: &mut Vec<(String, Rect)>) {
        match shape {
            Shape::Text(text) => {
                out.push((
                    text.galley.job.text.clone(),
                    text.galley.rect.translate(text.pos.to_vec2()),
                ));
            }
            Shape::Vec(shapes) => {
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

/// The bar rect and the absolute ink rect of the bar's inner text.
/// `make` builds the bar (a fresh `ProgressBar` per call — `ProgressBar` is not `Copy`).
fn painted_bar_text(rtl: bool, make: impl Fn() -> ProgressBar) -> (Rect, Rect) {
    let ctx = Context::default();
    if rtl {
        // Set before the first frame, so the flag reaches the first mirrored pass
        // (the progress bar's chrome follows the interface flag, like its fill).
        ctx.set_right_to_left(true);
    }
    let mut widget = Rect::NOTHING;
    let output = run(&ctx, input(SCREEN), |ui| {
        widget = ui.add(make()).rect;
    });
    let texts = painted_texts(&output);
    let text = texts.first().map(|(_, rect)| *rect);
    let text =
        text.unwrap_or_else(|| panic!("the progress bar's inner text should paint, got {texts:?}"));
    (widget, text)
}

/// The inner text hugs the reading-start edge (left under LTR, right under RTL),
/// inset by one `item_spacing.x`, and stays on the reading-start half of the bar —
/// for both the percentage text and a custom text.
#[test]
fn progress_bar_text_hugs_the_reading_start() {
    let spacing = egui::Style::default().spacing.item_spacing.x;
    for rtl in [false, true] {
        for make in [
            || ProgressBar::new(0.5).show_percentage(),
            || ProgressBar::new(0.5).text("Loading"),
        ] {
            let (bar_rect, text) = painted_bar_text(rtl, make);
            assert!(
                bar_rect.contains_rect(text),
                "the text should sit inside the bar (rtl = {rtl}: text {text:?}, bar {bar_rect:?})"
            );
            let inset = if rtl {
                bar_rect.right() - text.right()
            } else {
                text.left() - bar_rect.left()
            };
            assert!(
                (inset - spacing).abs() < 0.5,
                "the text should hug the reading-start edge (rtl = {rtl}: inset {inset}, \
                 spacing {spacing}, text {text:?}, bar {bar_rect:?})"
            );
            let on_start_half = if rtl {
                text.left() > bar_rect.center().x
            } else {
                text.right() < bar_rect.center().x
            };
            assert!(
                on_start_half,
                "the text should stay on the reading-start half (rtl = {rtl}: text {text:?}, \
                 bar {bar_rect:?})"
            );
        }
    }
}
