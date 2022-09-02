//! Experimental markup language

mod easy_mark_editor;
mod easy_mark_highlighter;
pub mod easy_mark_parser;
mod easy_mark_viewer;

pub use easy_mark_editor::EasyMarkEditor;
pub use easy_mark_highlighter::MemoizedEasymarkHighlighter;
pub use easy_mark_parser as parser;
pub use easy_mark_viewer::easy_mark;
