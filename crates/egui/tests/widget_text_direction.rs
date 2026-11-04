//! Regression tests for widget text following the interface's reading direction:
//! [`egui::WidgetText::into_galley_impl`], the [`egui::Label`] layout path and
//! the [`egui::TextEdit`] default layouter stamp the job's base direction from
//! [`egui::Style::right_to_left`], so a Latin-first label still reads as a
//! right-to-left row, while an explicit
//! [`egui::epaint::text::LayoutJob::direction`] still wins.
//!
//! Every stamped case also asserts its LTR counterpart, so this file doubles as
//! the LTR-unchanged regression. No snapshot assertions; painted text ink rects
//! and job metadata only.

use egui::epaint::Shape;
use egui::{
    Align, Context, FullOutput, Label, Layout, Pos2, RawInput, Rect, RichText, Ui, Vec2,
    WidgetText, vec2,
};
use egui::{epaint::text::TextDirection, text::LayoutJob};

const SCREEN: Vec2 = vec2(400.0, 300.0);

/// A mixed string that starts with a left-to-right character: under first-strong
/// inference it would lay out as an LTR paragraph, so the stamp is what makes it
/// read right-to-left in a right-to-left interface.
const MIXED: &str = "Settings: الإعدادات";

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

/// (text, absolute ink rect, job direction) for every text shape painted this pass.
///
/// `Galley::mesh_bounds` excludes leading/trailing whitespace, so the rect is the true
/// ink bounds of the glyphs, translated to their absolute position.
fn painted_texts(output: &FullOutput) -> Vec<(String, Rect, Option<TextDirection>)> {
    fn collect(shape: &Shape, out: &mut Vec<(String, Rect, Option<TextDirection>)>) {
        match shape {
            Shape::Text(text) => out.push((
                text.galley.job.text.clone(),
                text.galley.mesh_bounds.translate(text.pos.to_vec2()),
                text.galley.job.direction,
            )),
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

/// The painted entry of the first text equal to `text`, or `None`.
fn painted_text<'a>(
    texts: &'a [(String, Rect, Option<TextDirection>)],
    text: &str,
) -> Option<&'a (String, Rect, Option<TextDirection>)> {
    texts.iter().find(|(painted, _, _)| painted == text)
}

/// With the interface flag on, a label whose text starts with a left-to-right
/// character is still laid out right-to-left: the job carries an explicit
/// `RightToLeft` direction and the row anchors at the reading-start (right)
/// edge. With the flag off, the direction stays `None` (first-strong inference
/// remains epaint's fallback) and the row anchors at the left edge.
#[test]
fn label_follows_the_interface_direction() {
    for (rtl, want_direction) in [
        (false, Some(TextDirection::LeftToRight)),
        (true, Some(TextDirection::RightToLeft)),
    ] {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let output = run(&ctx, input(SCREEN), |ui| {
            ui.label(MIXED);
        });

        let texts = painted_texts(&output);
        let entry = painted_text(&texts, MIXED)
            .unwrap_or_else(|| panic!("the label should paint (rtl = {rtl}), got {texts:?}"));
        let (_, rect, direction) = entry;
        assert_eq!(
            *direction, want_direction,
            "the job direction follows the interface flag (rtl = {rtl})"
        );

        // The row anchors at the reading-start edge: right under the flag, left
        // without it (the root ui mirrors with the flag).
        if rtl {
            assert!(
                rect.right() > SCREEN.x * 0.75,
                "the label should anchor at the right edge under the flag (rtl = {rtl}): \
                 rect {rect:?}"
            );
        } else {
            assert!(
                rect.left() < SCREEN.x * 0.25,
                "the label should anchor at the left edge without the flag: rect {rect:?}"
            );
        }
    }
}

/// The same stamp on the other widget-text entry points: `Button` (via
/// `RichText`) follows the interface direction too.
#[test]
fn button_text_follows_the_interface_direction() {
    for (rtl, want_direction) in [
        (false, Some(TextDirection::LeftToRight)),
        (true, Some(TextDirection::RightToLeft)),
    ] {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let output = run(&ctx, input(SCREEN), |ui| {
            ui.add(egui::Button::new(RichText::new(MIXED)));
        });

        let texts = painted_texts(&output);
        let entry = painted_text(&texts, MIXED)
            .unwrap_or_else(|| panic!("the button label should paint (rtl = {rtl})"));
        assert_eq!(
            entry.2, want_direction,
            "the button text job direction follows the interface flag (rtl = {rtl})"
        );
    }
}

/// A `WidgetText::LayoutJob` that carries an explicit direction keeps it even in
/// a right-to-left interface: the stamp only fills jobs without one.
#[test]
fn explicit_job_direction_wins_over_the_interface() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let output = run(&ctx, input(SCREEN), |ui| {
        let mut job = LayoutJob::simple(
            MIXED.to_owned(),
            egui::TextStyle::Body.resolve(ui.style()),
            egui::Color32::PLACEHOLDER,
            200.0,
        );
        job.direction = Some(TextDirection::LeftToRight);
        ui.add(Label::new(WidgetText::LayoutJob(job.into())));
    });

    let texts = painted_texts(&output);
    let entry = painted_text(&texts, MIXED)
        .unwrap_or_else(|| panic!("the label should paint, got {texts:?}"));
    assert_eq!(
        entry.2,
        Some(TextDirection::LeftToRight),
        "an explicit job direction must win over the interface stamp"
    );
    // The row still lands at the reading-start (right) edge, because the RTL
    // interface mirrors the placement; only the paragraph direction inside the
    // galley stays left-to-right.
    assert!(
        entry.1.right() > SCREEN.x * 0.75,
        "the row anchors at the reading-start edge in an RTL interface: rect {:?}",
        entry.1
    );
}

/// The `into_galley_impl` `LayoutJob` arm keeps the stamp from clobbering an
/// explicit direction, even one that came in through the widget-text pipeline
/// (the `menu_item_rtl` pin flows through this path), while filling a missing
/// one from the interface. Raw `LayoutJob`s laid out by [`Label`] itself keep
/// first-strong inference by design, so this goes through [`egui::Button`],
/// which routes through `into_galley_impl`.
#[test]
fn layout_job_arm_fills_only_missing_directions() {
    for (explicit, want_direction) in [
        (None, Some(TextDirection::RightToLeft)),
        (
            Some(TextDirection::LeftToRight),
            Some(TextDirection::LeftToRight),
        ),
    ] {
        let ctx = Context::default();
        ctx.set_right_to_left(true);
        let output = run(&ctx, input(SCREEN), |ui| {
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                let mut job = LayoutJob::simple(
                    MIXED.to_owned(),
                    egui::TextStyle::Button.resolve(ui.style()),
                    egui::Color32::PLACEHOLDER,
                    200.0,
                );
                job.direction = explicit;
                ui.add(egui::Button::new(WidgetText::LayoutJob(job.into())));
            });
        });

        let texts = painted_texts(&output);
        let entry = painted_text(&texts, MIXED)
            .unwrap_or_else(|| panic!("the button label should paint, got {texts:?}"));
        assert_eq!(
            entry.2, want_direction,
            "the job direction (explicit = {explicit:?})"
        );
    }
}
