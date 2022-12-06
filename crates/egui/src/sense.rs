/// What sort of interaction is a widget sensitive to?
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// #[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Sense {
    /// buttons, sliders, windows, …
    pub click: bool,

    /// sliders, windows, scroll bars, scroll areas, …
    pub drag: bool,

    /// this widgets want focus.
    /// Anything interactive + labels that can be focused
    /// for the benefit of screen readers.
    pub focusable: bool,
}

impl Sense {
    /// Senses no clicks or drags. Only senses mouse hover.
    #[doc(alias = "none")]
    pub fn hover() -> Self {
        Self {
            click: false,
            drag: false,
            focusable: false,
        }
    }

    /// Senses no clicks or drags, but can be focused with the keyboard.
    /// Used for labels that can be focused for the benefit of screen readers.
    pub fn focusable_noninteractive() -> Self {
        Self {
            click: false,
            drag: false,
            focusable: true,
        }
    }

    /// Sense clicks and hover, but not drags.
    pub fn click() -> Self {
        Self {
            click: true,
            drag: false,
            focusable: true,
        }
    }

    /// Sense drags and hover, but not clicks.
    pub fn drag() -> Self {
        Self {
            click: false,
            drag: true,
            focusable: true,
        }
    }

    /// Sense both clicks, drags and hover (e.g. a slider or window).
    pub fn click_and_drag() -> Self {
        Self {
            click: true,
            drag: true,
            focusable: true,
        }
    }

    /// The logical "or" of two [`Sense`]s.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            click: self.click | other.click,
            drag: self.drag | other.drag,
            focusable: self.focusable | other.focusable,
        }
    }

    /// Returns true if we sense either clicks or drags.
    pub fn interactive(&self) -> bool {
        self.click || self.drag
    }
}
