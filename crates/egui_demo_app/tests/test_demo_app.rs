use egui::accesskit::Role;
use egui::Vec2;
use egui_demo_app::{Anchor, WrapApp};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::SnapshotResults;

#[test]
fn test_demo_app() {
    let mut harness = egui_kittest::Harness::builder()
        .with_size(Vec2::new(900.0, 600.0))
        .wgpu()
        .build_eframe(|cc| WrapApp::new(cc));

    let app = harness.state_mut();

    // Mock the fractal clock time so snapshots are consistent.
    app.state.clock.mock_time = Some(36383.0);

    let apps = app
        .apps_iter_mut()
        .map(|(name, anchor, _)| (name, anchor))
        .collect::<Vec<_>>();

    #[cfg(feature = "wgpu")]
    assert!(
        apps.iter()
            .any(|(_, anchor)| matches!(anchor, Anchor::Custom3d)),
        "Expected to find the Custom3d app.",
    );

    let mut results = SnapshotResults::new();

    for (name, anchor) in apps {
        harness.get_by_role_and_label(Role::Button, name).click();

        match anchor {
            // The widget gallery demo shows the current date, so we can't use it for snapshot testing
            Anchor::Demo => {
                continue;
            }
            // This is already tested extensively elsewhere
            Anchor::Rendering => {
                continue;
            }
            // We don't want to rely on a network connection for tests
            #[cfg(feature = "http")]
            Anchor::Http => {
                continue;
            }
            // Load a local image where we know it exists and loads quickly
            #[cfg(feature = "image_viewer")]
            Anchor::ImageViewer => {
                harness.step();

                harness
                    .get_by_role_and_label(Role::TextInput, "URI:")
                    .focus();
                harness.press_key_modifiers(egui::Modifiers::COMMAND, egui::Key::A);

                harness
                    .get_by_role_and_label(Role::TextInput, "URI:")
                    .type_text("file://../eframe/data/icon.png");

                harness.get_by_role_and_label(Role::Button, "✔").click();
            }
            _ => {}
        }

        // Can't use Harness::run because fractal clock keeps requesting repaints
        harness.run_steps(4);

        results.add(harness.try_snapshot(&anchor.to_string()));
    }
}
