/// What sort of interaction is a widget sensitive to?
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
// #[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Sense {
    /// Buttons, sliders, windows, …
    pub click: bool,

    /// Sliders, windows, scroll bars, scroll areas, …
    pub drag: bool,

    /// This widget wants focus.
    ///
    /// Anything interactive + labels that can be focused
    /// for the benefit of screen readers.
    pub focusable: bool,
}

impl Sense {
    /// Senses no clicks or drags. Only senses mouse hover.
    #[doc(alias = "none")]
    #[inline]
    pub fn hover() -> Self {
        Self {
            click: false,
            drag: false,
            focusable: false,
        }
    }

    /// Senses no clicks or drags, but can be focused with the keyboard.
    /// Used for labels that can be focused for the benefit of screen readers.
    #[inline]
    pub fn focusable_noninteractive() -> Self {
        Self {
            click: false,
            drag: false,
            focusable: true,
        }
    }

    /// Sense clicks and hover, but not drags.
    #[inline]
    pub fn click() -> Self {
        Self {
            click: true,
            drag: false,
            focusable: true,
        }
    }

    /// Sense drags and hover, but not clicks.
    #[inline]
    pub fn drag() -> Self {
        Self {
            click: false,
            drag: true,
            focusable: true,
        }
    }

    /// Sense both clicks, drags and hover (e.g. a slider or window).
    ///
    /// Note that this will introduce a latency when dragging,
    /// because when the user starts a press egui can't know if this is the start
    /// of a click or a drag, and it won't know until the cursor has
    /// either moved a certain distance, or the user has released the mouse button.
    ///
    /// See [`crate::PointerState::is_decidedly_dragging`] for details.
    #[inline]
    pub fn click_and_drag() -> Self {
        Self {
            click: true,
            drag: true,
            focusable: true,
        }
    }

    /// The logical "or" of two [`Sense`]s.
    #[must_use]
    #[inline]
    pub fn union(self, other: Self) -> Self {
        Self {
            click: self.click | other.click,
            drag: self.drag | other.drag,
            focusable: self.focusable | other.focusable,
        }
    }

    /// Returns true if we sense either clicks or drags.
    #[inline]
    pub fn interactive(&self) -> bool {
        self.click || self.drag
    }
}

impl std::ops::BitOr for Sense {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        self.union(rhs)
    }
}

impl std::ops::BitOrAssign for Sense {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.union(rhs);
    }
}
