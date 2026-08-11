//! Checks that `KITTEST_RECORD` records every harness and writes the GIFs next to the
//! snapshots, in `recordings/{test_name}.gif`.
//!
//! This is a test binary of its own because it changes the environment and the working
//! directory of the whole process.

#![cfg(all(feature = "recording", feature = "wgpu"))]
#![expect(unsafe_code)] // To set the environment variable.

use std::sync::OnceLock;

use egui_kittest::Harness;
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
            }

            dir
        })
        .path()
}

#[test]
fn env_var_records_every_harness() {
    let dir = setup();

    {
        let mut harness = Harness::new_ui(|ui| {
            ui.label("recorded by the environment variable");
        });
        harness.run();
        // Dropping the harness saves the recording.
    }

    let gif = dir
        .join("recordings")
        .join("env_var_records_every_harness.gif");
    let size = std::fs::metadata(&gif)
        .unwrap_or_else(|err| panic!("{} should exist: {err}", gif.display()))
        .len();
    assert!(size > 0, "the GIF should not be empty");
}
