use egui::accesskit::Role;
use egui_demo_app::{Anchor, WrapApp};
use egui_kittest::kittest::Queryable;

#[test]
fn test_demo_app() {
    let mut harness = egui_kittest::Harness::builder().build_eframe(|cc| WrapApp::new(cc));

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
            .find(|(_, anchor)| matches!(anchor, Anchor::Custom3d))
            .is_some(),
        "Expected to find the Custom3d app.",
    );

    let mut results = vec![];

    for (name, anchor) in apps {
        // The widget gallery demo shows the current date, so we can't use it for snapshot testing
        if matches!(anchor, Anchor::Demo) {
            continue;
        }

        harness.get_by_role_and_label(Role::Button, name).click();

        harness.run();

        harness
            .try_wgpu_snapshot(&anchor.to_string())
            .err()
            .map(|e| {
                results.push(e);
            });
    }

    for error in results {
        panic!("{error}");
    }
}
