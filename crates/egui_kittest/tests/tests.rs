use egui_kittest::{Harness, SnapshotResults};

#[test]
fn test_shrink() {
    let mut harness = Harness::new_ui(|ui| {
        ui.label("Hello, world!");
        ui.separator();
        ui.label("This is a test");
    });

    harness.fit_contents();

    let mut results = SnapshotResults::new();

    #[cfg(all(feature = "snapshot", feature = "wgpu"))]
    results.add(harness.try_snapshot("test_shrink"));
}
