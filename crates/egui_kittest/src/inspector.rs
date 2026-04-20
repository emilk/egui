//! Connect a [`crate::Harness`] to a `kittest_inspector` process for live debugging.
//!
//! The harness spawns the inspector as a child process with piped stdin/stdout. After every
//! step the harness writes a frame + accesskit tree update to the child's stdin and reads a
//! reply from its stdout, blocking until the user resumes (when paused).

use std::io::{BufReader, BufWriter, Write as _};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use egui::accesskit;
use kittest_inspector::{
    Frame, HarnessMessage, InspectorReply, read_message, write_message,
};

/// Environment variable: when set to a truthy value, every harness auto-launches an inspector.
pub const INSPECTOR_ENV_VAR: &str = "KITTEST_INSPECTOR";

/// Environment variable: explicit path to the `kittest_inspector` binary.
pub const INSPECTOR_PATH_ENV_VAR: &str = "KITTEST_INSPECTOR_PATH";

/// Errors that can occur attaching or talking to the inspector.
#[derive(Debug)]
pub enum InspectorError {
    /// Failed to launch the `kittest_inspector` binary.
    Launch(std::io::Error),
    /// Failed to set up the child's stdio pipes.
    Pipe(String),
}

impl std::fmt::Display for InspectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Launch(err) => write!(
                f,
                "failed to launch kittest_inspector (set {INSPECTOR_PATH_ENV_VAR} or put it on PATH): {err}"
            ),
            Self::Pipe(msg) => write!(f, "inspector pipe setup failed: {msg}"),
        }
    }
}

impl std::error::Error for InspectorError {}

/// An attached inspector. Owned by the [`crate::Harness`].
pub(crate) struct Inspector {
    writer: BufWriter<ChildStdin>,
    reader: BufReader<ChildStdout>,
    /// Keep the child alive until the harness drops.
    _child: Child,
    step: u64,
    label: Option<String>,
    /// True once the connection has failed; we stop trying to send.
    broken: bool,
}

impl Inspector {
    /// Launch a new `kittest_inspector` child process.
    ///
    /// Search order for the binary:
    /// 1. The path in `KITTEST_INSPECTOR_PATH` if set.
    /// 2. `kittest_inspector` from `PATH`.
    pub fn launch(label: Option<String>) -> Result<Self, InspectorError> {
        let bin = std::env::var(INSPECTOR_PATH_ENV_VAR)
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("kittest_inspector"));

        let mut child = Command::new(&bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(InspectorError::Launch)?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| InspectorError::Pipe("missing child stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| InspectorError::Pipe("missing child stdout".into()))?;

        Ok(Self {
            writer: BufWriter::new(stdin),
            reader: BufReader::new(stdout),
            _child: child,
            step: 0,
            label,
            broken: false,
        })
    }

    /// Send the current frame + accesskit tree and block until the inspector replies.
    /// Returns silently on send/receive failure (e.g. the inspector window was closed).
    pub fn send_step(
        &mut self,
        image: &image::RgbaImage,
        pixels_per_point: f32,
        accesskit: Option<accesskit::TreeUpdate>,
    ) {
        if self.broken {
            return;
        }
        self.step = self.step.saturating_add(1);
        let frame = Frame {
            step: self.step,
            width: image.width(),
            height: image.height(),
            pixels_per_point,
            rgba: image.as_raw().clone(),
            accesskit,
            label: self.label.clone(),
        };
        if let Err(err) = write_message(&mut self.writer, &HarnessMessage::Frame(frame)) {
            #[expect(clippy::print_stderr)]
            {
                eprintln!("egui_kittest inspector: send failed: {err}");
            }
            self.broken = true;
            return;
        }
        match read_message::<_, InspectorReply>(&mut self.reader) {
            Ok(InspectorReply::Continue) => {}
            Err(err) => {
                #[expect(clippy::print_stderr)]
                {
                    eprintln!("egui_kittest inspector: read failed: {err}");
                }
                self.broken = true;
            }
        }
    }

    pub fn say_goodbye(&mut self) {
        if self.broken {
            return;
        }
        let _ = write_message(&mut self.writer, &HarnessMessage::Goodbye);
        let _ = self.writer.flush();
    }
}

impl Drop for Inspector {
    fn drop(&mut self) {
        self.say_goodbye();
    }
}

/// Read [`INSPECTOR_ENV_VAR`] once and cache.
pub(crate) fn env_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| match std::env::var(INSPECTOR_ENV_VAR) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => false,
    })
}
