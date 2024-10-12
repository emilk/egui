use egui::Event::PointerButton;
use egui::{Event, Modifiers, Pos2};
use kittest::{ElementState, MouseButton, SimulatedEvent};

#[derive(Default)]
pub(crate) struct EventState {
    modifiers: Modifiers,
    last_mouse_pos: Pos2,
}

impl EventState {
    pub fn kittest_event_to_egui(&mut self, event: kittest::Event) -> Option<egui::Event> {
        match event {
            kittest::Event::ActionRequest(e) => Some(Event::AccessKitActionRequest(e)),
            kittest::Event::Simulated(e) => match e {
                SimulatedEvent::CursorMoved { position } => {
                    self.last_mouse_pos = Pos2::new(position.x as f32, position.y as f32);
                    Some(Event::PointerMoved(Pos2::new(
                        position.x as f32,
                        position.y as f32,
                    )))
                }
                SimulatedEvent::MouseInput { state, button } => {
                    pointer_button_to_egui(button).map(|button| PointerButton {
                        button,
                        modifiers: self.modifiers,
                        pos: self.last_mouse_pos,
                        pressed: matches!(state, ElementState::Pressed),
                    })
                }
                SimulatedEvent::Ime(text) => Some(Event::Text(text)),
                SimulatedEvent::KeyInput { state, key } => {
                    match key {
                        kittest::Key::Alt => {
                            self.modifiers.alt = matches!(state, ElementState::Pressed);
                        }
                        kittest::Key::Command => {
                            self.modifiers.command = matches!(state, ElementState::Pressed);
                        }
                        kittest::Key::Control => {
                            self.modifiers.ctrl = matches!(state, ElementState::Pressed);
                        }
                        kittest::Key::Shift => {
                            self.modifiers.shift = matches!(state, ElementState::Pressed);
                        }
                        _ => {}
                    }
                    kittest_key_to_egui(key).map(|key| Event::Key {
                        key,
                        modifiers: self.modifiers,
                        pressed: matches!(state, ElementState::Pressed),
                        repeat: false,
                        physical_key: None,
                    })
                }
            },
        }
    }
}

pub fn kittest_key_to_egui(value: kittest::Key) -> Option<egui::Key> {
    use egui::Key as EKey;
    use kittest::Key;
    match value {
        Key::ArrowDown => Some(EKey::ArrowDown),
        Key::ArrowLeft => Some(EKey::ArrowLeft),
        Key::ArrowRight => Some(EKey::ArrowRight),
        Key::ArrowUp => Some(EKey::ArrowUp),
        Key::Escape => Some(EKey::Escape),
        Key::Tab => Some(EKey::Tab),
        Key::Backspace => Some(EKey::Backspace),
        Key::Enter => Some(EKey::Enter),
        Key::Space => Some(EKey::Space),
        Key::Insert => Some(EKey::Insert),
        Key::Delete => Some(EKey::Delete),
        Key::Home => Some(EKey::Home),
        Key::End => Some(EKey::End),
        Key::PageUp => Some(EKey::PageUp),
        Key::PageDown => Some(EKey::PageDown),
        Key::Copy => Some(EKey::Copy),
        Key::Cut => Some(EKey::Cut),
        Key::Paste => Some(EKey::Paste),
        Key::Colon => Some(EKey::Colon),
        Key::Comma => Some(EKey::Comma),
        Key::Backslash => Some(EKey::Backslash),
        Key::Slash => Some(EKey::Slash),
        Key::Pipe => Some(EKey::Pipe),
        Key::Questionmark => Some(EKey::Questionmark),
        Key::OpenBracket => Some(EKey::OpenBracket),
        Key::CloseBracket => Some(EKey::CloseBracket),
        Key::Backtick => Some(EKey::Backtick),
        Key::Minus => Some(EKey::Minus),
        Key::Period => Some(EKey::Period),
        Key::Plus => Some(EKey::Plus),
        Key::Equals => Some(EKey::Equals),
        Key::Semicolon => Some(EKey::Semicolon),
        Key::Quote => Some(EKey::Quote),
        Key::Num0 => Some(EKey::Num0),
        Key::Num1 => Some(EKey::Num1),
        Key::Num2 => Some(EKey::Num2),
        Key::Num3 => Some(EKey::Num3),
        Key::Num4 => Some(EKey::Num4),
        Key::Num5 => Some(EKey::Num5),
        Key::Num6 => Some(EKey::Num6),
        Key::Num7 => Some(EKey::Num7),
        Key::Num8 => Some(EKey::Num8),
        Key::Num9 => Some(EKey::Num9),
        Key::A => Some(EKey::A),
        Key::B => Some(EKey::B),
        Key::C => Some(EKey::C),
        Key::D => Some(EKey::D),
        Key::E => Some(EKey::E),
        Key::F => Some(EKey::F),
        Key::G => Some(EKey::G),
        Key::H => Some(EKey::H),
        Key::I => Some(EKey::I),
        Key::J => Some(EKey::J),
        Key::K => Some(EKey::K),
        Key::L => Some(EKey::L),
        Key::M => Some(EKey::M),
        Key::N => Some(EKey::N),
        Key::O => Some(EKey::O),
        Key::P => Some(EKey::P),
        Key::Q => Some(EKey::Q),
        Key::R => Some(EKey::R),
        Key::S => Some(EKey::S),
        Key::T => Some(EKey::T),
        Key::U => Some(EKey::U),
        Key::V => Some(EKey::V),
        Key::W => Some(EKey::W),
        Key::X => Some(EKey::X),
        Key::Y => Some(EKey::Y),
        Key::Z => Some(EKey::Z),
        Key::F1 => Some(EKey::F1),
        Key::F2 => Some(EKey::F2),
        Key::F3 => Some(EKey::F3),
        Key::F4 => Some(EKey::F4),
        Key::F5 => Some(EKey::F5),
        Key::F6 => Some(EKey::F6),
        Key::F7 => Some(EKey::F7),
        Key::F8 => Some(EKey::F8),
        Key::F9 => Some(EKey::F9),
        Key::F10 => Some(EKey::F10),
        Key::F11 => Some(EKey::F11),
        Key::F12 => Some(EKey::F12),
        Key::F13 => Some(EKey::F13),
        Key::F14 => Some(EKey::F14),
        Key::F15 => Some(EKey::F15),
        Key::F16 => Some(EKey::F16),
        Key::F17 => Some(EKey::F17),
        Key::F18 => Some(EKey::F18),
        Key::F19 => Some(EKey::F19),
        Key::F20 => Some(EKey::F20),
        Key::F21 => Some(EKey::F21),
        Key::F22 => Some(EKey::F22),
        Key::F23 => Some(EKey::F23),
        Key::F24 => Some(EKey::F24),
        Key::F25 => Some(EKey::F25),
        Key::F26 => Some(EKey::F26),
        Key::F27 => Some(EKey::F27),
        Key::F28 => Some(EKey::F28),
        Key::F29 => Some(EKey::F29),
        Key::F30 => Some(EKey::F30),
        Key::F31 => Some(EKey::F31),
        Key::F32 => Some(EKey::F32),
        Key::F33 => Some(EKey::F33),
        Key::F34 => Some(EKey::F34),
        Key::F35 => Some(EKey::F35),
        _ => None,
    }
}

pub fn pointer_button_to_egui(value: MouseButton) -> Option<egui::PointerButton> {
    match value {
        MouseButton::Left => Some(egui::PointerButton::Primary),
        MouseButton::Right => Some(egui::PointerButton::Secondary),
        MouseButton::Middle => Some(egui::PointerButton::Middle),
        MouseButton::Back => Some(egui::PointerButton::Extra1),
        MouseButton::Forward => Some(egui::PointerButton::Extra2),
        MouseButton::Other(_) => None,
    }
}
