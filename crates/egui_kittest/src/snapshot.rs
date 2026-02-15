use std::fmt::Display;
use std::io::ErrorKind;
use std::path::PathBuf;

use image::ImageError;

use crate::{Harness, config::config};

pub type SnapshotResult = Result<(), SnapshotError>;

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct SnapshotOptions {
    /// The threshold for the image comparison.
    ///
    /// Can be configured via kittest.toml. The fallback is `0.6` (which is enough for most egui
    /// tests to pass across different wgpu backends).
    pub threshold: f32,

    /// The number of pixels that can differ before the snapshot is considered a failure.
    ///
    /// Preferably, you should use `threshold` to control the sensitivity of the image comparison.
    /// As a last resort, you can use this to allow a certain number of pixels to differ.
    /// Can be configured via kittest.toml. The fallback is `0` (meaning no pixels can differ).
    pub failed_pixel_count_threshold: usize,

    /// The path where the snapshots will be saved.
    ///
    /// This is relative to the current working directory (usually the crate root when
    /// running tests).
    ///
    /// Can be configured via kittest.toml. The fallback is `tests/snapshots`.
    pub output_path: PathBuf,
}

/// Helper struct to define the number of pixels that can differ before the snapshot is considered a failure.
///
/// This is useful if you want to set different thresholds for different operating systems.
///
/// [`OsThreshold::default`] gets the default from the config file (`kittest.toml`).
/// For `usize`, it's the `failed_pixel_count_threshold` value.
/// For `f32`, it's the `threshold` value.
///
/// Example usage:
/// ```no_run
///  use egui_kittest::{OsThreshold, SnapshotOptions};
///  let mut harness = egui_kittest::Harness::new_ui(|ui| {
///      ui.label("Hi!");
///  });
///  harness.snapshot_options(
///      "os_threshold_example",
///      &SnapshotOptions::new()
///          .threshold(OsThreshold::new(0.0).windows(10.0))
///          .failed_pixel_count_threshold(OsThreshold::new(0).windows(10).macos(53)
///  ))
/// ```
#[derive(Debug, Clone, Copy)]
pub struct OsThreshold<T> {
    pub windows: T,
    pub macos: T,
    pub linux: T,
    pub fallback: T,
}

impl Default for OsThreshold<usize> {
    /// Returns the default `failed_pixel_count_threshold` as configured in `kittest.toml`
    ///
    /// The fallback is `0`.
    fn default() -> Self {
        config().os_failed_pixel_count_threshold()
    }
}

impl Default for OsThreshold<f32> {
    /// Returns the default `threshold` as configured in `kittest.toml`
    ///
    /// The fallback is `0.6`.
    fn default() -> Self {
        config().os_threshold()
    }
}

impl From<usize> for OsThreshold<usize> {
    fn from(value: usize) -> Self {
        Self::new(value)
    }
}

impl From<f32> for OsThreshold<f32> {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

impl<T> OsThreshold<T>
where
    T: Copy,
{
    /// Use the same value for all
    pub fn new(same: T) -> Self {
        Self {
            windows: same,
            macos: same,
            linux: same,
            fallback: same,
        }
    }

    /// Set the threshold for Windows.
    #[inline]
    pub fn windows(mut self, threshold: T) -> Self {
        self.windows = threshold;
        self
    }

    /// Set the threshold for macOS.
    #[inline]
    pub fn macos(mut self, threshold: T) -> Self {
        self.macos = threshold;
        self
    }

    /// Set the threshold for Linux.
    #[inline]
    pub fn linux(mut self, threshold: T) -> Self {
        self.linux = threshold;
        self
    }

    /// Get the threshold for the current operating system.
    pub fn threshold(&self) -> T {
        if cfg!(target_os = "windows") {
            self.windows
        } else if cfg!(target_os = "macos") {
            self.macos
        } else if cfg!(target_os = "linux") {
            self.linux
        } else {
            self.fallback
        }
    }
}

impl From<OsThreshold<Self>> for usize {
    fn from(threshold: OsThreshold<Self>) -> Self {
        threshold.threshold()
    }
}

impl From<OsThreshold<Self>> for f32 {
    fn from(threshold: OsThreshold<Self>) -> Self {
        threshold.threshold()
    }
}

impl Default for SnapshotOptions {
    fn default() -> Self {
        Self {
            threshold: config().threshold(),
            output_path: config().output_path(),
            failed_pixel_count_threshold: config().failed_pixel_count_threshold(),
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
    pub fn threshold(mut self, threshold: impl Into<f32>) -> Self {
        self.threshold = threshold.into();
        self
    }

    /// Change the path where the snapshots will be saved.
    /// The default is `tests/snapshots`.
    #[inline]
    pub fn output_path(mut self, output_path: impl Into<PathBuf>) -> Self {
        self.output_path = output_path.into();
        self
    }

    /// Change the number of pixels that can differ before the snapshot is considered a failure.
    ///
    /// Preferably, you should use [`Self::threshold`] to control the sensitivity of the image comparison.
    /// As a last resort, you can use this to allow a certain number of pixels to differ.
    #[inline]
    pub fn failed_pixel_count_threshold(
        mut self,
        failed_pixel_count_threshold: impl Into<OsThreshold<usize>>,
    ) -> Self {
        let failed_pixel_count_threshold = failed_pixel_count_threshold.into().threshold();
        self.failed_pixel_count_threshold = failed_pixel_count_threshold;
        self
    }
}

#[derive(Debug)]
pub enum SnapshotError {
    /// Image did not match snapshot
    Diff {
        /// Name of the test
        name: String,

        /// Count of pixels that were different (above the per-pixel threshold).
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

    /// Error rendering the image
    RenderError {
        /// The error that occurred
        err: String,
    },
}

const HOW_TO_UPDATE_SCREENSHOTS: &str =
    "Run `UPDATE_SNAPSHOTS=1 cargo test --all-features` to update the snapshots.";

impl Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Diff {
                name,
                diff,
                diff_path,
            } => {
                let diff_path =
                    std::path::absolute(diff_path).unwrap_or_else(|_| diff_path.clone());
                write!(
                    f,
                    "'{name}' Image did not match snapshot. Diff: {diff}, {}. {HOW_TO_UPDATE_SCREENSHOTS}",
                    diff_path.display()
                )
            }
            Self::OpenSnapshot { path, err } => {
                let path = std::path::absolute(path).unwrap_or_else(|_| path.clone());
                match err {
                    ImageError::IoError(io) => match io.kind() {
                        ErrorKind::NotFound => {
                            write!(
                                f,
                                "Missing snapshot: {}. {HOW_TO_UPDATE_SCREENSHOTS}",
                                path.display()
                            )
                        }
                        err => {
                            write!(
                                f,
                                "Error reading snapshot: {err}\nAt: {}. {HOW_TO_UPDATE_SCREENSHOTS}",
                                path.display()
                            )
                        }
                    },
                    err => {
                        write!(
                            f,
                            "Error decoding snapshot: {err}\nAt: {}. Make sure git-lfs is setup correctly. Read the instructions here: https://github.com/emilk/egui/blob/main/CONTRIBUTING.md#making-a-pr",
                            path.display()
                        )
                    }
                }
            }
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
                let path = std::path::absolute(path).unwrap_or_else(|_| path.clone());
                write!(f, "Error writing snapshot: {err}\nAt: {}", path.display())
            }
            Self::RenderError { err } => {
                write!(f, "Error rendering image: {err}")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Test,
    UpdateFailing,
    UpdateAll,
}

impl Mode {
    fn from_env() -> Self {
        let Ok(value) = std::env::var("UPDATE_SNAPSHOTS") else {
            return Self::Test;
        };

        match value.as_str() {
            "false" | "0" | "no" | "off" => Self::Test,
            "true" | "1" | "yes" | "on" => Self::UpdateFailing,
            "force" => Self::UpdateAll,
            unknown => {
                panic!("Unsupported value for UPDATE_SNAPSHOTS: {unknown:?}");
            }
        }
    }

    fn is_update(&self) -> bool {
        match self {
            Self::Test => false,
            Self::UpdateFailing | Self::UpdateAll => true,
        }
    }
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
/// If the new image didn't match the snapshot, a diff image will be saved under `{output_path}/{name}.diff.png`.
///
/// If the env-var `UPDATE_SNAPSHOTS` is set, then the old image will backed up under `{output_path}/{name}.old.png`.
/// and then new image will be written to `{output_path}/{name}.png`
///
/// # Errors
/// Returns a [`SnapshotError`] if the image does not match the snapshot or if there was an error
/// reading or writing the snapshot.
pub fn try_image_snapshot_options(
    new: &image::RgbaImage,
    name: impl Into<String>,
    options: &SnapshotOptions,
) -> SnapshotResult {
    try_image_snapshot_options_impl(new, name.into(), options)
}

fn try_image_snapshot_options_impl(
    new: &image::RgbaImage,
    name: String,
    options: &SnapshotOptions,
) -> SnapshotResult {
    #![expect(clippy::print_stdout)]

    let mode = Mode::from_env();

    let SnapshotOptions {
        threshold,
        output_path,
        failed_pixel_count_threshold,
    } = options;

    let parent_path = if let Some(parent) = PathBuf::from(&name).parent() {
        output_path.join(parent)
    } else {
        output_path.clone()
    };
    std::fs::create_dir_all(parent_path).ok();

    // The one that is checked in to git
    let snapshot_path = output_path.join(format!("{name}.png"));

    // These should be in .gitignore:
    let diff_path = output_path.join(format!("{name}.diff.png"));
    let old_backup_path = output_path.join(format!("{name}.old.png"));
    let new_path = output_path.join(format!("{name}.new.png"));

    // Delete old temporary files if they exist:
    std::fs::remove_file(&diff_path).ok();
    std::fs::remove_file(&old_backup_path).ok();
    std::fs::remove_file(&new_path).ok();

    let update_snapshot = || {
        // Keep the old version so the user can compare it:
        std::fs::rename(&snapshot_path, &old_backup_path).ok();

        // Write the new file to the checked in path:
        new.save(&snapshot_path)
            .map_err(|err| SnapshotError::WriteSnapshot {
                err,
                path: snapshot_path.clone(),
            })?;

        // No need for an explicit `.new` file:
        std::fs::remove_file(&new_path).ok();

        println!("Updated snapshot: {}", snapshot_path.display());

        Ok(())
    };

    let write_new_png = || {
        new.save(&new_path)
            .map_err(|err| SnapshotError::WriteSnapshot {
                err,
                path: new_path.clone(),
            })?;
        Ok(())
    };

    let previous = match image::open(&snapshot_path) {
        Ok(image) => image.to_rgba8(),
        Err(err) => {
            // No previous snapshot - probably a new test.
            if mode.is_update() {
                return update_snapshot();
            } else {
                write_new_png()?;

                return Err(SnapshotError::OpenSnapshot {
                    path: snapshot_path.clone(),
                    err,
                });
            }
        }
    };

    if previous.dimensions() != new.dimensions() {
        if mode.is_update() {
            return update_snapshot();
        } else {
            write_new_png()?;

            return Err(SnapshotError::SizeMismatch {
                name,
                expected: previous.dimensions(),
                actual: new.dimensions(),
            });
        }
    }

    // Compare existing image to the new one:
    let threshold = if mode == Mode::UpdateAll {
        0.0 // Produce diff for any error, however small
    } else {
        *threshold
    };

    let result =
        dify::diff::get_results(previous, new.clone(), threshold, true, None, &None, &None);

    let Some((num_wrong_pixels, diff_image)) = result else {
        return Ok(()); // Difference below threshold
    };

    let below_threshold = num_wrong_pixels as i64 <= *failed_pixel_count_threshold as i64;

    if !below_threshold {
        diff_image
            .save(diff_path.clone())
            .map_err(|err| SnapshotError::WriteSnapshot {
                path: diff_path.clone(),
                err,
            })?;
    }

    match mode {
        Mode::Test => {
            if below_threshold {
                Ok(())
            } else {
                write_new_png()?;

                Err(SnapshotError::Diff {
                    name,
                    diff: num_wrong_pixels,
                    diff_path,
                })
            }
        }
        Mode::UpdateFailing => {
            if below_threshold {
                Ok(())
            } else {
                update_snapshot()
            }
        }
        Mode::UpdateAll => update_snapshot(),
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
/// If the new image didn't match the snapshot, a diff image will be saved under `{output_path}/{name}.diff.png`.
///
/// # Errors
/// Returns a [`SnapshotError`] if the image does not match the snapshot or if there was an error
/// reading or writing the snapshot.
pub fn try_image_snapshot(current: &image::RgbaImage, name: impl Into<String>) -> SnapshotResult {
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
/// If the new image didn't match the snapshot, a diff image will be saved under `{output_path}/{name}.diff.png`.
///
/// # Panics
/// Panics if the image does not match the snapshot or if there was an error reading or writing the
/// snapshot.
#[track_caller]
pub fn image_snapshot_options(
    current: &image::RgbaImage,
    name: impl Into<String>,
    options: &SnapshotOptions,
) {
    match try_image_snapshot_options(current, name, options) {
        Ok(_) => {}
        Err(err) => {
            panic!("{err}");
        }
    }
}

/// Image snapshot test.
///
/// The snapshot will be saved under `tests/snapshots/{name}.png`.
/// The new image from the last test run will be saved under `tests/snapshots/{name}.new.png`.
/// If the new image didn't match the snapshot, a diff image will be saved under `tests/snapshots/{name}.diff.png`.
///
/// # Panics
/// Panics if the image does not match the snapshot or if there was an error reading or writing the
/// snapshot.
#[track_caller]
pub fn image_snapshot(current: &image::RgbaImage, name: impl Into<String>) {
    match try_image_snapshot(current, name) {
        Ok(_) => {}
        Err(err) => {
            panic!("{err}");
        }
    }
}

#[cfg(any(feature = "wgpu", feature = "snapshot"))]
impl<State> Harness<'_, State> {
    /// The default options used for snapshot tests.
    /// set by [`crate::HarnessBuilder::with_options`].
    pub fn options(&self) -> &SnapshotOptions {
        &self.default_snapshot_options
    }

    /// Render an image using the setup [`crate::TestRenderer`] and compare it to the snapshot
    /// with custom options.
    ///
    /// These options will override the ones set by [`crate::HarnessBuilder::with_options`].
    ///
    /// If you want to change the default options for your whole project, you could create an
    /// [extension trait](http://xion.io/post/code/rust-extension-traits.html) to create a
    /// new `my_image_snapshot` function on the Harness that calls this function with the desired options.
    /// You could additionally use the
    /// [disallowed_methods](https://rust-lang.github.io/rust-clippy/master/#disallowed_methods)
    /// lint to disable use of the [`Harness::snapshot`] to prevent accidentally using the wrong defaults.
    ///
    /// The snapshot files will be saved under [`SnapshotOptions::output_path`].
    /// The snapshot will be saved under `{output_path}/{name}.png`.
    /// The new image from the most recent test run will be saved under `{output_path}/{name}.new.png`.
    /// If the new image didn't match the snapshot, a diff image will be saved under `{output_path}/{name}.diff.png`.
    ///
    /// # Errors
    /// Returns a [`SnapshotError`] if the image does not match the snapshot, if there was an
    /// error reading or writing the snapshot, if the rendering fails or if no default renderer is available.
    pub fn try_snapshot_options(
        &mut self,
        name: impl Into<String>,
        options: &SnapshotOptions,
    ) -> SnapshotResult {
        let image = self
            .render()
            .map_err(|err| SnapshotError::RenderError { err })?;
        try_image_snapshot_options(&image, name.into(), options)
    }

    /// Render an image using the setup [`crate::TestRenderer`] and compare it to the snapshot.
    ///
    /// This is like [`Self::try_snapshot_options`] but will use the options set by [`crate::HarnessBuilder::with_options`].
    ///
    /// The snapshot will be saved under `tests/snapshots/{name}.png`.
    /// The new image from the last test run will be saved under `tests/snapshots/{name}.new.png`.
    /// If the new image didn't match the snapshot, a diff image will be saved under `tests/snapshots/{name}.diff.png`.
    ///
    /// # Errors
    /// Returns a [`SnapshotError`] if the image does not match the snapshot, if there was an
    /// error reading or writing the snapshot, if the rendering fails or if no default renderer is available.
    pub fn try_snapshot(&mut self, name: impl Into<String>) -> SnapshotResult {
        let image = self
            .render()
            .map_err(|err| SnapshotError::RenderError { err })?;
        try_image_snapshot_options(&image, name.into(), &self.default_snapshot_options)
    }

    /// Render an image using the setup [`crate::TestRenderer`] and compare it to the snapshot
    /// with custom options.
    ///
    /// These options will override the ones set by [`crate::HarnessBuilder::with_options`].
    ///
    /// If you want to change the default options for your whole project, you could create an
    /// [extension trait](http://xion.io/post/code/rust-extension-traits.html) to create a
    /// new `my_image_snapshot` function on the Harness that calls this function with the desired options.
    /// You could additionally use the
    /// [disallowed_methods](https://rust-lang.github.io/rust-clippy/master/#disallowed_methods)
    /// lint to disable use of the [`Harness::snapshot`] to prevent accidentally using the wrong defaults.
    ///
    /// The snapshot files will be saved under [`SnapshotOptions::output_path`].
    /// The snapshot will be saved under `{output_path}/{name}.png`.
    /// The new image from the most recent test run will be saved under `{output_path}/{name}.new.png`.
    /// If the new image didn't match the snapshot, a diff image will be saved under `{output_path}/{name}.diff.png`.
    ///
    /// # Panics
    /// The result is added to the [`Harness`]'s internal [`SnapshotResults`].
    ///
    /// The harness will panic when dropped if there were any snapshot errors.
    ///
    /// Errors happen if the image does not match the snapshot, if there was an error reading or writing the
    /// snapshot, if the rendering fails or if no default renderer is available.
    #[track_caller]
    pub fn snapshot_options(&mut self, name: impl Into<String>, options: &SnapshotOptions) {
        let result = self.try_snapshot_options(name, options);
        self.snapshot_results.add(result);
    }

    /// Render an image using the setup [`crate::TestRenderer`] and compare it to the snapshot.
    ///
    /// This is like [`Self::snapshot_options`] but will use the options set by [`crate::HarnessBuilder::with_options`].
    ///
    /// The snapshot will be saved under `tests/snapshots/{name}.png`.
    /// The new image from the last test run will be saved under `tests/snapshots/{name}.new.png`.
    /// If the new image didn't match the snapshot, a diff image will be saved under `tests/snapshots/{name}.diff.png`.
    ///
    /// # Panics
    /// Panics if the image does not match the snapshot, if there was an error reading or writing the
    /// snapshot, if the rendering fails or if no default renderer is available.
    #[track_caller]
    pub fn snapshot(&mut self, name: impl Into<String>) {
        let result = self.try_snapshot(name);
        self.snapshot_results.add(result);
    }

    /// Render a snapshot, save it to a temp file and open it in the default image viewer.
    ///
    /// This method is marked as deprecated to trigger errors in CI (so that it's not accidentally
    /// committed).
    #[deprecated = "Only for debugging, don't commit this."]
    #[cfg(not(target_arch = "wasm32"))]
    pub fn debug_open_snapshot(&mut self) {
        let image = self
            .render()
            .map_err(|err| SnapshotError::RenderError { err })
            .unwrap();
        let temp_file = tempfile::Builder::new()
            .disable_cleanup(true) // we keep the file so it's accessible even after the test ends
            .prefix("kittest-snapshot")
            .suffix(".png")
            .tempfile()
            .expect("Failed to create temp file");

        let path = temp_file.path();

        image
            .save(temp_file.path())
            .map_err(|err| SnapshotError::WriteSnapshot {
                err,
                path: path.to_path_buf(),
            })
            .unwrap();

        // Close temp file so it isn't locked when `open` tries to launch it (on Windows)
        let path = temp_file.into_temp_path();

        #[expect(clippy::print_stdout)]
        {
            println!("Wrote debug snapshot to: {}", path.display());
        }
        let result = open::that(&path);
        if let Err(err) = result {
            #[expect(clippy::print_stderr)]
            {
                eprintln!(
                    "Failed to open image {} in default image viewer: {err}",
                    path.display()
                );
            }
        }
    }

    /// This removes the snapshot results from the harness. Useful if you e.g. want to merge it
    /// with the results from another harness (using [`SnapshotResults::add`]).
    pub fn take_snapshot_results(&mut self) -> SnapshotResults {
        std::mem::take(&mut self.snapshot_results)
    }
}

/// Utility to collect snapshot errors and display them at the end of the test.
///
/// # Example
/// ```
/// # let harness = MockHarness;
/// # struct MockHarness;
/// # impl MockHarness {
/// #     fn try_snapshot(&self, _: &str) -> Result<(), egui_kittest::SnapshotError> { Ok(()) }
/// # }
///
/// // [...] Construct a Harness
///
/// let mut results = egui_kittest::SnapshotResults::new();
///
/// // Call add for each snapshot in your test
/// results.add(harness.try_snapshot("my_test"));
///
/// // If there are any errors, SnapshotResults will panic once dropped.
/// ```
///
/// # Panics
/// Panics if there are any errors when dropped (this way it is impossible to forget to call `unwrap`).
/// If you don't want to panic, you can use [`SnapshotResults::into_result`] or [`SnapshotResults::into_inner`].
/// If you want to panic early, you can use [`SnapshotResults::unwrap`].
#[derive(Debug)]
pub struct SnapshotResults {
    errors: Vec<SnapshotError>,
    handled: bool,
    location: std::panic::Location<'static>,
}

impl Default for SnapshotResults {
    #[track_caller]
    fn default() -> Self {
        Self {
            errors: Vec::new(),
            handled: true, // If no snapshots were added, we should consider this handled.
            location: *std::panic::Location::caller(),
        }
    }
}

impl Display for SnapshotResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.errors.is_empty() {
            write!(f, "All snapshots passed")
        } else {
            writeln!(f, "Snapshot errors:")?;
            for error in &self.errors {
                writeln!(f, "  {error}")?;
            }
            Ok(())
        }
    }
}

impl SnapshotResults {
    #[track_caller]
    pub fn new() -> Self {
        Default::default()
    }

    /// Check if the result is an error and add it to the list of errors.
    pub fn add(&mut self, result: SnapshotResult) {
        self.handled = false;
        if let Err(err) = result {
            self.errors.push(err);
        }
    }

    /// Add all errors from another `SnapshotResults`.
    pub fn extend(&mut self, other: Self) {
        self.handled = false;
        self.errors.extend(other.into_inner());
    }

    /// Add all errors from a [`Harness`].
    pub fn extend_harness<T>(&mut self, harness: &mut Harness<'_, T>) {
        self.extend(harness.take_snapshot_results());
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Convert this into a `Result<(), Self>`.
    #[expect(clippy::missing_errors_doc)]
    pub fn into_result(self) -> Result<(), Self> {
        if self.has_errors() { Err(self) } else { Ok(()) }
    }

    /// Consume this and return the list of errors.
    pub fn into_inner(mut self) -> Vec<SnapshotError> {
        self.handled = true;
        std::mem::take(&mut self.errors)
    }

    /// Panics if there are any errors, displaying each.
    #[expect(clippy::unused_self)]
    pub fn unwrap(self) {
        // Panic is handled in drop
    }
}

impl From<SnapshotResults> for Vec<SnapshotError> {
    fn from(results: SnapshotResults) -> Self {
        results.into_inner()
    }
}

impl Drop for SnapshotResults {
    #[track_caller]
    fn drop(&mut self) {
        // Don't panic if we are already panicking (the test probably failed for another reason)
        if std::thread::panicking() {
            return;
        }
        #[expect(clippy::manual_assert)]
        if self.has_errors() {
            panic!("{}", self);
        }

        thread_local! {
            static UNHANDLED_SNAPSHOT_RESULTS_COUNTER: std::cell::RefCell<usize> = const { std::cell::RefCell::new(0) };
        }

        if !self.handled {
            let count = UNHANDLED_SNAPSHOT_RESULTS_COUNTER.with(|counter| {
                let mut count = counter.borrow_mut();
                *count += 1;
                *count
            });

            #[expect(clippy::manual_assert)]
            if count >= 2 {
                panic!(
                    r#"
Multiple SnapshotResults were dropped without being handled.

In order to allow consistent snapshot updates, all snapshot results within a test should be merged in a single SnapshotResults instance.
Usually this is handled internally in a harness. If you have multiple harnesses, you can merge the results using `Harness::take_snapshot_results` and `SnapshotResults::extend`.

The SnapshotResult was constructed at {}
                    "#,
                    self.location
                );
            }
        }
    }
}
