/// What sort of interaction is a widget sensitive to?
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// #[cfg_attr(feature = "persistence", derive(serde::Serialize))]
pub struct Sense {
    /// buttons, sliders, windows ...
    pub click: bool,

    /// sliders, windows, scroll bars, scroll areas ...
    pub drag: bool,
}

impl Sense {
    /// Senses no clicks or drags. Only senses mouse hover.
    pub fn hover() -> Self {
        Self {
            click: false,
            drag: false,
        }
    }

    #[deprecated = "Use hover()"]
    pub fn nothing() -> Self {
        Sense::hover()
    }

    /// Sense clicks and hover, but not drags.
    pub fn click() -> Self {
        Self {
            click: true,
            drag: false,
        }
    }

    /// Sense drags and hover, but not clicks.
    pub fn drag() -> Self {
        Self {
            click: false,
            drag: true,
        }
    }

    /// Sense both clicks, drags and hover (e.g. a slider or window).
    pub fn click_and_drag() -> Self {
        Self {
            click: true,
            drag: true,
        }
    }

    /// The logical "or" of two `Sense`s.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            click: self.click | other.click,
            drag: self.drag | other.drag,
        }
    }
}
