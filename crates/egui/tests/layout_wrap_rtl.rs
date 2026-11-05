//! Regression tests for vertical main-wrap columns ([`Layout::main_wrap`] with
//! a vertical main axis) under right-to-left reading: wrap columns grow from
//! the reading-start edge, so under RTL — or in an explicit vertical
//! `Align::Max` wrap body, the same settled predicate
//! ([`egui::Placer::prefer_right_to_left`] — pub(crate), exercised through the
//! public layout here) — the second column sits strictly LEFT of the first,
//! mirroring the horizontal wrap rows. Under LTR with `Align::Min`/`Center`
//! nothing changes: columns grow rightward exactly as before.
//!
//! Horizontal main-wrap rows keep wrapping downward (the wrap decision is
//! y-based) in both directions; these tests pin that, so the mirrored vertical
//! arms cannot disturb the horizontal ones.
//!
//! Every mirrored case carries its LTR twin, so this file doubles as the
//! LTR-unchanged regression. No snapshot assertions; integer geometry only
//! (the default `CentralPanel` has an 8 pt inner margin, egui rounds ui rects
//! with `round_ui`, and assertions use a 0.5 tolerance).

use egui::{Align, CentralPanel, Context, Layout, RawInput, Rect, Sense, Ui, Vec2, vec2};

/// The screen size of every test frame.
const SCREEN: Vec2 = vec2(600.0, 400.0);

/// The wrap region: 200 x 60 — wide enough for a 30 pt widget, too short for
/// two 40 pt-tall ones, so the second widget must open a new wrap column.
const REGION: Vec2 = vec2(200.0, 60.0);

/// A region tall enough for two stacked 40 pt widgets, so no wrap is needed
/// and the body must stay content-tight.
const TALL_REGION: Vec2 = vec2(200.0, 100.0);

/// A plain 30 x 40 widget: taller than half of [`REGION`], so two of them
/// overflow it vertically.
const WIDGET: Vec2 = vec2(30.0, 40.0);

/// The 100 pt-wide widgets used by the horizontal wrap-row pins.
const ROW_WIDGET: Vec2 = vec2(100.0, 10.0);

/// The row region: 150 pt wide — fits one [`ROW_WIDGET`] but not two.
const ROW_REGION: Vec2 = vec2(150.0, 60.0);

/// Assertion tolerance (ui rects are rounded to integers).
const TOL: f32 = 0.5;

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, size)),
        ..Default::default()
    }
}

/// Run one ui pass and drop the texture deltas (these tests never render).
fn run(ctx: &Context, mut run_ui: impl FnMut(&mut Ui)) {
    let output = ctx.run_ui(input(SCREEN), |ui| run_ui(ui));
    output.drop_without_applying_deltas();
}

/// One pass: a central panel containing a `region`-sized main-wrap body laid
/// out with `layout` (which must carry `with_main_wrap(true)` where a wrap is
/// wanted) and filled with two widgets. Returns the body's max rect, the two
/// widget rects, and the item spacing on the main axis used between them.
///
/// For vertical layouts the widgets are [`WIDGET`]; for horizontal ones
/// [`ROW_WIDGET`] (pick `region` accordingly).
fn wrap_rects(
    rtl: bool,
    layout: Layout,
    region_size: Vec2,
    widget: Vec2,
) -> (Rect, Rect, Rect, Vec2) {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }
    let mut region = Rect::NOTHING;
    let mut first = Rect::NOTHING;
    let mut second = Rect::NOTHING;
    let mut spacing = Vec2::ZERO;
    run(&ctx, |ui| {
        CentralPanel::default().show(ui, |ui| {
            ui.allocate_ui_with_layout(region_size, layout, |ui| {
                region = ui.max_rect();
                spacing = ui.spacing().item_spacing;
                first = ui.allocate_exact_size(widget, Sense::hover()).0;
                second = ui.allocate_exact_size(widget, Sense::hover()).0;
            });
        });
    });
    (region, first, second, spacing)
}

/// The vertical main-wrap mirror: under RTL the first column hugs the
/// reading-start (right) edge of the wrap region and the second column lies
/// strictly left of it, its content hugging its own reading-start edge.
#[test]
fn vertical_main_wrap_rtl_grows_leftward() {
    let (region, first, second, spacing) =
        wrap_rects(true, Layout::top_down(Align::Min).with_main_wrap(true), REGION, WIDGET);

    // Column 0 hugs the reading-start (right) edge of the wrap region:
    assert!(
        (first.right() - region.right()).abs() <= TOL,
        "rtl: the first wrap column must hug the reading-start (right) edge, \
         got {first:?} in {region:?}"
    );
    // Column 1 lies strictly left of column 0 (it grows leftward):
    assert!(
        second.right() < first.left(),
        "rtl: the second wrap column must lie strictly left of the first, \
         got {second:?} vs {first:?}"
    );
    // Content anchor: the RTL mirror canonicalizes the cross alignment, so
    // each column's content hugs that column's reading-start (right) edge —
    // the second column's right edge sits one item spacing left of the region.
    assert!(
        (second.right() - (region.left() - spacing.x)).abs() <= TOL,
        "rtl: the second column's content must hug the mirrored column's \
         reading-start edge, got {second:?} in {region:?} (spacing {spacing:?})"
    );
}

/// LTR twin: unchanged behavior — column 0 hugs the left edge of the wrap
/// region and column 1 grows strictly rightward.
#[test]
fn vertical_main_wrap_ltr_still_grows_rightward() {
    let (region, first, second, _) =
        wrap_rects(false, Layout::top_down(Align::Min).with_main_wrap(true), REGION, WIDGET);

    assert!(
        (first.left() - region.left()).abs() <= TOL,
        "ltr: the first wrap column must hug the left edge, got {first:?} in {region:?}"
    );
    assert!(
        second.left() > first.right(),
        "ltr: the second wrap column must lie strictly right of the first, \
         got {second:?} vs {first:?}"
    );
}

/// The settled predicate decides the wrap direction, not the interface flag:
/// an explicit LTR vertical `Align::Max` main-wrap body also wraps leftward —
/// the same term the mirrored columns and `Grid` use.
#[test]
fn vertical_main_wrap_ltr_max_grows_leftward() {
    let (region, first, second, _) =
        wrap_rects(false, Layout::top_down(Align::Max).with_main_wrap(true), REGION, WIDGET);

    assert!(
        (first.right() - region.right()).abs() <= TOL,
        "ltr max: the first wrap column hugs the right edge of the region, \
         got {first:?} in {region:?}"
    );
    assert!(
        second.right() < first.left(),
        "ltr max: a vertical `Align::Max` wrap body wraps leftward (one term \
         everywhere), got {second:?} vs {first:?}"
    );
}

/// Horizontal main-wrap rows keep wrapping downward in both directions — the
/// wrap decision is y-based and the horizontal arms are untouched by the
/// vertical mirror. Pins the row order and the reading-start hug of row 0.
#[test]
fn horizontal_main_wrap_rows_still_wrap_downward() {
    // LTR row: the first widget hugs the left edge; the second wraps to a new
    // row below it.
    let (region, first, second, spacing) = wrap_rects(
        false,
        Layout::left_to_right(Align::Min).with_main_wrap(true),
        ROW_REGION,
        ROW_WIDGET,
    );
    assert!(
        (first.left() - region.left()).abs() <= TOL,
        "ltr row: the first widget hugs the left edge, got {first:?} in {region:?}"
    );
    assert!(
        second.top() >= first.bottom() + spacing.y - TOL,
        "ltr row: the second widget must wrap to a new row below, \
         got {second:?} vs {first:?}"
    );

    // RTL row: the first widget hugs the right edge; the second still wraps
    // to a new row below it.
    let (region, first, second, spacing) = wrap_rects(
        true,
        Layout::right_to_left(Align::Min).with_main_wrap(true),
        ROW_REGION,
        ROW_WIDGET,
    );
    assert!(
        (first.right() - region.right()).abs() <= TOL,
        "rtl row: the first widget hugs the right edge, got {first:?} in {region:?}"
    );
    assert!(
        second.top() >= first.bottom() + spacing.y - TOL,
        "rtl row: the second widget must wrap to a new row below, \
         got {second:?} vs {first:?}"
    );
}

/// When the wrap columns fit, the mirrored `max_rect.min.x` expansion must not
/// leak phantom width into the measured span: the body stays content-tight in
/// the sizing pass and in the steady frame.
#[test]
fn vertical_main_wrap_rtl_fitting_columns_stay_content_tight() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let mut widths: Vec<f32> = Vec::new();
    for _ in 0..2 {
        let mut width = 0.0;
        run(&ctx, |ui| {
            CentralPanel::default().show(ui, |ui| {
                let response = ui.allocate_ui_with_layout(
                    TALL_REGION,
                    Layout::top_down(Align::Min).with_main_wrap(true),
                    |ui| {
                        ui.allocate_exact_size(WIDGET, Sense::hover());
                        ui.allocate_exact_size(WIDGET, Sense::hover());
                    },
                );
                width = response.response.rect.width();
            });
        });
        widths.push(width);
    }
    for (frame, width) in widths.iter().enumerate() {
        assert!(
            (*width - WIDGET.x).abs() <= TOL,
            "frame {frame}: fitting wrap columns must not leak phantom width \
             into the measured span, got {width}"
        );
    }
}

/// One pass: a central panel whose contents are produced by `build`; returns
/// the panel's max rect and whatever rect `build` captured (the first widget).
/// Used by the mirrored-horizontal-helper pins so each helper gets a fresh
/// full-height panel.
fn horizontal_first_rect(rtl: bool, mut build: impl FnMut(&mut Ui) -> Rect) -> (Rect, Rect) {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }
    let mut panel = Rect::NOTHING;
    let mut widget = Rect::NOTHING;
    run(&ctx, |ui| {
        CentralPanel::default().show(ui, |ui| {
            panel = ui.max_rect();
            widget = build(ui);
        });
    });
    (panel, widget)
}

/// The mirrored horizontal constructors share one reading-direction term: the
/// first widget hugs the reading-start edge under both directions, the row's
/// vertical alignment follows the helper (top vs centered), and the wrapped
/// variant hugs too.
#[test]
fn mirrored_horizontal_helpers_follow_the_reading_direction() {    let widget = vec2(50.0, 10.0);

    for rtl in [false, true] {
        let (panel, centered) = horizontal_first_rect(rtl, |ui| {
            ui.horizontal_centered(|ui| ui.allocate_exact_size(widget, Sense::hover()).0)
                .inner
        });
        let (panel_top, top) = horizontal_first_rect(rtl, |ui| {
            ui.horizontal_top(|ui| ui.allocate_exact_size(widget, Sense::hover()).0)
                .inner
        });
        let (panel_wrapped, wrapped) = horizontal_first_rect(rtl, |ui| {
            ui.horizontal_wrapped(|ui| ui.allocate_exact_size(widget, Sense::hover()).0)
                .inner
        });

        // Reading-start hug for all three helpers, in both directions:
        for (name, rect, region) in [
            ("horizontal_centered", centered, panel),
            ("horizontal_top", top, panel_top),
            ("horizontal_wrapped", wrapped, panel_wrapped),
        ] {
            if rtl {
                assert!(
                    (rect.right() - region.right()).abs() <= TOL,
                    "{name} (rtl): the first widget must hug the reading-start \
                     (right) edge, got {rect:?} in {region:?}"
                );
            } else {
                assert!(
                    (rect.left() - region.left()).abs() <= TOL,
                    "{name} (ltr): the first widget must hug the reading-start \
                     (left) edge, got {rect:?} in {region:?}"
                );
            }
        }

        // Vertical alignment follows the helper:
        assert!(
            (top.top() - panel_top.top()).abs() <= TOL,
            "horizontal_top: the first widget must hug the top, got {top:?} in {panel_top:?}"
        );
        assert!(
            (centered.center().y - panel.center().y).abs() <= TOL,
            "horizontal_centered: the first widget must be vertically \
             centered, got {centered:?} in {panel:?}"
        );
    }
}

/// `Layout::from_style` is the style-derived default: `top_down(Align::Min)`
/// under LTR and `top_down(Align::Max)` under RTL. Raw `Layout::default()`
/// intentionally stays the upstream-compatible LTR default.
#[test]
fn from_style_derives_the_default_layout_from_the_style() {
    assert_eq!(
        Layout::from_style(&egui::Style::default()),
        Layout::top_down(Align::Min),
        "the style-derived default is plain top-down Min under LTR"
    );
    let mut rtl_style = egui::Style::default();
    rtl_style.right_to_left = true;
    assert_eq!(
        Layout::from_style(&rtl_style),
        Layout::top_down(Align::Max),
        "the style-derived default is top-down Max under RTL"
    );
}

/// Behavior-identity pin for `Ui::new`'s style-derived default: a root `Ui`
/// with no explicit layout anchors bare content on the reading-start edge in
/// the sizing pass and the steady frame — exactly what the old
/// `Layout::default()` + mirror chain produced under RTL.
#[test]
fn default_layout_under_rtl_anchors_bare_content_on_the_reading_start_edge() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let mut rects: Vec<Rect> = Vec::new();
    for _ in 0..2 {
        let mut rect = Rect::NOTHING;
        run(&ctx, |ui| {
            CentralPanel::default().show(ui, |ui| {
                // No explicit layout: `Ui::new` derives it from the style.
                rect = ui.allocate_exact_size(WIDGET, Sense::hover()).0;
            });
        });
        rects.push(rect);
    }
    for (frame, rect) in rects.iter().enumerate() {
        assert!(
            (rect.right() - (SCREEN.x - 8.0)).abs() <= TOL,
            "frame {frame}: bare content with the style-derived default \
             layout must hug the reading-start (right) edge, got {rect:?}"
        );
        assert_eq!(
            rect.width(),
            WIDGET.x,
            "frame {frame}: the bare widget keeps its own width"
        );
    }
}

/// The manual-layout helpers are direction-safe by construction: `place`,
/// `put` and `centered_and_justified` produce identical geometry under RTL
/// and LTR — their `Align::Center` cross never mirrors and the vertical main
/// axis never mirrors — and a `columns` cell's content hugs the cell's
/// reading-start edge under RTL alongside the mirrored enumeration.
#[test]
fn manual_layout_helpers_are_direction_neutral() {
    let slot = Rect::from_min_size(egui::pos2(40.0, 60.0), vec2(120.0, 80.0));
    let lower_slot = Rect::from_min_size(egui::pos2(40.0, 200.0), vec2(120.0, 80.0));
    let widget_size = vec2(50.0, 30.0);

    let mut placed: Vec<Rect> = Vec::new();
    let mut put_rects: Vec<Rect> = Vec::new();
    let mut centered: Vec<Rect> = Vec::new();
    for rtl in [false, true] {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let mut p = Rect::NOTHING;
        let mut q = Rect::NOTHING;
        let mut c = Rect::NOTHING;
        run(&ctx, |ui| {
            CentralPanel::default().show(ui, |ui| {
                p = ui.place(slot, egui::Button::new("w")).rect;
                q = ui.put(lower_slot, egui::Button::new("w")).rect;
                c = ui
                    .centered_and_justified(|ui| {
                        ui.allocate_exact_size(widget_size, Sense::hover()).0
                    })
                    .inner;
            });
        });
        placed.push(p);
        put_rects.push(q);
        centered.push(c);
    }

    // Identical geometry in both directions (byte-identity pin), centered in
    // the given rect / available space:
    assert_eq!(
        placed[0], placed[1],
        "place must be direction-neutral, got {placed:?}"
    );
    assert_eq!(
        put_rects[0], put_rects[1],
        "put must be direction-neutral, got {put_rects:?}"
    );
    assert_eq!(
        centered[0], centered[1],
        "centered_and_justified must be direction-neutral, got {centered:?}"
    );
    assert!(
        (placed[0].center() - slot.center()).length() <= TOL,
        "the placed widget must center in its slot, got {:?} in {slot:?}",
        placed[0]
    );
    assert!(
        (put_rects[0].center() - lower_slot.center()).length() <= TOL,
        "the put widget must center in its slot, got {:?} in {lower_slot:?}",
        put_rects[0]
    );

    // A columns cell's content hugs the cell's reading-start (right) edge
    // under RTL:
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let mut cell = Rect::NOTHING;
    let mut content = Rect::NOTHING;
    run(&ctx, |ui| {
        CentralPanel::default().show(ui, |ui| {
            ui.columns(2, |cols| {
                cell = cols[0].max_rect();
                content = cols[0]
                    .allocate_exact_size(vec2(20.0, 10.0), Sense::hover())
                    .0;
            });
        });
    });
    assert!(
        (content.right() - cell.right()).abs() <= TOL,
        "rtl: a columns cell's content must hug the cell's reading-start \
         (right) edge, got {content:?} in {cell:?}"
    );
}
