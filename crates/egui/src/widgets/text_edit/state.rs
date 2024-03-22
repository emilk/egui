use std::sync::Arc;

use crate::mutex::Mutex;

use crate::*;

use self::text_selection::{CCursorRange, CursorRange, TextCursorState};

pub type TextEditUndoer = crate::util::undoer::Undoer<(CCursorRange, String)>;

/// The text edit state stored between frames.
///
/// Attention: You also need to `store` the updated state.
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut text = String::new();
/// use egui::text::{CCursor, CCursorRange};
///
/// let mut output = egui::TextEdit::singleline(&mut text).show(ui);
///
/// // Create a new selection range
/// let min = CCursor::new(0);
/// let max = CCursor::new(0);
/// let new_range = CCursorRange::two(min, max);
///
/// // Update the state
/// output.state.cursor.set_char_range(Some(new_range));
/// // Store the updated state
/// output.state.store(ui.ctx(), output.response.id);
/// # });
/// ```
#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TextEditState {
    /// Controls the text selection.
    pub cursor: TextCursorState,

    /// Wrapped in Arc for cheaper clones.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) undoer: Arc<Mutex<TextEditUndoer>>,

    // If IME candidate window is shown on this text edit.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) has_ime: bool,

    // cursor range for IME candidate.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) ime_cursor_range: CursorRange,

    // Visual offset when editing singleline text bigger than the width.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) singleline_offset: f32,
}

impl TextEditState {
    pub fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_persisted(id))
    }

    pub fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_persisted(id, self));
    }

    /// The the currently selected range of characters.
    #[deprecated = "Use `self.cursor.char_range` instead"]
    pub fn ccursor_range(&self) -> Option<CCursorRange> {
        self.cursor.char_range()
    }

    /// Sets the currently selected range of characters.
    #[deprecated = "Use `self.cursor.set_char_range` instead"]
    pub fn set_ccursor_range(&mut self, ccursor_range: Option<CCursorRange>) {
        self.cursor.set_char_range(ccursor_range);
    }

    #[deprecated = "Use `self.cursor.set_range` instead"]
    pub fn set_cursor_range(&mut self, cursor_range: Option<CursorRange>) {
        self.cursor.set_range(cursor_range);
    }

    pub fn undoer(&self) -> TextEditUndoer {
        self.undoer.lock().clone()
    }

    pub fn set_undoer(&mut self, undoer: TextEditUndoer) {
        *self.undoer.lock() = undoer;
    }

    pub fn clear_undoer(&mut self) {
        self.set_undoer(TextEditUndoer::default());
    }

    #[deprecated = "Use `self.cursor.range` instead"]
    pub fn cursor_range(&mut self, galley: &Galley) -> Option<CursorRange> {
        self.cursor.range(galley)
    }
}
