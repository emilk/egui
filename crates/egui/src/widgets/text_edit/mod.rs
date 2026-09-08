mod builder;
mod completion;
mod output;
mod state;
mod text_buffer;

pub use {
    crate::text_selection::TextCursorState,
    builder::TextEdit,
    completion::{CompletionOutput, CompletionPopup, CompletionQuery, Suggestion},
    output::TextEditOutput,
    state::TextEditState,
    text_buffer::TextBuffer,
};
