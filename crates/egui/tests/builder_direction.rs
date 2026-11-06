//! Tests for the per-subtree direction override on [`egui::UiBuilder`]:
//! `right_to_left(true/false)` flips the subtree's own style at construction,
//! so every direction consumer (the vertical mirror, [`egui::Ui::is_rtl`],
//! widget text direction, galley halign) follows by construction.
//!
//! The override is resolved against the parent's effective style, composes
//! through nesting (RTL → LTR → RTL re-mirrors), and is pinned for the
//! subtree's lifetime: it does not track later
//! [`egui::Context::set_right_to_left`] flips. No snapshot assertions;
//! integer geometry only (8 pt `CentralPanel` margin, TOL 0.5).

use egui::{CentralPanel, Context, Layout, RawInput, Rect, Sense, Ui, UiBuilder, Vec2, vec2};

/// The screen size of every test frame.
const SCREEN: Vec2 = vec2(600.0, 400.0);

/// A plain 50 x 20 widget.
const WIDGET: Vec2 = vec2(50.0, 20.0);

/// Assertion tolerance (ui rects are rounded to integers).
const TOL: f32 = 0.5;

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(egui::pos2(0.0, 0.0), size)),
        ..Default::default()
    }
}

/// Run one ui pass and drop the texture deltas (these tests never render).
fn run(ctx: &Context, mut run_ui: impl FnMut(&mut Ui)) {
    let output = ctx.run_ui(input(SCREEN), |ui| run_ui(ui));
    output.drop_without_applying_deltas();
}

/// The widget's rect and effective `is_rtl()` inside a subtree built with
/// `override_dir` (None = no override), under a parent running with
/// direction `rtl`. The subtree gets the explicit default layout
/// (`top_down(Align::Min)`), so the override's flip is the only mirror
/// input; the subtree spans the panel's available rect.
fn overridden_subtree(rtl: bool, override_dir: Option<bool>) -> (Rect, bool) {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }
    let mut rect = Rect::NOTHING;
    let mut subtree_rtl = false;
    run(&ctx, |ui| {
        CentralPanel::default().show(ui, |ui| {
            let mut builder = UiBuilder::new()
                .max_rect(ui.available_rect_before_wrap())
                .layout(Layout::default());
            if let Some(dir) = override_dir {
                builder = builder.right_to_left(dir);
            }
            let mut child = ui.new_child(builder);
            subtree_rtl = child.is_rtl();
            rect = child.allocate_exact_size(WIDGET, Sense::hover()).0;
        });
    });
    (rect, subtree_rtl)
}

/// RTL parent + `right_to_left(false)` child: the subtree flips to LTR — its
/// widgets hug the LEFT edge of the subtree's max rect and `is_rtl()` is
/// false. LTR parent + `right_to_left(true)`: mirrored twin.
#[test]
fn override_flips_the_subtree_direction() {
    let (rect, subtree_rtl) = overridden_subtree(true, Some(false));
    assert!(!subtree_rtl, "the overridden subtree must report LTR");
    assert!(
        (rect.left() - 8.0).abs() <= TOL,
        "rtl parent + forced-ltr subtree: the widget must hug the left edge, \
         got {rect:?}"
    );

    let (rect, subtree_rtl) = overridden_subtree(false, Some(true));
    assert!(subtree_rtl, "the overridden subtree must report RTL");
    assert!(
        (rect.right() - (SCREEN.x - 8.0)).abs() <= TOL,
        "ltr parent + forced-rtl subtree: the widget must hug the right \
         edge, got {rect:?}"
    );
}

/// No override (`right_to_left: None`) behaves exactly as before: the
/// subtree inherits the interface direction in both directions.
#[test]
fn no_override_inherits_the_interface_direction() {
    let (rect, subtree_rtl) = overridden_subtree(false, None);
    assert!(!subtree_rtl);
    assert!(
        (rect.left() - 8.0).abs() <= TOL,
        "ltr: an unoverridden subtree hugs the left edge, got {rect:?}"
    );

    let (rect, subtree_rtl) = overridden_subtree(true, None);
    assert!(subtree_rtl);
    assert!(
        (rect.right() - (SCREEN.x - 8.0)).abs() <= TOL,
        "rtl: an unoverridden subtree hugs the right edge, got {rect:?}"
    );
}

/// Nested overrides resolve against the parent's *effective* style: RTL →
/// forced-LTR → forced-RTL re-mirrors at the innermost boundary.
#[test]
fn nested_overrides_remirror() {
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    let mut outer = Rect::NOTHING;
    let mut inner = Rect::NOTHING;
    let mut outer_rtl = false;
    let mut inner_rtl = false;
    run(&ctx, |ui| {
        CentralPanel::default().show(ui, |ui| {
            let mut a = ui.new_child(
                UiBuilder::new()
                    .max_rect(ui.available_rect_before_wrap())
                    .layout(Layout::default())
                    .right_to_left(false),
            );
            outer_rtl = a.is_rtl();
            outer = a.allocate_exact_size(WIDGET, Sense::hover()).0;
            let mut b = a.new_child(
                UiBuilder::new()
                    .max_rect(a.available_rect_before_wrap())
                    .layout(Layout::default())
                    .right_to_left(true),
            );
            inner_rtl = b.is_rtl();
            inner = b.allocate_exact_size(WIDGET, Sense::hover()).0;
        });
    });

    assert!(
        !outer_rtl && inner_rtl,
        "each boundary resolves against its \
        parent's effective style, got outer {outer_rtl} inner {inner_rtl}"
    );
    assert!(
        (outer.left() - 8.0).abs() <= TOL,
        "the forced-LTR middle subtree hugs the left edge, got {outer:?}"
    );
    assert!(
        (inner.right() - (SCREEN.x - 8.0)).abs() <= TOL,
        "the forced-RTL innermost subtree re-mirrors to the right edge, got \
         {inner:?}"
    );
}

/// The override is pinned: it does not track later
/// [`egui::Context::set_right_to_left`] flips for the subtree's lifetime
/// (re-create the child with a new override to follow).
#[test]
fn override_is_pinned_against_later_context_flips() {
    let ctx = Context::default(); // LTR
    let mut child_rects: Vec<Rect> = Vec::new();
    let mut child_rtls: Vec<bool> = Vec::new();
    let mut parent_rtls: Vec<bool> = Vec::new();

    for frame in 0..2 {
        let mut child_rect = Rect::NOTHING;
        let mut child_rtl = false;
        let mut parent_rtl = false;
        let output = ctx.run_ui(input(SCREEN), |ui| {
            parent_rtl = ui.is_rtl();
            CentralPanel::default().show(ui, |ui| {
                let mut child = ui.new_child(
                    UiBuilder::new()
                        .max_rect(ui.available_rect_before_wrap())
                        .layout(Layout::default())
                        .right_to_left(false),
                );
                child_rtl = child.is_rtl();
                child_rect = child.allocate_exact_size(WIDGET, Sense::hover()).0;
            });
        });
        output.drop_without_applying_deltas();
        child_rects.push(child_rect);
        child_rtls.push(child_rtl);
        parent_rtls.push(parent_rtl);
        if frame == 0 {
            // Flip the interface direction between frames (outside any pass):
            ctx.set_right_to_left(true);
        }
    }

    assert!(
        !parent_rtls[0] && parent_rtls[1],
        "sanity: the interface must flip from LTR to RTL between frames, \
         got {parent_rtls:?}"
    );
    assert!(
        !child_rtls[0] && !child_rtls[1],
        "the overridden subtree stays LTR in both frames, got {child_rtls:?}"
    );
    for (frame, rect) in child_rects.iter().enumerate() {
        assert!(
            (rect.left() - 8.0).abs() <= TOL,
            "frame {frame}: the pinned-LTR subtree keeps hugging the left \
             edge even though the interface flipped to RTL, got {rect:?}"
        );
    }
}
