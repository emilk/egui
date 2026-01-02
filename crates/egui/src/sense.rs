/// What sort of interaction is a widget sensitive to?
#[derive(Clone, Copy, Eq, PartialEq)]
// #[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Sense(u8);

bitflags::bitflags! {
    impl Sense: u8 {

        const HOVER = 0;

        /// Buttons, sliders, windows, …
        const CLICK = 1<<0;

        /// Sliders, windows, scroll bars, scroll areas, …
        const DRAG = 1<<1;

        /// This widget wants focus.
        ///
        /// Anything interactive + labels that can be focused
        /// for the benefit of screen readers.
        const FOCUSABLE = 1<<2;
    }
}

impl std::fmt::Debug for Sense {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sense {{")?;
        if self.senses_click() {
            write!(f, " click")?;
        }
        if self.senses_drag() {
            write!(f, " drag")?;
        }
        if self.is_focusable() {
            write!(f, " focusable")?;
        }
        write!(f, " }}")
    }
}

impl Sense {
    /// Senses no clicks or drags. Only senses mouse hover.
    #[doc(alias = "none")]
    #[inline]
    pub fn hover() -> Self {
        Self::empty()
    }

    /// Senses no clicks or drags, but can be focused with the keyboard.
    /// Used for labels that can be focused for the benefit of screen readers.
    #[inline]
    pub fn focusable_noninteractive() -> Self {
        Self::FOCUSABLE
    }

    /// Sense clicks and hover, but not drags, and make the widget focusable.
    ///
    /// Use [`Sense::CLICK`] if you don't want the widget to be focusable.
    #[inline]
    pub fn click() -> Self {
        Self::CLICK | Self::FOCUSABLE
    }

    /// Sense drags and hover, but not clicks. Make the widget focusable.
    ///
    /// Use [`Sense::DRAG`] if you don't want the widget to be focusable
    #[inline]
    pub fn drag() -> Self {
        Self::DRAG | Self::FOCUSABLE
    }

    /// Sense both clicks, drags and hover (e.g. a slider or window), and make the widget focusable.
    ///
    /// Note that this will introduce a latency when dragging,
    /// because when the user starts a press egui can't know if this is the start
    /// of a click or a drag, and it won't know until the cursor has
    /// either moved a certain distance, or the user has released the mouse button.
    ///
    /// See [`crate::PointerState::is_decidedly_dragging`] for details.
    #[inline]
    pub fn click_and_drag() -> Self {
        Self::CLICK | Self::FOCUSABLE | Self::DRAG
    }

    /// Returns true if we sense either clicks or drags.
    #[inline]
    pub fn interactive(&self) -> bool {
        self.intersects(Self::CLICK | Self::DRAG)
    }

    #[inline]
    pub fn senses_click(&self) -> bool {
        self.contains(Self::CLICK)
    }

    #[inline]
    pub fn senses_drag(&self) -> bool {
        self.contains(Self::DRAG)
    }

    #[inline]
    pub fn is_focusable(&self) -> bool {
        self.contains(Self::FOCUSABLE)
    }
}
