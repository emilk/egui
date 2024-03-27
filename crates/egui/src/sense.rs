

/// What sort of interaction is a widget sensitive to?
// #[derive(Clone, Copy, Debug, Eq, PartialEq)]
// // #[cfg_attr(feature = "serde", derive(serde::Serialize))]
// pub struct Sense {
//     /// Buttons, sliders, windows, …
//     pub click: bool,
//
//     /// Sliders, windows, scroll bars, scroll areas, …
//     pub drag: bool,
//
//     /// This widget wants focus.
//     ///
//     /// Anything interactive + labels that can be focused
//     /// for the benefit of screen readers.
//     pub focusable: bool,
// }
/// What sort of interaction is a widget sensitive to?
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
 #[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Sense(u64);
#[derive(Debug, Clone)]
#[repr(u64)]
pub enum SenseBitField {
    /// Buttons, sliders, windows, …
    Click = 1 << 0,
    /// Sliders, windows, scroll bars, scroll areas, …
    Drag = 1 << 1,
    /// This widget wants focus.
    ///
    /// Anything interactive + labels that can be focused
    /// for the benefit of screen readers.
    Focusable = 1 << 2,
}
impl Sense {
    /// Senses no clicks or drags. Only senses mouse hover.
    #[doc(alias = "none")]
    #[inline]
    // pub fn hover() -> Self {
    //     Self {
    //         click: false,
    //         drag: false,
    //         focusable: false,
    //     }
    // }
    fn from_fields(click: bool, drag: bool, focusable: bool) -> Self {
        let mut bits = 0;
        if click {
            bits |= SenseBitField::Click as u64;
        };
        if drag {
            bits |= SenseBitField::Drag as u64;
        };
        if focusable {
            bits |= SenseBitField::Focusable as u64;
        };
        Self(bits)
    }
    pub fn sense_has(&self, field: SenseBitField) -> bool {
        (self.0 & (field as u64)) != 0
    }
    pub fn hover() -> Self {
        // Self {
        //     click: false,
        //     drag: false,
        //     focusable: false,
        // }
        Self::from_fields(false, false, false)
    }

    /// Senses no clicks or drags, but can be focused with the keyboard.
    /// Used for labels that can be focused for the benefit of screen readers.
    #[inline]
    pub fn focusable_noninteractive() -> Self {
        // Self {
        //     click: false,
        //     drag: false,
        //     focusable: true,
        // }
        Self::from_fields(false, false, true)
    }

    /// Sense clicks and hover, but not drags.
    #[inline]
    pub fn click() -> Self {
        // Self {
        //     click: true,
        //     drag: false,
        //     focusable: true,
        // }
        Self::from_fields(true, false, true)
    }

    /// Sense drags and hover, but not clicks.
    #[inline]
    pub fn drag() -> Self {
        // Self {
        //     click: false,
        //     drag: true,
        //     focusable: true,
        // }
        Self::from_fields(false, true, true)
    }

    pub fn modify_field(&mut self, add_or_remove: bool, field: SenseBitField) -> u64 {
        match add_or_remove {
            true => {
                if !self.sense_has(field.clone()) {
                    self.0 += field as u64
                }
            }
            false => {
                if self.sense_has(field.clone()) {
                    self.0 -= field as u64
                }
            }
        };
        self.0
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
        // Self {
        //     click: true,
        //     drag: true,
        //     focusable: true,
        // }
        Self::from_fields(true, true, true)
    }
    pub fn focusable(&self) -> bool {
        self.sense_has(SenseBitField::Focusable)
    }
    pub fn has_click(&self) -> bool {
        self.sense_has(SenseBitField::Click)
    }
    pub fn has_drag(&self) -> bool {
        self.sense_has(SenseBitField::Drag)
    }

    pub fn set_click(&mut self, value: bool) {
        self.modify_field(value, SenseBitField::Click);
    }
    pub fn set_drag(&mut self, value: bool) {
        self.modify_field(value, SenseBitField::Drag);
    }
    pub fn set_focusable(&mut self, value: bool) {
        self.modify_field(value, SenseBitField::Focusable);
    }

    /// The logical "or" of two [`Sense`]s.
    #[must_use]
    #[inline]
    pub fn union(self, other: Self) -> Self {
        // Self {
        //     click: self.click | other.click,
        //     drag: self.drag | other.drag,
        //     focusable: self.focusable | other.focusable,
        // }
        // Self::from_fields(self, false, false)
        Self::from_fields(
            self.sense_has(SenseBitField::Click) | other.sense_has(SenseBitField::Click),
            self.sense_has(SenseBitField::Drag) | other.sense_has(SenseBitField::Drag),
            self.sense_has(SenseBitField::Focusable) | other.sense_has(SenseBitField::Focusable),
        )
    }

    /// Returns true if we sense either clicks or drags.
    #[inline]
    pub fn interactive(&self) -> bool {
        // self.click || self.drag
        self.sense_has(SenseBitField::Click) || self.sense_has(SenseBitField::Drag)
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
