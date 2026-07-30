//! Tests for [`egui::Context::run_hosted_viewport`].

use egui::{Context, Event, Pos2, RawInput, Rect, Sense, ViewportClass, ViewportId, pos2, vec2};

const CHILD_SIZE: egui::Vec2 = vec2(100.0, 100.0);

fn input(size: egui::Vec2, events: Vec<Event>) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        events,
        ..Default::default()
    }
}

/// The child viewport must keep its state (input, hit-test data, focus) between passes.
///
/// Hovering only works if the previous pass' widget rects survived, so a widget that
/// reports `hovered()` on the second pass proves the state was not thrown away.
#[test]
fn hosted_viewport_keeps_its_state_between_passes() {
    let ctx = Context::default();
    let child_id = ViewportId::from_hash_of("child");

    let mut hovered = Vec::new();

    for _ in 0..3 {
        let parent_output = ctx.run_ui(input(vec2(300.0, 300.0), vec![]), |_ui| {
            let (child_output, ()) = ctx.run_hosted_viewport(
                child_id,
                input(CHILD_SIZE, vec![Event::PointerMoved(pos2(10.0, 10.0))]),
                |ui| {
                    let response = ui.allocate_response(vec2(50.0, 50.0), Sense::click());
                    hovered.push(response.hovered());
                },
            );
            child_output.drop_without_applying_deltas();
        });
        parent_output.drop_without_applying_deltas();
    }

    assert_eq!(
        hovered,
        vec![false, true, true],
        "the hosted viewport lost its state between passes"
    );
}

/// A hosted viewport is the application's business, so the integration must never see it -
/// otherwise a backend like eframe would open a window for it.
#[test]
fn hosted_viewport_is_hidden_from_the_integration() {
    let ctx = Context::default();
    let child_id = ViewportId::from_hash_of("child");

    let parent_output = ctx.run_ui(input(vec2(300.0, 300.0), vec![]), |_ui| {
        let (child_output, ()) =
            ctx.run_hosted_viewport(child_id, input(CHILD_SIZE, vec![]), |ui| {
                ui.label("hello");
            });
        child_output.drop_without_applying_deltas();
    });

    assert!(
        !parent_output.viewport_output.contains_key(&child_id),
        "a hosted viewport must be left out of FullOutput::viewport_output"
    );
    assert_eq!(
        ctx.viewport_for(child_id, |viewport| viewport.class),
        ViewportClass::Hosted
    );

    parent_output.drop_without_applying_deltas();
}

/// The child gets its own paint list; its shapes must not leak into the parent's.
#[test]
fn hosted_viewport_shapes_are_separate() {
    let ctx = Context::default();
    let child_id = ViewportId::from_hash_of("child");

    let parent_output = ctx.run_ui(input(vec2(300.0, 300.0), vec![]), |ui| {
        ui.label("parent");
        let (child_output, ()) =
            ctx.run_hosted_viewport(child_id, input(CHILD_SIZE, vec![]), |ui| {
                ui.label("child");
            });
        assert!(
            !child_output.shapes.is_empty(),
            "the child should have painted something"
        );
        child_output.drop_without_applying_deltas();
    });

    assert!(!parent_output.shapes.is_empty());
    parent_output.drop_without_applying_deltas();
}
