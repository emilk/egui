//! All the data sent between egui and the backend

pub mod input;
mod key;
pub mod output;
mod physical_key;
mod screenshot_callback;

pub use key::Key;
pub use physical_key::{KeyCode, PhysicalKey};
pub use screenshot_callback::ScreenshotCallback;
