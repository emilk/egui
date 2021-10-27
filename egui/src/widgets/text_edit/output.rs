use crate::*;

use std::sync::Arc;

use super::CursorRange;

/// The output from a `TextEdit`.
pub struct TextEditOutput {
    /// The interaction response.
    pub response: Response,

    /// How the text was displayed.
    pub galley: Arc<Galley>,

    /// Where the text cursor is.
    pub cursor_range: Option<CursorRange>,
}
