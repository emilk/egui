//! Window title-bar atom internals under the whole-UI direction flag
//! ([`egui::Style::right_to_left`]): a title built from several atoms (an icon
//! and a text) reads right-to-left under RTL, a plain-text title keeps its
//! exact geometry in both directions, and the title-bar hit-test carves
//! (collapse button at the reading-start end, close button at the far end)
//! keep working with a multi-atom title.
//!
//! Every RTL case has an LTR twin asserting the unchanged behavior. No
//! snapshot assertions; geometry only.

use egui::epaint::PathShape;
use egui::{
    Atom, Color32, Context, Event, FullOutput, Id, Modifiers, PointerButton, Pos2, RawInput, Rect,
    Ui, Window, epaint::Shape, pos2, vec2,
};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))
}

const ICON_COLOR: Color32 = Color32::from_rgb(255, 0, 0);

fn input(time: f64, events: Vec<Event>) -> RawInput {
    RawInput {
        screen_rect: Some(screen()),
        time: Some(time),
        events,
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, time: f64, events: Vec<Event>, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(input(time, events), run_ui);
    output.textures_delta.clear();
    output
}

fn pointer_move(pos: Pos2) -> Event {
    Event::PointerMoved(pos)
}

fn pointer_button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn flat_shapes(output: &FullOutput) -> Vec<&Shape> {
    fn scan<'a>(shape: &'a Shape, out: &mut Vec<&'a Shape>) {
        match shape {
            Shape::Vec(shapes) => {
                for shape in shapes {
                    scan(shape, out);
                }
            }
            _ => out.push(shape),
        }
    }
    let mut shapes = Vec::new();
    for clipped in &output.shapes {
        scan(&clipped.shape, &mut shapes);
    }
    shapes
}

/// The painted rect of the title icon: the only rect filled with
/// [`ICON_COLOR`].
fn icon_rect(output: &FullOutput) -> Option<Rect> {
    flat_shapes(output)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Rect(rect) if rect.fill == ICON_COLOR => Some(rect.rect),
            _ => None,
        })
}

/// The painted rect of a text galley whose text contains `needle`.
fn text_rect(output: &FullOutput, needle: &str) -> Option<Rect> {
    flat_shapes(output)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Text(text) if text.galley.text().contains(needle) => {
                Some(text.galley.rect.translate(text.pos.to_vec2()))
            }
            _ => None,
        })
}

/// The painted rect of the collapse triangle: a filled convex polygon with
/// three points.
fn collapse_triangle_rect(output: &FullOutput) -> Option<Rect> {
    flat_shapes(output)
        .into_iter()
        .find_map(|shape| match shape {
            Shape::Path(PathShape { points, fill, .. })
                if points.len() == 3 && *fill != Color32::TRANSPARENT =>
            {
                Some(Rect::from_points(points))
            }
            _ => None,
        })
}

/// The painted rect of the close button: the bounding box of the two crossed
/// diagonal line segments in the title bar (resize grips paint diagonals
/// further down).
fn close_button_rect(output: &FullOutput, title_bar_bottom: f32) -> Option<Rect> {
    let mut rect = Rect::NOTHING;
    for shape in flat_shapes(output) {
        if let Shape::LineSegment { points, .. } = shape {
            let [a, b] = points;
            let diagonal = (a.x - b.x).abs() > 0.5 && (a.y - b.y).abs() > 0.5;
            let in_title_bar = a.y.max(b.y) < title_bar_bottom;
            if diagonal && in_title_bar {
                rect = rect.union(Rect::from_points(points));
            }
        }
    }
    rect.is_positive().then_some(rect)
}

/// The painted window frame rect: the widest rect in the output, if anything
/// was painted.
fn window_rect(output: &FullOutput) -> Option<Rect> {
    flat_shapes(output)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Rect(rect) => Some(rect.rect),
            _ => None,
        })
        .max_by_key(|rect| rect.width() as i64)
}

fn same_rect(a: Rect, b: Rect) -> bool {
    (a.left() - b.left()).abs() <= 0.5
        && (a.right() - b.right()).abs() <= 0.5
        && (a.top() - b.top()).abs() <= 0.5
        && (a.bottom() - b.bottom()).abs() <= 0.5
}

/// The settled rects of the title icon and title text for a window titled
/// `(icon, "Title")`.
fn multi_atom_title_rects(rtl: bool) -> (Rect, Rect) {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    let id = Id::unique("window-title-rtl-multi-atom");
    let icon_atom = Atom::paint(vec2(16.0, 16.0), |ui, args| {
        ui.painter().rect_filled(args.rect, 0.0, ICON_COLOR);
    });
    let mut rects = (Rect::NOTHING, Rect::NOTHING);
    for frame in 0..3 {
        let output = run(&ctx, frame as f64, vec![], |ui| {
            Window::new((icon_atom.clone(), "Title"))
                .id(id)
                .fixed_pos(pos2(300.0, 60.0))
                .show(ui, |ui| {
                    ui.set_min_size(vec2(80.0, 30.0));
                });
        });
        if frame == 2 {
            rects = (
                icon_rect(&output).expect("icon should be painted"),
                text_rect(&output, "Title").expect("title text should be painted"),
            );
        }
    }
    rects
}

/// A multi-atom title (icon, text) reads right-to-left under RTL: the icon
/// sits at the reading start, to the right of the text.
#[test]
fn multi_atom_title_reads_right_to_left_under_rtl() {
    let (icon, text) = multi_atom_title_rects(true);
    assert!(
        icon.left() > text.right(),
        "the icon should sit to the right of the title text: icon {icon:?} text {text:?}"
    );
}

/// Twin: under LTR the icon sits to the left of the title text, byte-identical
/// to the pre-mirror behavior.
#[test]
fn multi_atom_title_reads_left_to_right_under_ltr() {
    let (icon, text) = multi_atom_title_rects(false);
    assert!(
        icon.right() < text.left(),
        "the icon should sit to the left of the title text: icon {icon:?} text {text:?}"
    );
}

/// An auto-sized window with a multi-atom title settles at a stable geometry:
/// its rect is identical across idle frames.
#[test]
fn auto_sized_window_with_multi_atom_title_settles() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let id = Id::unique("window-title-rtl-auto-sized");
    let icon_atom = Atom::paint(vec2(16.0, 16.0), |ui, args| {
        ui.painter().rect_filled(args.rect, 0.0, ICON_COLOR);
    });
    let mut rects = Vec::new();
    let mut icon_present = true;
    for frame in 0..4 {
        let output = run(&ctx, frame as f64, vec![], |ui| {
            Window::new((icon_atom.clone(), "Title"))
                .id(id)
                .auto_sized()
                .fixed_pos(pos2(300.0, 60.0))
                .show(ui, |ui| {
                    ui.set_min_size(vec2(80.0, 30.0));
                });
        });
        if frame > 0 {
            rects.push(window_rect(&output).expect("window should be painted"));
            icon_present &= icon_rect(&output).is_some();
        }
    }
    assert!(
        same_rect(rects[1], rects[2]),
        "the window rect should be stable across idle frames: {rects:?}"
    );
    assert!(
        icon_present,
        "the title icon should stay painted on every frame"
    );
}

/// A plain-text title keeps its exact painted geometry in both directions
/// (the byte-identity guard for the title-atom internals).
#[test]
fn single_text_title_geometry_is_unchanged() {
    let rects = |rtl: bool| {
        let ctx = Context::default();
        ctx.set_right_to_left(rtl);
        let id = Id::unique("window-title-rtl-single-text");
        let mut rect = Rect::NOTHING;
        for frame in 0..3 {
            let output = run(&ctx, frame as f64, vec![], |ui| {
                Window::new("Title")
                    .id(id)
                    .fixed_pos(pos2(300.0, 60.0))
                    .show(ui, |ui| {
                        ui.set_min_size(vec2(80.0, 30.0));
                    });
            });
            if frame == 2 {
                rect = text_rect(&output, "Title").expect("title text should be painted");
            }
        }
        rect
    };
    let (rtl, ltr) = (rects(true), rects(false));
    for (edge, expected) in [
        (rtl.left(), 122.7),
        (rtl.right(), 187.7),
        (rtl.top(), 67.0),
        (rtl.bottom(), 89.0),
        (ltr.left(), 452.3),
        (ltr.right(), 517.3),
        (ltr.top(), 67.0),
        (ltr.bottom(), 89.0),
    ] {
        assert!(
            (edge - expected).abs() <= 0.5,
            "single-text title geometry drifted (pinned pre-mirror): rtl {rtl:?} ltr {ltr:?}"
        );
    }
}

/// Under RTL the title-bar hit-test carves survive a multi-atom title: the
/// collapse button at the reading-start (right) end toggles the window and the
/// close button at the far (left) end closes it.
#[test]
fn title_hit_test_carves_survive_the_reversal_under_rtl() {
    carves(true);
}

/// Twin: under LTR the collapse button is at the left end and the close button
/// at the right end.
#[test]
fn title_hit_test_carves_survive_the_reversal_under_ltr() {
    carves(false);
}

fn carves(rtl: bool) {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    let id = Id::unique("window-title-rtl-carves");
    let icon_atom = Atom::paint(vec2(16.0, 16.0), |ui, args| {
        ui.painter().rect_filled(args.rect, 0.0, ICON_COLOR);
    });
    let mut open = true;

    let frame = |time: f64, events: Vec<Event>, open: &mut bool| {
        let mut window = Rect::NOTHING;
        let output = run(&ctx, time, events, |ui| {
            let response = Window::new((icon_atom.clone(), "Title"))
                .id(id)
                .fixed_pos(pos2(300.0, 60.0))
                .open(open)
                .show(ui, |ui| {
                    ui.set_min_size(vec2(80.0, 30.0));
                });
            if let Some(response) = response {
                window = response.response.rect;
            }
        });
        (
            window,
            collapse_triangle_rect(&output),
            close_button_rect(&output, window.top() + 40.0),
        )
    };

    let mut time = 0.0;
    // The first frame paints nothing (the layer is not yet visible); settle
    // with idle frames before capturing the geometry.
    let (mut window, mut triangle, mut close) = (Rect::NOTHING, None, None);
    for _ in 0..3 {
        (window, triangle, close) = frame(time, vec![], &mut open);
        time += 0.1;
    }
    assert!(open, "the window should start open");
    assert!(
        triangle.is_some() && close.is_some(),
        "both buttons should be painted"
    );

    let (mut triangle, mut close) = (
        triangle.expect("collapse triangle"),
        close.expect("close button"),
    );
    // The collapse button sits at the reading-start end, the close button at
    // the far end.
    if rtl {
        assert!(
            triangle.left() > close.right(),
            "rtl: {triangle:?} {close:?}"
        );
    } else {
        assert!(
            triangle.right() < close.left(),
            "ltr: {triangle:?} {close:?}"
        );
    }

    // Click the collapse button: the body collapses once the toggle animation
    // has settled.
    let pos = triangle.center();
    frame(time, vec![pointer_move(pos)], &mut open);
    time += 0.1;
    frame(time, vec![pointer_button(pos, true)], &mut open);
    time += 0.1;
    frame(time, vec![pointer_button(pos, false)], &mut open);
    for _ in 0..3 {
        time += 0.3;
        let (collapsed, fresh_triangle, _) = frame(time, vec![], &mut open);
        window = collapsed;
        triangle = fresh_triangle.expect("the collapse button should stay painted");
    }
    assert!(
        window.height() < 40.0,
        "the window should collapse to title-bar-only height: {window:?}"
    );

    // Click it again: the body expands back to the original geometry.
    let pos = triangle.center();
    frame(time, vec![pointer_move(pos)], &mut open);
    time += 0.1;
    frame(time, vec![pointer_button(pos, true)], &mut open);
    time += 0.1;
    frame(time, vec![pointer_button(pos, false)], &mut open);
    for _ in 0..3 {
        time += 0.3;
        let (reexpanded, _, fresh_close) = frame(time, vec![], &mut open);
        window = reexpanded;
        close = fresh_close.expect("the close button should stay painted");
    }
    assert!(
        window.height() > 60.0,
        "the window should expand back: {window:?}"
    );

    // Click the close button: the window closes.
    let pos = close.center();
    frame(time, vec![pointer_move(pos)], &mut open);
    time += 0.1;
    frame(time, vec![pointer_button(pos, true)], &mut open);
    time += 0.1;
    frame(time, vec![pointer_button(pos, false)], &mut open);
    assert!(!open, "the close button should close the window");
}
