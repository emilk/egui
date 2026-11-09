//! [`egui::Ui::scroll_with_delta`] scrolls the [`ScrollArea`] enclosing the `Ui` it is called on,
//! not whichever scroll area happens to end next in the frame.

use std::sync::Arc;

use egui::mutex::Mutex;
use egui::scroll_area::State;
use egui::{ScrollArea, Vec2, vec2};
use egui_kittest::Harness;

type SharedState = Arc<Mutex<Option<State>>>;

fn offset(state: &SharedState) -> Vec2 {
    state
        .lock()
        .as_ref()
        .expect("ScrollArea should have been shown")
        .clamped_offset()
}

#[test]
fn scroll_with_delta_before_a_nested_scroll_area_scrolls_the_outer_one() {
    let outer: SharedState = Default::default();
    let inner: SharedState = Default::default();
    let (o, i) = (Arc::clone(&outer), Arc::clone(&inner));

    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 100.0))
        .build_ui(move |ui| {
            let output = ScrollArea::vertical()
                .id_salt("outer")
                .auto_shrink(false)
                .show(ui, |ui| {
                    // Asked for in the outer area, before the inner one begins:
                    ui.scroll_with_delta(vec2(0.0, -50.0));

                    let output = ScrollArea::vertical()
                        .id_salt("inner")
                        .max_height(40.0)
                        .show(ui, |ui| {
                            for row in 0..100 {
                                ui.label(format!("Inner {row}"));
                            }
                        });
                    *i.lock() = Some(output.state);

                    for row in 0..100 {
                        ui.label(format!("Outer {row}"));
                    }
                });
            *o.lock() = Some(output.state);
        });

    harness.run_steps(3);

    assert!(
        offset(&outer).y > 0.0,
        "The outer scroll area should have scrolled, got {}",
        offset(&outer).y
    );
    assert_eq!(
        offset(&inner).y,
        0.0,
        "The nested scroll area should be untouched"
    );
}

#[test]
fn scroll_with_delta_inside_a_nested_scroll_area_scrolls_the_inner_one() {
    let outer: SharedState = Default::default();
    let inner: SharedState = Default::default();
    let (o, i) = (Arc::clone(&outer), Arc::clone(&inner));

    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 100.0))
        .build_ui(move |ui| {
            let output = ScrollArea::vertical()
                .id_salt("outer")
                .auto_shrink(false)
                .show(ui, |ui| {
                    let output = ScrollArea::vertical()
                        .id_salt("inner")
                        .max_height(40.0)
                        .show(ui, |ui| {
                            // Asked for inside the inner area:
                            ui.scroll_with_delta(vec2(0.0, -50.0));
                            for row in 0..100 {
                                ui.label(format!("Inner {row}"));
                            }
                        });
                    *i.lock() = Some(output.state);

                    for row in 0..100 {
                        ui.label(format!("Outer {row}"));
                    }
                });
            *o.lock() = Some(output.state);
        });

    harness.run_steps(3);

    assert!(
        offset(&inner).y > 0.0,
        "The inner scroll area should have scrolled, got {}",
        offset(&inner).y
    );
    assert_eq!(
        offset(&outer).y,
        0.0,
        "The outer scroll area should be untouched"
    );
}

#[test]
fn no_repaint_is_requested_when_there_is_nothing_left_to_scroll() {
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);

    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 100.0))
        .build_ui(move |ui| {
            let output = ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                // Keep asking to scroll down, every frame:
                ui.scroll_with_delta(vec2(0.0, -50.0));
                for row in 0..20 {
                    ui.label(format!("Row {row}"));
                }
            });
            *s.lock() = Some(output.state);
        });

    // Long enough to reach the end of the content:
    harness.run_steps(60);
    let at_end = offset(&state).y;
    harness.run_steps(2);
    assert_eq!(offset(&state).y, at_end, "Should be at the end");
    assert!(
        !harness.ctx.has_requested_repaint(),
        "Nothing moves, so nothing should request a repaint"
    );
}

#[test]
fn a_small_animated_scroll_far_down_reaches_its_target() {
    // At large offsets, an f32 lerp step can round to nothing on the first frames.
    let start = 300_000.0;
    let delta = 1.5;
    let state: SharedState = Default::default();
    let s = Arc::clone(&state);
    let frame = Arc::new(Mutex::new(0_usize));
    let f = Arc::clone(&frame);

    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 100.0))
        .with_step_dt(1.0 / 240.0)
        .build_ui(move |ui| {
            let frame = {
                let mut frame = f.lock();
                *frame += 1;
                *frame
            };
            let mut scroll_area = ScrollArea::vertical().auto_shrink(false);
            if frame == 1 {
                scroll_area = scroll_area.vertical_scroll_offset(start);
            }
            let output = scroll_area.show(ui, |ui| {
                if frame == 4 {
                    ui.scroll_with_delta_animation(
                        vec2(0.0, -delta),
                        egui::style::ScrollAnimation::duration(0.3),
                    );
                }
                ui.allocate_space(vec2(100.0, 1.0e7));
            });
            *s.lock() = Some(output.state);
        });

    harness.run_steps(3);
    // Like a reactive integration: only step again while a repaint is requested.
    for _ in 0..500 {
        harness.step();
        if !harness.ctx.has_requested_repaint() {
            break;
        }
    }
    assert_eq!(offset(&state).y, start + delta);
}
