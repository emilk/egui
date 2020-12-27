//! Miscellaneous tools used by the rest of Egui.

pub(crate) mod cache;
mod history;
pub mod mutex;
pub mod undoer;

pub(crate) use cache::Cache;
pub use history::History;
