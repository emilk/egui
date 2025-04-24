use crate::Harness;
use image::ImageError;
use std::fmt::Display;
use std::io::ErrorKind;
use std::path::PathBuf;

pub type SnapshotResult = Result<(), SnapshotError>;

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
                let diff_path = std::path::absolute(diff_path).unwrap_or(diff_path.clone());
                write!(
                    f,
                    "'{name}' Image did not match snapshot. Diff: {diff}, {diff_path:?}. {HOW_TO_UPDATE_SCREENSHOTS}"
                )
            }
            Self::OpenSnapshot { path, err } => {
                let path = std::path::absolute(path).unwrap_or(path.clone());
                match err {
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
                let path = std::path::absolute(path).unwrap_or(path.clone());
                write!(f, "Error writing snapshot: {err:?}\nAt: {path:?}")
            }
            Self::RenderError { err } => {
                write!(f, "Error rendering image: {err:?}")
            }
        }
    }
}

/// If this is set, we update the snapshots (if different),
/// and _succeed_ the test.
/// This is so that you can set `UPDATE_SNAPSHOTS=true` and update _all_ tests,
/// without `cargo test` failing on the first failing crate.
fn should_update_snapshots() -> bool {
    match std::env::var("UPDATE_SNAPSHOTS") {
        Ok(value) => !matches!(value.as_str(), "false" | "0" | "no" | "off"),
        Err(_) => false,
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
    name: &str,
    options: &SnapshotOptions,
) -> SnapshotResult {
    let SnapshotOptions {
        threshold,
        output_path,
    } = options;

    let parent_path = if let Some(parent) = PathBuf::from(name).parent() {
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

        println!("Updated snapshot: {snapshot_path:?}");

        Ok(())
    };

    // Always write a `.new` file so the user can compare:
    new.save(&new_path)
        .map_err(|err| SnapshotError::WriteSnapshot {
            err,
            path: new_path.clone(),
        })?;

    let previous = match image::open(&snapshot_path) {
        Ok(image) => image.to_rgba8(),
        Err(err) => {
            // No previous snapshot - probablye a new test.
            if should_update_snapshots() {
                return update_snapshot();
            } else {
                return Err(SnapshotError::OpenSnapshot {
                    path: snapshot_path.clone(),
                    err,
                });
            }
        }
    };

    if previous.dimensions() != new.dimensions() {
        if should_update_snapshots() {
            return update_snapshot();
        } else {
            return Err(SnapshotError::SizeMismatch {
                name: name.to_owned(),
                expected: previous.dimensions(),
                actual: new.dimensions(),
            });
        }
    }

    // Compare existing image to the new one:
    let result =
        dify::diff::get_results(previous, new.clone(), *threshold, true, None, &None, &None);

    if let Some((diff, result_image)) = result {
        result_image
            .save(diff_path.clone())
            .map_err(|err| SnapshotError::WriteSnapshot {
                path: diff_path.clone(),
                err,
            })?;
        if should_update_snapshots() {
            update_snapshot()
        } else {
            Err(SnapshotError::Diff {
                name: name.to_owned(),
                diff,
                diff_path,
            })
        }
    } else {
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
/// If the new image didn't match the snapshot, a diff image will be saved under `{output_path}/{name}.diff.png`.
///
/// # Errors
/// Returns a [`SnapshotError`] if the image does not match the snapshot or if there was an error
/// reading or writing the snapshot.
pub fn try_image_snapshot(current: &image::RgbaImage, name: &str) -> SnapshotResult {
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
pub fn image_snapshot_options(current: &image::RgbaImage, name: &str, options: &SnapshotOptions) {
    match try_image_snapshot_options(current, name, options) {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
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
    /// Render an image using the setup [`crate::TestRenderer`] and compare it to the snapshot
    /// with custom options.
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
        name: &str,
        options: &SnapshotOptions,
    ) -> SnapshotResult {
        let image = self
            .render()
            .map_err(|err| SnapshotError::RenderError { err })?;
        try_image_snapshot_options(&image, name, options)
    }

    /// Render an image using the setup [`crate::TestRenderer`] and compare it to the snapshot.
    /// The snapshot will be saved under `tests/snapshots/{name}.png`.
    /// The new image from the last test run will be saved under `tests/snapshots/{name}.new.png`.
    /// If the new image didn't match the snapshot, a diff image will be saved under `tests/snapshots/{name}.diff.png`.
    ///
    /// # Errors
    /// Returns a [`SnapshotError`] if the image does not match the snapshot, if there was an
    /// error reading or writing the snapshot, if the rendering fails or if no default renderer is available.
    pub fn try_snapshot(&mut self, name: &str) -> SnapshotResult {
        let image = self
            .render()
            .map_err(|err| SnapshotError::RenderError { err })?;
        try_image_snapshot(&image, name)
    }

    /// Render an image using the setup [`crate::TestRenderer`] and compare it to the snapshot
    /// with custom options.
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
    /// Panics if the image does not match the snapshot, if there was an error reading or writing the
    /// snapshot, if the rendering fails or if no default renderer is available.
    #[track_caller]
    pub fn snapshot_options(&mut self, name: &str, options: &SnapshotOptions) {
        match self.try_snapshot_options(name, options) {
            Ok(_) => {}
            Err(err) => {
                panic!("{}", err);
            }
        }
    }

    /// Render an image using the setup [`crate::TestRenderer`] and compare it to the snapshot.
    /// The snapshot will be saved under `tests/snapshots/{name}.png`.
    /// The new image from the last test run will be saved under `tests/snapshots/{name}.new.png`.
    /// If the new image didn't match the snapshot, a diff image will be saved under `tests/snapshots/{name}.diff.png`.
    ///
    /// # Panics
    /// Panics if the image does not match the snapshot, if there was an error reading or writing the
    /// snapshot, if the rendering fails or if no default renderer is available.
    #[track_caller]
    pub fn snapshot(&mut self, name: &str) {
        match self.try_snapshot(name) {
            Ok(_) => {}
            Err(err) => {
                panic!("{}", err);
            }
        }
    }
}

// Deprecated wgpu_snapshot functions
// TODO(lucasmerlin): Remove in 0.32
#[expect(clippy::missing_errors_doc)]
#[cfg(feature = "wgpu")]
impl<State> Harness<'_, State> {
    #[deprecated(
        since = "0.31.0",
        note = "Use `try_snapshot_options` instead. This function will be removed in 0.32"
    )]
    pub fn try_wgpu_snapshot_options(
        &mut self,
        name: &str,
        options: &SnapshotOptions,
    ) -> SnapshotResult {
        self.try_snapshot_options(name, options)
    }

    #[deprecated(
        since = "0.31.0",
        note = "Use `try_snapshot` instead. This function will be removed in 0.32"
    )]
    pub fn try_wgpu_snapshot(&mut self, name: &str) -> SnapshotResult {
        self.try_snapshot(name)
    }

    #[deprecated(
        since = "0.31.0",
        note = "Use `snapshot_options` instead. This function will be removed in 0.32"
    )]
    pub fn wgpu_snapshot_options(&mut self, name: &str, options: &SnapshotOptions) {
        self.snapshot_options(name, options);
    }

    #[deprecated(
        since = "0.31.0",
        note = "Use `snapshot` instead. This function will be removed in 0.32"
    )]
    pub fn wgpu_snapshot(&mut self, name: &str) {
        self.snapshot(name);
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
#[derive(Debug, Default)]
pub struct SnapshotResults {
    errors: Vec<SnapshotError>,
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
    pub fn new() -> Self {
        Default::default()
    }

    /// Check if the result is an error and add it to the list of errors.
    pub fn add(&mut self, result: SnapshotResult) {
        if let Err(err) = result {
            self.errors.push(err);
        }
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Convert this into a `Result<(), Self>`.
    #[expect(clippy::missing_errors_doc)]
    pub fn into_result(self) -> Result<(), Self> {
        if self.has_errors() {
            Err(self)
        } else {
            Ok(())
        }
    }

    pub fn into_inner(mut self) -> Vec<SnapshotError> {
        std::mem::take(&mut self.errors)
    }

    /// Panics if there are any errors, displaying each.
    #[expect(clippy::unused_self)]
    #[track_caller]
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
    }
}
