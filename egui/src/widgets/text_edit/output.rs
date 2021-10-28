use std::sync::Arc;

/// The output from a `TextEdit`.
pub struct TextEditOutput {
    /// The interaction response.
    pub response: crate::Response,

    /// How the text was displayed.
    pub galley: Arc<crate::Galley>,

    /// The state we stored after the run/
    pub state: super::TextEditState,

    /// Where the text cursor is.
    pub cursor_range: Option<super::CursorRange>,
}
