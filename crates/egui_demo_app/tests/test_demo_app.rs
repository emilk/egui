use egui::Vec2;
use egui::accesskit::Role;
use egui_demo_app::{Anchor, WrapApp};
use egui_kittest::SnapshotResults;
use egui_kittest::kittest::Queryable as _;

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
                harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);

                harness
                    .get_by_role_and_label(Role::TextInput, "URI:")
                    .type_text("file://../eframe/data/icon.png");

                harness.get_by_role_and_label(Role::Button, "✔️").click();

                // Wait for the image to load
                harness.try_run_realtime().ok();
            }
            _ => {}
        }

        // Can't use Harness::run because fractal clock keeps requesting repaints
        harness.run_steps(4);

        results.add(harness.try_snapshot(anchor.to_string()));
    }
}

/// The demo app runs with Arabic selected. Driving the UI by English kittest labels (as
/// `test_demo_app` does) requires the language to be `en`, so this test only checks that a few
/// frames render without panicking; it adds no snapshot, because Arabic glyph shapes are
/// system-font dependent.
#[test]
fn test_demo_app_in_arabic() {
    let mut harness = egui_kittest::Harness::builder()
        .with_size(Vec2::new(900.0, 600.0))
        .wgpu()
        // Arabic glyphs come from installed system fonts, which the test machine may not
        // have; tofu boxes are fine here, a panic is not.
        .allow_missing_glyphs()
        .build_eframe(|cc| {
            let mut app = WrapApp::new(cc);
            app.state.language = "ar".to_owned();
            app
        });

    harness.run_steps(5);
}

/// Picking Arabic turns the interface right-to-left; picking an LTR language turns it back.
#[test]
fn test_language_flips_interface_direction() {
    let mut harness = egui_kittest::Harness::builder()
        .with_size(Vec2::new(900.0, 600.0))
        .wgpu()
        .allow_missing_glyphs()
        .build_eframe(|cc| WrapApp::new(cc));

    harness.run_steps(2);
    assert!(!harness.ctx.is_rtl());

    harness.state_mut().state.language = "ar".to_owned();
    harness.run_steps(2);
    assert!(harness.ctx.is_rtl());

    harness.state_mut().state.language = "en".to_owned();
    harness.run_steps(2);
    assert!(!harness.ctx.is_rtl());
}
