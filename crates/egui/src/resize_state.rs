use crate::Response;

/// The interaction state of a resize handle, e.g. the line between two panels.
///
/// See [`crate::Ui::resize_line_stroke`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ResizeState {
    /// Neither hovered nor dragged.
    #[default]
    Idle,

    /// The pointer is resting on the handle.
    Hovered,

    /// The handle is being dragged.
    Dragged,
}

impl ResizeState {
    /// Dragging takes precedence over hovering.
    pub fn new(hovered: bool, dragged: bool) -> Self {
        if dragged {
            Self::Dragged
        } else if hovered {
            Self::Hovered
        } else {
            Self::Idle
        }
    }
}

impl From<&Response> for ResizeState {
    fn from(response: &Response) -> Self {
        Self::new(response.hovered(), response.dragged())
    }
}
