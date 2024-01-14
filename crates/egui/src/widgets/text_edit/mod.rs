mod builder;
pub mod cursor_interaction;
mod cursor_range;
mod output;
mod state;
mod text_buffer;

#[cfg(feature = "accesskit")]
pub mod accesskit_text;

pub use {
    builder::{paint_cursor_selection, TextEdit},
    cursor_range::*,
    output::TextEditOutput,
    state::TextCursorState,
    state::TextEditState,
    text_buffer::TextBuffer,
};
