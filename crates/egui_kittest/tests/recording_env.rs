//! Checks that `KITTEST_RECORD` records every harness and writes the MP4s next to the
//! snapshots, in `recordings/{test_name}_{recording_id}.mp4`.
//!
//! This is a test binary of its own because it changes the environment and the working
//! directory of the whole process.

#![cfg(all(feature = "recording", feature = "wgpu"))]
#![expect(unsafe_code)] // To set the environment variable.

use std::sync::OnceLock;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use tempfile::TempDir;

/// Run the process in a temporary directory, with recording turned on.
///
/// Both the environment variable and the `kittest.toml` are read once per process,
/// so this must happen before the first harness is built.
fn setup() -> &'static std::path::Path {
    static SETUP: OnceLock<TempDir> = OnceLock::new();

    SETUP
        .get_or_init(|| {
            let dir = tempfile::tempdir().expect("tempdir");

            // Write the recordings into the temporary directory.
            std::fs::write(dir.path().join("kittest.toml"), "output_path = \".\"\n")
                .expect("write kittest.toml");

            // SAFETY: the `OnceLock` runs this once, before any other thread reads the
            // environment or the working directory.
            unsafe {
                std::env::set_current_dir(dir.path()).expect("chdir to the tempdir");
                std::env::set_var(egui_kittest::RECORD_ENV_VAR, "1");
                std::env::set_var(egui_kittest::NATURAL_RECORD_ENV_VAR, "1");
            }

            dir
        })
        .path()
}

#[test]
fn env_var_records_every_harness() {
    let dir = setup();

    for label in ["first harness", "second harness"] {
        let mut harness = Harness::new_ui_state(
            |ui, clicks| {
                if ui.button(label).clicked() {
                    *clicks += 1;
                }
            },
            0,
        );
        harness.get_by_label(label).click();
        harness.run();
        assert_eq!(*harness.state(), 1);
        // Dropping the harness saves the recording.
    }

    for recording_id in 1..=2 {
        let mp4 = dir
            .join("recordings")
            .join(format!("env_var_records_every_harness_{recording_id}.mp4"));
        let size = std::fs::metadata(&mp4)
            .unwrap_or_else(|err| panic!("{} should exist: {err}", mp4.display()))
            .len();
        assert!(size > 0, "the MP4 should not be empty");
    }
}
