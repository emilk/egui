//! Row-width honesty for wrapped right-to-left text: the logical wrap path
//! must budget every row for the blocks the previous row's overflow already
//! carried onto it. A row budgeted short by its carried word renders wider
//! than its wrap width, and a window holding such a label grows frame by
//! frame from its first open frames. Every window case runs a left-to-right
//! twin of the same body shape; the twin also pins left-to-right rendering
//! unchanged. No snapshot assertions; geometry only.

use egui::{Color32, Context, FontId, Pos2, RawInput, Rect, Ui, Vec2, Window, pos2, vec2};

fn input_at(size: Vec2, time: f64) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        time: Some(time),
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, input: RawInput, run_ui: impl FnMut(&mut Ui)) -> egui::FullOutput {
    let mut output = ctx.run_ui(input, run_ui);
    output.textures_delta.clear();
    output
}

const SCREEN: Vec2 = vec2(700.0, 500.0);

/// A wrapped right-to-left label: uniform three-letter words, long enough
/// to wrap into several rows at the wrap width.
fn wrapped_rtl_text() -> String {
    "אאא ".repeat(60)
}

/// The left-to-right twin of [`wrapped_rtl_text`]: the same body shape with
/// left-to-right words. Right-to-left text keeps its own base direction in
/// either interface, so the direction twins differ in text direction, not
/// body shape.
fn wrapped_ltr_text() -> String {
    "word ".repeat(60)
}

fn title_less_window(pos: Pos2, size: Vec2) -> Window<'static> {
    Window::new("wrap width win")
        .title_bar(false)
        .resizable(false)
        .default_size(size)
        .default_pos(pos)
}

/// One window frame's outer rect.
fn window_frame_rect(
    ctx: &Context,
    time: f64,
    win: impl Fn() -> Window<'static>,
    content: impl Fn(&mut Ui),
) -> Rect {
    let mut rect = Rect::NOTHING;
    run(ctx, input_at(SCREEN, time), |_| {
        if let Some(response) = win().show(ctx, |ui| content(ui)) {
            rect = response.response.rect;
        }
    });
    rect
}

/// A wrapped right-to-left galley keeps every row within its wrap width:
/// neither the widest row nor the galley measurement may exceed the wrap
/// width beyond the pixel-rounding slack.
#[test]
fn wrapped_rtl_galley_rows_stay_within_wrap_width() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    // One throwaway pass builds the fonts; `run` clears its texture deltas
    // (these tests never render).
    run(&ctx, input_at(SCREEN, 0.0), |_| {});

    let wrap_width = 340.0;
    let galley = ctx.fonts_mut(|fonts| {
        fonts.layout(
            wrapped_rtl_text(),
            FontId::proportional(14.0),
            Color32::WHITE,
            wrap_width,
        )
    });

    assert!(
        galley.size().x <= wrap_width + 1.0,
        "the galley must not measure wider than its wrap width \
         (size {:?}, wrap {wrap_width})",
        galley.size()
    );
    for row in &galley.rows {
        assert!(
            row.size.x <= wrap_width + 0.5,
            "every row must fit the wrap width (row {:?}, wrap {wrap_width})",
            row.size
        );
    }
}

/// From the first measured frame on, the window must not grow past its
/// opening-frame width and must not move its reading-start edge: a label
/// whose rows exceed the wrap width would grow the window frame by frame.
fn assert_open_frames_hold_width(
    frame: impl Fn(f64) -> Rect,
    mut time: f64,
    reading_start: fn(Rect) -> f32,
    what: &str,
) {
    let first = frame(time);
    assert!(
        first.is_positive(),
        "{what}: the window must be laid out (got {first:?})"
    );
    let opening_width = first.width();
    let opening_edge = reading_start(first);
    for i in 0..30 {
        time += 0.1;
        let next = frame(time);
        assert!(
            next.width() <= opening_width + 0.5,
            "{what}: open frame {i} must not grow the window past its opening \
             width (opening {opening_width}, got {next:?})"
        );
        assert!(
            (reading_start(next) - opening_edge).abs() <= 0.5,
            "{what}: open frame {i} must not move the reading-start edge \
             (opening edge {opening_edge}, got {next:?})"
        );
    }
}

/// A title-less non-resizable window whose body is one long wrapped
/// right-to-left label must hold its opening width and its reading-start
/// (right) edge across its open frames.
#[test]
fn window_open_frames_hold_wrapped_rtl_width() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    // One throwaway pass builds the fonts; it does not open the window.
    run(&ctx, input_at(SCREEN, 0.0), |_| {});

    let win = || title_less_window(pos2(650.0, 50.0), vec2(340.0, 420.0));
    let content = |ui: &mut Ui| {
        ui.label(wrapped_rtl_text());
    };
    assert_open_frames_hold_width(
        |t| window_frame_rect(&ctx, t, win, content),
        0.0,
        |rect| rect.right(),
        "RTL window",
    );
}

/// The left-to-right twin: the same body shape with left-to-right words
/// holds its opening width and its reading-start (left) edge.
#[test]
fn window_open_frames_hold_wrapped_ltr_width() {
    let ctx = Context::default();
    // One throwaway pass builds the fonts; it does not open the window.
    run(&ctx, input_at(SCREEN, 0.0), |_| {});

    let win = || title_less_window(pos2(50.0, 50.0), vec2(340.0, 420.0));
    let content = |ui: &mut Ui| {
        ui.label(wrapped_ltr_text());
    };
    assert_open_frames_hold_width(
        |t| window_frame_rect(&ctx, t, win, content),
        0.0,
        |rect| rect.left(),
        "LTR window",
    );
}
