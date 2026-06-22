#![cfg(all(feature = "recording", feature = "wgpu"))]

use egui_kittest::{Harness, RecordingOptions, RecordingTrigger};
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

fn count_pngs(dir: &std::path::Path) -> usize {
    std::fs::read_dir(dir)
        .expect("png output dir exists")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("png"))
        .count()
}

#[test]
fn records_gif_with_diffing() {
    let dir = tempdir().expect("tempdir");
    let gif_path = dir.path().join("counter.gif");

    let mut value = 0u32;
    let mut harness = counter_harness(&mut value);
    harness.start_recording(RecordingOptions::gif(&gif_path, 12.0));

    harness.run();
    harness.run();
    harness.get_by_label_contains("count").click();
    harness.run();

    assert!(harness.is_recording());
    harness.finish_recording().expect("save gif");
    assert!(!harness.is_recording());

    let metadata = std::fs::metadata(&gif_path).expect("gif exists");
    assert!(metadata.len() > 0, "GIF should be non-empty");
}

#[test]
fn records_png_sequence() {
    let dir = tempdir().expect("tempdir");
    let out = dir.path().join("frames");

    let mut value = 0u32;
    let mut harness = counter_harness(&mut value);
    harness.start_recording(
        RecordingOptions::png_sequence(&out).with_trigger(RecordingTrigger::EveryStep),
    );

    harness.run();
    harness.get_by_label_contains("count").click();
    harness.run();

    harness.finish_recording().expect("save png sequence");

    assert!(count_pngs(&out) > 0, "expected at least one frame");
}

#[test]
fn diff_every_step_dedupes_unchanged_frames() {
    let dir = tempdir().expect("tempdir");
    let out = dir.path().join("frames");

    let mut value = 0u32;
    let mut harness = counter_harness(&mut value);
    harness.start_recording(
        RecordingOptions::png_sequence(&out).with_trigger(RecordingTrigger::DiffEveryStep),
    );

    for _ in 0..6 {
        harness.run();
    }
    harness.finish_recording().expect("save png sequence");

    assert_eq!(
        count_pngs(&out),
        1,
        "DiffEveryStep should dedupe unchanged frames"
    );
}

#[test]
fn on_run_trigger_captures_per_run_only() {
    let dir = tempdir().expect("tempdir");
    let out = dir.path().join("frames");

    let mut value = 0u32;
    let mut harness = counter_harness(&mut value);
    harness.start_recording(
        RecordingOptions::png_sequence(&out).with_trigger(RecordingTrigger::OnRun),
    );

    harness.run();
    harness.get_by_label_contains("count").click();
    harness.run();
    harness.run();

    harness.finish_recording().expect("save png sequence");

    assert_eq!(
        count_pngs(&out),
        3,
        "OnRun should produce one frame per run() call"
    );
}

#[test]
fn finish_recording_without_start_errors() {
    let mut value = 0u32;
    let mut harness = counter_harness(&mut value);
    let err = harness.finish_recording().expect_err("not recording");
    assert!(matches!(err, egui_kittest::RecordingError::NotRecording));
}
