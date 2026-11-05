//! Window title-bar regression tests for the whole-UI direction flag
//! ([`egui::Style::right_to_left`]): the mirrored title-bar atom order (close
//! button at the reading end's mirror, collapse button on the opposite end, the
//! title centred between the two buttons) and the mirrored title-bar drag-band
//! carve, driven through the headless [`egui_kittest::Harness`] with
//! accessibility queries, captured window rects and raw paint shapes only (no
//! snapshot assertions; renderers are platform-dependent).
//!
//! Every mirrored case asserts its LTR counterpart in the same test, so this file
//! doubles as the LTR-unchanged regression.

use egui::{Pos2, Rect, Vec2, pos2, vec2};
use egui_kittest::Harness;
use kittest::Queryable as _;

fn rtl_harness(rtl: bool) -> egui_kittest::HarnessBuilder<Captured> {
    Harness::builder()
        .with_size(vec2(400.0, 300.0))
        // Short frames, so that a press-hold-release stays a click and a press
        // followed by a big move stays a drag:
        .with_step_dt(1.0 / 60.0)
        .with_right_to_left(rtl)
}

/// Run two more frames: the window follows its content with a frame delay, so the
/// accessibility tree needs one extra frame past a structural change (a drag move,
/// a close) to be fully settled.
fn settle(harness: &mut Harness<'_, impl Sized>) {
    harness.run();
    harness.run();
}

/// Press at `from`, move by `delta` in one decisive step, release, then settle.
/// The frames while the button is down are plain steps: a held press keeps the
/// window repainting, which a settling [`Harness::run`] would spin on. The move
/// applies in `Window::show_dyn` the frame after the press, while the title-bar
/// response of the previous frame is `dragged()`.
fn drag(harness: &mut Harness<'_, impl Sized>, from: Pos2, delta: Vec2) {
    harness.drag_at(from);
    harness.run_steps(2);
    harness.hover_at(from + delta);
    harness.run_steps(2);
    harness.hover_at(from + delta);
    harness.run_steps(2);
    harness.drop_at(from + delta);
    settle(harness);
}

/// The window's outer rect, captured app-side from what `Window::show` returned.
/// (The `Role::Window` accessibility node keeps the pre-drag bounds, so it cannot
/// observe a title-bar move.)
#[derive(Default)]
struct Captured {
    open: bool,
    window: Option<Rect>,
}

fn capture(state: &mut Captured, shown: Option<egui::InnerResponse<Option<()>>>) {
    if let Some(response) = shown {
        state.window = Some(response.response.rect);
    }
}

fn window_app(ui: &mut egui::Ui, state: &mut Captured) {
    let shown = egui::Window::new("Test window")
        .open(&mut state.open)
        .show(ui.ctx(), |ui| {
            ui.label("Body");
        });
    capture(state, shown);
}

/// A fixed-position, fixed-size, non-resizable window: the title-bar buttons hug
/// the window edges and the dragged rect stays far inside the harness, so a
/// title-band drag moves it by exactly the drag delta (up to whole-pixel
/// position rounding).
fn sized_window_app(ui: &mut egui::Ui, state: &mut Captured) {
    let shown = egui::Window::new("Test window")
        .open(&mut state.open)
        .current_pos(pos2(80.0, 80.0))
        .default_size(vec2(200.0, 150.0))
        .resizable(false)
        .show(ui.ctx(), |ui| {
            ui.label("Body");
        });
    capture(state, shown);
}

/// B1: the close button sits at the reading end's mirror — hugging the window's
/// left edge under RTL (the Arabic-Windows convention), its right edge under LTR —
/// and clicking it still closes the window.
#[test]
fn close_button_sits_at_the_reading_end_mirror() {
    for rtl in [false, true] {
        let mut harness = rtl_harness(rtl).build_ui_state(
            sized_window_app,
            Captured {
                open: true,
                window: None,
            },
        );
        settle(&mut harness);

        let close = harness.get_by_label("Close window").rect();
        let window = harness.state().window;
        if rtl {
            assert!(
                matches!(window, Some(window) if close.left() - window.left() < 40.0),
                "rtl: the × should hug the window's left edge: {close:?} in {window:?}"
            );
        } else {
            assert!(
                matches!(window, Some(window) if window.right() - close.right() < 40.0),
                "ltr: the × should hug the window's right edge: {close:?} in {window:?}"
            );
        }

        // Clicking the close button still closes the window:
        harness.get_by_label("Close window").click();
        settle(&mut harness);
        assert!(
            harness.query_by_label("Close window").is_none(),
            "rtl = {rtl}: the window should be closed"
        );
    }
}

/// B2: the collapse triangle sits on the opposite end from the close button in
/// both directions (the mirrored atom list), and each button hugs its own window
/// edge.
#[test]
fn collapse_button_sits_on_the_opposite_end_from_close() {
    for rtl in [false, true] {
        let mut harness = rtl_harness(rtl).build_ui_state(
            sized_window_app,
            Captured {
                open: true,
                window: None,
            },
        );
        settle(&mut harness);

        let close = harness.get_by_label("Close window").rect();
        let collapse = harness.get_by_label("Hide").rect();
        let window = harness.state().window;

        assert_eq!(
            close.center().x < collapse.center().x,
            rtl,
            "rtl = {rtl}: the close and collapse buttons should swap ends: \
             close {close:?} vs collapse {collapse:?}"
        );
        if rtl {
            assert!(
                matches!(window, Some(window) if close.left() - window.left() < 40.0),
                "rtl: the × should hug the window's left edge: {close:?} in {window:?}"
            );
            assert!(
                matches!(window, Some(window) if window.right() - collapse.right() < 40.0),
                "rtl: the collapse triangle should hug the window's right edge: \
                 {collapse:?} in {window:?}"
            );
        } else {
            assert!(
                matches!(window, Some(window) if collapse.left() - window.left() < 40.0),
                "ltr: the collapse triangle should hug the window's left edge: \
                 {collapse:?} in {window:?}"
            );
            assert!(
                matches!(window, Some(window) if window.right() - close.right() < 40.0),
                "ltr: the × should hug the window's right edge: {close:?} in {window:?}"
            );
        }
    }
}

/// B3a: the mirrored carves keep both title-bar buttons out of the drag band: a
/// drag that starts inside the ×, or inside the collapse triangle (which the
/// carve cuts out with its own mirrored branch), moves the window not at all, in
/// either direction, and leaves it open.
#[test]
fn drag_on_the_close_button_does_not_move_the_window() {
    for rtl in [false, true] {
        let mut harness = rtl_harness(rtl).build_ui_state(
            sized_window_app,
            Captured {
                open: true,
                window: None,
            },
        );
        settle(&mut harness);

        let before = harness.state().window;
        let close = harness.get_by_label("Close window").rect();

        drag(&mut harness, close.center(), vec2(40.0, 20.0));

        let after = harness.state().window;
        assert!(
            matches!(
                (&before, &after),
                (Some(before), Some(after))
                    if (after.min.x - before.min.x).abs() < 1.0
                        && (after.min.y - before.min.y).abs() < 1.0
            ),
            "rtl = {rtl}: a drag starting on the close button must not move the window: \
             {before:?} -> {after:?}"
        );
        assert!(
            harness.query_by_label("Body").is_some(),
            "rtl = {rtl}: the window should still be open"
        );

        // The mirrored collapse-button carve gets the same contract: a drag that
        // starts inside the collapse triangle must not move the window either.
        // (A carve that cuts at the wrong edge leaves the button inside the drag
        // band; the band-midpoint drags of B3b would not notice that.)
        let collapse = harness.get_by_label("Hide").rect();
        let before = harness.state().window;

        drag(&mut harness, collapse.center(), vec2(40.0, 20.0));

        let after = harness.state().window;
        assert!(
            matches!(
                (&before, &after),
                (Some(before), Some(after))
                    if (after.min.x - before.min.x).abs() < 1.0
                        && (after.min.y - before.min.y).abs() < 1.0
            ),
            "rtl = {rtl}: a drag starting on the collapse button must not move the \
             window: {before:?} -> {after:?}"
        );
        assert!(
            harness.query_by_label("Body").is_some(),
            "rtl = {rtl}: the window should still be open"
        );
    }
}

/// B3b: the band between the two buttons stays the drag-to-move band in both
/// directions: a drag that starts on it moves the window by the drag delta.
#[test]
fn drag_on_the_title_band_moves_the_window() {
    for rtl in [false, true] {
        let mut harness = rtl_harness(rtl).build_ui_state(
            sized_window_app,
            Captured {
                open: true,
                window: None,
            },
        );
        settle(&mut harness);

        let close = harness.get_by_label("Close window").rect();
        let collapse = harness.get_by_label("Hide").rect();
        let before = harness.state().window;

        // A point on the carved band, strictly between the two buttons:
        let band = Rect::from_two_pos(close.center(), collapse.center()).center();

        drag(&mut harness, band, vec2(40.0, 20.0));

        let after = harness.state().window;
        assert!(
            matches!(
                (&before, &after),
                (Some(before), Some(after))
                    if (after.min.x - before.min.x - 40.0).abs() < 2.0
                        && (after.min.y - before.min.y - 20.0).abs() < 2.0
            ),
            "rtl = {rtl}: a drag starting on the title band must move the window by \
             the drag delta (40, 20): {before:?} -> {after:?}"
        );
    }
}

/// B4: the two `Atom::grow()` spacers keep the title galley centred between the
/// close and collapse buttons through the mirrored rebuild. The title text has no
/// accessibility node, so the observable is its paint shape.
#[test]
fn title_is_horizontally_centred_between_the_buttons() {
    for rtl in [false, true] {
        let mut harness = rtl_harness(rtl).build_ui_state(
            window_app,
            Captured {
                open: true,
                window: None,
            },
        );
        settle(&mut harness);

        let close = harness.get_by_label("Close window").rect();
        let collapse = harness.get_by_label("Hide").rect();

        let title_center_x =
            harness
                .output()
                .shapes
                .iter()
                .find_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) if text.galley.text() == "Test window" => {
                        // Measure the painted ink, not the galley anchor: under
                        // the widget-text stamp a Latin title in a right-to-left
                        // interface is laid out as a right-to-left paragraph, so
                        // `Shape::Text::pos` is its reading-start (right) anchor
                        // and the ink extends leftward from it; `pos + size / 2`
                        // would over-measure by a full title width. The tight
                        // ink bounds translate to the true painted centre in
                        // either anchoring (same observable as
                        // `egui/tests/widget_text_direction.rs`).
                        Some(
                            text.galley
                                .mesh_bounds
                                .translate(text.pos.to_vec2())
                                .center()
                                .x,
                        )
                    }
                    _ => None,
                });

        let (lo, hi) = if rtl {
            (close.right(), collapse.left())
        } else {
            (collapse.right(), close.left())
        };
        assert!(
            matches!(title_center_x, Some(center) if lo < center && center < hi),
            "rtl = {rtl}: the title galley should be painted strictly between the two \
             buttons (centre {title_center_x:?}, between {lo} and {hi})"
        );
        // Equal grows split the free space evenly, so the title centre must sit at the
        // band midpoint: dropping or zeroing one `Atom::grow()` de-centres it by tens of
        // pixels while it stays strictly between the buttons. A small tolerance absorbs
        // paint and rounding.
        assert!(
            matches!(title_center_x, Some(center) if ((lo + hi) * 0.5 - center).abs() < 2.0),
            "rtl = {rtl}: the title galley should be centred between the two buttons \
             (centre {title_center_x:?}, band midpoint {} between {lo} and {hi})",
            (lo + hi) * 0.5
        );
    }
}
