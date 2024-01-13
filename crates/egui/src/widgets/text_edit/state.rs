use std::sync::Arc;

use crate::mutex::Mutex;

use crate::*;

use super::{CCursorRange, CursorRange};

pub type TextEditUndoer = crate::util::undoer::Undoer<(CCursorRange, String)>;

/// The state of a text cursor selection.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TextCursorState {
    cursor_range: Option<CursorRange>,

    /// This is what is easiest to work with when editing text,
    /// so users are more likely to read/write this.
    ccursor_range: Option<CCursorRange>,
}

impl TextCursorState {
    pub fn is_empty(&self) -> bool {
        self.cursor_range.is_none() && self.ccursor_range.is_none()
    }

    /// The the currently selected range of characters.
    pub fn char_range(&self) -> Option<CCursorRange> {
        self.ccursor_range.or_else(|| {
            self.cursor_range
                .map(|cursor_range| cursor_range.as_ccursor_range())
        })
    }

    pub fn range(&mut self, galley: &Galley) -> Option<CursorRange> {
        self.cursor_range
            .map(|cursor_range| {
                // We only use the PCursor (paragraph number, and character offset within that paragraph).
                // This is so that if we resize the [`TextEdit`] region, and text wrapping changes,
                // we keep the same byte character offset from the beginning of the text,
                // even though the number of rows changes
                // (each paragraph can be several rows, due to word wrapping).
                // The column (character offset) should be able to extend beyond the last word so that we can
                // go down and still end up on the same column when we return.
                CursorRange {
                    primary: galley.from_pcursor(cursor_range.primary.pcursor),
                    secondary: galley.from_pcursor(cursor_range.secondary.pcursor),
                }
            })
            .or_else(|| {
                self.ccursor_range.map(|ccursor_range| CursorRange {
                    primary: galley.from_ccursor(ccursor_range.primary),
                    secondary: galley.from_ccursor(ccursor_range.secondary),
                })
            })
    }

    /// Sets the currently selected range of characters.
    pub fn set_char_range(&mut self, ccursor_range: Option<CCursorRange>) {
        self.cursor_range = None;
        self.ccursor_range = ccursor_range;
    }

    pub fn set_range(&mut self, cursor_range: Option<CursorRange>) {
        self.cursor_range = cursor_range;
        self.ccursor_range = None;
    }
}

/// The text edit state stored between frames.
///
/// Attention: You also need to `store` the updated state.
/// ```
/// # use egui::text::CCursor;
/// # use egui::text_edit::{CCursorRange, TextEditOutput};
/// # use egui::TextEdit;
/// # egui::__run_test_ui(|ui| {
/// # let mut text = String::new();
/// let mut output = TextEdit::singleline(&mut text).show(ui);
///
/// // Create a new selection range
/// let min = CCursor::new(0);
/// let max = CCursor::new(0);
/// let new_range = CCursorRange::two(min, max);
///
/// // Update the state
/// output.state.set_ccursor_range(Some(new_range));
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
