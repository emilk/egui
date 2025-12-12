use crate::input::InputEvent;
use crate::output::{CursorIcon, OutputState};

#[swift_bridge::bridge]
pub mod ffi {
    extern "Rust" {
        type InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_pointer_moved(x: f32, y: f32) -> InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_mouse_wheel(x: f32, y: f32) -> InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_left_mouse_down(x: f32, y: f32, pressed: bool) -> InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_right_mouse_down(x: f32, y: f32, pressed: bool) -> InputEvent;

        #[swift_bridge(associated_to = InputEvent)]
        fn from_window_focused(focused: bool) -> InputEvent;

        // Scene phase changed: 0 = background, 1 = inactive, 2 = active
        #[swift_bridge(associated_to = InputEvent)]
        fn from_scene_phase_changed(phase: u8) -> InputEvent;

        // Text input from native keyboard (after autocomplete/autocorrect)
        #[swift_bridge(associated_to = InputEvent)]
        fn from_text_commit(text: String) -> InputEvent;

        // IME preedit/composition text (e.g., partial CJK input)
        #[swift_bridge(associated_to = InputEvent)]
        fn from_ime_preedit(text: String) -> InputEvent;

        // Virtual keyboard visibility changed
        #[swift_bridge(associated_to = InputEvent)]
        fn from_keyboard_visibility(visible: bool) -> InputEvent;

        // Virtual key press (backspace=0, enter=1, tab=2, escape=3, arrows=4-7)
        #[swift_bridge(associated_to = InputEvent)]
        fn from_virtual_key(key_code: u8, pressed: bool) -> InputEvent;
    }

    extern "Rust" {
        type OutputState;

        fn get_cursor_icon(&self) -> &CursorIcon;

        // Whether egui wants the keyboard to be visible
        fn wants_keyboard(&self) -> bool;

        // IME cursor rect for keyboard positioning
        fn has_ime_rect(&self) -> bool;
        fn get_ime_rect_x(&self) -> f32;
        fn get_ime_rect_y(&self) -> f32;
        fn get_ime_rect_width(&self) -> f32;
        fn get_ime_rect_height(&self) -> f32;
    }

    extern "Rust" {
        type CursorIcon;

        fn is_default(&self) -> bool;
        fn is_pointing_hand(&self) -> bool;
        fn is_resize_horizontal(&self) -> bool;
        fn is_resize_vertical(&self) -> bool;
        fn is_text(&self) -> bool;
    }
}
