use egui::{Event, Modifiers};

/// Input events from Swift/iOS to egui
pub enum InputEvent {
    /// Pointer/touch moved to position
    PointerMoved(f32, f32),
    /// Mouse wheel / scroll gesture
    MouseWheel(f32, f32),
    /// Primary touch/click (x, y, pressed)
    LeftMouseDown(f32, f32, bool),
    /// Secondary touch/click (x, y, pressed)
    RightMouseDown(f32, f32, bool),
    /// Window focus changed
    WindowFocused(bool),
    /// Scene phase changed (iOS UIScene lifecycle)
    /// phase: 0 = background, 1 = inactive, 2 = active
    ScenePhaseChanged(u8),
    /// Committed text from keyboard (after autocomplete/autocorrect)
    TextCommit(String),
    /// IME preedit/composition text (e.g., partial CJK input)
    /// Empty string clears the preedit
    ImePreedit(String),
    /// Virtual keyboard shown/hidden
    KeyboardVisibility(bool),
    /// Key pressed (for special keys like backspace, return)
    /// key_code: 0=Backspace, 1=Enter, 2=Tab, 3=Escape, 4-7=Arrows
    VirtualKey(u8, bool),
}

/// iOS scene phase - maps to SwiftUI ScenePhase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenePhase {
    /// App is in background, not visible
    Background = 0,
    /// App is visible but not receiving events (e.g., during alerts)
    Inactive = 1,
    /// App is in foreground and receiving events
    Active = 2,
}

impl From<u8> for ScenePhase {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Background,
            1 => Self::Inactive,
            _ => Self::Active,
        }
    }
}

impl InputEvent {
    /// Convert to egui Event, returning None for events handled separately
    pub fn into_egui_event(self) -> Option<Event> {
        match self {
            InputEvent::PointerMoved(x, y) => Some(Event::PointerMoved(egui::Pos2::new(x, y))),
            InputEvent::MouseWheel(x, y) => Some(Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(x, y),
                phase: egui::TouchPhase::Move,
                modifiers: Modifiers::default(),
            }),
            InputEvent::LeftMouseDown(x, y, pressed) => Some(Event::PointerButton {
                pos: egui::Pos2::new(x, y),
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Modifiers::default(),
            }),
            InputEvent::RightMouseDown(x, y, pressed) => Some(Event::PointerButton {
                pos: egui::Pos2::new(x, y),
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: Modifiers::default(),
            }),
            InputEvent::WindowFocused(focused) => Some(Event::WindowFocused(focused)),
            InputEvent::ScenePhaseChanged(phase) => {
                Some(Event::WindowFocused(phase == ScenePhase::Active as u8))
            }
            InputEvent::TextCommit(text) => Some(Event::Text(text)),
            InputEvent::ImePreedit(text) => {
                Some(Event::Ime(egui::ImeEvent::Preedit(text)))
            }
            InputEvent::KeyboardVisibility(_) => None, // Handled separately
            InputEvent::VirtualKey(key_code, pressed) => virtual_key_to_event(key_code, pressed),
        }
    }
}

/// Map virtual key codes to egui Key events
fn virtual_key_to_event(key_code: u8, pressed: bool) -> Option<Event> {
    let key = match key_code {
        0 => egui::Key::Backspace,
        1 => egui::Key::Enter,
        2 => egui::Key::Tab,
        3 => egui::Key::Escape,
        4 => egui::Key::ArrowUp,
        5 => egui::Key::ArrowDown,
        6 => egui::Key::ArrowLeft,
        7 => egui::Key::ArrowRight,
        _ => return None,
    };
    Some(Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::default(),
    })
}

// FFI factory methods called from Swift
impl InputEvent {
    pub fn from_pointer_moved(x: f32, y: f32) -> Self {
        Self::PointerMoved(x, y)
    }

    pub fn from_mouse_wheel(x: f32, y: f32) -> Self {
        Self::MouseWheel(x, y)
    }

    pub fn from_left_mouse_down(x: f32, y: f32, pressed: bool) -> Self {
        Self::LeftMouseDown(x, y, pressed)
    }

    pub fn from_right_mouse_down(x: f32, y: f32, pressed: bool) -> Self {
        Self::RightMouseDown(x, y, pressed)
    }

    pub fn from_window_focused(focused: bool) -> Self {
        Self::WindowFocused(focused)
    }

    pub fn from_scene_phase_changed(phase: u8) -> Self {
        Self::ScenePhaseChanged(phase)
    }

    pub fn from_text_commit(text: String) -> Self {
        Self::TextCommit(text)
    }

    pub fn from_ime_preedit(text: String) -> Self {
        Self::ImePreedit(text)
    }

    pub fn from_keyboard_visibility(visible: bool) -> Self {
        Self::KeyboardVisibility(visible)
    }

    pub fn from_virtual_key(key_code: u8, pressed: bool) -> Self {
        Self::VirtualKey(key_code, pressed)
    }
}
