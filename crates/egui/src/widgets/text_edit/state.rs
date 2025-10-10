use std::sync::Arc;

use crate::mutex::Mutex;

use crate::{
    Context, Id, Vec2,
    text_selection::{CCursorRange, TextCursorState},
};

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

    // TODO(tye-exe): Should this be public?
    /// Displayed string.
    /// This may differ from the value represented if the user is actively editing the string.
    pub text: Option<String>,

    /// Wrapped in Arc for cheaper clones.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) undoer: Arc<Mutex<TextEditUndoer>>,

    // If IME candidate window is shown on this text edit.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) ime_enabled: bool,

    // cursor range for IME candidate.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) ime_cursor_range: CCursorRange,

    // Text offset within the widget area.
    // Used for sensing and singleline text clipping.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) text_offset: Vec2,

    /// When did the user last press a key or click on the `TextEdit`.
    /// Used to pause the cursor animation when typing.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) last_interaction_time: f64,
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
}
