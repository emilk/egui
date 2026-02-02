use egui::{Event, InputState, Key, KeyboardShortcut, Modifiers, RawInput};

/// High-level actions triggered from raw input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputAction {
    /// Toggle the agent command palette.
    ToggleCommandPalette,

    /// Request clearing of all history.
    ClearHistory,

    /// Cancel the active task.
    CancelActiveTask,
}

/// Helper that inspects [`egui::RawInput`] and emits [`InputAction`]s.
pub struct AgentInputAdapter {
    command_palette_shortcut: KeyboardShortcut,
    clear_shortcut: KeyboardShortcut,
    cancel_shortcut: KeyboardShortcut,
    pending_actions: Vec<InputAction>,
}

impl Default for AgentInputAdapter {
    fn default() -> Self {
        Self::new(
            KeyboardShortcut::new(Modifiers::COMMAND, Key::K),
            KeyboardShortcut::new(Modifiers::COMMAND, Key::L),
            KeyboardShortcut::new(Modifiers::COMMAND, Key::Period),
        )
    }
}

impl AgentInputAdapter {
    /// Create a new adapter with the provided shortcuts (`command palette`, `clear`, `cancel`).
    pub fn new(
        command_palette_shortcut: KeyboardShortcut,
        clear_shortcut: KeyboardShortcut,
        cancel_shortcut: KeyboardShortcut,
    ) -> Self {
        Self {
            command_palette_shortcut,
            clear_shortcut,
            cancel_shortcut,
            pending_actions: Vec::new(),
        }
    }

    /// Process [`egui::RawInput`] and queue up actions.
    pub fn process(&mut self, raw_input: &mut RawInput) {
        for event in raw_input.events.iter() {
            self.handle_event(event);
        }
    }

    fn handle_event(&mut self, event: &Event) {
        if let Event::Key {
            key,
            pressed: true,
            modifiers,
            repeat: false,
            ..
        } = event
        {
            let pressed = map_modifiers(modifiers);
            if shortcut_matches(&pressed, *key, self.command_palette_shortcut) {
                self.pending_actions.push(InputAction::ToggleCommandPalette);
            } else if shortcut_matches(&pressed, *key, self.clear_shortcut) {
                self.pending_actions.push(InputAction::ClearHistory);
            } else if shortcut_matches(&pressed, *key, self.cancel_shortcut) {
                self.pending_actions.push(InputAction::CancelActiveTask);
            }
        }
    }

    /// Drain collected actions into `target`.
    pub fn drain_actions(&mut self, target: &mut Vec<InputAction>) {
        if self.pending_actions.is_empty() {
            return;
        }
        target.append(&mut self.pending_actions);
    }

    /// Helper for tests: run adapter against [`InputState`].
    pub fn process_input_state(&mut self, input_state: &InputState) -> Vec<InputAction> {
        self.pending_actions.clear();
        for event in &input_state.events {
            self.handle_event(event);
        }
        self.pending_actions.clone()
    }
}

fn map_modifiers(modifiers: &Modifiers) -> Modifiers {
    Modifiers {
        alt: modifiers.alt,
        ctrl: modifiers.ctrl,
        shift: modifiers.shift,
        mac_cmd: modifiers.mac_cmd,
        command: modifiers.command,
    }
}

fn shortcut_matches(modifiers: &Modifiers, key: Key, shortcut: KeyboardShortcut) -> bool {
    key == shortcut.logical_key && modifiers.matches_logically(shortcut.modifiers)
}
