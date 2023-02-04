//! Miscellaneous tools used by the rest of egui.

pub mod cache;
pub(crate) mod fixed_cache;
pub mod id_type_map;
pub mod undoer;

pub use id_type_map::IdTypeMap;

pub use epaint::emath::History;
pub use epaint::util::{hash, hash_with};
