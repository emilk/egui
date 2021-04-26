//! Experimental markup language

mod easy_mark_editor;
pub mod easy_mark_parser;
mod easy_mark_viewer;

pub use easy_mark_editor::EasyMarkEditor;
pub use easy_mark_parser as parser;
pub use easy_mark_viewer::easy_mark;
