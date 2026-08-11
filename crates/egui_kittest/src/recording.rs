//! Record an egui session as an animated GIF or a sequence of PNG files.
//!
//! The recorder is an [`egui::Plugin`], so it can record any [`egui::Context`],
//! not just a [`crate::Harness`]. It renders every pass with its own [`TestRenderer`]
//! and keeps the frames in memory until you save them.
//!
//! See [`crate::Harness::start_recording`] / [`crate::Harness::finish_recording`].

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use egui::{Context, FullOutput, TexturesDelta};
use image::RgbaImage;
use image::codecs::gif::{GifEncoder, Repeat};

use crate::TestRenderer;

/// Name of the environment variable that records every [`crate::Harness`] in the process.
///
/// Every harness records itself and saves a GIF when it is dropped,
/// whether the test passed or not:
///
/// - `KITTEST_RECORD=1` writes to `{output_path}/recordings/{test_name}.gif`
/// - `KITTEST_RECORD=open` writes to a temporary file and shows it in the default image viewer
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
}

impl std::fmt::Display for RecordingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRecording => write!(f, "No recording is running"),
            Self::NoFrames => write!(f, "The recording contains no frames"),
            Self::Io { path, err } => write!(f, "Failed to write {}: {err}", path.display()),
            Self::Encode(err) => write!(f, "Failed to encode the recording: {err}"),
        }
    }
}

impl std::error::Error for RecordingError {}

impl From<image::ImageError> for RecordingError {
    fn from(err: image::ImageError) -> Self {
        Self::Encode(err)
    }
}

/// Records an [`egui::Context`] by rendering each pass to an image.
///
/// Register it with [`egui::Context::add_plugin`], or let [`crate::Harness::start_recording`]
/// do it for you.
///
/// The plugin renders with its own [`TestRenderer`] (a `wgpu` one by default), so it does not
/// interfere with the renderer of the harness.
pub struct RecordingPlugin {
    options: RecordingOptions,
    renderer: LazyRenderer,
    frames: Vec<RgbaImage>,
    pass_nr: u32,

    /// While `false` the plugin still tracks textures, but captures no frames.
    active: bool,

    /// Did we give our renderer the whole font atlas?
    ///
    /// A plugin that is registered after the first pass never saw the font texture being
    /// allocated, only the partial updates that follow it.
    uploaded_font_atlas: bool,

    /// Set when the harness started the recording by itself (see [`crate::Harness`]).
    pub(crate) auto_save: Option<AutoSaveMode>,
}

impl std::fmt::Debug for RecordingPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    ///
    /// It still follows the textures of the [`egui::Context`], so that it can render
    /// correctly once it starts.
    pub fn idle() -> Self {
        Self {
            options: RecordingOptions::gif(PathBuf::new(), AUTO_FRAME_RATE),
            renderer: LazyRenderer::default(),
            frames: Vec::new(),
            pass_nr: 0,
            active: false,
            uploaded_font_atlas: false,
            auto_save: None,
        }
    }

    /// Render with this renderer instead of the default `wgpu` one.
    ///
    /// The renderer must be `Send + Sync`, because [`egui::Plugin`] requires it.
    #[inline]
    #[must_use]
    pub fn with_renderer(mut self, renderer: impl TestRenderer + Send + Sync + 'static) -> Self {
        self.renderer = LazyRenderer::Ready(Box::new(renderer));
        self
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
        self.active = true;
        self.auto_save = None;
    }

    /// Stop capturing and drop all frames.
    pub fn stop(&mut self) {
        self.frames.clear();
        self.active = false;
        self.auto_save = None;
    }

    /// Write the captured frames to disk.
    ///
    /// # Errors
    /// Returns an error if there are no frames, or if writing fails.
    pub fn save(&self) -> Result<(), RecordingError> {
        if self.frames.is_empty() {
            return Err(RecordingError::NoFrames);
        }

        match &self.options.kind {
            RecordKind::Gif { path, frame_rate } => save_gif(path, &self.frames, *frame_rate),
            RecordKind::PngSequence { directory } => save_png_sequence(directory, &self.frames),
        }
    }

    /// Where the recording will be written.
    pub(crate) fn output_path(&self) -> &Path {
        match &self.options.kind {
            RecordKind::Gif { path, .. } => path,
            RecordKind::PngSequence { directory } => directory,
        }
    }

    /// Change where the recording will be written.
    pub(crate) fn set_output_path(&mut self, new_path: PathBuf) {
        match &mut self.options.kind {
            RecordKind::Gif { path, .. } => *path = new_path,
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
    fn push_frame(&mut self, image: RgbaImage) {
        if matches!(self.options.trigger, RecordingTrigger::ChangedFrames)
            && let Some(previous) = self.frames.last()
            && previous.as_raw() == image.as_raw()
        {
            return;
        }

        self.frames.push(image);
    }
}

impl egui::Plugin for RecordingPlugin {
    fn debug_name(&self) -> &'static str {
        "egui_kittest::RecordingPlugin"
    }

    fn output_hook(&mut self, ctx: &Context, output: &mut FullOutput) {
        if !self.uploaded_font_atlas {
            self.uploaded_font_atlas = true;
            self.renderer.handle_delta(&mut font_atlas_delta(ctx));
        }

        // Our renderer needs the same textures as the renderer of the integration,
        // so apply a copy of the deltas. Do this even while inactive, so that we can
        // start recording at any time.
        let mut textures_delta = output.textures_delta.clone();
        self.renderer.handle_delta(&mut textures_delta);

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

        // `FullOutput` cannot be cloned without cloning the texture deltas
        // (which panic if they are dropped unapplied), so build the render input by hand.
        // Renderers only need the shapes.
        let mut shapes = output.shapes.clone();
        crate::push_cursor_shape(ctx, &mut shapes);

        let render_output = FullOutput {
            shapes,
            pixels_per_point: output.pixels_per_point,
            viewport_output: output.viewport_output.clone(),
            ..Default::default()
        };

        match self.renderer.render(ctx, &render_output) {
            Ok(image) => self.push_frame(image),
            Err(err) => {
                log::error!("egui_kittest recording: failed to render a frame: {err}");
                if self.renderer.is_failed() {
                    // Nothing will ever render, so stop instead of complaining every pass.
                    self.active = false;
                }
            }
        }
    }
}

/// How a recording that the harness started by itself is saved when the harness is dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutoSaveMode {
    /// Save only if the test failed. Written to `{output_path}/failures/{test_name}.gif`.
    OnFailure,

    /// Always save. Written to `{output_path}/recordings/{test_name}.gif`.
    Always,

    /// Always save to a temporary file, and show it in the default image viewer.
    Open,
}

impl AutoSaveMode {
    /// Where to write the recording of the test we are running.
    fn path(self) -> PathBuf {
        let name = std::thread::current()
            .name()
            .map_or_else(|| "recording".to_owned(), sanitize_file_name);

        let subdirectory = match self {
            Self::OnFailure => "failures",
            Self::Always => "recordings",
            Self::Open => {
                if let Some(path) = temp_gif_path(&name) {
                    return path;
                }
                "recordings" // Fall back to a normal recording.
            }
        };

        crate::config::config()
            .output_path()
            .join(subdirectory)
            .join(format!("{name}.gif"))
    }
}

/// A GIF in the temporary directory, which we keep after the test, so that the image
/// viewer can still read it.
fn temp_gif_path(name: &str) -> Option<PathBuf> {
    tempfile::Builder::new()
        .disable_cleanup(true)
        .prefix(&format!("kittest-recording-{name}-"))
        .suffix(".gif")
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

        match value.trim().to_ascii_lowercase().as_str() {
            "open" => Some(AutoSaveMode::Open),
            "1" | "true" | "yes" | "on" => Some(AutoSaveMode::Always),
            "" | "0" | "false" | "no" | "off" => None,
            other => {
                log::warn!("Ignoring {RECORD_ENV_VAR}={other:?}: expected `1` or `open`");
                None
            }
        }
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
    /// The recording renders with its own renderer, which by default needs the `wgpu` feature.
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
        self.recording_auto_save = None;
    }

    /// Stop the recording and write it to disk.
    ///
    /// # Errors
    /// Returns [`RecordingError::NotRecording`] if nothing was being recorded,
    /// [`RecordingError::NoFrames`] if no frame was captured,
    /// or an I/O or encoding error if writing failed.
    pub fn finish_recording(&mut self) -> Result<(), RecordingError> {
        self.recording_auto_save = None;

        let result = self.ctx.with_plugin::<RecordingPlugin, _>(|plugin| {
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

    /// Start recording if the environment variable or the `kittest.toml` asks for it.
    pub(crate) fn maybe_start_auto_recording(&mut self) {
        let mode = if let Some(mode) = record_env_var() {
            mode
        } else if crate::config::config().save_gif_on_failure() {
            AutoSaveMode::OnFailure
        } else {
            return;
        };

        // The file name contains the test name, which we only look up when we save,
        // so record to a placeholder path for now.
        let options = RecordingOptions::gif(PathBuf::new(), AUTO_FRAME_RATE);
        install(&self.ctx, options, Some(mode));
        self.recording_auto_save = Some(AutoSaveOnDrop {
            ctx: self.ctx.clone(),
        });
    }
}

/// Register an idle [`RecordingPlugin`], if there is none yet.
///
/// The harness does this before the first pass, so that the plugin sees every texture that
/// egui allocates, no matter when the recording starts.
pub(crate) fn install_idle(ctx: &Context) {
    ctx.add_plugin(RecordingPlugin::idle());
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

/// Saves a recording that the harness started by itself, when the harness is dropped.
pub(crate) struct AutoSaveOnDrop {
    pub ctx: Context,
}

#[expect(clippy::print_stderr)] // We are (probably) in a panic, so logging may not be shown.
impl Drop for AutoSaveOnDrop {
    fn drop(&mut self) {
        self.ctx.with_plugin::<RecordingPlugin, _>(|plugin| {
            let Some(mode) = plugin.auto_save.take() else {
                return;
            };

            // A failing test panics, either from an assert or from the snapshot results,
            // which are dropped before this.
            if mode == AutoSaveMode::OnFailure && !std::thread::panicking() {
                plugin.stop();
                return;
            }

            plugin.set_output_path(mode.path());
            let path = plugin.output_path().to_path_buf();

            match plugin.save() {
                Ok(()) => {
                    eprintln!("egui_kittest: saved a recording to {}", path.display());

                    if mode == AutoSaveMode::Open
                        && let Err(err) = open::that_detached(&path)
                    {
                        eprintln!(
                            "egui_kittest: failed to open {} in the default image viewer: {err}",
                            path.display()
                        );
                    }
                }
                Err(RecordingError::NoFrames) => {}
                Err(err) => eprintln!("egui_kittest: failed to save the recording: {err}"),
            }

            plugin.stop();
        });
    }
}

// ----------------------------------------------------------------------------
// Renderer

/// A [`TestRenderer`] that is created when it is first used.
///
/// This mirrors [`crate::LazyRenderer`], but is `Send + Sync`, as [`egui::Plugin`] requires.
enum LazyRenderer {
    Uninitialized {
        textures_delta: TexturesDelta,
    },
    Ready(Box<dyn TestRenderer + Send + Sync>),

    /// We failed to create a renderer, and already told the user about it.
    #[cfg_attr(feature = "wgpu", expect(dead_code))]
    // Only reachable without the `wgpu` feature.
    Failed,
}

impl Default for LazyRenderer {
    fn default() -> Self {
        Self::Uninitialized {
            textures_delta: TexturesDelta::default(),
        }
    }
}

/// A delta that sets the whole font atlas, as it looks right now.
fn font_atlas_delta(ctx: &Context) -> TexturesDelta {
    let image = ctx.fonts(|fonts| fonts.image());

    let mut delta = TexturesDelta::default();
    delta.push(
        egui::TextureId::default(), // The font atlas is always the first texture.
        egui::epaint::ImageDelta::full(image, egui::TextureOptions::default()),
    );
    delta
}

impl LazyRenderer {
    fn handle_delta(&mut self, delta: &mut TexturesDelta) {
        match self {
            Self::Uninitialized { textures_delta } => textures_delta.append(std::mem::take(delta)),
            Self::Ready(renderer) => renderer.handle_delta(delta),
            Self::Failed => delta.clear(), // Don't panic when the delta is dropped.
        }
    }

    fn render(&mut self, ctx: &Context, output: &FullOutput) -> Result<RgbaImage, String> {
        if let Self::Uninitialized { textures_delta } = self {
            #[cfg(feature = "wgpu")]
            {
                let mut renderer = crate::wgpu::WgpuTestRenderer::new();
                renderer.handle_delta(textures_delta);
                *self = Self::Ready(Box::new(renderer));
            }

            #[cfg(not(feature = "wgpu"))]
            {
                textures_delta.clear(); // Don't panic when the deltas are dropped.
                *self = Self::Failed;
            }
        }

        match self {
            Self::Ready(renderer) => renderer.render(ctx, output),
            Self::Uninitialized { .. } | Self::Failed => Err("A recording needs a renderer. \
                Enable the `wgpu` feature, or pass one to `RecordingPlugin::with_renderer`."
                .to_owned()),
        }
    }

    /// Will this renderer never render anything?
    fn is_failed(&self) -> bool {
        matches!(self, Self::Failed)
    }
}

impl Drop for LazyRenderer {
    fn drop(&mut self) {
        if let Self::Uninitialized { textures_delta } = self {
            textures_delta.clear(); // Don't panic when dropping unapplied deltas.
        }
    }
}

// ----------------------------------------------------------------------------
// Saving

fn save_gif(path: &Path, frames: &[RgbaImage], frame_rate: f32) -> Result<(), RecordingError> {
    create_parent_dir(path)?;

    let file = File::create(path).map_err(|err| RecordingError::Io {
        path: path.to_path_buf(),
        err,
    })?;
    let mut encoder = GifEncoder::new(BufWriter::new(file));
    encoder.set_repeat(Repeat::Infinite)?;

    let fps = frame_rate.clamp(1.0, 100.0).round() as u32;
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

    Ok(())
}

fn save_png_sequence(directory: &Path, frames: &[RgbaImage]) -> Result<(), RecordingError> {
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

    Ok(())
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
