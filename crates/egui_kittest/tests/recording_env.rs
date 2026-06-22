//! Verifies that the `KITTEST_RECORD` env var auto-records harnesses and writes them next to
//! snapshots under `recordings/{test_name}.gif`.
//!
//! This is its own integration test binary because the env var is read once via `OnceLock`,
//! and we redirect the snapshot output path via `kittest.toml` lookup is process-global too.

#![cfg(all(feature = "recording", feature = "snapshot", feature = "wgpu"))]
#![allow(unsafe_code)] // tests need set_var / set_current_dir

use std::sync::OnceLock;

use egui_kittest::Harness;
use tempfile::TempDir;

static SETUP: OnceLock<TempDir> = OnceLock::new();

fn setup_env() -> &'static std::path::Path {
    SETUP
        .get_or_init(|| {
            let dir = tempfile::tempdir().expect("tempdir");

            // Point the snapshot output at our temp dir (used as the recording root).
            std::fs::write(dir.path().join("kittest.toml"), "output_path = \".\"\n")
                .expect("write kittest.toml");

            // SAFETY: this OnceLock guarantees a single initialization before any harness
            // reads the env var or cwd, so no concurrent env access happens.
            unsafe {
                std::env::set_current_dir(dir.path()).expect("chdir to tmp");
                std::env::set_var("KITTEST_RECORD", "1");
            }
            dir
        })
        .path()
}

#[test]
fn env_var_records_to_recordings_dir() {
    let dir = setup_env();

    {
        let mut harness = Harness::new_ui(|ui| {
            ui.label("env-recorded");
        });
        harness.run();
        // Drop here triggers the auto-save.
    }

    let recordings = dir.join("recordings");
    let entries: Vec<_> = std::fs::read_dir(&recordings)
        .expect("recordings dir exists")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("gif"))
        .collect();
    assert!(
        !entries.is_empty(),
        "KITTEST_RECORD should produce a GIF in {}",
        recordings.display()
    );
    for entry in &entries {
        let len = std::fs::metadata(entry).expect("stat").len();
        assert!(len > 0, "GIF {} should be non-empty", entry.display());
    }
}
