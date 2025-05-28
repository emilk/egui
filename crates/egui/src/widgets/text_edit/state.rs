use std::ops::Range;
use std::sync::Arc;

use crate::mutex::Mutex;

use crate::{text::Selection, text_selection::TextCursorState, Context, Id};

pub type TextEditUndoer = crate::util::undoer::Undoer<(Selection, String)>;

/// The text edit state stored between frames.
///
/// Attention: You also need to `store` the updated state.
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut text = String::new();
/// let mut output = egui::TextEdit::singleline(&mut text).show(ui);
///
/// // Update the state
/// output.state.cursor.select_byte_range(0..0);
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
    pub(crate) ime_enabled: bool,

    // cursor range for IME candidate.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) ime_selection: Selection,

    // Visual offset when editing singleline text bigger than the width.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) singleline_offset: f32,

    /// When did the user last press a key or click on the `TextEdit`.
    /// Used to pause the cursor animation when typing.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) last_interaction_time: f64,

    /// Byte selection set by whatever's controlling this `TextEdit`, to be
    /// resolved into a `Selection` the next time the `TextEdit` widget is
    /// shown.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) pending_selection: Option<Range<usize>>,
}

impl TextEditState {
    pub fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_persisted(id))
    }

    pub fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_persisted(id, self));
    }

    pub fn undoer(&self) -> TextEditUndoer {
        self.undoer.lock().clone()
    }

    #[expect(clippy::needless_pass_by_ref_mut)] // Intentionally hide interiority of mutability
    pub fn set_undoer(&mut self, undoer: TextEditUndoer) {
        *self.undoer.lock() = undoer;
    }

    pub fn clear_undoer(&mut self) {
        self.set_undoer(TextEditUndoer::default());
    }

    /// Select a specific byte range next time the `TextEdit` is shown. This is
    /// processed before any events on the same frame.
    pub fn select_byte_range(&mut self, range: Range<usize>) {
        self.pending_selection = Some(range);
    }
}
