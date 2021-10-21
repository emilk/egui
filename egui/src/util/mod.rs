//! Miscellaneous tools used by the rest of egui.

pub mod cache;
pub(crate) mod fixed_cache;
mod history;
pub mod undoer;

pub use history::History;

pub use epaint::util::{hash, hash_with};
