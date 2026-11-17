use crate::Response;

/// How the user is interacting with something that can be highlighted,
/// e.g. the resize line between two panels.
///
/// See [`crate::Ui::delayed_highlight`] and [`crate::Ui::resize_line_stroke`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HighlightState {
    /// Neither hovered nor active.
    #[default]
    Idle,

    /// The pointer is resting on it.
    Hovered,

    /// It is being interacted with, e.g. pressed or dragged.
    Active,
}

impl HighlightState {
    /// `active` means the user is interacting with the thing right now,
    /// e.g. pressing or dragging it, or scrolling with the mouse wheel.
    /// An active thing is highlighted right away, without the hover delay.
    ///
    /// Being active takes precedence over being hovered.
    pub fn new(hovered: bool, active: bool) -> Self {
        if active {
            Self::Active
        } else if hovered {
            Self::Hovered
        } else {
            Self::Idle
        }
    }
}

impl From<&Response> for HighlightState {
    /// [`Self::Active`] while dragged.
    fn from(response: &Response) -> Self {
        Self::new(response.hovered(), response.dragged())
    }
}
