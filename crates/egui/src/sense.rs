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

        /// This widget wants to receive leftward scroll events.
        const SCROLL_LEFT = 1<<3;

        /// This widget wants to receive rightward scroll events.
        const SCROLL_RIGHT = 1<<4;

        /// This widget wants to receive upward scroll events.
        const SCROLL_UP = 1<<5;

        /// This widget wants to receive downward scroll events.
        const SCROLL_DOWN = 1<<6;
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
        if self.senses_scroll_left() {
            write!(f, " scroll_left")?;
        }
        if self.senses_scroll_right() {
            write!(f, " scroll_right")?;
        }
        if self.senses_scroll_up() {
            write!(f, " scroll_up")?;
        }
        if self.senses_scroll_down() {
            write!(f, " scroll_down")?;
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

    /// Sense scroll events in all four directions.
    #[inline]
    pub fn scroll() -> Self {
        Self::SCROLL_LEFT | Self::SCROLL_RIGHT | Self::SCROLL_UP | Self::SCROLL_DOWN
    }

    /// Sense only horizontal scroll events (left and right).
    #[inline]
    pub fn scroll_horizontal() -> Self {
        Self::SCROLL_LEFT | Self::SCROLL_RIGHT
    }

    /// Sense only vertical scroll events (up and down).
    #[inline]
    pub fn scroll_vertical() -> Self {
        Self::SCROLL_UP | Self::SCROLL_DOWN
    }

    /// Sense only leftward scroll events.
    #[inline]
    pub fn scroll_left() -> Self {
        Self::SCROLL_LEFT
    }

    /// Sense only rightward scroll events.
    #[inline]
    pub fn scroll_right() -> Self {
        Self::SCROLL_RIGHT
    }

    /// Sense only upward scroll events.
    #[inline]
    pub fn scroll_up() -> Self {
        Self::SCROLL_UP
    }

    /// Sense only downward scroll events.
    #[inline]
    pub fn scroll_down() -> Self {
        Self::SCROLL_DOWN
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

    /// Does this sense any scroll events?
    #[inline]
    pub fn senses_scroll(&self) -> bool {
        self.intersects(Self::SCROLL_LEFT | Self::SCROLL_RIGHT | Self::SCROLL_UP | Self::SCROLL_DOWN)
    }

    /// Does this sense horizontal scroll events (left or right)?
    #[inline]
    pub fn senses_scroll_horizontal(&self) -> bool {
        self.intersects(Self::SCROLL_LEFT | Self::SCROLL_RIGHT)
    }

    /// Does this sense vertical scroll events (up or down)?
    #[inline]
    pub fn senses_scroll_vertical(&self) -> bool {
        self.intersects(Self::SCROLL_UP | Self::SCROLL_DOWN)
    }

    /// Does this sense leftward scroll events?
    #[inline]
    pub fn senses_scroll_left(&self) -> bool {
        self.contains(Self::SCROLL_LEFT)
    }

    /// Does this sense rightward scroll events?
    #[inline]
    pub fn senses_scroll_right(&self) -> bool {
        self.contains(Self::SCROLL_RIGHT)
    }

    /// Does this sense upward scroll events?
    #[inline]
    pub fn senses_scroll_up(&self) -> bool {
        self.contains(Self::SCROLL_UP)
    }

    /// Does this sense downward scroll events?
    #[inline]
    pub fn senses_scroll_down(&self) -> bool {
        self.contains(Self::SCROLL_DOWN)
    }
}
