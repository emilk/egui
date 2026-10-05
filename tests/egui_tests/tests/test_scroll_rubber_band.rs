//! Rubber-banding past the edge of a [`ScrollArea`] when drag-scrolling.
//!
//! The rules and the maths are unit tested in `egui::containers::scroll_physics`.
//! This checks that they are wired up to the actual input and `ScrollArea`.

use std::sync::Arc;

use egui::mutex::Mutex;
use egui::scroll_area::{DragScroll, ScrollSource, State};
use egui::{Pos2, ScrollArea, Vec2, pos2};
use egui_kittest::Harness;

/// A vertically scrollable area that fills the window,
/// returning the latest [`State`] of the scroll area after each frame.
fn harness() -> (Harness<'static>, Arc<Mutex<Option<State>>>) {
    let state = Arc::new(Mutex::new(None));
    let state_clone = Arc::clone(&state);

    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 200.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui(move |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            // Selectable labels would steal the drag from the scroll area:
            ui.style_mut().interaction.selectable_labels = false;
            let output = ScrollArea::vertical()
                .auto_shrink(false) // Make the whole viewport draggable, not just the content width.
                .scroll_source(ScrollSource {
                    drag: DragScroll::Always,
                    ..ScrollSource::NONE
                })
                .show(ui, |ui| {
                    for i in 0..100 {
                        ui.label(format!("Row {i}"));
                    }
                });
            *state_clone.lock() = Some(output.state);
        });

    harness.run_steps(2); // Let the scroll area measure its content.
    (harness, state)
}

fn state_of(state: &Arc<Mutex<Option<State>>>) -> State {
    state.lock().expect("ScrollArea should have been shown")
}

/// Press at `from`, move the pointer in steps to `to`, stepping the harness between each move.
fn drag(harness: &mut Harness<'_>, from: Pos2, to: Pos2, steps: usize) {
    harness.drag_at(from);
    harness.step();
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        harness.hover_at(from.lerp(to, t));
        harness.step();
    }
}

#[test]
fn dragging_past_the_top_rubber_bands_and_springs_back() {
    let (mut harness, state) = harness();
    assert_eq!(state_of(&state).unclamped_offset(), Vec2::ZERO);

    // Drag downwards from the top, i.e. past the start of the content:
    let drag_distance = 100.0;
    drag(
        &mut harness,
        pos2(100.0, 50.0),
        pos2(100.0, 50.0 + drag_distance),
        10,
    );

    let dragged = state_of(&state);
    assert!(
        dragged.unclamped_offset().y < 0.0,
        "Should be overscrolled past the top, got {}",
        dragged.unclamped_offset().y
    );
    assert!(
        -drag_distance < dragged.unclamped_offset().y,
        "Overscroll should be resisted, got {}",
        dragged.unclamped_offset().y
    );
    assert_eq!(dragged.clamped_offset(), Vec2::ZERO);
    assert_eq!(dragged.overscroll(), dragged.unclamped_offset());

    // Hold still so the pointer velocity decays to zero, then let go:
    harness.run_steps(12);
    harness.drop_at(pos2(100.0, 50.0 + drag_distance));
    harness.step();
    harness.step();
    let releasing = state_of(&state);
    assert!(
        dragged.unclamped_offset().y < releasing.unclamped_offset().y
            && releasing.unclamped_offset().y < 0.0,
        "Should be springing back towards zero, got {} -> {}",
        dragged.unclamped_offset().y,
        releasing.unclamped_offset().y
    );

    // After a while we should be back at exactly zero, and at rest:
    harness.run_steps(60);
    let settled = state_of(&state);
    assert_eq!(settled.unclamped_offset(), Vec2::ZERO);
    assert_eq!(settled.velocity(), Vec2::ZERO);
}
