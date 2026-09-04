//! Record an egui session as an animated GIF, an MP4 video, or a sequence of PNG files.
//!
//! The recorder is an [`egui::Plugin`], so it can record any [`egui::Context`],
//! not just a [`crate::Harness`]. It requests a screenshot from the integration for each
//! selected pass and keeps the frames in memory until you save them.
//!
//! See [`crate::Harness::start_recording`] / [`crate::Harness::finish_recording`].

use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use egui::{Context, FullOutput};
use image::RgbaImage;
use image::codecs::gif::{GifEncoder, Repeat};

/// Name of the environment variable that records every [`crate::Harness`] in the process.
///
/// Every harness records itself and saves a GIF when it is dropped,
/// whether the test passed or not:
///
/// - `KITTEST_RECORD=1` writes to `{output_path}/recordings/{test_name}.gif`
/// - `KITTEST_RECORD=mp4` writes an MP4 instead of a GIF
/// - `KITTEST_RECORD=open` writes to a temporary file and shows it in the default viewer
/// - `KITTEST_RECORD=open-mp4` does both
pub const RECORD_ENV_VAR: &str = "KITTEST_RECORD";

/// What to write when the recording is saved.
#[derive(Debug, Clone)]
pub enum RecordKind {
    /// Save an animated GIF to `path` (looping forever).
    Gif {
        /// Where to write the GIF.
        path: PathBuf,

        /// Frames per second. The GIF format stores delays in 10 ms ticks,
        /// so a frame rate that is not a divisor of 100 is approximated.
        frame_rate: f32,
    },

    /// Save an H.264 MP4 to `path`.
    ///
    /// This pipes the frames into [`ffmpeg`](https://ffmpeg.org/), which must be installed
    /// and on the `PATH`. Without it we save a GIF next to `path` instead.
    ///
    /// MP4 has no alpha channel, so transparent pixels turn black.
    Mp4 {
        /// Where to write the video.
        path: PathBuf,

        /// Frames per second.
        frame_rate: f32,
    },

    /// Save a sequence of PNG files (`frame_0000.png`, `frame_0001.png`, …) into `directory`.
    PngSequence {
        /// Directory to write the PNG files into. It is created if it is missing.
        directory: PathBuf,
    },
}

/// Which passes to capture.
///
/// Passes that egui discards (see [`egui::Context::request_discard`]) are never captured,
/// since they are never shown to the user either.
#[derive(Debug, Clone, Copy, Default)]
pub enum RecordingTrigger {
    /// Capture every pass, but drop a frame if it looks exactly like the frame before it.
    ///
    /// This is the default. It gives the smallest recordings, because most passes
    /// change nothing on screen.
    #[default]
    ChangedFrames,

    /// Capture every pass, even if nothing changed.
    EveryFrame,

    /// Capture every `N`-th pass. `EveryNthFrame(1)` is the same as [`Self::EveryFrame`].
    EveryNthFrame(u32),
}

/// How to record. Pass this to [`crate::Harness::start_recording`] or [`RecordingPlugin::new`].
#[derive(Debug, Clone)]
pub struct RecordingOptions {
    /// What to write when the recording is saved.
    pub kind: RecordKind,

    /// Which passes to capture. Defaults to [`RecordingTrigger::ChangedFrames`].
    pub trigger: RecordingTrigger,
}

impl RecordingOptions {
    /// Record a GIF to `path` at the given frame rate,
    /// with the default trigger ([`RecordingTrigger::ChangedFrames`]).
    pub fn gif(path: impl Into<PathBuf>, frame_rate: f32) -> Self {
        Self {
            kind: RecordKind::Gif {
                path: path.into(),
                frame_rate,
            },
            trigger: RecordingTrigger::default(),
        }
    }

    /// Record an MP4 to `path` at the given frame rate,
    /// with the default trigger ([`RecordingTrigger::ChangedFrames`]).
    ///
    /// Needs [`ffmpeg`](https://ffmpeg.org/) on the `PATH`; see [`RecordKind::Mp4`].
    pub fn mp4(path: impl Into<PathBuf>, frame_rate: f32) -> Self {
        Self {
            kind: RecordKind::Mp4 {
                path: path.into(),
                frame_rate,
            },
            trigger: RecordingTrigger::default(),
        }
    }

    /// Record a PNG sequence into `directory`,
    /// with the default trigger ([`RecordingTrigger::ChangedFrames`]).
    pub fn png_sequence(directory: impl Into<PathBuf>) -> Self {
        Self {
            kind: RecordKind::PngSequence {
                directory: directory.into(),
            },
            trigger: RecordingTrigger::default(),
        }
    }

    /// Replace the trigger.
    #[inline]
    #[must_use]
    pub fn with_trigger(mut self, trigger: RecordingTrigger) -> Self {
        self.trigger = trigger;
        self
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

    /// Failed to encode the image data.
    Encode(image::ImageError),

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
            Self::Encode(err) => write!(f, "Failed to encode the recording: {err}"),
            Self::Ffmpeg { message } => write!(f, "ffmpeg failed: {message}"),
        }
    }
}

impl core::error::Error for RecordingError {}

impl From<image::ImageError> for RecordingError {
    fn from(err: image::ImageError) -> Self {
        Self::Encode(err)
    }
}

/// Records an [`egui::Context`] using the integration's screenshot support.
///
/// Register it with [`egui::Context::add_plugin`], or let [`crate::Harness::start_recording`]
/// do it for you.
///
/// The plugin is self-contained: it adds a [`egui::ViewportCommand::ScreenshotCallback`] to each
/// selected pass and receives the rendered image directly from the integration.
pub struct RecordingPlugin {
    options: RecordingOptions,
    frames: Vec<RgbaImage>,
    pass_nr: u32,
    recording_id: u64,

    /// While `false` the plugin captures no frames.
    active: bool,

    /// Set when the harness started the recording by itself (see [`crate::Harness`]).
    pub(crate) auto_save: Option<AutoSave>,
}

impl core::fmt::Debug for RecordingPlugin {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RecordingPlugin")
            .field("options", &self.options)
            .field("active", &self.active)
            .field("frames", &self.frames.len())
            .finish_non_exhaustive()
    }
}

impl RecordingPlugin {
    /// Create a plugin that starts recording right away.
    pub fn new(options: RecordingOptions) -> Self {
        Self {
            options,
            active: true,
            ..Self::idle()
        }
    }

    /// Create a plugin that captures nothing until [`Self::restart`] is called.
    pub fn idle() -> Self {
        Self {
            options: RecordingOptions::gif(PathBuf::new(), AUTO_FRAME_RATE),
            frames: Vec::new(),
            pass_nr: 0,
            recording_id: 0,
            active: false,
            auto_save: None,
        }
    }

    /// The options this recording uses.
    #[inline]
    pub fn options(&self) -> &RecordingOptions {
        &self.options
    }

    /// The options this recording uses, mutably.
    #[inline]
    pub fn options_mut(&mut self) -> &mut RecordingOptions {
        &mut self.options
    }

    /// Is the plugin capturing frames?
    #[inline]
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// The frames captured so far.
    #[inline]
    pub fn frames(&self) -> &[RgbaImage] {
        &self.frames
    }

    /// Start capturing again, with new options. Any earlier frames are dropped.
    pub fn restart(&mut self, options: RecordingOptions) {
        self.options = options;
        self.frames.clear();
        self.pass_nr = 0;
        self.recording_id = self.recording_id.wrapping_add(1);
        self.active = true;
        self.auto_save = None;
    }

    /// Stop capturing and drop all frames.
    pub fn stop(&mut self) {
        self.frames.clear();
        self.recording_id = self.recording_id.wrapping_add(1);
        self.active = false;
        self.auto_save = None;
    }

    /// Write the captured frames to disk, and return the file or directory that was written.
    ///
    /// The returned path is not always the one in the options: an MP4 falls back to a GIF
    /// when `ffmpeg` is missing.
    ///
    /// # Errors
    /// Returns an error if there are no frames, or if writing fails.
    pub fn save(&self) -> Result<PathBuf, RecordingError> {
        if self.frames.is_empty() {
            return Err(RecordingError::NoFrames);
        }

        match &self.options.kind {
            RecordKind::Gif { path, frame_rate } => save_gif(path, &self.frames, *frame_rate),
            RecordKind::Mp4 { path, frame_rate } => save_mp4(path, &self.frames, *frame_rate),
            RecordKind::PngSequence { directory } => save_png_sequence(directory, &self.frames),
        }
    }

    /// Change where the recording will be written.
    pub(crate) fn set_output_path(&mut self, new_path: PathBuf) {
        match &mut self.options.kind {
            RecordKind::Gif { path, .. } | RecordKind::Mp4 { path, .. } => *path = new_path,
            RecordKind::PngSequence { directory } => *directory = new_path,
        }
    }

    /// Should we capture this pass?
    fn should_capture(&mut self) -> bool {
        let pass_nr = self.pass_nr;
        self.pass_nr = self.pass_nr.wrapping_add(1);

        match self.options.trigger {
            RecordingTrigger::ChangedFrames | RecordingTrigger::EveryFrame => true,
            RecordingTrigger::EveryNthFrame(n) => pass_nr.is_multiple_of(n.max(1)),
        }
    }

    /// Add a frame, dropping it if the trigger says it is a duplicate.
    fn push_frame(&mut self, recording_id: u64, image: RgbaImage) {
        // A screenshot can finish after the recording was stopped or restarted.
        if !self.active || self.recording_id != recording_id {
            return;
        }

        if matches!(self.options.trigger, RecordingTrigger::ChangedFrames)
            && let Some(previous) = self.frames.last()
            && previous.as_raw() == image.as_raw()
        {
            return;
        }

        self.frames.push(image);
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

        if !self.should_capture() {
            return;
        }

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
    /// Always save. Written to `{output_path}/recordings/{test_name}.{ext}`.
    Always,

    /// Always save to a temporary file, and show it in the default viewer.
    Open,
}

/// What such a recording is saved as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutoSaveFormat {
    Gif,
    Mp4,
}

impl AutoSaveFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Gif => "gif",
            Self::Mp4 => "mp4",
        }
    }

    fn options(self, path: PathBuf) -> RecordingOptions {
        match self {
            Self::Gif => RecordingOptions::gif(path, AUTO_FRAME_RATE),
            Self::Mp4 => RecordingOptions::mp4(path, AUTO_FRAME_RATE),
        }
    }
}

/// How a recording that the harness started by itself is saved when the harness is dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AutoSave {
    pub mode: AutoSaveMode,
    pub format: AutoSaveFormat,
}

impl AutoSave {
    /// Where to write the recording of the test we are running.
    fn path(self) -> PathBuf {
        let name = std::thread::current()
            .name()
            .map_or_else(|| "recording".to_owned(), sanitize_file_name);
        let extension = self.format.extension();

        let subdirectory = match self.mode {
            AutoSaveMode::Always => "recordings",
            AutoSaveMode::Open => {
                if let Some(path) = temp_recording_path(&name, extension) {
                    return path;
                }
                "recordings" // Fall back to a normal recording.
            }
        };

        crate::config::config()
            .output_path()
            .join(subdirectory)
            .join(format!("{name}.{extension}"))
    }
}

/// A file in the temporary directory, which we keep after the test, so that the
/// viewer can still read it.
fn temp_recording_path(name: &str, extension: &str) -> Option<PathBuf> {
    tempfile::Builder::new()
        .disable_cleanup(true)
        .prefix(&format!("kittest-recording-{name}-"))
        .suffix(&format!(".{extension}"))
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
pub(crate) fn record_env_var() -> Option<AutoSave> {
    static MODE: std::sync::OnceLock<Option<AutoSave>> = std::sync::OnceLock::new();
    *MODE.get_or_init(|| {
        let value = std::env::var(RECORD_ENV_VAR).ok()?;

        let (mode, format) = match value.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" | "gif" => (AutoSaveMode::Always, AutoSaveFormat::Gif),
            "mp4" => (AutoSaveMode::Always, AutoSaveFormat::Mp4),
            "open" | "open-gif" => (AutoSaveMode::Open, AutoSaveFormat::Gif),
            "open-mp4" => (AutoSaveMode::Open, AutoSaveFormat::Mp4),
            "" | "0" | "false" | "no" | "off" => return None,
            other => {
                log::warn!(
                    "Ignoring {RECORD_ENV_VAR}={other:?}: expected \
                     `1`, `mp4`, `open` or `open-mp4`"
                );
                return None;
            }
        };

        Some(AutoSave { mode, format })
    })
}

// ----------------------------------------------------------------------------
// Harness integration

/// Frame rate of recordings that the harness starts by itself.
const AUTO_FRAME_RATE: f32 = 10.0;

/// A [`crate::Harness`] can record itself.
impl<State> crate::Harness<'_, State> {
    /// Record the rest of this test session.
    ///
    /// One frame is captured per egui pass, as configured by [`RecordingOptions::trigger`].
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
    /// harness.start_recording(RecordingOptions::gif("hello.gif", 10.0));
    /// harness.run();
    /// harness.finish_recording().unwrap();
    /// ```
    pub fn start_recording(&mut self, options: RecordingOptions) {
        install(&self.ctx, options, None);
    }

    /// Stop the recording and write it to disk, returning the path that was written.
    ///
    /// # Errors
    /// Returns [`RecordingError::NotRecording`] if nothing was being recorded,
    /// [`RecordingError::NoFrames`] if no frame was captured,
    /// or an I/O or encoding error if writing failed.
    pub fn finish_recording(&mut self) -> Result<PathBuf, RecordingError> {
        let result = self.ctx.with_plugin::<RecordingPlugin, _>(|plugin| {
            plugin.auto_save = None;
            if !plugin.is_active() {
                return Err(RecordingError::NotRecording);
            }
            let result = plugin.save();
            plugin.stop();
            result
        });

        result.unwrap_or(Err(RecordingError::NotRecording))
    }

    /// Is the harness recording?
    pub fn is_recording(&self) -> bool {
        self.ctx
            .with_plugin::<RecordingPlugin, _>(|plugin| plugin.is_active())
            .unwrap_or(false)
    }

    /// Access the [`RecordingPlugin`], e.g. to read the captured frames.
    ///
    /// Returns `None` if the harness never recorded anything.
    pub fn with_recording<R>(&self, f: impl FnOnce(&mut RecordingPlugin) -> R) -> Option<R> {
        self.ctx.with_plugin::<RecordingPlugin, _>(f)
    }

    /// Start recording if the environment variable asks for it.
    pub(crate) fn maybe_start_auto_recording(&mut self) {
        let Some(auto_save) = record_env_var() else {
            return;
        };

        // The file name contains the test name, which we only look up when we save,
        // so record to a placeholder path for now.
        let options = auto_save.format.options(PathBuf::new());
        install(&self.ctx, options, Some(auto_save));
    }
}

/// Register a [`RecordingPlugin`] on `ctx`, or restart the one that is already registered.
fn install(ctx: &Context, options: RecordingOptions, auto_save: Option<AutoSave>) {
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

        self.set_output_path(auto_save.path());

        match self.save() {
            Ok(path) => {
                eprintln!("egui_kittest: saved a recording to {}", path.display());

                if auto_save.mode == AutoSaveMode::Open
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

        self.stop();
    }
}

// ----------------------------------------------------------------------------
// Saving

fn save_gif(path: &Path, frames: &[RgbaImage], frame_rate: f32) -> Result<PathBuf, RecordingError> {
    create_parent_dir(path)?;

    let file = File::create(path).map_err(|err| RecordingError::Io {
        path: path.to_path_buf(),
        err,
    })?;
    let mut encoder = GifEncoder::new(BufWriter::new(file));
    encoder.set_repeat(Repeat::Infinite)?;

    let fps = frame_rate.clamp(1.0, MAX_FRAME_RATE).round() as u32;
    let frame_delay = image::Delay::from_numer_denom_ms(1000, fps);
    // Hold the last frame for a second, so it is obvious where the loop restarts.
    let last_delay = image::Delay::from_numer_denom_ms(1000, 1);

    // All frames of a GIF share one canvas, so grow the smaller ones to fit.
    let size = max_size(frames);

    let last_index = frames.len() - 1;
    for (i, frame) in frames.iter().enumerate() {
        let delay = if i == last_index {
            last_delay
        } else {
            frame_delay
        };
        let image = pad_to(frame, size);
        encoder.encode_frame(image::Frame::from_parts(image, 0, 0, delay))?;
    }

    Ok(path.to_path_buf())
}

/// The encoder we pipe the frames into. It must be on the `PATH`.
const FFMPEG: &str = "ffmpeg";

/// Encode the frames as an H.264 MP4, or save a GIF if `ffmpeg` is not installed.
fn save_mp4(path: &Path, frames: &[RgbaImage], frame_rate: f32) -> Result<PathBuf, RecordingError> {
    create_parent_dir(path)?;

    // All frames of a video share one size, and H.264 wants both sides to be even.
    let (width, height) = max_size(frames);
    let size = (round_up_to_even(width), round_up_to_even(height));

    let mut child = match spawn_ffmpeg(path, size, frame_rate) {
        Ok(child) => child,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let gif_path = path.with_extension("gif");
            log::warn!(
                "egui_kittest: `{FFMPEG}` is not installed, so {} is saved as {} instead",
                path.display(),
                gif_path.display()
            );
            return save_gif(&gif_path, frames, frame_rate);
        }
        Err(err) => {
            return Err(RecordingError::Io {
                path: PathBuf::from(FFMPEG),
                err,
            });
        }
    };

    // Hold the last frame for a second, like the GIF does.
    let hold = frame_rate.clamp(1.0, MAX_FRAME_RATE).round() as usize;
    let mut frames = frames.iter().chain(core::iter::repeat_n(
        frames.last().expect("`save` rejects empty recordings"),
        hold,
    ));

    // If ffmpeg dies early the pipe breaks; report what it said instead of the pipe error.
    let mut stdin = child.stdin.take().expect("`spawn_ffmpeg` pipes stdin");
    let write_result = frames.try_for_each(|frame| stdin.write_all(pad_to(frame, size).as_raw()));
    drop(stdin); // Closing stdin tells ffmpeg to finish the file.

    let output = child.wait_with_output().map_err(|err| RecordingError::Io {
        path: PathBuf::from(FFMPEG),
        err,
    })?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(RecordingError::Ffmpeg {
            message: if message.is_empty() {
                format!("{} while writing {}", output.status, path.display())
            } else {
                message
            },
        });
    }

    write_result.map_err(|err| RecordingError::Io {
        path: path.to_path_buf(),
        err,
    })?;

    Ok(path.to_path_buf())
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

fn save_png_sequence(directory: &Path, frames: &[RgbaImage]) -> Result<PathBuf, RecordingError> {
    std::fs::create_dir_all(directory).map_err(|err| RecordingError::Io {
        path: directory.to_path_buf(),
        err,
    })?;

    for (i, frame) in frames.iter().enumerate() {
        let path = directory.join(format!("frame_{i:04}.png"));
        frame.save(&path).map_err(|err| match err {
            image::ImageError::IoError(err) => RecordingError::Io { path, err },
            err => RecordingError::Encode(err),
        })?;
    }

    Ok(directory.to_path_buf())
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

/// The size of the largest frame, per axis.
fn max_size(frames: &[RgbaImage]) -> (u32, u32) {
    frames.iter().fold((1, 1), |(w, h), frame| {
        (w.max(frame.width()), h.max(frame.height()))
    })
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
