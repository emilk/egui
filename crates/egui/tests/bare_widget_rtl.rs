//! Bare widgets added directly to a vertical body follow the body's effective
//! cross align: under a right-to-left interface a bare `ui.label` or
//! `ui.button` in a window body hugs the reading-start (right) edge — the same
//! term that mirrors the body's nested rows — so mixed bodies hug
//! consistently, title-less windows with bare content hug the content width,
//! and a plain [`egui::Resize`] with bare content shrinks and hugs under RTL.
//! Explicit `Align::Max` / `Center` bodies are not double mirrored, the
//! `right_to_left = false` scope escape hatch stays physical-left, and an
//! explicit label `halign` override wins over the mirrored default.
//!
//! Every RTL case carries an LTR twin asserting the unchanged behavior. No
//! snapshot assertions; geometry only.

use core::cell::Cell;
use core::num::NonZeroUsize;

use egui::{
    Align, CentralPanel, Context, Event, FullOutput, Id, Label, Layout, Modifiers, PointerButton,
    Pos2, RawInput, Rect, Ui, UiBuilder, Vec2, Window, pos2, vec2,
};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))
}

fn input(screen: Rect, events: Vec<Event>) -> RawInput {
    RawInput {
        screen_rect: Some(screen),
        events,
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, events: Vec<Event>, show: &mut dyn FnMut(&mut Ui)) {
    let mut output = ctx.run_ui(input(screen(), events), show);
    output.textures_delta.clear();
}

fn pointer_press(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

fn same_rect(a: Rect, b: Rect) -> bool {
    (a.left() - b.left()).abs() <= 0.5
        && (a.right() - b.right()).abs() <= 0.5
        && (a.top() - b.top()).abs() <= 0.5
        && (a.bottom() - b.bottom()).abs() <= 0.5
}

/// `inner` inside `outer`, with a point of tolerance on every edge.
fn rect_within(inner: Rect, outer: Rect, tol: f32) -> bool {
    inner.left() >= outer.left() - tol
        && inner.right() <= outer.right() + tol
        && inner.top() >= outer.top() - tol
        && inner.bottom() <= outer.bottom() + tol
}

fn window_margin(ctx: &Context) -> f32 {
    ctx.style_of(ctx.theme()).spacing.window_margin.left as f32
}

/// Run idle frames until the window rect stops changing, then return the
/// settled rect.
fn settle_window(
    ctx: &Context,
    win: impl Fn() -> Window<'static> + Copy,
    content: impl Fn(&mut Ui) + Copy,
) -> Rect {
    let mut settled = Rect::NOTHING;
    let mut prev = Rect::NOTHING;
    for i in 0..8 {
        let mut rect = Rect::NOTHING;
        run(ctx, vec![], &mut |ui| {
            if let Some(response) = win().show(ui.ctx(), |ui| content(ui)) {
                rect = response.response.rect;
            }
        });
        if i > 0 && same_rect(rect, prev) {
            return rect;
        }
        prev = rect;
        settled = rect;
    }
    settled
}

/// The painted row rects of every text shape in the output, in paint order.
fn text_row_rects(output: &FullOutput) -> Vec<Rect> {
    let mut rows = Vec::new();
    for clipped in &output.shapes {
        if let egui::Shape::Text(text) = &clipped.shape {
            for row in &text.galley.rows {
                rows.push(row.rect().translate(text.pos.to_vec2()));
            }
        }
    }
    rows
}

/// The window builder shared by the bare-widget tests: placed so the settled
/// rect fits on screen in either direction.
fn bare_window(pos: Pos2, size: Vec2) -> Window<'static> {
    Window::new("bare win")
        .resizable(false)
        .default_size(size)
        .default_pos(pos)
}

/// The title-less window builder for the content-hug tests: its size follows
/// the content measurement (no title bar, not user-resizable).
fn title_less_window(pos: Pos2, size: Vec2) -> Window<'static> {
    Window::new("hug win")
        .title_bar(false)
        .resizable(false)
        .default_size(size)
        .default_pos(pos)
}

// ---------------------------------------------------------------------------

/// Bare `ui.label` and `ui.button` widgets added directly to a window body hug
/// the reading-start edge under RTL (an Arabic label covers mixed-direction
/// content), and a nested window's bare label hugs its own body too. The LTR
/// twin hugs the left edge, unchanged.
#[test]
fn bare_widgets_hug_the_reading_start_edge_of_a_window_body() {
    const POS: Pos2 = pos2(400.0, 50.0);
    const SIZE: Vec2 = vec2(340.0, 420.0);

    // RTL: the bare widgets hug the window's right inner edge:
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let label = Cell::new(Rect::NOTHING);
    let button = Cell::new(Rect::NOTHING);
    let inner_label = Cell::new(Rect::NOTHING);
    let inner_window = Cell::new(Rect::NOTHING);
    let settled = settle_window(
        &ctx,
        || bare_window(POS, SIZE),
        |ui| {
            label.set(ui.add(Label::new("مرحبا world")).rect);
            button.set(ui.button("btn").rect);
            if let Some(response) = Window::new("inner").show(ui.ctx(), |ui| {
                inner_label.set(ui.label("inner").rect);
            }) {
                inner_window.set(response.response.rect);
            }
        },
    );
    assert!(settled.is_positive(), "window should be laid out");
    let margin = window_margin(&ctx);
    assert!(
        (label.get().right() - (settled.right() - margin)).abs() <= 2.0,
        "RTL: the bare label must hug the reading-start (right) inner edge \
         (window {settled:?}, label {:?}, margin {margin})",
        label.get()
    );
    assert!(
        (button.get().right() - (settled.right() - margin)).abs() <= 2.0,
        "RTL: the bare button must hug the reading-start (right) inner edge \
         (window {settled:?}, button {:?}, margin {margin})",
        button.get()
    );
    assert!(
        rect_within(label.get(), settled, 1.0) && rect_within(button.get(), settled, 1.0),
        "RTL: the bare widgets must paint inside the window \
         (window {settled:?}, label {:?}, button {:?})",
        label.get(),
        button.get()
    );
    // Nested window: its bare label hugs its own reading-start edge too:
    let inner = inner_window.get();
    assert!(inner.is_positive(), "the inner window should be laid out");
    assert!(
        (inner_label.get().right() - (inner.right() - margin)).abs() <= 2.0,
        "RTL: the nested window's bare label must hug its reading-start inner edge \
         (inner window {inner:?}, label {:?})",
        inner_label.get()
    );

    // LTR twin: the bare widgets hug the left inner edge, unchanged:
    let ctx = Context::default();
    let label = Cell::new(Rect::NOTHING);
    let button = Cell::new(Rect::NOTHING);
    let settled = settle_window(
        &ctx,
        || bare_window(POS, SIZE),
        |ui| {
            label.set(ui.add(Label::new("hello world")).rect);
            button.set(ui.button("btn").rect);
        },
    );
    assert!(settled.is_positive(), "window should be laid out");
    let margin = window_margin(&ctx);
    assert!(
        (label.get().left() - (settled.left() + margin)).abs() <= 2.0,
        "LTR: the bare label must hug the left inner edge \
         (window {settled:?}, label {:?}, margin {margin})",
        label.get()
    );
    assert!(
        (button.get().left() - (settled.left() + margin)).abs() <= 2.0,
        "LTR: the bare button must hug the left inner edge \
         (window {settled:?}, button {:?}, margin {margin})",
        button.get()
    );
}

/// A body mixing bare widgets and a nested row hugs consistently: under RTL
/// the bare label, the bare button and the row share one reading-start edge.
#[test]
fn mixed_body_hugs_consistently() {
    const POS: Pos2 = pos2(400.0, 50.0);
    const SIZE: Vec2 = vec2(340.0, 420.0);

    // RTL: label, button and row share their right (reading-start) edge:
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let label = Cell::new(Rect::NOTHING);
    let button = Cell::new(Rect::NOTHING);
    let row = Cell::new(Rect::NOTHING);
    let settled = settle_window(
        &ctx,
        || bare_window(POS, SIZE),
        |ui| {
            label.set(ui.label("label").rect);
            button.set(ui.button("btn").rect);
            row.set(ui.horizontal(|ui| ui.label("row")).response.rect);
        },
    );
    assert!(settled.is_positive(), "window should be laid out");
    assert!(
        (label.get().right() - button.get().right()).abs() <= 1.0,
        "RTL: the bare label and the button must share the reading-start edge \
         (label {:?}, button {:?})",
        label.get(),
        button.get()
    );
    assert!(
        (row.get().right() - button.get().right()).abs() <= 1.0,
        "RTL: the row must share the reading-start edge with the bare widgets \
         (row {:?}, button {:?})",
        row.get(),
        button.get()
    );
    let margin = window_margin(&ctx);
    assert!(
        (button.get().right() - (settled.right() - margin)).abs() <= 2.0,
        "RTL: the mixed body must hug the reading-start inner edge \
         (window {settled:?}, button {:?})",
        button.get()
    );

    // LTR twin: all three share their left edge, unchanged:
    let ctx = Context::default();
    let label = Cell::new(Rect::NOTHING);
    let button = Cell::new(Rect::NOTHING);
    let row = Cell::new(Rect::NOTHING);
    let settled = settle_window(
        &ctx,
        || bare_window(POS, SIZE),
        |ui| {
            label.set(ui.label("label").rect);
            button.set(ui.button("btn").rect);
            row.set(ui.horizontal(|ui| ui.label("row")).response.rect);
        },
    );
    assert!(settled.is_positive(), "window should be laid out");
    assert!(
        (label.get().left() - button.get().left()).abs() <= 1.0,
        "LTR: the bare label and the button must share the left edge \
         (label {:?}, button {:?})",
        label.get(),
        button.get()
    );
    assert!(
        (row.get().left() - button.get().left()).abs() <= 1.0,
        "LTR: the row must share the left edge with the bare widgets \
         (row {:?}, button {:?})",
        row.get(),
        button.get()
    );
}

/// Explicit `Align::Max` and `Align::Center` vertical bodies keep their
/// physical placement under RTL: `Max` bodies are already right-anchored and
/// must not move, `Center` bodies stay centered.
#[test]
fn explicit_max_and_center_vertical_bodies_are_not_double_mirrored() {
    const POS: Pos2 = pos2(400.0, 50.0);
    const SIZE: Vec2 = vec2(340.0, 420.0);

    let body = |ui: &mut Ui, max_label: &Cell<Rect>, center_label: &Cell<Rect>| {
        ui.with_layout(Layout::top_down(Align::Max), |ui| {
            max_label.set(ui.label("max").rect);
        });
        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            center_label.set(ui.label("center").rect);
        });
    };

    // RTL:
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let max_label = Cell::new(Rect::NOTHING);
    let center_label = Cell::new(Rect::NOTHING);
    let settled = settle_window(
        &ctx,
        || bare_window(POS, SIZE),
        |ui| {
            body(ui, &max_label, &center_label);
        },
    );
    assert!(settled.is_positive(), "window should be laid out");
    let margin = window_margin(&ctx);
    assert!(
        (max_label.get().right() - (settled.right() - margin)).abs() <= 2.0,
        "RTL: the explicit `Align::Max` body must stay right-anchored \
         (window {settled:?}, label {:?})",
        max_label.get()
    );
    assert!(
        (center_label.get().center().x - settled.center().x).abs() <= 2.0,
        "RTL: the explicit `Align::Center` body must stay centered \
         (window {settled:?}, label {:?})",
        center_label.get()
    );

    // LTR twin: identical physical placement:
    let ctx = Context::default();
    let max_label = Cell::new(Rect::NOTHING);
    let center_label = Cell::new(Rect::NOTHING);
    let settled = settle_window(
        &ctx,
        || bare_window(POS, SIZE),
        |ui| {
            body(ui, &max_label, &center_label);
        },
    );
    assert!(settled.is_positive(), "window should be laid out");
    let margin = window_margin(&ctx);
    assert!(
        (max_label.get().right() - (settled.right() - margin)).abs() <= 2.0,
        "LTR: the explicit `Align::Max` body must stay right-anchored \
         (window {settled:?}, label {:?})",
        max_label.get()
    );
    assert!(
        (center_label.get().center().x - settled.center().x).abs() <= 2.0,
        "LTR: the explicit `Align::Center` body must stay centered \
         (window {settled:?}, label {:?})",
        center_label.get()
    );
}

/// Inside an explicit left-to-right scope under an RTL interface, bare widgets
/// stay left-anchored: the scope sets `Style::right_to_left = false` before
/// construction (via the `UiBuilder`) and pins the vertical `Align::Min`
/// layout, so the construction-time cross-align mirror does not apply (the
/// escape hatch for physical-left content; a post-construction
/// `ui.style_mut()` cannot un-mirror a layout that was already resolved).
#[test]
fn physical_left_scope_stays_left() {
    const POS: Pos2 = pos2(400.0, 50.0);
    const SIZE: Vec2 = vec2(340.0, 420.0);

    let body = |ui: &mut Ui, label: &Cell<Rect>| {
        let mut ltr_style = (**ui.style()).clone();
        ltr_style.right_to_left = false;
        ui.scope_builder(
            UiBuilder::new()
                .style(ltr_style)
                .layout(Layout::top_down(Align::Min)),
            |ui| {
                label.set(ui.label("left").rect);
            },
        );
    };

    // RTL context, LTR scope: the label stays left-anchored:
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let label = Cell::new(Rect::NOTHING);
    let settled = settle_window(
        &ctx,
        || bare_window(POS, SIZE),
        |ui| {
            body(ui, &label);
        },
    );
    assert!(settled.is_positive(), "window should be laid out");
    let margin = window_margin(&ctx);
    assert!(
        (label.get().left() - (settled.left() + margin)).abs() <= 2.0,
        "RTL: the scoped label must stay left-anchored \
         (window {settled:?}, label {:?})",
        label.get()
    );
    assert!(
        label.get().right() < settled.center().x,
        "RTL: the scoped label must sit on the physical left half \
         (window {settled:?}, label {:?})",
        label.get()
    );

    // LTR twin: identical:
    let ctx = Context::default();
    let label = Cell::new(Rect::NOTHING);
    let settled = settle_window(
        &ctx,
        || bare_window(POS, SIZE),
        |ui| {
            body(ui, &label);
        },
    );
    assert!(settled.is_positive(), "window should be laid out");
    let margin = window_margin(&ctx);
    assert!(
        (label.get().left() - (settled.left() + margin)).abs() <= 2.0,
        "LTR: the scoped label must stay left-anchored \
         (window {settled:?}, label {:?})",
        label.get()
    );
}

/// A title-less RTL window whose body is a bare label settles at
/// label-width + frame margins, stable across idle frames, with the
/// reading-start edge fixed; the LTR twin is unchanged.
#[test]
fn title_less_window_with_bare_label_hugs_content_width() {
    const POS: Pos2 = pos2(400.0, 50.0);
    const START_SIZE: Vec2 = vec2(340.0, 420.0);

    // RTL: the window hugs the label's width:
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let label = Cell::new(Rect::NOTHING);
    let win = || title_less_window(POS, START_SIZE);
    let content = |ui: &mut Ui| {
        label.set(ui.label("hi").rect);
    };
    let settled = settle_window(&ctx, win, content);
    assert!(settled.is_positive(), "window should be laid out");
    for i in 0..3 {
        let next = settle_window(&ctx, win, content);
        assert!(
            same_rect(next, settled),
            "RTL: idle frames must not move or resize the window \
             (settled {settled:?}, frame {i} {next:?})"
        );
    }
    assert!(
        (settled.right() - POS.x).abs() <= 1.0,
        "RTL: the reading-start (right) edge must stay at the default pos \
         (settled {settled:?})"
    );
    let margin = window_margin(&ctx);
    assert!(
        (settled.width() - (label.get().width() + 2.0 * margin)).abs() <= 2.0,
        "RTL: the window must hug the label width plus the frame margins \
         (settled {settled:?}, label {:?}, margin {margin})",
        label.get()
    );
    assert!(
        rect_within(label.get(), settled, 1.0),
        "RTL: the label must paint inside the window \
         (window {settled:?}, label {:?})",
        label.get()
    );

    // LTR twin: unchanged (hugs content width as before):
    let ctx = Context::default();
    let label = Cell::new(Rect::NOTHING);
    let content = |ui: &mut Ui| {
        label.set(ui.label("hi").rect);
    };
    let settled = settle_window(&ctx, win, content);
    assert!(settled.is_positive(), "window should be laid out");
    let margin = window_margin(&ctx);
    assert!(
        (settled.width() - (label.get().width() + 2.0 * margin)).abs() <= 2.0,
        "LTR: the window must hug the label width plus the frame margins \
         (settled {settled:?}, label {:?}, margin {margin})",
        label.get()
    );
    assert!(
        rect_within(label.get(), settled, 1.0),
        "LTR: the label must paint inside the window \
         (window {settled:?}, label {:?})",
        label.get()
    );
}

/// Run one ui pass with an explicit screen rect and return the full output.
fn run_output(
    ctx: &Context,
    screen: Rect,
    time: f64,
    events: Vec<Event>,
    show: impl FnMut(&mut Ui),
) -> FullOutput {
    let input = RawInput {
        screen_rect: Some(screen),
        time: Some(time),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, show);
    output.textures_delta.clear();
    output
}

/// The painted `Resize` container rect: the stroke outline is the container
/// rect expanded by 2.0 per side, so shrink it back.
fn container_rect_from_shapes(output: &FullOutput) -> Rect {
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

/// A plain `Resize` under RTL whose content is a bare label reports a
/// requested shrink on the drag frames and hugs the content width when the
/// drag ends; the LTR twin is unchanged.
#[test]
fn plain_resize_with_bare_label_shrinks_and_hugs_under_rtl() {
    const SCREEN: Vec2 = vec2(400.0, 300.0);
    const START_SIZE: Vec2 = vec2(200.0, 100.0);
    const SHRINK: f32 = 60.0;
    const PAST_CONTENT: f32 = 180.0;
    let screen = Rect::from_min_size(Pos2::ZERO, SCREEN);

    let shrink = |rtl: bool, delta: f32, hug_on_release: bool| {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        // One pass per frame: the sizing pass is the visible output, so the
        // re-pin (or its absence) is observable on the first drag frame.
        ctx.options_mut(|o| o.max_passes = NonZeroUsize::MIN);

        let id = Id::unique("plain_resize_bare_label");
        let label = Cell::new(Rect::NOTHING);

        let container = |events: Vec<Event>, time: f64| {
            let output = run_output(&ctx, screen, time, events, |ui| {
                CentralPanel::default().show(ui, |ui| {
                    egui::Resize::default()
                        .id(id)
                        .default_size(START_SIZE)
                        .show(ui, |ui| {
                            label.set(ui.label("hi").rect);
                        });
                });
            });
            container_rect_from_shapes(&output)
        };

        // Settle at the start size (a bare label must not balloon the
        // container to the offered span):
        let mut time = 0.0;
        let mut settled = Rect::NOTHING;
        for i in 0..8 {
            time += 0.1;
            let next = container(vec![], time);
            if i > 0 && same_rect(next, settled) {
                settled = next;
                break;
            }
            settled = next;
        }
        assert!(
            (settled.width() - START_SIZE.x).abs() <= 2.0,
            "rtl = {rtl}: the container must settle at its start size, not balloon to the \
             offered span (settled {settled:?})"
        );

        // Grab the corner grip (reading-end bottom corner):
        let grip_center = ctx
            .read_response(id.with("__resize_corner"))
            .map(|corner| corner.rect.center())
            .filter(|center| settled.contains(*center))
            .unwrap_or_else(|| {
                if rtl {
                    pos2(settled.left() + 8.0, settled.bottom() - 8.0)
                } else {
                    pos2(settled.right() - 8.0, settled.bottom() - 8.0)
                }
            });
        time += 1.0;
        container(vec![Event::PointerMoved(grip_center)], time);
        time += 0.1;
        container(vec![pointer_press(grip_center, true)], time);

        // First drag frame: the requested shrink is reported immediately:
        time += 0.1;
        let moved = if rtl {
            grip_center + vec2(delta, 0.0)
        } else {
            grip_center + vec2(-delta, 0.0)
        };
        let dragging = container(vec![Event::PointerMoved(moved)], time);
        if hug_on_release {
            // The requested shrink stops at the content minimum:
            assert!(
                dragging.width() <= (START_SIZE.x - delta) + 2.0,
                "rtl = {rtl}: the drag frame must shrink to at most the requested size \
                 (requested ~{}, got {dragging:?})",
                START_SIZE.x - delta
            );
        } else {
            assert!(
                (dragging.width() - (START_SIZE.x - delta)).abs() <= 2.0,
                "rtl = {rtl}: the first drag frame must report the requested shrink \
                 (expected ~{}, got {dragging:?})",
                START_SIZE.x - delta
            );
        }
        if rtl {
            assert!(
                (dragging.right() - settled.right()).abs() <= 1.0,
                "rtl = {rtl}: the reading-start (right) edge must stay fixed on the drag frame \
                 (settled {settled:?}, dragging {dragging:?})"
            );
        } else {
            assert!(
                (dragging.left() - settled.left()).abs() <= 1.0,
                "rtl = {rtl}: the reading-start (left) edge must stay fixed on the drag frame \
                 (settled {settled:?}, dragging {dragging:?})"
            );
        }

        // Release: past the content minimum the container hugs the content
        // width; above it, the requested shrink survives the release:
        time += 0.1;
        container(vec![pointer_press(moved, false)], time);
        time += 0.1;
        let mut released = Rect::NOTHING;
        for _ in 0..8 {
            time += 0.1;
            let next = container(vec![], time);
            if same_rect(next, released) {
                break;
            }
            released = next;
        }
        if hug_on_release {
            assert!(
                (released.width() - label.get().width()).abs() <= 2.0,
                "rtl = {rtl}: the released container must hug the content width \
                 (label {:?}, released {released:?})",
                label.get()
            );
        } else {
            assert!(
                (released.width() - (START_SIZE.x - delta)).abs() <= 2.0,
                "rtl = {rtl}: the shrunk size must survive the release \
                 (expected ~{}, released {released:?})",
                START_SIZE.x - delta
            );
        }
    };

    // RTL:
    shrink(true, SHRINK, false);
    shrink(true, PAST_CONTENT, true);

    // LTR twin: identical numbers:
    shrink(false, SHRINK, false);
    shrink(false, PAST_CONTENT, true);
}

/// `.halign(Align::LEFT)` keeps left galley alignment inside a right-hugging
/// rect under RTL; the LTR twin is unchanged. The label sits in a plain
/// `Resize` container wider than the galley, so both the widget-rect anchor
/// and the galley's internal row alignment are observable.
#[test]
fn explicit_label_halign_override_wins() {
    const SCREEN: Vec2 = vec2(400.0, 300.0);
    const START_SIZE: Vec2 = vec2(200.0, 100.0);
    let screen = Rect::from_min_size(Pos2::ZERO, SCREEN);

    // A label that wraps into rows of different widths, so the galley's
    // internal halign is observable in the painted rows.
    const TEXT: &str = "wwwww wwwwwww wwwww wwwwwwwww wwwwwww wwwww wwww";

    let body = |rtl: bool| {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let id = Id::unique("halign_override_resize");
        let label = Cell::new(Rect::NOTHING);
        let output = run_output(&ctx, screen, 0.0, vec![], |ui| {
            CentralPanel::default().show(ui, |ui| {
                egui::Resize::default()
                    .id(id)
                    .default_size(START_SIZE)
                    .show(ui, |ui| {
                        label.set(ui.add(Label::new(TEXT).halign(Align::LEFT)).rect);
                    });
            });
        });
        let settled = container_rect_from_shapes(&output);
        assert!(settled.is_positive(), "the container should be laid out");
        // The widget rect hugs the body's effective cross-align edge:
        if rtl {
            assert!(
                (label.get().right() - settled.right()).abs() <= 2.0,
                "RTL: the label widget rect must hug the reading-start (right) inner edge \
                 (container {settled:?}, label {:?})",
                label.get()
            );
        } else {
            assert!(
                (label.get().left() - settled.left()).abs() <= 2.0,
                "LTR: the label widget rect must hug the left inner edge \
                 (container {settled:?}, label {:?})",
                label.get()
            );
        }
        // The override keeps the rows left-aligned inside the galley:
        let rows = text_row_rects(&output);
        assert!(
            rows.len() >= 2,
            "the label should wrap into several rows (got {rows:?})"
        );
        let first_left = rows[0].left();
        assert!(
            rows.iter()
                .all(|row| (row.left() - first_left).abs() <= 1.0),
            "rtl = {rtl}: the halign override must keep the rows left-aligned in the galley \
             (rows {rows:?})"
        );
    };

    body(true);
    body(false);
}
