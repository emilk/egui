//! JSONL protocol definitions and a runner for closed-loop UI automation.

use std::fmt;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Shared optional metadata for all record types.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "mcp_sse", derive(schemars::JsonSchema))]
pub struct RecordMeta {
    /// Stable identifier for logs and diffs.
    #[serde(default)]
    pub id: Option<String>,

    /// Human-readable description.
    #[serde(default)]
    pub label: Option<String>,

    /// Timestamp string (ISO-8601 or any string).
    #[serde(default)]
    pub ts: Option<String>,
}

/// Script metadata (first record).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "mcp_sse", derive(schemars::JsonSchema))]
pub struct MetaRecord {
    /// Common metadata.
    #[serde(flatten)]
    pub meta: RecordMeta,

    /// Schema version.
    pub schema_version: u32,

    /// Optional application name.
    #[serde(default)]
    pub app: Option<String>,

    /// Optional environment metadata.
    #[serde(default)]
    pub env: Option<Value>,
}

/// Action record for driving the UI.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "mcp_sse", derive(schemars::JsonSchema))]
pub struct ActionRecord {
    /// Common metadata.
    #[serde(flatten)]
    pub meta: RecordMeta,

    /// Action kind.
    pub action: ActionKind,

    /// Target for UI actions that require it.
    #[serde(default)]
    pub target: Option<Target>,

    /// Text payload for type_text.
    #[serde(default)]
    pub text: Option<String>,

    /// Key payload for press_key.
    #[serde(default)]
    pub key: Option<String>,

    /// Optional modifiers for press_key.
    #[serde(default)]
    pub modifiers: Vec<KeyModifier>,

    /// Step count for run_steps.
    #[serde(default)]
    pub steps: Option<u32>,

    /// Step delta time for run_steps (seconds).
    #[serde(default)]
    pub dt: Option<f32>,

    /// Sleep duration for sleep_ms.
    #[serde(default)]
    pub ms: Option<u64>,
}

/// Supported action kinds.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "mcp_sse", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    /// Click the target node.
    Click,

    /// Focus the target node.
    Focus,

    /// Type text into the target node.
    TypeText,

    /// Press a key with optional modifiers.
    PressKey,

    /// Run a fixed number of UI steps.
    RunSteps,

    /// Sleep or advance time by a number of milliseconds.
    SleepMs,
}

/// Expectation record for verifying outcomes.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "mcp_sse", derive(schemars::JsonSchema))]
pub struct ExpectRecord {
    /// Common metadata.
    #[serde(flatten)]
    pub meta: RecordMeta,

    /// Checks that must pass.
    pub checks: Vec<Check>,

    /// Optional deadline (ms) for all checks.
    #[serde(default)]
    pub within_ms: Option<u64>,
}

/// Check variants.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "mcp_sse", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Check {
    /// Check that a UI element exists.
    UiExists {
        /// Target selector.
        target: Target,
    },

    /// Check visible text contains a substring.
    UiTextContains {
        /// Substring to find in visible text.
        value: String,
    },

    /// Check a JSON state path equals a value.
    StatePathEquals {
        /// JSON pointer path (RFC 6901).
        path: String,

        /// Expected JSON value.
        value: Value,
    },

    /// Check a JSON state path contains a string value.
    StatePathContains {
        /// JSON pointer path (RFC 6901).
        path: String,

        /// Substring to find in the JSON string value.
        value: String,
    },
}

/// Target selector used by UI actions and checks.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[cfg_attr(feature = "mcp_sse", derive(schemars::JsonSchema))]
pub struct Target {
    /// Selector type.
    pub by: TargetBy,

    /// Selector value.
    pub value: String,
}

/// Selector types for targets.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "mcp_sse", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum TargetBy {
    /// Match by accessible label.
    Label,

    /// Match by accessibility role.
    Role,

    /// Match by visible text.
    Text,

    /// Match by widget id.
    Id,
}

/// Key modifiers for press_key.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "mcp_sse", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum KeyModifier {
    /// Control modifier (Ctrl).
    Ctrl,

    /// Shift modifier.
    Shift,

    /// Alt modifier.
    Alt,

    #[serde(alias = "cmd")]
    /// Command modifier.
    Command,

    /// macOS Command modifier (alias of Command).
    MacCmd,
}

/// Script entries with line numbers for diagnostics.
#[derive(Clone, Debug, PartialEq)]
pub struct JsonlEntry {
    /// One-based line number.
    pub line: usize,

    /// Parsed record.
    pub record: ScriptRecord,
}

/// Action/expect records stored in a script.
#[derive(Clone, Debug, PartialEq)]
pub enum ScriptRecord {
    /// Action record entry.
    Action(ActionRecord),

    /// Expect record entry.
    Expect(ExpectRecord),
}

/// Parsed JSONL script (meta + ordered records).
#[derive(Clone, Debug, PartialEq)]
pub struct JsonlScript {
    /// Script metadata.
    pub meta: MetaRecord,

    /// Ordered action/expect entries.
    pub records: Vec<JsonlEntry>,
}

impl JsonlScript {
    /// Parse a JSONL script from a string.
    pub fn parse(source: &str) -> Result<Self, JsonlParseError> {
        parse_jsonl_script(source)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum JsonlRecord {
    Meta(MetaRecord),
    Action(ActionRecord),
    Expect(ExpectRecord),
}

/// Parse a JSONL script from a string.
pub fn parse_jsonl_script(source: &str) -> Result<JsonlScript, JsonlParseError> {
    let mut meta: Option<(MetaRecord, usize)> = None;
    let mut records = Vec::new();
    let mut seen_non_empty = false;

    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let line_number = index + 1;
        let record: JsonlRecord =
            serde_json::from_str(trimmed).map_err(|err| JsonlParseError::Json {
                line: line_number,
                message: err.to_string(),
            })?;

        let is_meta = matches!(record, JsonlRecord::Meta(_));
        if !seen_non_empty {
            seen_non_empty = true;
            if !is_meta {
                return Err(JsonlParseError::MetaNotFirst { line: line_number });
            }
        } else if is_meta && meta.is_none() {
            return Err(JsonlParseError::MetaNotFirst { line: line_number });
        }

        match record {
            JsonlRecord::Meta(record) => {
                if meta.is_some() {
                    return Err(JsonlParseError::DuplicateMeta { line: line_number });
                }
                meta = Some((record, line_number));
            }
            JsonlRecord::Action(record) => records.push(JsonlEntry {
                line: line_number,
                record: ScriptRecord::Action(record),
            }),
            JsonlRecord::Expect(record) => records.push(JsonlEntry {
                line: line_number,
                record: ScriptRecord::Expect(record),
            }),
        }
    }

    let (meta, meta_line) = meta.ok_or(JsonlParseError::MissingMeta)?;
    if meta.schema_version != 1 {
        return Err(JsonlParseError::UnsupportedSchema {
            line: meta_line,
            schema_version: meta.schema_version,
        });
    }

    Ok(JsonlScript { meta, records })
}

/// Errors returned while parsing a JSONL script.
#[derive(Debug)]
pub enum JsonlParseError {
    /// JSON decode error.
    Json {
        /// One-based line number.
        line: usize,

        /// Decoder error message.
        message: String,
    },

    /// Missing meta record.
    MissingMeta,

    /// Meta record was not the first non-empty record.
    MetaNotFirst {
        /// One-based line number.
        line: usize,
    },

    /// More than one meta record encountered.
    DuplicateMeta {
        /// One-based line number.
        line: usize,
    },

    /// Unsupported schema version.
    UnsupportedSchema {
        /// One-based line number.
        line: usize,

        /// Unsupported schema version value.
        schema_version: u32,
    },
}

impl fmt::Display for JsonlParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonlParseError::Json { line, message } => {
                write!(f, "json decode error at line {line}: {message}")
            }
            JsonlParseError::MissingMeta => write!(f, "missing meta record"),
            JsonlParseError::MetaNotFirst { line } => {
                write!(f, "meta record must be first (line {line})")
            }
            JsonlParseError::DuplicateMeta { line } => {
                write!(f, "duplicate meta record at line {line}")
            }
            JsonlParseError::UnsupportedSchema {
                line,
                schema_version,
            } => {
                write!(
                    f,
                    "unsupported schema_version {schema_version} at line {line}"
                )
            }
        }
    }
}

impl std::error::Error for JsonlParseError {}

/// Driver that executes JSONL actions and checks.
pub trait JsonlDriver {
    /// Error type returned by driver operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Click a UI target.
    fn click(&mut self, target: &Target) -> Result<(), Self::Error>;

    /// Focus a UI target.
    fn focus(&mut self, target: &Target) -> Result<(), Self::Error>;

    /// Type text into a UI target.
    fn type_text(&mut self, target: &Target, text: &str) -> Result<(), Self::Error>;

    /// Press a key with optional modifiers.
    fn press_key(&mut self, key: &str, modifiers: &[KeyModifier]) -> Result<(), Self::Error>;

    /// Run a number of UI steps.
    fn run_steps(&mut self, steps: u32, dt: Option<f32>) -> Result<(), Self::Error>;

    /// Sleep or advance time by `ms`.
    fn sleep_ms(&mut self, ms: u64) -> Result<(), Self::Error>;

    /// Check whether a UI element exists.
    fn ui_exists(&mut self, target: &Target) -> Result<bool, Self::Error>;

    /// Check whether visible text contains `value`.
    fn ui_text_contains(&mut self, value: &str) -> Result<bool, Self::Error>;

    /// Provide a state snapshot (JSON). Return None if unsupported.
    fn state_snapshot(&mut self) -> Result<Option<Value>, Self::Error>;
}

/// Runner options.
#[derive(Clone, Copy, Debug)]
pub struct JsonlRunnerOptions {
    /// Default deadline for expect records (ms).
    pub default_within_ms: u64,

    /// Poll interval used while waiting (ms).
    pub poll_interval_ms: u64,
}

impl Default for JsonlRunnerOptions {
    fn default() -> Self {
        Self {
            default_within_ms: 3_000,
            poll_interval_ms: 50,
        }
    }
}

/// JSONL runner that drives a [`JsonlDriver`].
pub struct JsonlRunner {
    options: JsonlRunnerOptions,
}

impl JsonlRunner {
    /// Create a runner with custom options.
    pub fn new(options: JsonlRunnerOptions) -> Self {
        Self { options }
    }

    /// Create a runner with default options.
    pub fn default_runner() -> Self {
        Self::new(JsonlRunnerOptions::default())
    }

    /// Run a parsed script with the provided driver.
    pub fn run_script<D: JsonlDriver>(
        &self,
        script: &JsonlScript,
        driver: &mut D,
    ) -> Result<(), JsonlRunError<D::Error>> {
        for entry in &script.records {
            match &entry.record {
                ScriptRecord::Action(record) => {
                    self.run_action(entry, record, driver)?;
                }
                ScriptRecord::Expect(record) => {
                    self.run_expect(entry, record, driver)?;
                }
            }
        }
        Ok(())
    }

    fn run_action<D: JsonlDriver>(
        &self,
        entry: &JsonlEntry,
        record: &ActionRecord,
        driver: &mut D,
    ) -> Result<(), JsonlRunError<D::Error>> {
        let id = record.meta.id.clone();
        match record.action {
            ActionKind::Click => {
                let target = record.target.as_ref().ok_or_else(|| {
                    JsonlRunError::invalid_action(entry.line, id.clone(), "missing target")
                })?;
                driver
                    .click(target)
                    .map_err(|err| JsonlRunError::driver(entry.line, id, err))?;
            }
            ActionKind::Focus => {
                let target = record.target.as_ref().ok_or_else(|| {
                    JsonlRunError::invalid_action(entry.line, id.clone(), "missing target")
                })?;
                driver
                    .focus(target)
                    .map_err(|err| JsonlRunError::driver(entry.line, id, err))?;
            }
            ActionKind::TypeText => {
                let target = record.target.as_ref().ok_or_else(|| {
                    JsonlRunError::invalid_action(entry.line, id.clone(), "missing target")
                })?;
                let text = record.text.as_deref().ok_or_else(|| {
                    JsonlRunError::invalid_action(entry.line, id.clone(), "missing text")
                })?;
                driver
                    .type_text(target, text)
                    .map_err(|err| JsonlRunError::driver(entry.line, id, err))?;
            }
            ActionKind::PressKey => {
                let key = record.key.as_deref().ok_or_else(|| {
                    JsonlRunError::invalid_action(entry.line, id.clone(), "missing key")
                })?;
                driver
                    .press_key(key, &record.modifiers)
                    .map_err(|err| JsonlRunError::driver(entry.line, id, err))?;
            }
            ActionKind::RunSteps => {
                let steps = record.steps.ok_or_else(|| {
                    JsonlRunError::invalid_action(entry.line, id.clone(), "missing steps")
                })?;
                driver
                    .run_steps(steps, record.dt)
                    .map_err(|err| JsonlRunError::driver(entry.line, id, err))?;
            }
            ActionKind::SleepMs => {
                let ms = record.ms.ok_or_else(|| {
                    JsonlRunError::invalid_action(entry.line, id.clone(), "missing ms")
                })?;
                driver
                    .sleep_ms(ms)
                    .map_err(|err| JsonlRunError::driver(entry.line, id, err))?;
            }
        }
        Ok(())
    }

    fn run_expect<D: JsonlDriver>(
        &self,
        entry: &JsonlEntry,
        record: &ExpectRecord,
        driver: &mut D,
    ) -> Result<(), JsonlRunError<D::Error>> {
        let id = record.meta.id.clone();
        if record.checks.is_empty() {
            return Err(JsonlRunError::invalid_action(
                entry.line,
                id,
                "expect record has no checks",
            ));
        }

        let within_ms = record.within_ms.unwrap_or(self.options.default_within_ms);
        let deadline = Instant::now() + Duration::from_millis(within_ms);

        loop {
            if self.expect_checks_pass(entry, record, driver)? {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(JsonlRunError::ExpectTimeout {
                    line: entry.line,
                    id,
                    within_ms,
                });
            }
            if self.options.poll_interval_ms > 0 {
                driver
                    .sleep_ms(self.options.poll_interval_ms)
                    .map_err(|err| JsonlRunError::driver(entry.line, id.clone(), err))?;
            }
        }
    }

    fn expect_checks_pass<D: JsonlDriver>(
        &self,
        entry: &JsonlEntry,
        record: &ExpectRecord,
        driver: &mut D,
    ) -> Result<bool, JsonlRunError<D::Error>> {
        let id = record.meta.id.clone();
        let mut snapshot: Option<Value> = None;

        for check in &record.checks {
            match check {
                Check::UiExists { target } => {
                    let exists = driver
                        .ui_exists(target)
                        .map_err(|err| JsonlRunError::driver(entry.line, id.clone(), err))?;
                    if !exists {
                        return Ok(false);
                    }
                }
                Check::UiTextContains { value } => {
                    let contains = driver
                        .ui_text_contains(value)
                        .map_err(|err| JsonlRunError::driver(entry.line, id.clone(), err))?;
                    if !contains {
                        return Ok(false);
                    }
                }
                Check::StatePathEquals { path, value } => {
                    let snapshot_ref =
                        self.state_snapshot(entry, id.clone(), &mut snapshot, driver)?;
                    let matched = snapshot_ref
                        .pointer(path)
                        .map(|current| current == value)
                        .unwrap_or(false);
                    if !matched {
                        return Ok(false);
                    }
                }
                Check::StatePathContains { path, value } => {
                    let snapshot_ref =
                        self.state_snapshot(entry, id.clone(), &mut snapshot, driver)?;
                    let matched = snapshot_ref
                        .pointer(path)
                        .and_then(Value::as_str)
                        .map(|current| current.contains(value))
                        .unwrap_or(false);
                    if !matched {
                        return Ok(false);
                    }
                }
            }
        }

        Ok(true)
    }

    fn state_snapshot<'a, D: JsonlDriver>(
        &self,
        entry: &JsonlEntry,
        id: Option<String>,
        snapshot: &'a mut Option<Value>,
        driver: &mut D,
    ) -> Result<&'a Value, JsonlRunError<D::Error>> {
        if snapshot.is_none() {
            let next = driver
                .state_snapshot()
                .map_err(|err| JsonlRunError::driver(entry.line, id.clone(), err))?;
            let Some(value) = next else {
                return Err(JsonlRunError::UnsupportedStateCheck {
                    line: entry.line,
                    id,
                });
            };
            *snapshot = Some(value);
        }
        Ok(snapshot.as_ref().expect("snapshot set"))
    }
}

impl Default for JsonlRunner {
    fn default() -> Self {
        Self::default_runner()
    }
}

/// Errors returned while running a JSONL script.
#[derive(Debug)]
pub enum JsonlRunError<E: std::error::Error + 'static> {
    /// Action/expect record invalid for execution.
    InvalidAction {
        /// One-based line number.
        line: usize,

        /// Optional record id.
        id: Option<String>,

        /// Diagnostic message.
        message: String,
    },

    /// Driver error.
    Driver {
        /// One-based line number.
        line: usize,

        /// Optional record id.
        id: Option<String>,

        /// Driver error.
        source: E,
    },

    /// Expectation timed out.
    ExpectTimeout {
        /// One-based line number.
        line: usize,

        /// Optional record id.
        id: Option<String>,

        /// Timeout budget in milliseconds.
        within_ms: u64,
    },

    /// State checks requested but driver does not support snapshots.
    UnsupportedStateCheck {
        /// One-based line number.
        line: usize,

        /// Optional record id.
        id: Option<String>,
    },
}

impl<E: std::error::Error + 'static> JsonlRunError<E> {
    fn driver(line: usize, id: Option<String>, source: E) -> Self {
        Self::Driver { line, id, source }
    }

    fn invalid_action(line: usize, id: Option<String>, message: impl Into<String>) -> Self {
        Self::InvalidAction {
            line,
            id,
            message: message.into(),
        }
    }
}

impl<E: std::error::Error + 'static> fmt::Display for JsonlRunError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonlRunError::InvalidAction { line, id, message } => {
                write!(f, "invalid record at line {line}")?;
                if let Some(id) = id {
                    write!(f, " (id={id})")?;
                }
                write!(f, ": {message}")
            }
            JsonlRunError::Driver { line, id, source } => {
                write!(f, "driver error at line {line}")?;
                if let Some(id) = id {
                    write!(f, " (id={id})")?;
                }
                write!(f, ": {source}")
            }
            JsonlRunError::ExpectTimeout {
                line,
                id,
                within_ms,
            } => {
                write!(f, "expect timeout at line {line}")?;
                if let Some(id) = id {
                    write!(f, " (id={id})")?;
                }
                write!(f, ": within_ms={within_ms}")
            }
            JsonlRunError::UnsupportedStateCheck { line, id } => {
                write!(f, "state checks unsupported at line {line}")?;
                if let Some(id) = id {
                    write!(f, " (id={id})")?;
                }
                Ok(())
            }
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for JsonlRunError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            JsonlRunError::Driver { source, .. } => Some(source),
            _ => None,
        }
    }
}
