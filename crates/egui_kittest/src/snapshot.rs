use image::ImageError;
use std::fmt::Display;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum SnapshotError {
    /// Image did not match snapshot
    Diff {
        /// Count of pixels that were different
        diff: i32,
        /// Path where the diff image was saved
        diff_path: PathBuf,
    },
    /// Error opening the existing snapshot (it probably doesn't exist, check the
    /// [`ImageError`] for more information)
    OpenSnapshot {
        /// Path where the snapshot was expected to be
        path: PathBuf,
        /// The error that occurred
        err: ImageError,
    },
    /// The size of the image did not match the snapshot
    SizeMismatch {
        /// Expected size
        expected: (u32, u32),
        /// Actual size
        actual: (u32, u32),
    },
}

impl Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Diff { diff, diff_path } => {
                write!(
                    f,
                    "Image did not match snapshot. Diff: {diff}, {diff_path:?}"
                )
            }
            Self::OpenSnapshot { path, err } => match err {
                ImageError::IoError(io) => match io.kind() {
                    ErrorKind::NotFound => {
                        write!(f, "Missing snapshot: {path:?}")
                    }
                    err => {
                        write!(f, "Error reading snapshot: {err:?}")
                    }
                },
                err => {
                    write!(f, "Error decoding snapshot: {err:?}")
                }
            },
            Self::SizeMismatch { expected, actual } => {
                write!(
                    f,
                    "Image size did not match snapshot. Expected: {expected:?}, Actual: {actual:?}"
                )
            }
        }
    }
}

/// Image snapshot test.
///
/// # Errors
/// Returns a [`SnapshotError`] if the image does not match the snapshot.
pub fn try_image_snapshot(current: &image::RgbaImage, name: &str) -> Result<(), SnapshotError> {
    let snapshots_path = Path::new("tests/snapshots");

    let path = snapshots_path.join(format!("{name}.png"));
    std::fs::create_dir_all(path.parent().unwrap()).ok();

    let diff_path = snapshots_path.join(format!("{name}.diff.png"));
    let current_path = snapshots_path.join(format!("{name}.new.png"));

    current.save(&current_path).unwrap();

    let previous = match image::open(&path) {
        Ok(image) => image.to_rgba8(),
        Err(err) => {
            maybe_update_snapshot(&path, current);
            return Err(SnapshotError::OpenSnapshot { path, err });
        }
    };

    if previous.dimensions() != current.dimensions() {
        maybe_update_snapshot(&path, current);
        return Err(SnapshotError::SizeMismatch {
            expected: previous.dimensions(),
            actual: current.dimensions(),
        });
    }

    // Looking at dify's source code, the threshold is based on the distance between two colors in
    // YIQ color space.
    // The default is 0.1, but we'll try 0.0 because ideally the output should not change at all.
    // We might have to increase the threshold if there are minor differences when running tests
    // on different gpus or different backends.
    let threshold = 0.0;
    let result = dify::diff::get_results(
        previous,
        current.clone(),
        threshold,
        true,
        None,
        &None,
        &None,
    );

    if let Some((diff, result_image)) = result {
        result_image.save(diff_path.clone()).unwrap();
        maybe_update_snapshot(&path, current);
        return Err(SnapshotError::Diff { diff, diff_path });
    } else {
        // Delete old diff if it exists
        std::fs::remove_file(diff_path).ok();
    }

    Ok(())
}

fn should_update_snapshots() -> bool {
    std::env::var("UPDATE_SNAPSHOTS").is_ok()
}

fn maybe_update_snapshot(snapshot_path: &Path, current: &image::RgbaImage) {
    if should_update_snapshots() {
        current.save(snapshot_path).unwrap();
        println!("Updated snapshot: {snapshot_path:?}");
    }
}

/// Image snapshot test.
///
/// # Panics
/// Panics if the image does not match the snapshot.
#[track_caller]
pub fn image_snapshot(current: &image::RgbaImage, name: &str) {
    match try_image_snapshot(current, name) {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    }
}
