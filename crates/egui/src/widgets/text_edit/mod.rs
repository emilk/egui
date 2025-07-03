mod builder;
mod output;
mod state;
mod text_buffer;
mod widget;

pub use {
    crate::text_selection::TextCursorState, builder::TextEdit, output::TextEditOutput,
    state::TextEditState, text_buffer::TextBuffer,
};
