//! All the data sent between egui and the backend

pub mod input;
mod key;
pub mod output;
mod screenshot_callback;

pub use key::Key;
pub use screenshot_callback::ScreenshotCallback;
