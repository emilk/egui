//! Helpers regarding text selection for labels and text edit.

pub mod accesskit_text;

mod cursor_range;
mod label_text_selection;
pub mod text_cursor_state;
pub mod visuals;

pub use cursor_range::CCursorRange;
pub use label_text_selection::LabelSelectionState;
pub use text_cursor_state::TextCursorState;

pub(crate) fn scroll_to_cursor(
    ui: &crate::Ui,
    cursor_rect: emath::Rect,
    paged_from: Option<emath::Rect>,
) {
    let mut cursor_rect = cursor_rect;
    if let Some(from) = paged_from {
        let delta = emath::vec2(0.0, from.top() - cursor_rect.top());
        ui.scroll_with_delta(delta);
        cursor_rect = cursor_rect.translate(delta);
    }
    ui.scroll_to_rect(cursor_rect, None);
}
