//! Miscellaneous tools used by the rest of egui.

pub(crate) mod cache;
mod history;
pub mod undoer;

pub(crate) use cache::Cache;
pub use history::History;

mod lines_with_ending;
pub use lines_with_ending::LinesWithEnding;
