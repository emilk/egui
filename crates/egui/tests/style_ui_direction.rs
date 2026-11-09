//! Regression test for the style panel's reset button: resetting the style must
//! keep the interface direction (`Style::right_to_left`) that was toggled in the
//! same panel, while still resetting everything else to `Style::default()`.
//!
//! Raw pointer events drive the button; no snapshot assertions.

use egui::epaint::Shape;
use egui::{
    Context, Event, FullOutput, Modifiers, PointerButton, Pos2, RawInput, Rect, Style, Vec2, vec2,
};

const SCREEN: Vec2 = vec2(600.0, 900.0);

fn input_at(size: Vec2, time: f64, events: Vec<Event>) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        time: Some(time),
        events,
        ..Default::default()
    }
}

/// Run one ui pass painting `style.ui`, and discard the texture deltas (this
/// test never renders).
fn run(ctx: &Context, input: RawInput, style: &mut Style) -> FullOutput {
    let mut output = ctx.run_ui(input, |ui| {
        style.ui(ui);
    });
    output.textures_delta.clear();
    output
}

/// (text, absolute ink rect) for every text shape painted this pass.
fn painted_texts(output: &FullOutput) -> Vec<(String, Rect)> {
    fn collect(shape: &Shape, out: &mut Vec<(String, Rect)>) {
        match shape {
            Shape::Text(text) => {
                out.push((
                    text.galley.job.text.clone(),
                    text.galley.mesh_bounds.translate(text.pos.to_vec2()),
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

/// Click the "Reset style" button (the only one in the panel): hover, press and
/// release it, and return the style after the release frame.
fn click_reset_style(ctx: &Context, style: &mut Style) {
    // A settle frame paints the panel; the label is unique in it:
    let output = run(ctx, input_at(SCREEN, 0.0, vec![]), style);
    let reset_rect = *painted_texts(&output)
        .iter()
        .find(|(text, _)| text == "Reset style")
        .map(|(_, rect)| rect)
        .expect("the 'Reset style' button label should paint");

    let center = reset_rect.center();
    let press = Event::PointerButton {
        pos: center,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    };
    let release = Event::PointerButton {
        pos: center,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    };
    run(
        ctx,
        input_at(SCREEN, 0.1, vec![Event::PointerMoved(center), press]),
        style,
    );
    run(
        ctx,
        input_at(SCREEN, 0.2, vec![Event::PointerMoved(center), release]),
        style,
    );
}

/// Resetting the style panel keeps the interface direction: a panel with
/// `right_to_left` set keeps it after clicking "Reset style".
#[test]
fn reset_style_keeps_the_interface_direction() {
    let ctx = Context::default();
    let mut style = Style {
        right_to_left: true,
        ..Default::default()
    };
    click_reset_style(&ctx, &mut style);
    assert!(
        style.right_to_left,
        "the 'Reset style' button must not clear the interface direction"
    );
}

/// The reset still resets everything else, in both directions: an LTR panel
/// with a modified spacing loses the modification, and the direction stays off.
#[test]
fn reset_style_still_resets_everything_else() {
    let ctx = Context::default();
    let mut style = Style::default();
    style.spacing.menu_spacing = 99.0;
    click_reset_style(&ctx, &mut style);
    assert_eq!(
        style.spacing.menu_spacing,
        Style::default().spacing.menu_spacing,
        "the reset must restore the default spacing"
    );
    assert!(
        !style.right_to_left,
        "an LTR panel stays LTR across the reset"
    );
}
