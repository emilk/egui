/// IME event.
///
/// See <https://docs.rs/winit/latest/winit/event/enum.Ime.html>
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ImeEvent {
    /// Notifies when the IME was enabled.
    #[deprecated = "No longer used by egui"]
    Enabled,

    /// A new IME candidate is being suggested.
    ///
    /// An empty preedit string indicates that the IME has been dismissed, while
    /// a non-empty preedit string indicates that the IME is active.
    Preedit {
        text: String,
        active_range_chars: Option<core::ops::Range<usize>>,
    },

    /// IME composition ended with this final result.
    ///
    /// The IME is considered dismissed after this event.
    Commit(String),

    /// Notifies when the text surrounding the cursor should be deleted.
    ///
    /// `before_chars` and `after_chars` are the number of characters (not
    /// bytes) to delete before and after the cursor, respectively.
    DeleteSurrounding {
        before_chars: usize,
        after_chars: usize,
    },

    /// Notifies when the text surrounding the cursor should be deleted,
    /// counted in UTF-8 bytes.
    ///
    /// This is the unit IMEs and windowing systems actually report: winit's
    /// `Ime::DeleteSurrounding { before_bytes, after_bytes }` (0.31+,
    /// <https://github.com/rust-windowing/winit/pull/4294>) and Wayland's
    /// `zwp_text_input_v3::delete_surrounding_text` are both byte-counted.
    ///
    /// Sent right before the [`ImeEvent::Commit`] it accompanies (which may be
    /// empty), so the delete and the commit stay in one undo step.
    ///
    /// Counts saturate at the text bounds and are snapped to UTF-8 character
    /// boundaries: a count that ends mid-character leaves that character
    /// intact.
    DeleteSurroundingBytes {
        before_bytes: usize,
        after_bytes: usize,
    },

    /// Notifies when the IME was disabled.
    #[deprecated = "No longer used by egui"]
    Disabled,
}
