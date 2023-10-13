use egui::Modifiers;

pub trait ToEguiKey {
    /// Convert the struct to an egui key
    fn to_egui_key(&self) -> Option<egui::Key>;
}

impl ToEguiKey for sdl2::keyboard::Keycode {
    fn to_egui_key(&self) -> Option<egui::Key> {
        use egui::Key;
        use sdl2::keyboard::Keycode::*;
        Some(match *self {
            Left => Key::ArrowLeft,
            Up => Key::ArrowUp,
            Right => Key::ArrowRight,
            Down => Key::ArrowDown,

            Escape => Key::Escape,
            Tab => Key::Tab,
            Backspace => Key::Backspace,
            Space => Key::Space,
            Return => Key::Enter,

            Insert => Key::Insert,
            Home => Key::Home,
            Delete => Key::Delete,
            End => Key::End,
            PageDown => Key::PageDown,
            PageUp => Key::PageUp,

            Kp0 | Num0 => Key::Num0,
            Kp1 | Num1 => Key::Num1,
            Kp2 | Num2 => Key::Num2,
            Kp3 | Num3 => Key::Num3,
            Kp4 | Num4 => Key::Num4,
            Kp5 | Num5 => Key::Num5,
            Kp6 | Num6 => Key::Num6,
            Kp7 | Num7 => Key::Num7,
            Kp8 | Num8 => Key::Num8,
            Kp9 | Num9 => Key::Num9,

            A => Key::A,
            B => Key::B,
            C => Key::C,
            D => Key::D,
            E => Key::E,
            F => Key::F,
            G => Key::G,
            H => Key::H,
            I => Key::I,
            J => Key::J,
            K => Key::K,
            L => Key::L,
            M => Key::M,
            N => Key::N,
            O => Key::O,
            P => Key::P,
            Q => Key::Q,
            R => Key::R,
            S => Key::S,
            T => Key::T,
            U => Key::U,
            V => Key::V,
            W => Key::W,
            X => Key::X,
            Y => Key::Y,
            Z => Key::Z,

            _ => {
                return None;
            }
        })
    }
}

pub trait ToEguiModifiers {
    fn to_egui_modifier(&self) -> Modifiers;
}

impl ToEguiModifiers for sdl2::keyboard::Mod {
    fn to_egui_modifier(&self) -> Modifiers {
        use sdl2::keyboard::Mod;
        Modifiers {
            alt: (*self & Mod::LALTMOD == Mod::LALTMOD) || (*self & Mod::RALTMOD == Mod::RALTMOD),
            ctrl: (*self & Mod::LCTRLMOD == Mod::LCTRLMOD)
                || (*self & Mod::RCTRLMOD == Mod::RCTRLMOD),
            shift: (*self & Mod::LSHIFTMOD == Mod::LSHIFTMOD)
                || (*self & Mod::RSHIFTMOD == Mod::RSHIFTMOD),
            mac_cmd: *self & Mod::LGUIMOD == Mod::LGUIMOD,

            //TOD: Test on both windows and mac
            command: (*self & Mod::LCTRLMOD == Mod::LCTRLMOD)
                || (*self & Mod::LGUIMOD == Mod::LGUIMOD),
        }
    }
}

pub trait ToSdl2SystemCursor {
    fn to_sdl2_cursor(&self) -> Option<sdl2::mouse::SystemCursor>;
}
impl ToSdl2SystemCursor for egui::CursorIcon {
    fn to_sdl2_cursor(&self) -> Option<sdl2::mouse::SystemCursor> {
        use sdl2::mouse::SystemCursor;
        match *self {
            egui::CursorIcon::None => None,
            egui::CursorIcon::Crosshair => Some(SystemCursor::Crosshair),
            egui::CursorIcon::Default => Some(SystemCursor::Arrow),
            egui::CursorIcon::Grab => Some(SystemCursor::Hand),
            egui::CursorIcon::Grabbing => Some(SystemCursor::SizeAll),
            egui::CursorIcon::Move => Some(SystemCursor::SizeAll),
            egui::CursorIcon::PointingHand => Some(SystemCursor::Hand),
            egui::CursorIcon::ResizeHorizontal => Some(SystemCursor::SizeWE),
            egui::CursorIcon::ResizeNeSw => Some(SystemCursor::SizeNESW),
            egui::CursorIcon::ResizeNwSe => Some(SystemCursor::SizeNWSE),
            egui::CursorIcon::ResizeVertical => Some(SystemCursor::SizeNS),
            egui::CursorIcon::Text => Some(SystemCursor::IBeam),
            egui::CursorIcon::NotAllowed | egui::CursorIcon::NoDrop => Some(SystemCursor::No),
            egui::CursorIcon::Wait => Some(SystemCursor::Wait),
            _ => Some(SystemCursor::Arrow),
        }
    }
}

pub trait ToEguiPointerButton {
    fn to_egui_pointer_button(&self) -> Option<egui::PointerButton>;
}

impl ToEguiPointerButton for sdl2::mouse::MouseButton {
    fn to_egui_pointer_button(&self) -> Option<egui::PointerButton> {
        use egui::PointerButton;
        use sdl2::mouse::MouseButton;
        match self {
            MouseButton::Left => Some(PointerButton::Primary),
            MouseButton::Middle => Some(PointerButton::Middle),
            MouseButton::Right => Some(PointerButton::Secondary),
            _ => None,
        }
    }
}
