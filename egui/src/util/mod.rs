//! Tools used by Egui, but that doesn't depend on anything in Egui.

pub(crate) mod cache;
mod history;
pub mod mutex;

pub(crate) use cache::Cache;
pub use history::History;
