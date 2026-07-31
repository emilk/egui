//! Snapshot tests for `Panel`'s drag-to-close and drag-to-expand gestures.
//!
//! Covers:
//! * [`Panel::show_collapsible`] — drag-to-close on a `Left` panel.
//! * [`Panel::show_collapsible`] — drag-to-open via the grab handle a fully
//!   collapsed panel leaves behind, plus [`Panel::drag_to_open`] opting out of it.
//! * [`Panel::show_switched`] — drag-to-close on the expanded panel
//!   followed by drag-to-expand on the collapsed panel, both via the shared
//!   resize handle.

use egui::{Panel, Pos2, Vec2};
use egui_kittest::{Harness, SnapshotResults};

/// Pure-data state for the kittest UI closure.
#[derive(Default)]
struct State {
    is_expanded: bool,

    /// The panel's live _outer_ width, recorded each pass.
    ///
    /// `None` while the panel is fully collapsed.
    ///
    /// We can't read this back from [`egui::PanelState`], because a panel
    /// deliberately doesn't persist its size while its resize handle is being
    /// dragged — which is exactly when these tests need to observe it.
    panel_width: Option<f32>,
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
                    .show_collapsible(ui, &mut state.is_expanded, |ui| {
                        ui.label("Left panel content");
                    });
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.label("Central");
                });
            },
            State {
                is_expanded: true,
                ..Default::default()
            },
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

/// The size range of the collapsible left panel used by the drag-to-open tests.
const MIN_SIZE: f32 = 60.0;
const DEFAULT_SIZE: f32 = 80.0;

/// A harness with a single collapsible, resizable left panel.
///
/// `drag_to_open` is passed straight through to [`Panel::drag_to_open`].
fn collapsible_left_panel_harness(drag_to_open: bool) -> Harness<'static, State> {
    Harness::builder()
        .with_size(Vec2::new(400.0, 200.0))
        .build_ui_state(
            move |ui, state: &mut State| {
                let response = Panel::left("test_left_panel")
                    .resizable(true)
                    .drag_to_open(drag_to_open)
                    .default_size(DEFAULT_SIZE)
                    .min_size(MIN_SIZE)
                    .show_collapsible(ui, &mut state.is_expanded, |ui| {
                        ui.label("Left panel content");
                        // Without this the frame shrinks to fit the label, and the
                        // panel's rect would report the content width instead of
                        // the width the panel was resized to.
                        ui.take_available_space();
                    });
                state.panel_width = response.map(|response| response.response.rect.width());
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.label("Central");
                });
            },
            State {
                is_expanded: true,
                panel_width: None,
            },
        )
}

/// The panel's live _outer_ width, as of the last completed pass.
fn panel_width(harness: &Harness<'_, State>) -> f32 {
    harness
        .state()
        .panel_width
        .expect("the panel should be showing")
}

/// Collapse the panel by dragging its resize edge past `min_size`, and return the
/// panel's fixed (left) edge — where the grab handle it leaves behind sits.
fn collapse_by_drag(harness: &mut Harness<'_, State>) -> Pos2 {
    harness.run();
    assert!(harness.state().is_expanded, "should start expanded");

    // Query the actual resize edge from PanelState (avoids assumptions about
    // Frame margins and the harness's ui padding).
    let panel_state = egui::PanelState::load(&harness.ctx, egui::Id::new("test_left_panel"))
        .expect("PanelState should be persisted after the first frame");
    let fixed_edge = Pos2::new(
        panel_state.outer_rect.left(),
        panel_state.outer_rect.center().y,
    );

    let drag_start = Pos2::new(panel_state.outer_rect.right(), fixed_edge.y);
    let drag_end = Pos2::new(drag_start.x - 200.0, fixed_edge.y);

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

    // Move the pointer away so the handle isn't left hovered.
    harness.hover_at(Pos2::new(300.0, fixed_edge.y));
    harness.run();

    fixed_edge
}

#[test]
fn drag_to_open_collapsed_panel() {
    let mut results = SnapshotResults::new();

    let mut harness = collapsible_left_panel_harness(true);
    let fixed_edge = collapse_by_drag(&mut harness);
    // Grab just inside the fixed edge, where the handle is.
    let handle_pos = fixed_edge + Vec2::new(1.0, 0.0);

    // The handle is invisible until hovered:
    results.add(harness.try_snapshot("panel_drag/collapsed_handle_idle"));

    harness.hover_at(handle_pos);
    harness.run();
    results.add(harness.try_snapshot("panel_drag/collapsed_handle_hovered"));

    // Dragging out but not as far as `min_size` must not reopen the panel.
    harness.drag_at(handle_pos);
    harness.run();
    let short_of_min = Pos2::new(fixed_edge.x + MIN_SIZE - 10.0, fixed_edge.y);
    harness.hover_at(short_of_min);
    harness.run();
    assert!(
        !harness.state().is_expanded,
        "dragging out less than min_size should not reopen the panel"
    );

    // …but continuing past `min_size` should, without releasing the drag. The
    // panel opens at the size the pointer is already at, so it never jumps ahead.
    let past_min = Pos2::new(fixed_edge.x + MIN_SIZE + 20.0, fixed_edge.y);
    harness.hover_at(past_min);
    harness.run();
    assert!(
        harness.state().is_expanded,
        "dragging out past min_size should have reopened the panel"
    );
    assert_eq!(
        panel_width(&harness),
        past_min.x - fixed_edge.x,
        "the reopened panel's edge should sit under the pointer"
    );

    harness.drop_at(past_min);
    harness.run();
    assert!(
        harness.state().is_expanded,
        "the panel should stay open after the drag is released"
    );
    results.add(harness.try_snapshot("panel_drag/collapsed_handle_reopened"));
}

#[test]
fn drag_to_open_can_be_opted_out_of() {
    let mut harness = collapsible_left_panel_harness(false);
    let handle_pos = collapse_by_drag(&mut harness) + Vec2::new(1.0, 0.0);

    harness.drag_at(handle_pos);
    harness.run();
    harness.hover_at(Pos2::new(handle_pos.x + 150.0, handle_pos.y));
    harness.run();
    harness.drop_at(Pos2::new(handle_pos.x + 150.0, handle_pos.y));
    harness.run();

    assert!(
        !harness.state().is_expanded,
        "with `drag_to_open(false)` there should be no grab handle to reopen the panel with"
    );
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
                Panel::show_switched(
                    ui,
                    &mut state.is_expanded,
                    collapsed,
                    expanded,
                    |ui, expanded| {
                        if expanded {
                            ui.heading("Expanded panel");
                            ui.separator();
                            for i in 0..6 {
                                ui.label(format!(
                                    "Row {i}: filler content so the \
                                    expanded panel is clearly taller than the \
                                    collapsed one in the snapshot."
                                ));
                            }
                        } else {
                            ui.label("Collapsed");
                        }
                    },
                );
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.label("Central");
                });
            },
            State {
                is_expanded: true,
                ..Default::default()
            },
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

/// State for the animated-close test: records the panel's live top edge.
#[derive(Default)]
struct SwitchedState {
    is_expanded: bool,

    /// Bottom of whatever space is left after the panel — i.e. the top edge of
    /// the panel that is currently showing.
    ///
    /// Read from the ui rather than [`egui::PanelState`], which a panel doesn't
    /// persist while its resize handle is held.
    panel_top: f32,
}

/// The sizes a `show_switched` bottom panel moves between in these tests.
///
/// The expanded minimum sits well above the collapsed size, so the gap between
/// the two shows up in the panel's edge.
const SWITCHED_COLLAPSED_SIZE: f32 = 20.0;
const SWITCHED_EXPANDED_MIN: f32 = 80.0;

fn switched_bottom_panel_harness(start_expanded: bool) -> Harness<'static, SwitchedState> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(400.0, 300.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            move |ui, state: &mut SwitchedState| {
                Panel::show_switched(
                    ui,
                    &mut state.is_expanded,
                    Panel::bottom("switched_collapsed")
                        .resizable(true)
                        .exact_size(SWITCHED_COLLAPSED_SIZE),
                    Panel::bottom("switched_expanded")
                        .resizable(true)
                        .default_size(160.0)
                        .min_size(SWITCHED_EXPANDED_MIN)
                        .max_size(250.0),
                    |ui, _expanded| ui.take_available_space(),
                );
                state.panel_top = ui.available_rect_before_wrap().bottom();
                egui::CentralPanel::default().show(ui, |_ui| {});
            },
            SwitchedState {
                is_expanded: start_expanded,
                ..Default::default()
            },
        );
    // kittest disables animations by default, and these tests are about one.
    harness
        .ctx
        .all_styles_mut(|style| style.animation_time = 0.25);
    for _ in 0..4 {
        harness.step();
    }
    harness
}

/// Assert that the panel edge crossed `gap` gradually, rather than in one frame.
fn assert_crossed_gradually(tops: &[f32], gap: std::ops::Range<f32>) {
    let frames_in_gap = tops.iter().filter(|top| gap.contains(top)).count();
    assert!(
        3 <= frames_in_gap,
        "expected the panel to be animated across the gap between the collapsed \
         size and the expanded min_size, but only {frames_in_gap} frame(s) landed \
         inside {gap:?}: {tops:?}"
    );
}

/// Dragging the expanded panel shut animates it the rest of the way, rather than
/// snapping, even while the drag is still held.
///
/// The expanded panel can't shrink past its own `min_size`, so a drag that goes
/// below that leaves a gap between where the panel is stuck and the collapsed
/// panel's size. That gap has to be animated, or the panel jumps.
#[test]
fn drag_to_close_switched_animates_while_held() {
    let collapsed_size = SWITCHED_COLLAPSED_SIZE;
    let expanded_min = SWITCHED_EXPANDED_MIN;

    let mut harness = switched_bottom_panel_harness(true);

    let expanded = egui::PanelState::load(&harness.ctx, egui::Id::new("switched_expanded"))
        .expect("PanelState should be persisted after the first frame");
    let (x, bottom) = (expanded.outer_rect.center().x, expanded.outer_rect.bottom());
    let collapsed_top = bottom - collapsed_size;

    // Drag the top edge down well past the collapsed size, and keep holding.
    harness.drag_at(Pos2::new(x, expanded.outer_rect.top()));
    harness.step();
    harness.hover_at(Pos2::new(x, bottom - 10.0));
    harness.step();

    assert!(
        !harness.state().is_expanded,
        "dragging past the collapsed size should have collapsed the panel"
    );
    let top_at_collapse = harness.state().panel_top;
    assert_eq!(
        top_at_collapse,
        bottom - expanded_min,
        "the expanded panel should be stuck at its min_size when the collapse fires"
    );

    // Follow the close, still holding the drag.
    let mut tops = vec![top_at_collapse];
    for _ in 0..40 {
        harness.step();
        tops.push(harness.state().panel_top);
    }

    assert!(
        tops.windows(2).all(|w| w[0] <= w[1]),
        "the panel should only ever move towards being shut, never jump back open: {tops:?}"
    );
    assert!(
        (tops.last().copied().unwrap_or_default() - collapsed_top).abs() < 1.0,
        "the close should end at the collapsed panel's size, got {:?}",
        tops.last()
    );

    // The gap between min_size and the collapsed size must be crossed over
    // several frames, not in one jump.
    assert_crossed_gradually(&tops, (top_at_collapse + 1.0)..(collapsed_top - 1.0));
}

/// The mirror image: dragging the collapsed panel open animates across the same
/// gap, instead of snapping straight out to the expanded panel's `min_size`.
#[test]
fn drag_to_open_switched_animates_while_held() {
    let mut harness = switched_bottom_panel_harness(false);

    let collapsed = egui::PanelState::load(&harness.ctx, egui::Id::new("switched_collapsed"))
        .expect("PanelState should be persisted after the first frame");
    let (x, collapsed_top, bottom) = (
        collapsed.outer_rect.center().x,
        collapsed.outer_rect.top(),
        collapsed.outer_rect.bottom(),
    );
    let expanded_min_top = bottom - SWITCHED_EXPANDED_MIN;

    // Nudge the collapsed panel's top edge out past its `exact_size` cap, and keep
    // holding. The pointer stays far short of the expanded panel's `min_size`.
    harness.drag_at(Pos2::new(x, collapsed_top));
    harness.step();
    harness.hover_at(Pos2::new(x, collapsed_top - 10.0));
    harness.step();

    assert!(
        harness.state().is_expanded,
        "a small outward drag past the collapsed panel's cap should expand it"
    );

    let mut tops = vec![harness.state().panel_top];
    for _ in 0..40 {
        harness.step();
        tops.push(harness.state().panel_top);
    }

    assert!(
        tops.windows(2).all(|w| w[1] <= w[0]),
        "the panel should only ever grow, never jump back shut: {tops:?}"
    );
    assert!(
        (tops.last().copied().unwrap_or_default() - expanded_min_top).abs() < 1.0,
        "the panel should settle at the expanded min_size (top {expanded_min_top}), \
         since the pointer never got further out than that, got {:?}",
        tops.last()
    );
    assert_crossed_gradually(&tops, (expanded_min_top + 1.0)..(collapsed_top - 1.0));
}
