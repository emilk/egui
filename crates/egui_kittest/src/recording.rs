//! Record an egui session as an MP4 video.
//!
//! The recorder is an [`egui::Plugin`], so it can record any [`egui::Context`],
//! not just a [`crate::Harness`]. It requests a screenshot from the integration for each
//! selected pass and streams the frames to `ffmpeg`.
//!
//! See [`crate::Harness::start_recording`] / [`crate::Harness::finish_recording`].

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use egui::{Context, FullOutput};
use image::RgbaImage;

/// Name of the environment variable that records every [`crate::Harness`] in the process.
///
/// Every harness records itself and saves an MP4 when it is dropped,
/// whether the test passed or not:
///
/// - `KITTEST_RECORD=1` writes to `{output_path}/recordings/{test_name}_{recording_id}.mp4`
/// - `KITTEST_RECORD=open` writes to a temporary file and shows it in the default viewer
pub const RECORD_ENV_VAR: &str = "KITTEST_RECORD";

/// How to record. Pass this to [`crate::Harness::start_recording`] or [`RecordingPlugin::new`].
#[derive(Debug, Clone)]
pub struct RecordingOptions {
    /// Where to write the MP4.
    pub path: PathBuf,

    /// Frames per second.
    pub frame_rate: f32,
}

impl RecordingOptions {
    /// Record an MP4 to `path` at the given frame rate.
    ///
    /// This pipes frames into [`ffmpeg`](https://ffmpeg.org/), which must be installed and on the
    /// `PATH`. Frames are streamed as they are captured. If the viewport later shrinks, frames are
    /// padded; if it grows, frames are scaled down to fit the initial size.
    pub fn mp4(path: impl Into<PathBuf>, frame_rate: f32) -> Self {
        Self {
            path: path.into(),
            frame_rate,
        }
    }
}

/// What went wrong when saving a recording.
#[derive(Debug)]
pub enum RecordingError {
    /// No recording was running.
    NotRecording,

    /// The recording did not capture a single frame.
    NoFrames,

    /// Failed to create or write the output file or directory.
    Io {
        /// The file or directory we failed to write.
        path: PathBuf,

        /// The underlying error.
        err: std::io::Error,
    },

    /// `ffmpeg` ran, but did not produce a video.
    Ffmpeg {
        /// What `ffmpeg` complained about.
        message: String,
    },
}

impl core::fmt::Display for RecordingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotRecording => write!(f, "No recording is running"),
            Self::NoFrames => write!(f, "The recording contains no frames"),
            Self::Io { path, err } => write!(f, "Failed to write {}: {err}", path.display()),
            Self::Ffmpeg { message } => write!(f, "ffmpeg failed: {message}"),
        }
    }
}

impl core::error::Error for RecordingError {}

/// Records an [`egui::Context`] using the integration's screenshot support.
///
/// Register it with [`egui::Context::add_plugin`], or let [`crate::Harness::start_recording`]
/// do it for you.
///
/// The plugin is self-contained: it adds a [`egui::ViewportCommand::ScreenshotCallback`] to each
/// selected pass and receives the rendered image directly from the integration.
pub struct RecordingPlugin {
    options: RecordingOptions,
    mp4_stream: Option<Mp4Stream>,
    error: Option<RecordingError>,
    recording_id: u64,

    /// While `false` the plugin captures no frames.
    active: bool,

    /// Set when the harness started the recording by itself (see [`crate::Harness`]).
    pub(crate) auto_save: Option<AutoSaveMode>,
}

impl core::fmt::Debug for RecordingPlugin {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RecordingPlugin")
            .field("options", &self.options)
            .field("active", &self.active)
            .finish_non_exhaustive()
    }
}

impl RecordingPlugin {
    /// Create a plugin that starts recording right away.
    pub fn new(options: RecordingOptions) -> Self {
        Self {
            options,
            mp4_stream: None,
            error: None,
            recording_id: 0,
            active: true,
            auto_save: None,
        }
    }

    /// Is the plugin capturing frames?
    #[inline]
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Start capturing again, with new options. Any earlier frames are dropped.
    fn restart(&mut self, options: RecordingOptions) {
        self.cancel_mp4();
        self.options = options;
        self.error = None;
        self.recording_id = self.recording_id.wrapping_add(1);
        self.active = true;
        self.auto_save = None;
    }

    /// Finish writing the captured frames and return the MP4 path.
    ///
    /// # Errors
    /// Returns an error if there are no frames, or if writing fails.
    pub fn finish(&mut self) -> Result<PathBuf, RecordingError> {
        self.active = false;
        self.recording_id = self.recording_id.wrapping_add(1);
        self.auto_save = None;

        if let Some(err) = self.error.take() {
            self.cancel_mp4();
            Err(err)
        } else if self.mp4_stream.is_none() {
            Err(RecordingError::NoFrames)
        } else {
            self.mp4_stream
                .take()
                .expect("an MP4 with frames has a stream")
                .finish()
        }
    }

    /// Add a frame, dropping it if it is a duplicate.
    fn push_frame(&mut self, recording_id: u64, image: RgbaImage) {
        // A screenshot can finish after the recording was stopped or restarted.
        if !self.active || self.recording_id != recording_id {
            return;
        }

        if self.error.is_some() {
            return;
        }

        if let Err(err) = self.push_mp4_frame(image) {
            self.error = Some(err);
        }
    }

    fn push_mp4_frame(&mut self, image: RgbaImage) -> Result<(), RecordingError> {
        if self.mp4_stream.is_none() {
            let path = &self.options.path;
            let frame_rate = self.options.frame_rate;
            create_parent_dir(path)?;
            let source_size = image.dimensions();
            let canvas_size = (
                round_up_to_even(source_size.0),
                round_up_to_even(source_size.1),
            );
            match spawn_ffmpeg(path, canvas_size, frame_rate) {
                Ok(child) => {
                    self.mp4_stream = Some(Mp4Stream::new(
                        child,
                        path.clone(),
                        frame_rate,
                        source_size,
                        canvas_size,
                    ));
                }
                Err(err) => {
                    return Err(RecordingError::Io {
                        path: PathBuf::from(FFMPEG),
                        err,
                    });
                }
            }
        }

        let stream = self.mp4_stream.as_mut().expect("initialized above");
        let image = stream.prepare_frame(image);
        if stream
            .last_frame
            .as_ref()
            .is_some_and(|previous| previous.as_raw() == image.as_raw())
        {
            return Ok(());
        }
        stream.push(image)?;
        Ok(())
    }

    fn cancel_mp4(&mut self) {
        if let Some(mut stream) = self.mp4_stream.take() {
            drop(stream.stdin.take());
            let _ = stream.child.kill();
            let _ = stream.child.wait();
            let _ = std::fs::remove_file(&stream.path);
        }
    }
}

impl Drop for RecordingPlugin {
    fn drop(&mut self) {
        self.cancel_mp4();
    }
}

fn color_image_to_rgba(image: &egui::ColorImage) -> RgbaImage {
    let pixels = image
        .pixels
        .iter()
        .flat_map(|pixel| pixel.to_array())
        .collect();
    RgbaImage::from_raw(image.width() as u32, image.height() as u32, pixels)
        .expect("ColorImage dimensions match its pixels")
}

impl egui::Plugin for RecordingPlugin {
    fn debug_name(&self) -> &'static str {
        "egui_kittest::RecordingPlugin"
    }

    fn on_exit(&mut self, _ctx: &Context) {
        self.save_automatically();
    }

    fn output_hook(&mut self, ctx: &Context, output: &mut FullOutput) {
        if !self.active {
            return;
        }

        if output.platform_output.requested_discard() {
            // This pass is thrown away and never shown, so don't record it.
            return;
        }

        crate::push_cursor_shape(ctx, &mut output.shapes);

        let recording_id = self.recording_id;
        let plugin = ctx.plugin::<Self>();
        let callback = egui::ScreenshotCallback::new(move |image| {
            plugin
                .lock()
                .push_frame(recording_id, color_image_to_rgba(&image));
        });

        // `output_hook` runs after `Context::end_pass`, so sending this through `Context` would
        // put it in the next pass. Add it to the output being returned instead, ensuring that the
        // final pass of a recording is captured without requesting another repaint.
        let viewport_id = ctx.viewport_id();
        if let Some(viewport) = output.viewport_output.get_mut(&viewport_id) {
            viewport
                .commands
                .push(egui::ViewportCommand::ScreenshotCallback(callback));
        } else {
            log::error!("egui_kittest recording: output is missing viewport {viewport_id:?}");
        }
    }
}

/// When a recording that the harness started by itself is saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutoSaveMode {
    /// Always save. Written to `{output_path}/recordings/{test_name}_{recording_id}.mp4`.
    Always,

    /// Always save to a temporary file, and show it in the default viewer.
    Open,
}

/// Where to write an automatically started recording.
fn auto_recording_path(mode: AutoSaveMode, recording_id: usize) -> PathBuf {
    let name = std::thread::current()
        .name()
        .map_or_else(|| "recording".to_owned(), sanitize_file_name);
    let subdirectory = match mode {
        AutoSaveMode::Always => "recordings",
        AutoSaveMode::Open => {
            if let Some(path) = temp_recording_path(&name) {
                return path;
            }
            "recordings" // Fall back to a normal recording.
        }
    };

    crate::config::config()
        .output_path()
        .join(subdirectory)
        .join(format!("{name}_{recording_id}.mp4"))
}

/// A file in the temporary directory, which we keep after the test, so that the
/// viewer can still read it.
fn temp_recording_path(name: &str) -> Option<PathBuf> {
    tempfile::Builder::new()
        .disable_cleanup(true)
        .prefix(&format!("kittest-recording-{name}-"))
        .suffix(".mp4")
        .tempfile()
        .inspect_err(|err| log::error!("egui_kittest: failed to create a temporary file: {err}"))
        .ok()
        .map(|file| file.path().to_path_buf())
}

/// Test threads are named after the test (e.g. `menu::tests::close_on_click`).
fn sanitize_file_name(name: &str) -> String {
    name.replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_")
}

/// What [`RECORD_ENV_VAR`] asks for.
///
/// Read once, then cached, so that a test cannot change it halfway through a run.
pub(crate) fn record_env_var() -> Option<AutoSaveMode> {
    static MODE: std::sync::OnceLock<Option<AutoSaveMode>> = std::sync::OnceLock::new();
    *MODE.get_or_init(|| {
        let value = std::env::var(RECORD_ENV_VAR).ok()?;

        let mode = match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => AutoSaveMode::Always,
            "open" => AutoSaveMode::Open,
            "" | "0" | "false" | "no" | "off" => return None,
            other => {
                log::warn!(
                    "Ignoring {RECORD_ENV_VAR}={other:?}: expected a truthy value or `open`"
                );
                return None;
            }
        };

        Some(mode)
    })
}

// ----------------------------------------------------------------------------
// Harness integration

/// Frame rate of recordings that the harness starts by itself.
const AUTO_FRAME_RATE: f32 = 10.0;

/// Gives every automatically started recording a unique file name.
static NEXT_RECORDING_ID: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(1);

/// A [`crate::Harness`] can record itself.
impl<State> crate::Harness<'_, State> {
    /// Record the rest of this test session.
    ///
    /// One frame is captured per egui pass, with unchanged frames omitted.
    /// Call [`Self::finish_recording`] to write the result.
    ///
    /// This registers a [`RecordingPlugin`] on the [`egui::Context`] of the harness, and
    /// restarts the recording if there already is one.
    ///
    /// Recording uses the harness renderer through its screenshot support. The default renderer
    /// needs the `wgpu` feature.
    ///
    /// ```no_run
    /// # use egui_kittest::{Harness, RecordingOptions};
    /// let mut harness = Harness::new_ui(|ui| {
    ///     ui.label("Hello!");
    /// });
    /// harness.start_recording(RecordingOptions::mp4("hello.mp4", 10.0));
    /// harness.run();
    /// harness.finish_recording().unwrap();
    /// ```
    pub fn start_recording(&self, options: RecordingOptions) {
        install(&self.ctx, options, None);
    }

    /// Stop the recording and write it to disk, returning the path that was written.
    ///
    /// # Errors
    /// Returns [`RecordingError::NotRecording`] if nothing was being recorded,
    /// [`RecordingError::NoFrames`] if no frame was captured,
    /// or an I/O or encoding error if writing failed.
    pub fn finish_recording(&self) -> Result<PathBuf, RecordingError> {
        let result = self.ctx.with_plugin::<RecordingPlugin, _>(|plugin| {
            if !plugin.is_active() {
                return Err(RecordingError::NotRecording);
            }
            plugin.finish()
        });

        result.unwrap_or(Err(RecordingError::NotRecording))
    }

    /// Is the harness recording?
    pub fn is_recording(&self) -> bool {
        self.ctx
            .with_plugin::<RecordingPlugin, _>(|plugin| plugin.is_active())
            .unwrap_or(false)
    }

    /// Start recording if the environment variable asks for it.
    pub(crate) fn maybe_start_auto_recording(&self) {
        let Some(auto_save) = record_env_var() else {
            return;
        };
        let recording_id = NEXT_RECORDING_ID.fetch_add(1, core::sync::atomic::Ordering::Relaxed);

        let options = RecordingOptions::mp4(
            auto_recording_path(auto_save, recording_id),
            AUTO_FRAME_RATE,
        );
        install(&self.ctx, options, Some(auto_save));
    }
}

/// Register a [`RecordingPlugin`] on `ctx`, or restart the one that is already registered.
fn install(ctx: &Context, options: RecordingOptions, auto_save: Option<AutoSaveMode>) {
    let restarted = ctx
        .with_plugin::<RecordingPlugin, _>(|plugin| {
            plugin.restart(options.clone());
            plugin.auto_save = auto_save;
        })
        .is_some();

    if !restarted {
        let mut plugin = RecordingPlugin::new(options);
        plugin.auto_save = auto_save;
        ctx.add_plugin(plugin);
    }
}

#[expect(clippy::print_stderr)]
impl RecordingPlugin {
    fn save_automatically(&mut self) {
        let Some(auto_save) = self.auto_save.take() else {
            return;
        };

        match self.finish() {
            Ok(path) => {
                eprintln!("egui_kittest: saved a recording to {}", path.display());

                if auto_save == AutoSaveMode::Open
                    && let Err(err) = open::that_detached(&path)
                {
                    eprintln!(
                        "egui_kittest: failed to open {} in the default viewer: {err}",
                        path.display()
                    );
                }
            }
            Err(RecordingError::NoFrames) => {}
            Err(err) => eprintln!("egui_kittest: failed to save the recording: {err}"),
        }
    }
}

// ----------------------------------------------------------------------------
// Saving

/// The encoder we pipe the frames into. It must be on the `PATH`.
const FFMPEG: &str = "ffmpeg";

struct Mp4Stream {
    child: std::process::Child,
    stdin: Option<std::process::ChildStdin>,
    path: PathBuf,
    frame_rate: f32,
    source_size: (u32, u32),
    canvas_size: (u32, u32),
    last_frame: Option<RgbaImage>,
    warned_about_resize: bool,
}

impl Mp4Stream {
    fn new(
        mut child: std::process::Child,
        path: PathBuf,
        frame_rate: f32,
        source_size: (u32, u32),
        canvas_size: (u32, u32),
    ) -> Self {
        let stdin = child.stdin.take().expect("`spawn_ffmpeg` pipes stdin");
        Self {
            child,
            stdin: Some(stdin),
            path,
            frame_rate,
            source_size,
            canvas_size,
            last_frame: None,
            warned_about_resize: false,
        }
    }

    fn prepare_frame(&mut self, image: RgbaImage) -> RgbaImage {
        let size = image.dimensions();
        if size != self.source_size && !self.warned_about_resize {
            let action = if size.0 <= self.source_size.0 && size.1 <= self.source_size.1 {
                "padding it"
            } else {
                "scaling it to fit"
            };
            log::warn!(
                "egui_kittest: MP4 recording changed size from {}x{} to {}x{}; {action} into the {}x{} recording canvas",
                self.source_size.0,
                self.source_size.1,
                size.0,
                size.1,
                self.canvas_size.0,
                self.canvas_size.1,
            );
            self.warned_about_resize = true;
        }
        let image = fit_to_canvas(image, self.source_size);
        pad_to(&image, self.canvas_size)
    }

    fn push(&mut self, image: RgbaImage) -> Result<(), RecordingError> {
        if let Some(previous) = self.last_frame.replace(image) {
            self.stdin
                .as_mut()
                .expect("unfinished stream has stdin")
                .write_all(previous.as_raw())
                .map_err(|err| RecordingError::Io {
                    path: self.path.clone(),
                    err,
                })?;
        }
        Ok(())
    }

    fn finish(mut self) -> Result<PathBuf, RecordingError> {
        let last_frame = self.last_frame.take().expect("a stream with frames");
        let hold = self.frame_rate.clamp(1.0, MAX_FRAME_RATE).round() as usize;
        let stdin = self.stdin.as_mut().expect("unfinished stream has stdin");
        let write_result = core::iter::repeat_n(&last_frame, hold + 1)
            .try_for_each(|frame| stdin.write_all(frame.as_raw()));
        drop(self.stdin.take());

        let output = self
            .child
            .wait_with_output()
            .map_err(|err| RecordingError::Io {
                path: PathBuf::from(FFMPEG),
                err,
            })?;
        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            return Err(RecordingError::Ffmpeg {
                message: if message.is_empty() {
                    format!("{} while writing {}", output.status, self.path.display())
                } else {
                    message
                },
            });
        }
        write_result.map_err(|err| RecordingError::Io {
            path: self.path.clone(),
            err,
        })?;
        Ok(self.path)
    }
}

/// Start `ffmpeg`, ready to read raw RGBA frames of the given size from its stdin.
fn spawn_ffmpeg(
    path: &Path,
    (width, height): (u32, u32),
    frame_rate: f32,
) -> std::io::Result<std::process::Child> {
    Command::new(FFMPEG)
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        // Input: what we write to stdin.
        .args(["-f", "rawvideo", "-pix_fmt", "rgba"])
        .args(["-s", &format!("{width}x{height}")])
        .args([
            "-framerate",
            &frame_rate.clamp(1.0, MAX_FRAME_RATE).to_string(),
        ])
        .args(["-i", "-"])
        // Output: H.264 in an MP4 that any browser and player can show.
        .args(["-c:v", "libx264", "-pix_fmt", "yuv420p"])
        .args(["-movflags", "+faststart"])
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
}

/// The highest frame rate we ask an encoder for.
const MAX_FRAME_RATE: f32 = 100.0;

fn round_up_to_even(value: u32) -> u32 {
    value + value % 2
}

fn create_parent_dir(path: &Path) -> Result<(), RecordingError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|err| RecordingError::Io {
            path: parent.to_path_buf(),
            err,
        })?;
    }
    Ok(())
}

/// Copy `image` into the top-left corner of a transparent image of the given size.
fn pad_to(image: &RgbaImage, (width, height): (u32, u32)) -> RgbaImage {
    if image.dimensions() == (width, height) {
        return image.clone();
    }

    let mut padded = RgbaImage::new(width, height);
    for (x, y, pixel) in image.enumerate_pixels() {
        padded.put_pixel(x, y, *pixel);
    }
    padded
}

/// Fit a frame into the fixed recording canvas.
///
/// Smaller frames keep their original size and are padded. Frames that exceed the canvas are
/// scaled down proportionally, then padded along the remaining axis.
fn fit_to_canvas(image: RgbaImage, (width, height): (u32, u32)) -> RgbaImage {
    let (image_width, image_height) = image.dimensions();
    if (image_width, image_height) == (width, height) {
        return image;
    }
    if image_width <= width && image_height <= height {
        return pad_to(&image, (width, height));
    }

    let scale = (width as f64 / image_width as f64).min(height as f64 / image_height as f64);
    let scaled_width = ((image_width as f64 * scale).round() as u32).clamp(1, width);
    let scaled_height = ((image_height as f64 * scale).round() as u32).clamp(1, height);
    let scaled = image::imageops::resize(
        &image,
        scaled_width,
        scaled_height,
        image::imageops::FilterType::Triangle,
    );
    pad_to(&scaled, (width, height))
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::fit_to_canvas;

    #[test]
    fn smaller_frames_are_padded() {
        let image = RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255]));
        let fitted = fit_to_canvas(image, (4, 4));

        assert_eq!(fitted.dimensions(), (4, 4));
        assert_eq!(fitted[(0, 0)], Rgba([255, 0, 0, 255]));
        assert_eq!(fitted[(3, 3)], Rgba([0, 0, 0, 0]));
    }

    #[test]
    fn larger_frames_are_scaled_to_fit_and_padded() {
        let image = RgbaImage::from_pixel(8, 4, Rgba([255, 0, 0, 255]));
        let fitted = fit_to_canvas(image, (4, 4));

        assert_eq!(fitted.dimensions(), (4, 4));
        assert_eq!(fitted[(3, 1)], Rgba([255, 0, 0, 255]));
        assert_eq!(fitted[(3, 3)], Rgba([0, 0, 0, 0]));
    }

    #[test]
    fn mp4_frames_are_streamed() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("stream.mp4");
        let mut plugin = super::RecordingPlugin::new(super::RecordingOptions::mp4(&path, 10.0));

        plugin.push_frame(0, RgbaImage::from_pixel(4, 4, Rgba([255, 0, 0, 255])));
        plugin.push_frame(0, RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255])));
        plugin.push_frame(0, RgbaImage::from_pixel(8, 8, Rgba([255, 0, 0, 255])));

        let saved_path = plugin.finish().expect("finish recording");
        assert!(saved_path.exists());
    }
}
