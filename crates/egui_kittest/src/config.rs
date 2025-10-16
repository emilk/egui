use crate::OsThreshold;
use std::io;
use std::path::PathBuf;

/// Configuration for `egui_kittest`.
///
/// It's loaded once (per process) by searching for a `kittest.toml` file in the project root
/// (the directory containing `Cargo.lock`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    output_path: PathBuf,

    threshold: f32,
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
    threshold: Option<f32>,
    failed_pixel_count_threshold: Option<usize>,
}

fn find_project_root() -> io::Result<std::path::PathBuf> {
    let mut current_dir = std::env::current_dir()?;

    loop {
        // Check if Cargo.toml exists in this directory
        if current_dir.join("Cargo.lock").exists() {
            return Ok(current_dir);
        }

        // Move up one directory
        if !current_dir.pop() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Project root not found",
            ));
        }
    }
}

fn load_config() -> Config {
    let project_root = find_project_root();

    if let Ok(project_root) = project_root {
        let config_path = project_root.join("kittest.toml");
        if config_path.exists() {
            let config_str =
                std::fs::read_to_string(config_path).expect("Failed to read config file");
            match toml::from_str(&config_str) {
                Ok(config) => return config,
                Err(e) => panic!("Failed to parse config file: {e}")
            };
        }
    }

    Config::default()
}

pub fn config() -> &'static Config {
    Config::get()
}

impl Config {
    pub fn get() -> &'static Self {
        static INSTANCE: std::sync::LazyLock<Config> = std::sync::LazyLock::new(load_config);
        &INSTANCE
    }

    pub fn os_threshold(&self) -> OsThreshold<f32> {
        let fallback = self.threshold;
        OsThreshold {
            windows: self.windows.threshold.unwrap_or(fallback),
            macos: self.mac.threshold.unwrap_or(fallback),
            linux: self.linux.threshold.unwrap_or(fallback),
            fallback,
        }
    }

    pub fn os_failed_pixel_count_threshold(&self) -> OsThreshold<usize> {
        let fallback = self.failed_pixel_count_threshold;
        OsThreshold {
            windows: self.windows.failed_pixel_count_threshold.unwrap_or(fallback),
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

    /// The output path for image snapshots.
    ///
    /// Default is "tests/snapshots".
    pub fn output_path(&self) -> PathBuf {
        self.output_path.clone()
    }
}
