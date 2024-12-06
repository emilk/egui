use crate::Harness;
use image::ImageError;
use std::fmt::Display;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[non_exhaustive]
pub struct SnapshotOptions {
    /// The threshold for the image comparison.
    /// The default is `0.6` (which is enough for most egui tests to pass across different
    /// wgpu backends).
    pub threshold: f32,

    /// The path where the snapshots will be saved.
    /// The default is `tests/snapshots`.
    pub output_path: PathBuf,
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        Self {
            threshold: 0.6,
            output_path: PathBuf::from("tests/snapshots"),
        }
    }
}

impl SnapshotOptions {
    /// Create a new [`SnapshotOptions`] with the default values.
    pub fn new() -> Self {
        Default::default()
    }

    /// Change the threshold for the image comparison.
    /// The default is `0.6` (which is enough for most egui tests to pass across different
    /// wgpu backends).
    #[inline]
    pub fn threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// Change the path where the snapshots will be saved.
    /// The default is `tests/snapshots`.
    #[inline]
    pub fn output_path(mut self, output_path: impl Into<PathBuf>) -> Self {
        self.output_path = output_path.into();
        self
    }
}

#[derive(Debug)]
pub enum SnapshotError {
    /// Image did not match snapshot
    Diff {
        /// Name of the test
        name: String,

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
        /// Name of the test
        name: String,

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

const HOW_TO_UPDATE_SCREENSHOTS: &str =
    "Run `UPDATE_SNAPSHOTS=1 cargo test` to update the snapshots.";

impl Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Diff {
                name,
                diff,
                diff_path,
            } => {
                write!(
                    f,
                    "'{name}' Image did not match snapshot. Diff: {diff}, {diff_path:?}. {HOW_TO_UPDATE_SCREENSHOTS}"
                )
            }
            Self::OpenSnapshot { path, err } => match err {
                ImageError::IoError(io) => match io.kind() {
                    ErrorKind::NotFound => {
                        write!(f, "Missing snapshot: {path:?}. {HOW_TO_UPDATE_SCREENSHOTS}")
                    }
                    err => {
                        write!(f, "Error reading snapshot: {err:?}\nAt: {path:?}. {HOW_TO_UPDATE_SCREENSHOTS}")
                    }
                },
                err => {
                    write!(f, "Error decoding snapshot: {err:?}\nAt: {path:?}. Make sure git-lfs is setup correctly. Read the instructions here: https://github.com/emilk/egui/blob/master/CONTRIBUTING.md#making-a-pr")
                }
            },
            Self::SizeMismatch {
                name,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "'{name}' Image size did not match snapshot. Expected: {expected:?}, Actual: {actual:?}. {HOW_TO_UPDATE_SCREENSHOTS}"
                )
            }
            Self::WriteSnapshot { path, err } => {
                write!(f, "Error writing snapshot: {err:?}\nAt: {path:?}")
            }
        }
    }
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

/// Image snapshot test with custom options.
///
/// If you want to change the default options for your whole project, it's recommended to create a
/// new `my_image_snapshot` function in your project that calls this function with the desired options.
/// You could additionally use the
/// [disallowed_methods](https://rust-lang.github.io/rust-clippy/master/#disallowed_methods)
/// lint to disable use of the [`image_snapshot`] to prevent accidentally using the wrong defaults.
///
/// The snapshot files will be saved under [`SnapshotOptions::output_path`].
/// The snapshot will be saved under `{output_path}/{name}.png`.
/// The new image from the most recent test run will be saved under `{output_path}/{name}.new.png`.
/// If new image didn't match the snapshot, a diff image will be saved under `{output_path}/{name}.diff.png`.
///
/// # Errors
/// Returns a [`SnapshotError`] if the image does not match the snapshot or if there was an error
/// reading or writing the snapshot.
pub fn try_image_snapshot_options(
    current: &image::RgbaImage,
    name: &str,
    options: &SnapshotOptions,
) -> Result<(), SnapshotError> {
    let SnapshotOptions {
        threshold,
        output_path,
    } = options;

    let path = output_path.join(format!("{name}.png"));
    std::fs::create_dir_all(path.parent().expect("Could not get snapshot folder")).ok();

    let diff_path = output_path.join(format!("{name}.diff.png"));
    let current_path = output_path.join(format!("{name}.new.png"));

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
            name: name.to_owned(),
            expected: previous.dimensions(),
            actual: current.dimensions(),
        });
    }

    let result = dify::diff::get_results(
        previous,
        current.clone(),
        *threshold,
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
        Err(SnapshotError::Diff {
            name: name.to_owned(),
            diff,
            diff_path,
        })
    } else {
        // Delete old diff if it exists
        std::fs::remove_file(diff_path).ok();
        Ok(())
    }
}

/// Image snapshot test.
///
/// This uses the default [`SnapshotOptions`]. Use [`try_image_snapshot_options`] if you want to
/// e.g. change the threshold or output path.
///
/// The snapshot files will be saved under [`SnapshotOptions::output_path`].
/// The snapshot will be saved under `{output_path}/{name}.png`.
/// The new image from the most recent test run will be saved under `{output_path}/{name}.new.png`.
/// If new image didn't match the snapshot, a diff image will be saved under `{output_path}/{name}.diff.png`.
///
/// # Errors
/// Returns a [`SnapshotError`] if the image does not match the snapshot or if there was an error
/// reading or writing the snapshot.
pub fn try_image_snapshot(current: &image::RgbaImage, name: &str) -> Result<(), SnapshotError> {
    try_image_snapshot_options(current, name, &SnapshotOptions::default())
}

/// Image snapshot test with custom options.
///
/// If you want to change the default options for your whole project, it's recommended to create a
/// new `my_image_snapshot` function in your project that calls this function with the desired options.
/// You could additionally use the
/// [disallowed_methods](https://rust-lang.github.io/rust-clippy/master/#disallowed_methods)
/// lint to disable use of the [`image_snapshot`] to prevent accidentally using the wrong defaults.
///
/// The snapshot files will be saved under [`SnapshotOptions::output_path`].
/// The snapshot will be saved under `{output_path}/{name}.png`.
/// The new image from the most recent test run will be saved under `{output_path}/{name}.new.png`.
/// If new image didn't match the snapshot, a diff image will be saved under `{output_path}/{name}.diff.png`.
///
/// # Panics
/// Panics if the image does not match the snapshot or if there was an error reading or writing the
/// snapshot.
#[track_caller]
pub fn image_snapshot_options(current: &image::RgbaImage, name: &str, options: &SnapshotOptions) {
    match try_image_snapshot_options(current, name, options) {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    }
}

/// Image snapshot test.
/// The snapshot will be saved under `tests/snapshots/{name}.png`.
/// The new image from the last test run will be saved under `tests/snapshots/{name}.new.png`.
/// If new image didn't match the snapshot, a diff image will be saved under `tests/snapshots/{name}.diff.png`.
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
impl<State> Harness<'_, State> {
    /// Render a image using a default [`crate::wgpu::TestRenderer`] and compare it to the snapshot
    /// with custom options.
    ///
    /// If you want to change the default options for your whole project, you could create an
    /// [extension trait](http://xion.io/post/code/rust-extension-traits.html) to create a
    /// new `my_image_snapshot` function on the Harness that calls this function with the desired options.
    /// You could additionally use the
    /// [disallowed_methods](https://rust-lang.github.io/rust-clippy/master/#disallowed_methods)
    /// lint to disable use of the [`Harness::wgpu_snapshot`] to prevent accidentally using the wrong defaults.
    ///
    /// The snapshot files will be saved under [`SnapshotOptions::output_path`].
    /// The snapshot will be saved under `{output_path}/{name}.png`.
    /// The new image from the most recent test run will be saved under `{output_path}/{name}.new.png`.
    /// If new image didn't match the snapshot, a diff image will be saved under `{output_path}/{name}.diff.png`.
    ///
    /// # Errors
    /// Returns a [`SnapshotError`] if the image does not match the snapshot or if there was an error
    /// reading or writing the snapshot.
    pub fn try_wgpu_snapshot_options(
        &self,
        name: &str,
        options: &SnapshotOptions,
    ) -> Result<(), SnapshotError> {
        let image = crate::wgpu::TestRenderer::new().render(self);
        try_image_snapshot_options(&image, name, options)
    }

    /// Render a image using a default [`crate::wgpu::TestRenderer`] and compare it to the snapshot.
    /// The snapshot will be saved under `tests/snapshots/{name}.png`.
    /// The new image from the last test run will be saved under `tests/snapshots/{name}.new.png`.
    /// If new image didn't match the snapshot, a diff image will be saved under `tests/snapshots/{name}.diff.png`.
    ///
    /// # Errors
    /// Returns a [`SnapshotError`] if the image does not match the snapshot or if there was an error
    /// reading or writing the snapshot.
    pub fn try_wgpu_snapshot(&self, name: &str) -> Result<(), SnapshotError> {
        let image = crate::wgpu::TestRenderer::new().render(self);
        try_image_snapshot(&image, name)
    }

    /// Render a image using a default [`crate::wgpu::TestRenderer`] and compare it to the snapshot
    /// with custom options.
    ///
    /// If you want to change the default options for your whole project, you could create an
    /// [extension trait](http://xion.io/post/code/rust-extension-traits.html) to create a
    /// new `my_image_snapshot` function on the Harness that calls this function with the desired options.
    /// You could additionally use the
    /// [disallowed_methods](https://rust-lang.github.io/rust-clippy/master/#disallowed_methods)
    /// lint to disable use of the [`Harness::wgpu_snapshot`] to prevent accidentally using the wrong defaults.
    ///
    /// The snapshot files will be saved under [`SnapshotOptions::output_path`].
    /// The snapshot will be saved under `{output_path}/{name}.png`.
    /// The new image from the most recent test run will be saved under `{output_path}/{name}.new.png`.
    /// If new image didn't match the snapshot, a diff image will be saved under `{output_path}/{name}.diff.png`.
    ///
    /// # Panics
    /// Panics if the image does not match the snapshot or if there was an error reading or writing the
    /// snapshot.
    #[track_caller]
    pub fn wgpu_snapshot_options(&self, name: &str, options: &SnapshotOptions) {
        match self.try_wgpu_snapshot_options(name, options) {
            Ok(_) => {}
            Err(err) => {
                panic!("{}", err);
            }
        }
    }

    /// Render a image using a default [`crate::wgpu::TestRenderer`] and compare it to the snapshot.
    /// The snapshot will be saved under `tests/snapshots/{name}.png`.
    /// The new image from the last test run will be saved under `tests/snapshots/{name}.new.png`.
    /// If new image didn't match the snapshot, a diff image will be saved under `tests/snapshots/{name}.diff.png`.
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
