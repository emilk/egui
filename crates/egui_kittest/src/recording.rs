//! Capture a [`crate::Harness`] session as an animated GIF or a sequence of PNG files.
//!
//! See [`crate::Harness::start_recording`] / [`crate::Harness::finish_recording`].

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use image::RgbaImage;
use image::codecs::gif::{GifEncoder, Repeat};

/// What kind of output to produce when the recording is finished.
#[derive(Debug, Clone)]
pub enum RecordKind {
    /// Save an animated GIF to `path` (looping forever).
    Gif {
        /// Where to write the GIF.
        path: PathBuf,

        /// Frames per second. The GIF spec stores delays in 10 ms ticks,
        /// so frame rates that aren't a divisor of 100 fps will be slightly approximated.
        frame_rate: f32,
    },

    /// Save a sequence of PNG files (`frame_0000.png`, `frame_0001.png`, ...) into `directory`.
    PngSequence {
        /// Directory to write the PNG files into. It will be created if missing.
        directory: PathBuf,
    },
}

/// When to capture a frame during a recording session.
#[derive(Debug, Clone, Copy, Default)]
pub enum RecordingTrigger {
    /// Render after every step. If the rendered frame is byte-identical to the
    /// previously captured frame, drop it.
    ///
    /// This is the default and produces the smallest recordings for typical UIs,
    /// since most steps don't visibly change anything.
    #[default]
    DiffEveryStep,

    /// Render after every step. Keep every frame, even if visually identical.
    EveryStep,

    /// Capture exactly one frame at the end of each [`crate::Harness::run`] call.
    /// No frames are captured during plain [`crate::Harness::step`] calls.
    OnRun,

    /// Capture every `N`-th step. `EveryNSteps(1)` is equivalent to [`Self::EveryStep`].
    EveryNSteps(u32),
}

/// Configuration for a recording session. Pass to [`crate::Harness::start_recording`].
#[derive(Debug, Clone)]
pub struct RecordingOptions {
    /// What output to produce.
    pub kind: RecordKind,

    /// When to capture a frame. Defaults to [`RecordingTrigger::DiffEveryStep`].
    pub trigger: RecordingTrigger,
}

impl RecordingOptions {
    /// Record a GIF at `path` with the default trigger ([`RecordingTrigger::DiffEveryStep`])
    /// and the given frame rate.
    pub fn gif(path: impl Into<PathBuf>, frame_rate: f32) -> Self {
        Self {
            kind: RecordKind::Gif {
                path: path.into(),
                frame_rate,
            },
            trigger: RecordingTrigger::default(),
        }
    }

    /// Record a PNG sequence into `directory` with the default trigger
    /// ([`RecordingTrigger::DiffEveryStep`]).
    pub fn png_sequence(directory: impl Into<PathBuf>) -> Self {
        Self {
            kind: RecordKind::PngSequence {
                directory: directory.into(),
            },
            trigger: RecordingTrigger::default(),
        }
    }

    /// Replace the trigger.
    #[must_use]
    pub fn with_trigger(mut self, trigger: RecordingTrigger) -> Self {
        self.trigger = trigger;
        self
    }
}

/// Errors produced when finishing a recording.
#[derive(Debug)]
pub enum RecordingError {
    /// No recording was active when [`crate::Harness::finish_recording`] was called.
    NotRecording,

    /// Failed to create or write to the output file/directory.
    Io { path: PathBuf, err: std::io::Error },

    /// Failed to encode the GIF.
    Encode(image::ImageError),
}

impl std::fmt::Display for RecordingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRecording => write!(f, "No recording is currently active"),
            Self::Io { path, err } => write!(f, "I/O error writing {}: {err}", path.display()),
            Self::Encode(err) => write!(f, "Failed to encode recording: {err}"),
        }
    }
}

impl std::error::Error for RecordingError {}

impl From<image::ImageError> for RecordingError {
    fn from(err: image::ImageError) -> Self {
        Self::Encode(err)
    }
}

/// How a recording auto-started by the harness should be saved on `Drop`.
#[derive(Debug, Clone, Copy)]
pub(crate) enum AutoSaveMode {
    /// Save only when the test failed. Path resolved to `{output}/failures/{name}.gif`.
    OnFailure,
    /// Save unconditionally (e.g. driven by `KITTEST_RECORD`).
    /// Path resolved to `{output}/recordings/{name}.gif`.
    Always,
}

/// In-memory state of an active recording. Stored on the [`crate::Harness`].
pub(crate) struct RecordingState {
    pub(crate) options: RecordingOptions,
    pub(crate) frames: Vec<RgbaImage>,
    pub(crate) last_frame: Option<RgbaImage>,
    pub(crate) step_counter: u32,
    /// Set when the recording was started automatically by the harness (config or env var)
    /// rather than by an explicit `start_recording` call. Drives the `Drop` save path.
    pub(crate) auto_save_mode: Option<AutoSaveMode>,
}

impl RecordingState {
    pub(crate) fn new(options: RecordingOptions) -> Self {
        Self {
            options,
            frames: Vec::new(),
            last_frame: None,
            step_counter: 0,
            auto_save_mode: None,
        }
    }

    pub(crate) fn with_auto_save(mut self, mode: AutoSaveMode) -> Self {
        self.auto_save_mode = Some(mode);
        self
    }

    /// Decide whether to capture a frame on this tick.
    ///
    /// `after_run` is true when the call site is the end of [`crate::Harness::run`]
    /// (used by the [`RecordingTrigger::OnRun`] trigger). Other triggers fire only on
    /// per-step ticks (`after_run == false`).
    pub(crate) fn should_capture(&mut self, after_run: bool) -> bool {
        match self.options.trigger {
            RecordingTrigger::DiffEveryStep | RecordingTrigger::EveryStep => !after_run,
            RecordingTrigger::OnRun => after_run,
            RecordingTrigger::EveryNSteps(n) => {
                if after_run {
                    return false;
                }
                let n = n.max(1);
                let counter = self.step_counter;
                self.step_counter = self.step_counter.wrapping_add(1);
                counter.is_multiple_of(n)
            }
        }
    }

    /// Push a freshly rendered frame, applying the configured diffing policy.
    pub(crate) fn push_frame(&mut self, image: RgbaImage) {
        if matches!(self.options.trigger, RecordingTrigger::DiffEveryStep) {
            if let Some(prev) = &self.last_frame
                && prev.as_raw() == image.as_raw()
            {
                return;
            }
            self.last_frame = Some(image.clone());
        }
        self.frames.push(image);
    }

    pub(crate) fn save(self) -> Result<(), RecordingError> {
        match self.options.kind {
            RecordKind::Gif { path, frame_rate } => save_gif(&path, &self.frames, frame_rate),
            RecordKind::PngSequence { directory } => save_png_sequence(&directory, &self.frames),
        }
    }
}

fn save_gif(path: &Path, frames: &[RgbaImage], frame_rate: f32) -> Result<(), RecordingError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|err| RecordingError::Io {
            path: parent.to_path_buf(),
            err,
        })?;
    }

    let file = File::create(path).map_err(|err| RecordingError::Io {
        path: path.to_path_buf(),
        err,
    })?;
    let writer = BufWriter::new(file);
    let mut encoder = GifEncoder::new(writer);
    encoder.set_repeat(Repeat::Infinite)?;

    let denom = frame_rate.max(0.1).round().clamp(1.0, u32::MAX as f32) as u32;
    let frame_delay = image::Delay::from_numer_denom_ms(1000, denom);
    // Hold the final frame for a full second so the loop point is obvious.
    let final_delay = image::Delay::from_numer_denom_ms(1000, 1);

    let last_idx = frames.len().saturating_sub(1);
    for (i, frame) in frames.iter().enumerate() {
        let delay = if i == last_idx { final_delay } else { frame_delay };
        let frame = image::Frame::from_parts(frame.clone(), 0, 0, delay);
        encoder.encode_frame(frame)?;
    }
    Ok(())
}

/// Name of the environment variable that enables auto-recording for every harness in the process.
///
/// When set to `1` / `true` / `yes`, every harness records itself and saves a GIF to
/// `{snapshot_output}/recordings/{test_name}.gif` when dropped (regardless of pass/fail).
pub const RECORD_ENV_VAR: &str = "KITTEST_RECORD";

/// Read [`RECORD_ENV_VAR`] once and cache the result.
pub(crate) fn record_env_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| match std::env::var(RECORD_ENV_VAR) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => false,
    })
}

fn save_png_sequence(directory: &Path, frames: &[RgbaImage]) -> Result<(), RecordingError> {
    std::fs::create_dir_all(directory).map_err(|err| RecordingError::Io {
        path: directory.to_path_buf(),
        err,
    })?;

    for (i, frame) in frames.iter().enumerate() {
        let path = directory.join(format!("frame_{i:04}.png"));
        frame.save(&path).map_err(|err| match err {
            image::ImageError::IoError(io_err) => RecordingError::Io { path, err: io_err },
            other => RecordingError::Encode(other),
        })?;
    }
    Ok(())
}
