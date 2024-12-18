use egui_kittest::Harness;

#[test]
fn test_shrink() {
    let mut harness = Harness::new_ui(|ui| {
        ui.label("Hello, world!");
        ui.separator();
        ui.label("This is a test");
    });

    harness.fit_contents();

    #[cfg(all(feature = "snapshot", feature = "wgpu"))]
    harness.wgpu_snapshot("test_shrink");
}
