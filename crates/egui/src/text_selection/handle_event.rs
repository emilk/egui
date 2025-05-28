use epaint::{
    text::cursor::{ByteCursor, Selection},
    Galley,
};

use crate::{os::OperatingSystem, Event, Id, Key, Modifiers};

pub trait SelectionExt: Sized {
    /// Check for events that modify the cursor range.
    ///
    /// Returns `true` if such an event was found and handled.
    fn on_event(
        &self,
        os: OperatingSystem,
        event: &Event,
        galley: &Galley,
        _widget_id: Id,
    ) -> Option<Self>;

    /// Check for key presses that are moving the cursor.
    ///
    /// Returns `true` if we did mutate `self`.
    fn on_key_press(
        &self,
        os: OperatingSystem,
        galley: &Galley,
        modifiers: &Modifiers,
        key: Key,
    ) -> Option<Self>;
}

impl SelectionExt for Selection {
    fn on_event(
        &self,
        os: OperatingSystem,
        event: &Event,
        galley: &Galley,
        _widget_id: Id,
    ) -> Option<Self> {
        match event {
            Event::Key {
                modifiers,
                key,
                pressed: true,
                ..
            } => self.on_key_press(os, galley, modifiers, *key),

            #[cfg(feature = "accesskit")]
            Event::AccessKitActionRequest(accesskit::ActionRequest {
                action: accesskit::Action::SetTextSelection,
                target,
                data: Some(accesskit::ActionData::SetTextSelection(selection)),
            }) if _widget_id.accesskit_id() == *target => {
                galley.selection(|s| s.from_accesskit_selection(selection))
            }

            _ => None,
        }
    }

    fn on_key_press(
        &self,
        os: OperatingSystem,
        galley: &Galley,
        modifiers: &Modifiers,
        key: Key,
    ) -> Option<Self> {
        match key {
            Key::A if modifiers.command => Some(galley.selection(|s| s.select_all())),

            Key::ArrowLeft | Key::ArrowRight if modifiers.is_none() && !self.is_empty() => {
                if key == Key::ArrowLeft {
                    Some(galley.selection(|s| s.select_prev_character(self, false)))
                } else {
                    Some(galley.selection(|s| s.select_next_character(self, false)))
                }
            }

            Key::ArrowLeft
            | Key::ArrowRight
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::Home
            | Key::End => move_single_cursor(os, self, galley, key, modifiers),

            Key::P | Key::N | Key::B | Key::F | Key::A | Key::E
                if os == OperatingSystem::Mac && modifiers.ctrl && !modifiers.shift =>
            {
                move_single_cursor(os, self, galley, key, modifiers)
            }

            _ => None,
        }
    }
}

/// Move a text cursor based on keyboard
fn move_single_cursor(
    os: OperatingSystem,
    selection: &Selection,
    galley: &Galley,
    key: Key,
    modifiers: &Modifiers,
) -> Option<Selection> {
    if os == OperatingSystem::Mac && modifiers.ctrl && !modifiers.shift {
        match key {
            Key::A => Some(galley.selection(|s| s.select_row_start(selection, modifiers.shift))),
            Key::E => Some(galley.selection(|s| s.select_row_end(selection, modifiers.shift))),
            Key::P => Some(galley.selection(|s| s.select_prev_row(selection, modifiers.shift))),
            Key::N => Some(galley.selection(|s| s.select_next_row(selection, modifiers.shift))),
            Key::B => {
                Some(galley.selection(|s| s.select_prev_character(selection, modifiers.shift)))
            }
            Key::F => {
                Some(galley.selection(|s| s.select_next_character(selection, modifiers.shift)))
            }
            _ => None,
        }
    } else {
        match key {
            Key::ArrowLeft => {
                if modifiers.alt || modifiers.ctrl {
                    // alt on mac, ctrl on windows
                    Some(galley.selection(|s| s.select_prev_word(selection, modifiers.shift)))
                } else if modifiers.mac_cmd {
                    Some(galley.selection(|s| s.select_row_start(selection, modifiers.shift)))
                } else {
                    Some(galley.selection(|s| s.select_prev_character(selection, modifiers.shift)))
                }
            }
            Key::ArrowRight => {
                if modifiers.alt || modifiers.ctrl {
                    // alt on mac, ctrl on windows
                    Some(galley.selection(|s| s.select_next_word(selection, modifiers.shift)))
                } else if modifiers.mac_cmd {
                    Some(galley.selection(|s| s.select_row_end(selection, modifiers.shift)))
                } else {
                    Some(galley.selection(|s| s.select_next_character(selection, modifiers.shift)))
                }
            }
            Key::ArrowUp => {
                match (modifiers.command, modifiers.shift) {
                    (true, true) => {
                        // mac and windows behavior
                        Some(galley.selection(|s| {
                            s.extend_selection_to_cursor(selection, &ByteCursor::START)
                        }))
                    }
                    (true, false) => {
                        Some(galley.selection(|s| s.select_at_cursor(&ByteCursor::START)))
                    }
                    (false, extend) => {
                        Some(galley.selection(|s| s.select_prev_row(selection, extend)))
                    }
                }
            }
            Key::ArrowDown => {
                match (modifiers.command, modifiers.shift) {
                    (true, true) => {
                        // mac and windows behavior
                        Some(galley.selection(|s| {
                            s.extend_selection_to_cursor(selection, &ByteCursor::END)
                        }))
                    }
                    (true, false) => {
                        Some(galley.selection(|s| s.select_at_cursor(&ByteCursor::END)))
                    }
                    (false, extend) => {
                        Some(galley.selection(|s| s.select_next_row(selection, extend)))
                    }
                }
            }

            Key::Home => {
                match (modifiers.command, modifiers.shift) {
                    (true, true) => {
                        // windows behavior
                        Some(galley.selection(|s| {
                            s.extend_selection_to_cursor(selection, &ByteCursor::START)
                        }))
                    }
                    (true, false) => {
                        Some(galley.selection(|s| s.select_at_cursor(&ByteCursor::START)))
                    }
                    (false, extend) => {
                        Some(galley.selection(|s| s.select_row_start(selection, extend)))
                    }
                }
            }
            Key::End => {
                match (modifiers.command, modifiers.shift) {
                    (true, true) => {
                        // windows behavior
                        Some(galley.selection(|s| {
                            s.extend_selection_to_cursor(selection, &ByteCursor::END)
                        }))
                    }
                    (true, false) => {
                        Some(galley.selection(|s| s.select_at_cursor(&ByteCursor::START)))
                    }
                    (false, extend) => {
                        Some(galley.selection(|s| s.select_row_end(selection, extend)))
                    }
                }
            }

            _ => unreachable!(),
        }
    }
}
