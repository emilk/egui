mod app_icon;
mod epi_integration;
/// Helpers for loading [`egui::IconData`].
pub mod icon_data;
pub mod run;

/// File storage which can be used by native backends.
#[cfg(feature = "persistence")]
pub mod file_storage;
