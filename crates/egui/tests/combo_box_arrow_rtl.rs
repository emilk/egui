//! The [`egui::ComboBox`] arrow sits at the widget's far end along the reading
//! direction: under the whole-UI direction flag ([`egui::Style::right_to_left`])
//! it hangs at the reading-end (left) edge while the selected text hugs the
//! reading-start (right) edge; under LTR it stays at the right edge next to the
//! text. The row itself already mirrors — only the arrow's alignment inside its
//! grown cell needs to be direction-aware.
//!
//! No snapshot assertions; geometry only. Every RTL case carries its LTR twin.

use core::cell::Cell;

use egui::{ComboBox, Context, Pos2, RawInput, Rect, Ui, Vec2, vec2};

const SCREEN: Vec2 = vec2(400.0, 300.0);

/// Slack for the button frame margin and glyph bearings around an "hugs this
/// edge" assertion on a 200 px wide combo box with a short label.
const EDGE_TOL: f32 = 12.0;

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, run_ui: impl FnMut(&mut Ui)) -> egui::FullOutput {
    let mut output = ctx.run_ui(input(SCREEN), run_ui);
    output.textures_delta.clear();
    output
}

/// (painted text, absolute ink rect) for every text shape painted this pass.
fn painted_texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<(String, Rect)> {
    fn collect(shape: &egui::epaint::Shape, out: &mut Vec<(String, Rect)>) {
        match shape {
            egui::epaint::Shape::Text(text) => {
                out.push((
                    text.galley.job.text.clone(),
                    text.galley.mesh_bounds.translate(text.pos.to_vec2()),
                ));
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
    for clipped in shapes {
        collect(&clipped.shape, &mut out);
    }
    out
}

/// Show a 200 px wide combo box with a short selected text and return
/// (button rect, arrow paint rect, selected-text ink rect).
fn combo_geometry(rtl: bool) -> (Rect, Rect, Rect) {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);

    let arrow = std::rc::Rc::new(core::cell::RefCell::new(None::<Rect>));
    let button = Cell::new(None::<Rect>);

    let output = run(&ctx, |ui| {
        let response = ComboBox::from_id_salt("combo")
            .width(200.0)
            .selected_text("Sel")
            .icon({
                let arrow = std::rc::Rc::clone(&arrow);
                move |_, rect, _, _| *arrow.borrow_mut() = Some(rect)
            })
            .show_ui(ui, |_| {})
            .response;
        button.set(Some(response.rect));
    });

    let arrow_rect = arrow.borrow().expect("the arrow icon should paint");
    let button_rect = button.get().expect("the combo box button should show");
    let text_rect = *painted_texts(&output.shapes)
        .iter()
        .find(|(painted, _)| painted == "Sel")
        .map(|(_, rect)| rect)
        .expect("the selected text should paint");
    (button_rect, arrow_rect, text_rect)
}

/// Under the interface flag the selected text hugs the reading-start (right)
/// edge and the arrow hangs at the reading-end (left) edge, far from the text.
#[test]
fn arrow_sits_at_the_reading_end_under_rtl() {
    let (button, arrow, text) = combo_geometry(true);

    assert!(
        text.right() >= button.right() - EDGE_TOL,
        "the selected text should hug the reading-start (right) edge of the button \
         (button {button:?}, text {text:?})"
    );
    assert!(
        arrow.left() <= button.left() + EDGE_TOL,
        "the arrow should hang at the reading-end (left) edge of the button, \
         not next to the text (button {button:?}, arrow {arrow:?}, text {text:?})"
    );
    assert!(
        arrow.right() < text.left() - 2.0,
        "the arrow should be separated from the selected text by more than the icon gap \
         (button {button:?}, arrow {arrow:?}, text {text:?})"
    );
}

/// LTR twin: the text stays at the left and the arrow at the right edge.
#[test]
fn arrow_stays_at_the_right_edge_under_ltr() {
    let (button, arrow, text) = combo_geometry(false);

    assert!(
        text.left() <= button.left() + EDGE_TOL,
        "the selected text should hug the left edge of the button \
         (button {button:?}, text {text:?})"
    );
    assert!(
        arrow.right() >= button.right() - EDGE_TOL,
        "the arrow should hang at the right edge of the button \
         (button {button:?}, arrow {arrow:?}, text {text:?})"
    );
}
