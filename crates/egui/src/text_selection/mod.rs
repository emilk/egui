//! Helpers regarding text selection for labels and text edit.

#[cfg(feature = "accesskit")]
pub mod accesskit_text;

pub mod handle_event;
mod label_text_selection;
pub mod text_cursor_state;
pub mod visuals;

pub use label_text_selection::LabelSelectionState;
pub use text_cursor_state::TextCursorState;
