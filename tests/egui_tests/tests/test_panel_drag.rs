//! Snapshot tests for `Panel`'s drag-to-close and drag-to-expand gestures.
//!
//! Covers:
//! * [`Panel::show_animated_inside`] — drag-to-close on a `Left` panel.
//! * [`Panel::show_animated_between_inside`] — drag-to-close on the expanded panel
//!   followed by drag-to-expand on the collapsed panel, both via the shared
//!   resize handle.

use egui::{Panel, Pos2, Vec2};
use egui_kittest::{Harness, SnapshotResults};

/// Pure-data state for the kittest UI closure.
#[derive(Default)]
struct State {
    is_expanded: bool,
}

#[test]
fn drag_to_close_animated_inside() {
    let mut results = SnapshotResults::new();

    let mut harness = Harness::builder()
        .with_size(Vec2::new(400.0, 200.0))
        .build_ui_state(
            |ui, state: &mut State| {
                Panel::left("test_left_panel")
                    .resizable(true)
                    .default_size(120.0)
                    .min_size(60.0)
                    .show_animated_inside(ui, &mut state.is_expanded, |ui| {
                        ui.label("Left panel content");
                    });
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    ui.label("Central");
                });
            },
            State { is_expanded: true },
        );

    harness.run();
    assert!(harness.state().is_expanded, "should start expanded");
    results.add(harness.try_snapshot("panel_drag/inside_initial"));

    // Query the actual resize edge from PanelState (avoids assumptions about
    // Frame margins and the harness's ui padding).
    let panel_state = egui::PanelState::load(&harness.ctx, egui::Id::new("test_left_panel"))
        .expect("PanelState should be persisted after the first frame");
    let resize_x = panel_state.outer_rect.right();
    let resize_y = panel_state.outer_rect.center().y;

    let drag_start = Pos2::new(resize_x, resize_y);
    let drag_end = Pos2::new(resize_x - 200.0, resize_y);

    harness.drag_at(drag_start);
    harness.run();
    harness.hover_at(drag_end);
    harness.run();
    harness.drop_at(drag_end);
    harness.run();

    assert!(
        !harness.state().is_expanded,
        "drag past min_size should have closed the panel"
    );
    results.add(harness.try_snapshot("panel_drag/inside_closed"));
}

#[test]
fn drag_to_close_and_reopen_animated_between() {
    let mut results = SnapshotResults::new();

    let panel_size = 400.0_f32;
    let expanded_size = 120.0_f32;
    let collapsed_size = 28.0_f32;

    let mut harness = Harness::builder()
        .with_size(Vec2::new(panel_size, 300.0))
        .build_ui_state(
            |ui, state: &mut State| {
                let collapsed = Panel::bottom("between_collapsed")
                    .resizable(true)
                    .exact_size(collapsed_size);
                let expanded = Panel::bottom("between_expanded")
                    .resizable(true)
                    .default_size(expanded_size);
                Panel::show_animated_between_inside(
                    ui,
                    &mut state.is_expanded,
                    collapsed,
                    expanded,
                    |ui, expanded| {
                        if expanded {
                            ui.heading("Expanded panel");
                            ui.separator();
                            for i in 0..6 {
                                ui.label(format!("Row {i}: filler content so the \
                                    expanded panel is clearly taller than the \
                                    collapsed one in the snapshot."));
                            }
                        } else {
                            ui.label("Collapsed");
                        }
                    },
                );
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    ui.label("Central");
                });
            },
            State { is_expanded: true },
        );

    harness.run();
    assert!(harness.state().is_expanded, "should start expanded");
    results.add(harness.try_snapshot("panel_drag/between_initial_expanded"));

    // Drag-to-close: grab the top edge of the expanded bottom panel and drag
    // it down past the panel's minimum height to collapse.
    let expanded_state = egui::PanelState::load(&harness.ctx, egui::Id::new("between_expanded"))
        .expect("expanded PanelState should be persisted");
    let expanded_resize_y = expanded_state.outer_rect.top();
    let drag_x = expanded_state.outer_rect.center().x;
    let bottom_y = expanded_state.outer_rect.bottom();

    harness.drag_at(Pos2::new(drag_x, expanded_resize_y));
    harness.run();
    harness.hover_at(Pos2::new(drag_x, bottom_y - 1.0));
    harness.run();
    harness.drop_at(Pos2::new(drag_x, bottom_y - 1.0));
    harness.run();

    assert!(
        !harness.state().is_expanded,
        "drag past min should have closed the expanded panel"
    );
    results.add(harness.try_snapshot("panel_drag/between_collapsed"));

    // Drag-to-expand: grab the top edge of the (now visible) collapsed panel
    // and drag it upward past the collapsed panel's exact_size cap.
    let collapsed_state = egui::PanelState::load(&harness.ctx, egui::Id::new("between_collapsed"))
        .expect("collapsed PanelState should be persisted");
    let collapsed_resize_y = collapsed_state.outer_rect.top();

    harness.drag_at(Pos2::new(drag_x, collapsed_resize_y));
    harness.run();
    harness.hover_at(Pos2::new(drag_x, collapsed_resize_y - 200.0));
    harness.run();
    harness.drop_at(Pos2::new(drag_x, collapsed_resize_y - 200.0));
    harness.run();

    assert!(
        harness.state().is_expanded,
        "drag past collapsed exact_size should have reopened the panel"
    );
    results.add(harness.try_snapshot("panel_drag/between_reopened"));
}
