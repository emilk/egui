#![cfg(all(feature = "recording", feature = "wgpu"))]

use egui_kittest::{Harness, RecordingOptions, RecordingPlugin, RecordingTrigger};
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
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "png"))
        .count()
}

#[test]
fn records_a_gif() {
    let dir = tempdir().expect("tempdir");
    let gif_path = dir.path().join("counter.gif");

    let mut value = 0;
    let mut harness = counter_harness(&mut value);
    harness.start_recording(RecordingOptions::gif(&gif_path, 12.0));

    harness.run();
    harness.get_by_label_contains("count").click();
    harness.run();

    assert!(harness.is_recording());
    harness.finish_recording().expect("save gif");
    assert!(!harness.is_recording());

    let frames = decode_gif_frames(&gif_path);
    assert!(
        frames >= 2,
        "the click should have produced at least two different frames, got {frames}"
    );
}

/// Without `ffmpeg` this saves a GIF next to the requested path instead.
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

    if which_ffmpeg() {
        assert_eq!(path, mp4_path);
    } else {
        assert_eq!(
            path,
            mp4_path.with_extension("gif"),
            "without ffmpeg we should fall back to a GIF"
        );
    }
    let size = std::fs::metadata(&path)
        .expect("the recording exists")
        .len();
    assert!(size > 0, "the recording should not be empty");
}

fn which_ffmpeg() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

fn decode_gif_frames(path: &std::path::Path) -> usize {
    use image::AnimationDecoder as _;

    let file = std::io::BufReader::new(std::fs::File::open(path).expect("gif exists"));
    image::codecs::gif::GifDecoder::new(file)
        .expect("decode gif")
        .into_frames()
        .count()
}

#[test]
fn records_a_png_sequence() {
    let dir = tempdir().expect("tempdir");
    let out = dir.path().join("frames");

    let mut value = 0;
    let mut harness = counter_harness(&mut value);
    harness.start_recording(
        RecordingOptions::png_sequence(&out).with_trigger(RecordingTrigger::EveryFrame),
    );

    harness.run();
    harness.get_by_label_contains("count").click();
    harness.run();

    harness.finish_recording().expect("save png sequence");

    assert!(count_pngs(&out) > 0, "expected at least one frame");
}

#[test]
fn changed_frames_drops_identical_frames() {
    let dir = tempdir().expect("tempdir");
    let out = dir.path().join("frames");

    let mut value = 0;
    let mut harness = counter_harness(&mut value);
    harness.start_recording(
        RecordingOptions::png_sequence(&out).with_trigger(RecordingTrigger::ChangedFrames),
    );

    for _ in 0..6 {
        harness.run();
    }
    harness.finish_recording().expect("save png sequence");

    assert_eq!(
        count_pngs(&out),
        1,
        "nothing changed, so only the first frame should be kept"
    );
}

#[test]
fn every_nth_frame_skips_frames() {
    let mut value = 0;
    let mut harness = counter_harness(&mut value);

    harness.start_recording(
        RecordingOptions::gif(std::path::PathBuf::new(), 10.0)
            .with_trigger(RecordingTrigger::EveryNthFrame(2)),
    );
    harness.run_steps(4);

    let frames = harness
        .with_recording(|plugin| plugin.frames().len())
        .expect("the plugin is registered");
    assert_eq!(frames, 2, "every second of the 4 passes should be captured");
}

#[test]
fn finishing_without_recording_is_an_error() {
    let mut value = 0;
    let mut harness = counter_harness(&mut value);

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
    let mut harness = counter_harness(&mut value);
    harness.start_recording(RecordingOptions::gif(dir.path().join("empty.gif"), 10.0));

    let err = harness.finish_recording().expect_err("no frames");
    assert!(matches!(err, egui_kittest::RecordingError::NoFrames));
}

/// The recorder is a plain [`egui::Plugin`]: it records any [`egui::Context`],
/// with no harness and no renderer of your own.
#[test]
fn records_a_plain_context() {
    let dir = tempdir().expect("tempdir");
    let gif_path = dir.path().join("plain.gif");

    let ctx = egui::Context::default();
    ctx.add_plugin(RecordingPlugin::new(RecordingOptions::gif(&gif_path, 10.0)));

    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::Vec2::new(120.0, 60.0),
        )),
        ..Default::default()
    };

    for pass in 0..3 {
        let output = ctx.run_ui(input.clone(), |ui| {
            ui.label(format!("pass {pass}"));
        });
        // We have no renderer of our own; the plugin already rendered what it needed.
        output.drop_without_applying_deltas();
    }

    let frames = ctx
        .with_plugin::<RecordingPlugin, _>(|plugin| {
            plugin.save().expect("save gif");
            plugin.frames().len()
        })
        .expect("the plugin is registered");

    assert_eq!(frames, 3, "each pass shows a different label");
    assert!(gif_path.exists(), "the GIF should have been written");
}
