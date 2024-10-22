use crate::Harness;
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

    /// Error writing the snapshot output
    WriteSnapshot {
        /// Path where a file was expected to be written
        path: PathBuf,

        /// The error that occurred
        err: ImageError,
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
                        write!(f, "Error reading snapshot: {err:?}\nAt: {path:?}")
                    }
                },
                err => {
                    write!(f, "Error decoding snapshot: {err:?}\nAt: {path:?}")
                }
            },
            Self::SizeMismatch { expected, actual } => {
                write!(
                    f,
                    "Image size did not match snapshot. Expected: {expected:?}, Actual: {actual:?}"
                )
            }
            Self::WriteSnapshot { path, err } => {
                write!(f, "Error writing snapshot: {err:?}\nAt: {path:?}")
            }
        }
    }
}

/// Image snapshot test.
///
/// # Errors
/// Returns a [`SnapshotError`] if the image does not match the snapshot or if there was an error
/// reading or writing the snapshot.
pub fn try_image_snapshot(current: &image::RgbaImage, name: &str) -> Result<(), SnapshotError> {
    let snapshots_path = Path::new("tests/snapshots");

    let path = snapshots_path.join(format!("{name}.png"));
    std::fs::create_dir_all(path.parent().expect("Could not get snapshot folder")).ok();

    let diff_path = snapshots_path.join(format!("{name}.diff.png"));
    let current_path = snapshots_path.join(format!("{name}.new.png"));

    current
        .save(&current_path)
        .map_err(|err| SnapshotError::WriteSnapshot {
            err,
            path: current_path,
        })?;

    let previous = match image::open(&path) {
        Ok(image) => image.to_rgba8(),
        Err(err) => {
            maybe_update_snapshot(&path, current)?;
            return Err(SnapshotError::OpenSnapshot { path, err });
        }
    };

    if previous.dimensions() != current.dimensions() {
        maybe_update_snapshot(&path, current)?;
        return Err(SnapshotError::SizeMismatch {
            expected: previous.dimensions(),
            actual: current.dimensions(),
        });
    }

    // Looking at dify's source code, the threshold is based on the distance between two colors in
    // YIQ color space.
    // The default is 0.1.
    // We currently need 2.1 because there are slight rendering differences between the different
    // wgpu rendering backends, graphics cards and/or operating systems.
    // After some testing it seems like 0.6 should be enough for almost all tests to pass.
    // Only the `Bézier Curve` demo seems to need a threshold of 2.1.
    let threshold = 2.1;
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
        result_image
            .save(diff_path.clone())
            .map_err(|err| SnapshotError::WriteSnapshot {
                path: diff_path.clone(),
                err,
            })?;
        maybe_update_snapshot(&path, current)?;
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

fn maybe_update_snapshot(
    snapshot_path: &Path,
    current: &image::RgbaImage,
) -> Result<(), SnapshotError> {
    if should_update_snapshots() {
        current
            .save(snapshot_path)
            .map_err(|err| SnapshotError::WriteSnapshot {
                err,
                path: snapshot_path.into(),
            })?;
        println!("Updated snapshot: {snapshot_path:?}");
    }
    Ok(())
}

/// Image snapshot test.
///
/// # Panics
/// Panics if the image does not match the snapshot or if there was an error reading or writing the
/// snapshot.
#[track_caller]
pub fn image_snapshot(current: &image::RgbaImage, name: &str) {
    match try_image_snapshot(current, name) {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    }
}

#[cfg(feature = "wgpu")]
impl Harness<'_> {
    /// Render a image using a default [`crate::wgpu::TestRenderer`] and compare it to the snapshot.
    ///
    /// # Errors
    /// Returns a [`SnapshotError`] if the image does not match the snapshot or if there was an error
    /// reading or writing the snapshot.
    #[track_caller]
    pub fn try_wgpu_snapshot(&self, name: &str) -> Result<(), SnapshotError> {
        let image = crate::wgpu::TestRenderer::new().render(self);
        try_image_snapshot(&image, name)
    }

    /// Render a image using a default [`crate::wgpu::TestRenderer`] and compare it to the snapshot.
    ///
    /// # Panics
    /// Panics if the image does not match the snapshot or if there was an error reading or writing the
    /// snapshot.
    #[track_caller]
    pub fn wgpu_snapshot(&self, name: &str) {
        match self.try_wgpu_snapshot(name) {
            Ok(_) => {}
            Err(err) => {
                panic!("{}", err);
            }
        }
    }
}
