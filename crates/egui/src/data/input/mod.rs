//! The input needed by egui.

mod dropped_file;
mod event;
mod event_filter;
mod hovered_file;
mod ime_event;
mod keyboard_shortcut;
mod modifier_names;
mod modifiers;
mod mouse_wheel_unit;
mod pointer_button;
mod raw_input;
mod safe_area_insets;
mod touch;
mod viewport_info;

pub use self::{
    dropped_file::DroppedFile,
    event::Event,
    event_filter::EventFilter,
    hovered_file::HoveredFile,
    ime_event::ImeEvent,
    keyboard_shortcut::KeyboardShortcut,
    modifier_names::ModifierNames,
    modifiers::Modifiers,
    mouse_wheel_unit::MouseWheelUnit,
    pointer_button::{NUM_POINTER_BUTTONS, PointerButton},
    raw_input::RawInput,
    safe_area_insets::SafeAreaInsets,
    touch::{TouchDeviceId, TouchId, TouchPhase},
    viewport_info::{ViewportEvent, ViewportInfo},
};
