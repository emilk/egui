//! All the data sent between egui and the backend

pub mod input;
pub mod output;
mod screenshot_callback;

mod full_key;
mod key;
mod key_variant;
mod physical_key;
mod shortcut_key;

pub use full_key::FullKey;
pub use key::{Key, NamedKey};
pub use key_variant::KeyVariant;
pub use physical_key::{KeyCode, PhysicalKey};
pub use screenshot_callback::ScreenshotCallback;
pub use shortcut_key::{ShortcutKey, ShortcutKeyExt};
