//! Idle-frame width stability for windows and plain `Resize` containers
//! holding wrapped right-to-left text: once the container settles, repeated
//! idle frames must hold its width and its reading-start edge (the right
//! edge under RTL, the left edge under LTR) — the measured content must not
//! creep wider frame by frame.
//!
//! Every case runs an LTR twin of the same body shape; the LTR twins
//! (left-to-right text) double as the LTR-unchanged regression. No snapshot
//! assertions; geometry only.

use core::cell::RefCell;

use egui::{CentralPanel, Context, Pos2, RawInput, Rect, TextEdit, Ui, Vec2, Window, pos2, vec2};

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

fn same_rect(a: Rect, b: Rect) -> bool {
    (a.left() - b.left()).abs() <= 0.5
        && (a.right() - b.right()).abs() <= 0.5
        && (a.top() - b.top()).abs() <= 0.5
        && (a.bottom() - b.bottom()).abs() <= 0.5
}

/// The painted `Resize` container rect: the stroke outline is the container
/// rect expanded by 2.0 per side (LTR and RTL alike), so shrink it back.
fn container_rect_from_shapes(output: &egui::FullOutput) -> Rect {
    use egui::epaint::RectShape;
    use egui::epaint::Shape;

    let mut container = Rect::NOTHING;
    for clipped in &output.shapes {
        if let Shape::Rect(RectShape {
            rect, fill, stroke, ..
        }) = &clipped.shape
            && *fill == egui::Color32::TRANSPARENT
            && !stroke.is_empty()
        {
            container = rect.shrink(2.0);
        }
    }
    container
}

/// A wrapped right-to-left paragraph: long enough to stay multi-row while a
/// container ratchets, with a space at every break candidate.
const WRAPPED_RTL_TEXT: &str = "هذا نص عربي طويل يحتوي على مسافات كثيرة حتى يلتف عبر عدة أسطر داخل النافذة الضيقة، ويكمل الالتفاف عند كل مسافة من مسافات النص الطويل هذا";

/// The left-to-right twin of [`WRAPPED_RTL_TEXT`]: the same body shape with
/// left-to-right text. Right-to-left text keeps its own base direction in
/// either interface (its rows anchor reading-forward by the first-strong
/// rule), so the direction twins differ in text direction, not body shape.
const WRAPPED_LTR_TEXT: &str = "This is a long english paragraph with many spaces so that it wraps across several rows inside the narrow window, and it keeps wrapping at every space of this long text";

const SCREEN: Vec2 = vec2(700.0, 500.0);

fn title_less_window(pos: Pos2, size: Vec2) -> Window<'static> {
    Window::new("idle width win")
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

/// Settle the container, then hold it across 30 idle frames: the width must
/// stay within 1 px of the settled width and the reading-start edge (right
/// under RTL, left under LTR) within 1 px of its settled position.
fn assert_idle_frames_hold_width(
    frame: impl Fn(f64) -> Rect,
    mut time: f64,
    rtl: bool,
    what: &str,
) {
    let mut prev = frame(time);
    for _ in 0..12 {
        time += 0.1;
        let next = frame(time);
        if same_rect(next, prev) {
            break;
        }
        prev = next;
    }
    let settled = prev;
    assert!(
        settled.is_positive(),
        "{what}: the container must be laid out (got {settled:?})"
    );

    let reading_start = |r: Rect| if rtl { r.right() } else { r.left() };
    for i in 0..30 {
        time += 0.1;
        let next = frame(time);
        assert!(
            (next.width() - settled.width()).abs() <= 1.0,
            "{what}: idle frame {i} must not change the width \
             (settled {settled:?}, got {next:?})"
        );
        assert!(
            (reading_start(next) - reading_start(settled)).abs() <= 1.0,
            "{what}: idle frame {i} must not move the reading-start edge \
             (settled {settled:?}, got {next:?})"
        );
    }
}

/// A title-less window whose body is one long wrapped label must hold its
/// width and its reading-start edge across idle frames, in both directions.
#[test]
fn window_idle_frames_hold_wrapped_width_stable() {
    // RTL: the reading-start (right) edge stays put and the width must not
    // ratchet one break-space per idle frame:
    let rtl = Context::default();
    rtl.set_right_to_left(true);
    let win = || title_less_window(pos2(650.0, 50.0), vec2(340.0, 420.0));
    let content = |ui: &mut Ui| {
        ui.label(WRAPPED_RTL_TEXT);
    };
    assert_idle_frames_hold_width(
        |t| window_frame_rect(&rtl, t, win, content),
        0.0,
        true,
        "RTL window",
    );

    // LTR twin: the same body shape with left-to-right text:
    let ltr = Context::default();
    let win = || title_less_window(pos2(50.0, 50.0), vec2(340.0, 420.0));
    let content = |ui: &mut Ui| {
        ui.label(WRAPPED_LTR_TEXT);
    };
    assert_idle_frames_hold_width(
        |t| window_frame_rect(&ltr, t, win, content),
        0.0,
        false,
        "LTR window",
    );
}

/// The same stability for a grid of wrapped right-to-left labels: the cells
/// size to their galleys, so a galley wider than the wrap budget would ratchet
/// the window just the same.
#[test]
fn window_idle_frames_hold_rtl_grid_width_stable() {
    let rtl = Context::default();
    rtl.set_right_to_left(true);
    let win = || title_less_window(pos2(650.0, 50.0), vec2(340.0, 420.0));
    let content = |ui: &mut Ui| {
        egui::Grid::new("wrapped grid").show(ui, |ui| {
            ui.label(WRAPPED_RTL_TEXT);
            ui.end_row();
            ui.label(WRAPPED_RTL_TEXT);
            ui.end_row();
        });
    };
    assert_idle_frames_hold_width(
        |t| window_frame_rect(&rtl, t, win, content),
        0.0,
        true,
        "RTL grid window",
    );
}

/// The same stability for a width-filling label body: the label fills the
/// offered width, so only an honest galley measurement keeps the window from
/// growing.
#[test]
fn window_idle_frames_hold_rtl_width_filling_label_width_stable() {
    let rtl = Context::default();
    rtl.set_right_to_left(true);
    let win = || title_less_window(pos2(650.0, 50.0), vec2(340.0, 420.0));
    let content = |ui: &mut Ui| {
        ui.add_sized(ui.available_size(), egui::Label::new(WRAPPED_RTL_TEXT));
    };
    assert_idle_frames_hold_width(
        |t| window_frame_rect(&rtl, t, win, content),
        0.0,
        true,
        "RTL width-filling label window",
    );
}

/// The window whose body combines a wrapped right-to-left label with a
/// width-filling multiline `TextEdit` (the shape that fills an area with a
/// widget): the label's wrapped rows must not ratchet the window across idle
/// frames.
#[test]
fn window_idle_frames_hold_rtl_width_stable_with_label_and_sized_text_edit() {
    let rtl = Context::default();
    rtl.set_right_to_left(true);
    let edited = RefCell::new(WRAPPED_RTL_TEXT.to_owned());
    let win = || title_less_window(pos2(650.0, 50.0), vec2(340.0, 420.0));
    let content = |ui: &mut Ui| {
        ui.label(WRAPPED_RTL_TEXT);
        let mut text = edited.borrow_mut();
        ui.add_sized(ui.available_size(), TextEdit::multiline(&mut *text));
    };
    assert_idle_frames_hold_width(
        |t| window_frame_rect(&rtl, t, win, content),
        0.0,
        true,
        "RTL label + TextEdit window",
    );
}

/// A width-filling multiline `TextEdit` alone keeps the window stable: its
/// rows keep the trailing whitespace inside their aligned span, so the
/// measured galley width is already bounded by the wrap budget. This pins
/// that shape against regressions in both directions.
#[test]
fn window_idle_frames_hold_rtl_width_stable_with_sized_text_edit_only() {
    let rtl = Context::default();
    rtl.set_right_to_left(true);
    let edited = RefCell::new(WRAPPED_RTL_TEXT.to_owned());
    let win = || title_less_window(pos2(650.0, 50.0), vec2(340.0, 420.0));
    let content = |ui: &mut Ui| {
        let mut text = edited.borrow_mut();
        ui.add_sized(ui.available_size(), TextEdit::multiline(&mut *text));
    };
    assert_idle_frames_hold_width(
        |t| window_frame_rect(&rtl, t, win, content),
        0.0,
        true,
        "RTL TextEdit-only window",
    );
}

/// A plain `Resize` container (no window) with a long wrapped right-to-left
/// label must hold its width and reading-start edge across idle frames, in
/// both directions: the grow-to-content loop is reachable without a window.
#[test]
fn resize_idle_frames_hold_wrapped_width_stable() {
    for (rtl, text, what) in [
        (true, WRAPPED_RTL_TEXT, "RTL resize"),
        (false, WRAPPED_LTR_TEXT, "LTR resize"),
    ] {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let frame = |t: f64| -> Rect {
            let output = run(&ctx, input_at(SCREEN, t), |ui| {
                CentralPanel::default().show(ui, |ui| {
                    egui::Resize::default()
                        .id(egui::Id::unique("idle width resize"))
                        .default_size(vec2(340.0, 300.0))
                        .show(ui, |ui| {
                            ui.label(text);
                        });
                });
            });
            container_rect_from_shapes(&output)
        };
        assert_idle_frames_hold_width(frame, 0.0, rtl, what);
    }
}
