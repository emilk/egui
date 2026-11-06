//! The AccessKit scroll actions scroll in the direction the action names.
//!
//! An assistive technology driving `Action::ScrollLeft` asks to reveal what
//! lies to the *left* of the viewport. `PassState::scroll_delta` is
//! *content-relative* (positive X moves the content right), so revealing the
//! content to the left moves the content right: the dispatch arms must express
//! the action in content space, not feed the viewport motion in verbatim.
//!
//! The vertical actions already followed that convention (`ScrollDown` moves
//! the content *up*, revealing what is below); this pins the horizontal ones
//! to the same reading, in both reading directions (the scroll area applies
//! the horizontal delta un-negated under RTL, where offset and content move
//! together — the same content-space sign stays correct there).

use egui::{CentralPanel, Context, Event, Rect, ScrollArea, accesskit, pos2, vec2};

fn target_id() -> egui::Id {
    egui::Id::unique("scroll_direction_target")
}

fn scroll_event(action: accesskit::Action) -> Event {
    Event::AccessKitActionRequest(accesskit::ActionRequest {
        action,
        target_tree: accesskit::TreeId::ROOT,
        target_node: target_id().accesskit_id(),
        data: None,
    })
}

/// One frame: a horizontally overflowing widget inside a non-animated scroll
/// area, pre-scrolled 200 px into the content (so both directions have room).
/// Returns the widget's screen rect.
fn run_frame(ctx: &Context, events: Vec<Event>, time: f64) -> Rect {
    let mut rect = None;
    let input = egui::RawInput {
        time: Some(time),
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 200.0))),
        events,
        ..egui::RawInput::default()
    };
    let mut output = ctx.run_ui(input, |ui| {
        CentralPanel::default().show(ui, |ui| {
            ScrollArea::both()
                .id_salt("scroll_direction_area")
                .animated(false)
                .show(ui, |ui| {
                    // Reserve the layout space, then create the widget over it
                    // with an explicit id (the one the action requests target):
                    let (widget_rect, _) =
                        ui.allocate_exact_size(vec2(800.0, 40.0), egui::Sense::click());
                    let response = ui.interact(widget_rect, target_id(), egui::Sense::click());
                    rect = Some(response.rect);
                });
        });
    });
    output.textures_delta.clear(); // these tests never render
    rect.expect("the scroll target widget has a rect")
}

/// Drives the two horizontal actions in the order that makes sense for the
/// current reading direction (content is anchored at the reading start, so the
/// reading-end side has content to reveal first) and asserts each action
/// reveals what its name names.
fn assert_scroll_actions_scroll_in_the_named_direction(
    ctx: &Context,
    first: accesskit::Action,
    second: accesskit::Action,
    first_moves_content: f32,
) {
    let initial = run_frame(ctx, vec![], 1.0);

    // The scroll offset a dispatch computes applies from the NEXT pass (the
    // scroll area consumes the delta after its content was laid out), so each
    // action is followed by a quiet frame whose rect is asserted:
    run_frame(ctx, vec![scroll_event(first)], 2.0);
    let after_first = run_frame(ctx, vec![], 3.0);
    assert!(
        (after_first.min.x - initial.min.x) * first_moves_content > 50.0,
        "{first:?} must reveal the content it names (the content moves          {}): {initial:?} -> {after_first:?}",
        if first_moves_content > 0.0 {
            "right"
        } else {
            "left"
        }
    );

    run_frame(ctx, vec![scroll_event(second)], 4.0);
    let after_second = run_frame(ctx, vec![], 5.0);
    assert!(
        (after_second.min.x - after_first.min.x) * -first_moves_content > 50.0,
        "{second:?} must reveal the content it names (the content moves          {}): {after_first:?} -> {after_second:?}",
        if first_moves_content > 0.0 {
            "left"
        } else {
            "right"
        }
    );
}

#[test]
fn accesskit_scroll_actions_scroll_in_the_named_direction_in_ltr() {
    let ctx = Context::default();
    // LTR anchors the content at the left: scrolling right reveals content
    // first (the content moves left), then scrolling left moves it back:
    assert_scroll_actions_scroll_in_the_named_direction(
        &ctx,
        accesskit::Action::ScrollRight,
        accesskit::Action::ScrollLeft,
        -1.0,
    );
}

#[test]
fn accesskit_scroll_actions_scroll_in_the_named_direction_in_rtl() {
    // Screen-space actions: "left" is left regardless of the reading
    // direction, and the content-relative delta keeps the same sign (the RTL
    // axis handling lives in the scroll area's consumer). RTL anchors the
    // content at the right: scrolling left reveals content first (the content
    // moves right), then scrolling right moves it back:
    let ctx = Context::default();
    ctx.set_right_to_left(true);
    assert_scroll_actions_scroll_in_the_named_direction(
        &ctx,
        accesskit::Action::ScrollLeft,
        accesskit::Action::ScrollRight,
        1.0,
    );
}
