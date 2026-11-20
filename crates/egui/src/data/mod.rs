//! All the data sent between egui and the backend

pub mod input;
pub mod output;
mod screenshot_callback;

mod key;
mod physical_key;

pub use key::{Key, NamedKey};
pub use physical_key::{KeyCode, PhysicalKey};
pub use screenshot_callback::ScreenshotCallback;
