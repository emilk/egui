use egui::{Color32, Frame, Id, Stroke, Vec2};
use egui_kittest::{Harness, SnapshotResults, kittest::Queryable as _};

/// Shows how [`egui::Ui::dnd_drop_zone`] paints its frame:
///
/// * An unstyled frame (transparent fill, no stroke) falls back to the inactive widget style.
/// * A styled frame keeps its own fill and stroke while idle.
/// * During a drag, the zone under the pointer is highlighted and the others use the inactive style.
#[test]
fn drop_zone_styling() {
    let styled_frame = Frame::NONE
        .inner_margin(8)
        .fill(Color32::from_rgb(40, 60, 110))
        .stroke(Stroke::new(2.0, Color32::from_rgb(240, 170, 40)))
        .corner_radius(8);

    let mut harness = Harness::builder().build_ui(|ui| {
        ui.dnd_drag_source(Id::unique("drag_source"), 42_u32, |ui| {
            ui.label("Drag me");
        });
        ui.horizontal(|ui| {
            ui.dnd_drop_zone::<u32, _>(Frame::default().inner_margin(4.0), |ui| {
                ui.set_min_size(Vec2::new(100.0, 50.0));
                ui.label("Unstyled frame");
            });
            ui.dnd_drop_zone::<u32, _>(styled_frame, |ui| {
                ui.set_min_size(Vec2::new(100.0, 50.0));
                ui.label("Styled frame");
            });
        });
    });

    let mut results = SnapshotResults::new();

    harness.fit_contents();
    harness.run();
    results.add(harness.try_snapshot("drop_zone_styling_idle"));

    // Grab the drag source at its left edge, and hover the upper part of the styled zone,
    // so the dragged label doesn't cover the zone's own label:
    let source = harness.get_by_label("Drag me").rect();
    let from = source.left_center() + Vec2::new(2.0, 0.0);
    let zone_label = harness.get_by_label("Styled frame").rect();
    let to = zone_label.center_top() - Vec2::new(0.0, 17.0);
    harness.drag_at(from);
    harness.run_steps(2);
    harness.hover_at(to);
    harness.run_steps(2);
    results.add(harness.try_snapshot("drop_zone_styling_dragging"));

    harness.drop_at(to);
    harness.run();
}
