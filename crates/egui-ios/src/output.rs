/// Output state from egui to Swift/iOS
pub struct OutputState {
    cursor_icon: CursorIcon,
    /// Whether egui wants the keyboard visible (text field focused)
    wants_keyboard: bool,
    /// IME cursor area for keyboard positioning (x, y, width, height in points)
    ime_rect: Option<(f32, f32, f32, f32)>,
}

impl OutputState {
    pub fn new(cursor_icon: CursorIcon) -> Self {
        Self {
            cursor_icon,
            wants_keyboard: false,
            ime_rect: None,
        }
    }

    pub fn with_keyboard_state(
        cursor_icon: CursorIcon,
        wants_keyboard: bool,
        ime_rect: Option<egui::Rect>,
    ) -> Self {
        Self {
            cursor_icon,
            wants_keyboard,
            ime_rect: ime_rect.map(|r| (r.min.x, r.min.y, r.width(), r.height())),
        }
    }

    pub fn get_cursor_icon(&self) -> &CursorIcon {
        &self.cursor_icon
    }

    pub fn wants_keyboard(&self) -> bool {
        self.wants_keyboard
    }

    pub fn get_ime_rect(&self) -> Option<(f32, f32, f32, f32)> {
        self.ime_rect
    }

    // FFI accessors for swift-bridge (can't return Option across FFI)
    pub fn has_ime_rect(&self) -> bool {
        self.ime_rect.is_some()
    }

    pub fn get_ime_rect_x(&self) -> f32 {
        self.ime_rect.map(|r| r.0).unwrap_or(0.0)
    }

    pub fn get_ime_rect_y(&self) -> f32 {
        self.ime_rect.map(|r| r.1).unwrap_or(0.0)
    }

    pub fn get_ime_rect_width(&self) -> f32 {
        self.ime_rect.map(|r| r.2).unwrap_or(0.0)
    }

    pub fn get_ime_rect_height(&self) -> f32 {
        self.ime_rect.map(|r| r.3).unwrap_or(0.0)
    }
}

/// Cursor icon for iOS (subset of egui icons that map to iOS)
pub enum CursorIcon {
    Default,
    PointingHand,
    ResizeHorizontal,
    ResizeVertical,
    Text,
}

impl From<egui::CursorIcon> for CursorIcon {
    fn from(cursor_icon: egui::CursorIcon) -> Self {
        match cursor_icon {
            egui::CursorIcon::Default => Self::Default,
            egui::CursorIcon::PointingHand => Self::PointingHand,
            egui::CursorIcon::ResizeHorizontal | egui::CursorIcon::ResizeColumn => {
                Self::ResizeHorizontal
            }
            egui::CursorIcon::ResizeVertical | egui::CursorIcon::ResizeRow => Self::ResizeVertical,
            egui::CursorIcon::Text => Self::Text,
            _ => Self::Default,
        }
    }
}

impl CursorIcon {
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Default)
    }

    pub fn is_pointing_hand(&self) -> bool {
        matches!(self, Self::PointingHand)
    }

    pub fn is_resize_horizontal(&self) -> bool {
        matches!(self, Self::ResizeHorizontal)
    }

    pub fn is_resize_vertical(&self) -> bool {
        matches!(self, Self::ResizeVertical)
    }

    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text)
    }
}
