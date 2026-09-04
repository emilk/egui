#![cfg(all(feature = "recording", feature = "wgpu"))]

use egui_kittest::{Harness, RecordingOptions, RecordingPlugin};
use kittest::Queryable as _;
use tempfile::tempdir;

fn counter_harness(value: &mut u32) -> Harness<'_, &mut u32> {
    Harness::builder()
        .with_size(egui::Vec2::new(120.0, 60.0))
        .build_ui_state(
            |ui, state| {
                if ui.button(format!("count: {state}")).clicked() {
                    **state += 1;
                }
            },
            value,
        )
}

#[test]
fn records_an_mp4() {
    let dir = tempdir().expect("tempdir");
    let mp4_path = dir.path().join("counter.mp4");

    let mut value = 0;
    let mut harness = counter_harness(&mut value);
    harness.start_recording(RecordingOptions::mp4(&mp4_path, 12.0));

    harness.run();
    harness.get_by_label_contains("count").click();
    harness.run();

    let path = harness.finish_recording().expect("save mp4");

    assert_eq!(path, mp4_path);
    let size = std::fs::metadata(&path)
        .expect("the recording exists")
        .len();
    assert!(size > 0, "the recording should not be empty");
}

#[test]
fn finishing_without_recording_is_an_error() {
    let mut value = 0;
    let harness = counter_harness(&mut value);

    let err = harness.finish_recording().expect_err("not recording");
    assert!(matches!(
        err,
        egui_kittest::RecordingError::NotRecording | egui_kittest::RecordingError::NoFrames
    ));
}

#[test]
fn recording_without_frames_is_an_error() {
    let dir = tempdir().expect("tempdir");

    let mut value = 0;
    let harness = counter_harness(&mut value);
    harness.start_recording(RecordingOptions::mp4(dir.path().join("empty.mp4"), 10.0));

    let err = harness.finish_recording().expect_err("no frames");
    assert!(matches!(err, egui_kittest::RecordingError::NoFrames));
}

/// The recorder is a plain [`egui::Plugin`]: the harness only needs to support screenshot
/// callbacks and has no recording-specific integration.
#[test]
fn records_when_registered_directly_as_a_plugin() {
    let dir = tempdir().expect("tempdir");
    let mp4_path = dir.path().join("plain.mp4");

    let mut value = 0;
    let mut harness = counter_harness(&mut value);
    harness
        .ctx
        .add_plugin(RecordingPlugin::new(RecordingOptions::mp4(&mp4_path, 10.0)));

    // The final step must be captured immediately, without a follow-up pass.
    harness.run_steps(3);

    harness
        .ctx
        .with_plugin::<RecordingPlugin, _>(|plugin| plugin.finish().expect("finish mp4"))
        .expect("the plugin is registered");

    assert!(!harness.is_recording());
    assert!(mp4_path.exists(), "the MP4 should have been written");
}
