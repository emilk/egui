/// Implements [`log::Log`] to log messages to `console.log`, `console.warn`, etc.
pub struct WebLogger {
    filter: log::LevelFilter,
}

impl WebLogger {
    /// Install a new `WebLogger`, piping all [`log`] events to the web console.
    pub fn init(filter: log::LevelFilter) -> Result<(), log::SetLoggerError> {
        log::set_max_level(filter);
        log::set_boxed_logger(Box::new(Self::new(filter)))
    }

    /// Create a new [`WebLogger`] with the given filter,
    /// but don't install it.
    pub fn new(filter: log::LevelFilter) -> Self {
        Self { filter }
    }
}

impl log::Log for WebLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        /// Never log anything less serious than a `INFO` from these crates.
        const CRATES_AT_INFO_LEVEL: &[&str] = &[
            // wgpu crates spam a lot on debug level, which is really annoying
            "naga",
            "wgpu_core",
            "wgpu_hal",
        ];

        if CRATES_AT_INFO_LEVEL
            .iter()
            .any(|crate_name| metadata.target().starts_with(crate_name))
        {
            return metadata.level() <= log::LevelFilter::Info;
        }

        metadata.level() <= self.filter
    }

    fn log(&self, record: &log::Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let msg = if let (Some(file), Some(line)) = (record.file(), record.line()) {
            let file = shorten_file_path(file);
            format!("[{}] {file}:{line}: {}", record.target(), record.args())
        } else {
            format!("[{}] {}", record.target(), record.args())
        };

        match record.level() {
            log::Level::Trace => console::trace(&msg),
            log::Level::Debug => console::debug(&msg),
            log::Level::Info => console::info(&msg),
            log::Level::Warn => console::warn(&msg),

            // Using console.error causes crashes for unknown reason
            // https://github.com/emilk/egui/pull/2961
            // log::Level::Error => console::error(&msg),
            log::Level::Error => console::warn(&format!("ERROR: {msg}")),
        }
    }

    fn flush(&self) {}
}

/// js-bindings for console.log, console.warn, etc
mod console {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        /// `console.trace`
        #[wasm_bindgen(js_namespace = console)]
        pub fn trace(s: &str);

        /// `console.debug`
        #[wasm_bindgen(js_namespace = console)]
        pub fn debug(s: &str);

        /// `console.info`
        #[wasm_bindgen(js_namespace = console)]
        pub fn info(s: &str);

        /// `console.warn`
        #[wasm_bindgen(js_namespace = console)]
        pub fn warn(s: &str);

        // Using console.error causes crashes for unknown reason
        // https://github.com/emilk/egui/pull/2961
        // /// `console.error`
        // #[wasm_bindgen(js_namespace = console)]
        // pub fn error(s: &str);
    }
}

/// Shorten a path to a Rust source file.
///
/// Example input:
/// * `/Users/emilk/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.24.1/src/runtime/runtime.rs`
/// * `crates/rerun/src/main.rs`
/// * `/rustc/d5a82bbd26e1ad8b7401f6a718a9c57c96905483/library/core/src/ops/function.rs`
///
/// Example output:
/// * `tokio-1.24.1/src/runtime/runtime.rs`
/// * `rerun/src/main.rs`
/// * `core/src/ops/function.rs`
#[allow(dead_code)] // only used on web and in tests
fn shorten_file_path(file_path: &str) -> &str {
    if let Some(i) = file_path.rfind("/src/") {
        if let Some(prev_slash) = file_path[..i].rfind('/') {
            &file_path[prev_slash + 1..]
        } else {
            file_path
        }
    } else {
        file_path
    }
}

#[test]
fn test_shorten_file_path() {
    for (before, after) in [
        ("/Users/emilk/.cargo/registry/src/github.com-1ecc6299db9ec823/tokio-1.24.1/src/runtime/runtime.rs", "tokio-1.24.1/src/runtime/runtime.rs"),
        ("crates/rerun/src/main.rs", "rerun/src/main.rs"),
        ("/rustc/d5a82bbd26e1ad8b7401f6a718a9c57c96905483/library/core/src/ops/function.rs", "core/src/ops/function.rs"),
        ("/weird/path/file.rs", "/weird/path/file.rs"),
        ]
    {
        assert_eq!(shorten_file_path(before), after);
    }
}
