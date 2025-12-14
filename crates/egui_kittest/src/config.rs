#![cfg(feature = "snapshot")]

use std::io;
use std::path::PathBuf;

/// Configuration for `egui_kittest`.
///
/// It's loaded once (per process) by searching for a `kittest.toml` file in the project root
/// (the directory containing `Cargo.lock`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// The output path for image snapshots.
    ///
    /// Default is "tests/snapshots" (relative to the working directory / crate root).
    output_path: PathBuf,

    /// The per-pixel threshold.
    ///
    /// Default is 0.6.
    threshold: f32,

    /// The number of pixels that can differ before the test is considered failed.
    ///
    /// Default is 0.
    failed_pixel_count_threshold: usize,

    windows: OsConfig,
    mac: OsConfig,
    linux: OsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            output_path: PathBuf::from("tests/snapshots"),
            threshold: 0.6,
            failed_pixel_count_threshold: 0,
            windows: Default::default(),
            mac: Default::default(),
            linux: Default::default(),
        }
    }
}
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OsConfig {
    /// Override the per-pixel threshold for this OS.
    threshold: Option<f32>,

    /// Override the failed pixel count threshold for this OS.
    failed_pixel_count_threshold: Option<usize>,
}

fn find_kittest_toml() -> io::Result<std::path::PathBuf> {
    let mut current_dir = std::env::current_dir()?;

    loop {
        let current_kittest = current_dir.join("kittest.toml");
        // Check if Cargo.toml exists in this directory
        if current_kittest.exists() {
            return Ok(current_kittest);
        }

        // Move up one directory
        if !current_dir.pop() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "kittest.toml not found",
            ));
        }
    }
}

fn load_config() -> Config {
    if let Ok(config_path) = find_kittest_toml() {
        match std::fs::read_to_string(&config_path) {
            Ok(config_str) => match toml::from_str(&config_str) {
                Ok(config) => config,
                Err(e) => panic!("Failed to parse {}: {e}", &config_path.display()),
            },
            Err(err) => {
                panic!("Failed to read {}: {}", config_path.display(), err);
            }
        }
    } else {
        Config::default()
    }
}

/// Get the global configuration.
///
/// See [`Config::global`] for details.
pub fn config() -> &'static Config {
    Config::global()
}

impl Config {
    /// Get or load the global configuration.
    ///
    /// This is either
    ///  - Based on a `kittest.toml`, found by searching from the current working directory
    ///    (for tests that is the crate root) upwards.
    ///  - The default [Config], if no `kittest.toml` is found.
    pub fn global() -> &'static Self {
        static INSTANCE: std::sync::LazyLock<Config> = std::sync::LazyLock::new(load_config);
        &INSTANCE
    }

    /// The output path for image snapshots.
    ///
    /// Default is "tests/snapshots".
    pub fn output_path(&self) -> PathBuf {
        self.output_path.clone()
    }
}

#[cfg(feature = "snapshot")]
impl Config {
    pub fn os_threshold(&self) -> crate::OsThreshold<f32> {
        let fallback = self.threshold;
        crate::OsThreshold {
            windows: self.windows.threshold.unwrap_or(fallback),
            macos: self.mac.threshold.unwrap_or(fallback),
            linux: self.linux.threshold.unwrap_or(fallback),
            fallback,
        }
    }

    pub fn os_failed_pixel_count_threshold(&self) -> crate::OsThreshold<usize> {
        let fallback = self.failed_pixel_count_threshold;
        crate::OsThreshold {
            windows: self
                .windows
                .failed_pixel_count_threshold
                .unwrap_or(fallback),
            macos: self.mac.failed_pixel_count_threshold.unwrap_or(fallback),
            linux: self.linux.failed_pixel_count_threshold.unwrap_or(fallback),
            fallback,
        }
    }

    /// The threshold.
    ///
    /// Default is 1.0.
    pub fn threshold(&self) -> f32 {
        self.os_threshold().threshold()
    }

    /// The number of pixels that can differ before the test is considered failed.
    ///
    /// Default is 0.
    pub fn failed_pixel_count_threshold(&self) -> usize {
        self.os_failed_pixel_count_threshold().threshold()
    }
}
