use std::sync::Arc;

/// The output from a [`TextEdit`](crate::TextEdit).
pub struct TextEditOutput {
    /// The interaction response.
    pub response: crate::Response,

    /// How the text was displayed.
    pub galley: Arc<crate::Galley>,

    /// Where the text in [`Self::galley`] ended up on the screen.
    pub text_draw_pos: crate::Pos2,

    /// The text was clipped to this rectangle when painted.
    pub text_clip_rect: crate::Rect,

    /// The state we stored after the run.
    pub state: super::TextEditState,

    /// Where the text cursor is.
    pub cursor_range: Option<super::CursorRange>,
}

// TODO(emilk): add `output.paint` and `output.store` and split out that code from `TextEdit::show`.
