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

    /// The maximum weighted squared YIQ color distance between two corresponding pixels.
    ///
    /// Pixels that differ by more than this are counted as failing.
    /// This is an absolute, per-pixel value, and does not depend on the image dimensions.
    ///
    /// Default is 0.6.
    threshold: f32,

    /// The number of pixels that may fail the [`Self::threshold`] before the test is
    /// considered failed.
    ///
    /// Default is 0.
    #[serde(alias = "failed_pixel_count_threshold")]
    max_failed_pixels: usize,

    windows: OsConfig,
    mac: OsConfig,
    linux: OsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            output_path: PathBuf::from("tests/snapshots"),
            threshold: 0.6,
            max_failed_pixels: 0,
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

    /// Override the maximum number of failing pixels for this OS.
    #[serde(alias = "failed_pixel_count_threshold")]
    max_failed_pixels: Option<usize>,
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

/// The old name of `max_failed_pixels` is still accepted, but warned about.
fn warn_about_deprecated_keys(config_str: &str) {
    let Ok(config) = toml::from_str::<toml::Table>(config_str) else {
        return;
    };

    let mut sections = vec![("", &config)];
    for name in ["windows", "mac", "linux"] {
        if let Some(table) = config.get(name).and_then(toml::Value::as_table) {
            sections.push((name, table));
        }
    }

    for (section, table) in sections {
        if table.contains_key("failed_pixel_count_threshold") {
            let prefix = if section.is_empty() {
                String::new()
            } else {
                format!("{section}.")
            };
            log::warn!(
                "`{prefix}failed_pixel_count_threshold` in kittest.toml is deprecated; \
                 use `{prefix}max_failed_pixels` instead."
            );
        }
    }
}

fn load_config() -> Config {
    if let Ok(config_path) = find_kittest_toml() {
        match std::fs::read_to_string(&config_path) {
            Ok(config_str) => {
                warn_about_deprecated_keys(&config_str);
                match toml::from_str(&config_str) {
                    Ok(config) => config,
                    Err(err) => panic!("Failed to parse {}: {err}", config_path.display()),
                }
            }
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

    pub fn os_max_failed_pixels(&self) -> crate::OsThreshold<usize> {
        let fallback = self.max_failed_pixels;
        crate::OsThreshold {
            windows: self.windows.max_failed_pixels.unwrap_or(fallback),
            macos: self.mac.max_failed_pixels.unwrap_or(fallback),
            linux: self.linux.max_failed_pixels.unwrap_or(fallback),
            fallback,
        }
    }

    /// The maximum weighted squared YIQ color distance between two corresponding pixels.
    ///
    /// This is an absolute, per-pixel value, and does not depend on the image dimensions.
    ///
    /// Default is 0.6.
    pub fn threshold(&self) -> f32 {
        self.os_threshold().threshold()
    }

    /// The number of pixels that may fail the [`Self::threshold`] before the test is
    /// considered failed.
    ///
    /// Default is 0.
    pub fn max_failed_pixels(&self) -> usize {
        self.os_max_failed_pixels().threshold()
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn deprecated_failed_pixel_count_threshold_key_is_accepted() {
        let config: Config = toml::from_str(
            r"
                failed_pixel_count_threshold = 1

                [windows]
                failed_pixel_count_threshold = 2

                [mac]
                failed_pixel_count_threshold = 3

                [linux]
                failed_pixel_count_threshold = 4
            ",
        )
        .unwrap_or_else(|err| panic!("Failed to parse config: {err}"));

        assert_eq!(config.max_failed_pixels, 1);
        assert_eq!(config.windows.max_failed_pixels, Some(2));
        assert_eq!(config.mac.max_failed_pixels, Some(3));
        assert_eq!(config.linux.max_failed_pixels, Some(4));
    }
}
