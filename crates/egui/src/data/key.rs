/// Keyboard keys.
///
/// egui usually uses logical keys, i.e. after applying any user keymap.
// TODO(emilk): split into `LogicalKey` and `PhysicalKey`
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Key {
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,

    Escape,
    Tab,
    Backspace,
    Enter,
    Space,

    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,

    Copy,
    Cut,
    Paste,

    // ----------------------------------------------
    // Punctuation:
    /// `:`
    Colon,

    /// `,`
    Comma,

    /// '\\'
    Backslash,

    /// '/'
    Slash,

    /// '|', a vertical bar
    Pipe,

    /// `?`
    Questionmark,

    // '['
    OpenBracket,

    // ']'
    CloseBracket,

    /// '`', also known as "backquote" or "grave"
    Backtick,

    /// `-`
    Minus,

    /// `.`
    Period,

    /// `+`
    Plus,

    /// `=`
    Equals,

    /// `;`
    Semicolon,

    // ----------------------------------------------
    // Digits:
    /// Either from the main row or from the numpad.
    Num0,

    /// Either from the main row or from the numpad.
    Num1,

    /// Either from the main row or from the numpad.
    Num2,

    /// Either from the main row or from the numpad.
    Num3,

    /// Either from the main row or from the numpad.
    Num4,

    /// Either from the main row or from the numpad.
    Num5,

    /// Either from the main row or from the numpad.
    Num6,

    /// Either from the main row or from the numpad.
    Num7,

    /// Either from the main row or from the numpad.
    Num8,

    /// Either from the main row or from the numpad.
    Num9,

    // ----------------------------------------------
    // Letters:
    A, // Used for cmd+A (select All)
    B,
    C, // |CMD COPY|
    D, // |CMD BOOKMARK|
    E, // |CMD SEARCH|
    F, // |CMD FIND firefox & chrome|
    G, // |CMD FIND chrome|
    H, // |CMD History|
    I, // italics
    J, // |CMD SEARCH firefox/DOWNLOAD chrome|
    K, // Used for ctrl+K (delete text after cursor)
    L,
    M,
    N,
    O, // |CMD OPEN|
    P, // |CMD PRINT|
    Q,
    R, // |CMD REFRESH|
    S, // |CMD SAVE|
    T, // |CMD TAB|
    U, // Used for ctrl+U (delete text before cursor)
    V, // |CMD PASTE|
    W, // Used for ctrl+W (delete previous word)
    X, // |CMD CUT|
    Y,
    Z, // |CMD UNDO|

    // ----------------------------------------------
    // Function keys:
    F1,
    F2,
    F3,
    F4,
    F5, // |CMD REFRESH|
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    F25,
    F26,
    F27,
    F28,
    F29,
    F30,
    F31,
    F32,
    F33,
    F34,
    F35,
    // When adding keys, remember to also update `crates/egui-winit/src/lib.rs`
    // and [`Self::ALL`].
    // Also: don't add keys last; add them to the group they best belong to.
}

impl Key {
    /// All egui keys
    pub const ALL: &'static [Self] = &[
        Self::ArrowDown,
        Self::ArrowLeft,
        Self::ArrowRight,
        Self::ArrowUp,
        Self::Escape,
        Self::Tab,
        Self::Backspace,
        Self::Enter,
        Self::Insert,
        Self::Delete,
        Self::Home,
        Self::End,
        Self::PageUp,
        Self::PageDown,
        Self::Copy,
        Self::Cut,
        Self::Paste,
        // Punctuation:
        Self::Space,
        Self::Colon,
        Self::Comma,
        Self::Minus,
        Self::Period,
        Self::Plus,
        Self::Equals,
        Self::Semicolon,
        Self::OpenBracket,
        Self::CloseBracket,
        Self::Backtick,
        Self::Backslash,
        Self::Slash,
        Self::Pipe,
        Self::Questionmark,
        // Digits:
        Self::Num0,
        Self::Num1,
        Self::Num2,
        Self::Num3,
        Self::Num4,
        Self::Num5,
        Self::Num6,
        Self::Num7,
        Self::Num8,
        Self::Num9,
        // Letters:
        Self::A,
        Self::B,
        Self::C,
        Self::D,
        Self::E,
        Self::F,
        Self::G,
        Self::H,
        Self::I,
        Self::J,
        Self::K,
        Self::L,
        Self::M,
        Self::N,
        Self::O,
        Self::P,
        Self::Q,
        Self::R,
        Self::S,
        Self::T,
        Self::U,
        Self::V,
        Self::W,
        Self::X,
        Self::Y,
        Self::Z,
        // Function keys:
        Self::F1,
        Self::F2,
        Self::F3,
        Self::F4,
        Self::F5,
        Self::F6,
        Self::F7,
        Self::F8,
        Self::F9,
        Self::F10,
        Self::F11,
        Self::F12,
        Self::F13,
        Self::F14,
        Self::F15,
        Self::F16,
        Self::F17,
        Self::F18,
        Self::F19,
        Self::F20,
        Self::F21,
        Self::F22,
        Self::F23,
        Self::F24,
        Self::F25,
        Self::F26,
        Self::F27,
        Self::F28,
        Self::F29,
        Self::F30,
        Self::F31,
        Self::F32,
        Self::F33,
        Self::F34,
        Self::F35,
    ];

    /// Converts `"A"` to `Key::A`, `Space` to `Key::Space`, etc.
    ///
    /// Makes sense for logical keys.
    ///
    /// This will parse the output of both [`Self::name`] and [`Self::symbol_or_name`],
    /// but will also parse single characters, so that both `"-"` and `"Minus"` will return `Key::Minus`.
    ///
    /// This should support both the names generated in a web browser,
    /// and by winit. Please test on both with `eframe`.
    pub fn from_name(key: &str) -> Option<Self> {
        Some(match key {
            "⏷" | "ArrowDown" | "Down" => Self::ArrowDown,
            "⏴" | "ArrowLeft" | "Left" => Self::ArrowLeft,
            "⏵" | "ArrowRight" | "Right" => Self::ArrowRight,
            "⏶" | "ArrowUp" | "Up" => Self::ArrowUp,

            "Escape" | "Esc" => Self::Escape,
            "Tab" => Self::Tab,
            "Backspace" => Self::Backspace,
            "Enter" | "Return" => Self::Enter,

            "Help" | "Insert" => Self::Insert,
            "Delete" => Self::Delete,
            "Home" => Self::Home,
            "End" => Self::End,
            "PageUp" => Self::PageUp,
            "PageDown" => Self::PageDown,

            "Copy" => Self::Copy,
            "Cut" => Self::Cut,
            "Paste" => Self::Paste,

            " " | "Space" => Self::Space,
            ":" | "Colon" => Self::Colon,
            "," | "Comma" => Self::Comma,
            "-" | "−" | "Minus" => Self::Minus,
            "." | "Period" => Self::Period,
            "+" | "Plus" => Self::Plus,
            "=" | "Equal" | "Equals" | "NumpadEqual" => Self::Equals,
            ";" | "Semicolon" => Self::Semicolon,
            "\\" | "Backslash" => Self::Backslash,
            "/" | "Slash" => Self::Slash,
            "|" | "Pipe" => Self::Pipe,
            "?" | "Questionmark" => Self::Questionmark,
            "[" | "OpenBracket" => Self::OpenBracket,
            "]" | "CloseBracket" => Self::CloseBracket,
            "`" | "Backtick" | "Backquote" | "Grave" => Self::Backtick,

            "0" | "Digit0" | "Numpad0" => Self::Num0,
            "1" | "Digit1" | "Numpad1" => Self::Num1,
            "2" | "Digit2" | "Numpad2" => Self::Num2,
            "3" | "Digit3" | "Numpad3" => Self::Num3,
            "4" | "Digit4" | "Numpad4" => Self::Num4,
            "5" | "Digit5" | "Numpad5" => Self::Num5,
            "6" | "Digit6" | "Numpad6" => Self::Num6,
            "7" | "Digit7" | "Numpad7" => Self::Num7,
            "8" | "Digit8" | "Numpad8" => Self::Num8,
            "9" | "Digit9" | "Numpad9" => Self::Num9,

            "a" | "A" => Self::A,
            "b" | "B" => Self::B,
            "c" | "C" => Self::C,
            "d" | "D" => Self::D,
            "e" | "E" => Self::E,
            "f" | "F" => Self::F,
            "g" | "G" => Self::G,
            "h" | "H" => Self::H,
            "i" | "I" => Self::I,
            "j" | "J" => Self::J,
            "k" | "K" => Self::K,
            "l" | "L" => Self::L,
            "m" | "M" => Self::M,
            "n" | "N" => Self::N,
            "o" | "O" => Self::O,
            "p" | "P" => Self::P,
            "q" | "Q" => Self::Q,
            "r" | "R" => Self::R,
            "s" | "S" => Self::S,
            "t" | "T" => Self::T,
            "u" | "U" => Self::U,
            "v" | "V" => Self::V,
            "w" | "W" => Self::W,
            "x" | "X" => Self::X,
            "y" | "Y" => Self::Y,
            "z" | "Z" => Self::Z,

            "F1" => Self::F1,
            "F2" => Self::F2,
            "F3" => Self::F3,
            "F4" => Self::F4,
            "F5" => Self::F5,
            "F6" => Self::F6,
            "F7" => Self::F7,
            "F8" => Self::F8,
            "F9" => Self::F9,
            "F10" => Self::F10,
            "F11" => Self::F11,
            "F12" => Self::F12,
            "F13" => Self::F13,
            "F14" => Self::F14,
            "F15" => Self::F15,
            "F16" => Self::F16,
            "F17" => Self::F17,
            "F18" => Self::F18,
            "F19" => Self::F19,
            "F20" => Self::F20,
            "F21" => Self::F21,
            "F22" => Self::F22,
            "F23" => Self::F23,
            "F24" => Self::F24,
            "F25" => Self::F25,
            "F26" => Self::F26,
            "F27" => Self::F27,
            "F28" => Self::F28,
            "F29" => Self::F29,
            "F30" => Self::F30,
            "F31" => Self::F31,
            "F32" => Self::F32,
            "F33" => Self::F33,
            "F34" => Self::F34,
            "F35" => Self::F35,

            _ => return None,
        })
    }

    /// Emoji or name representing the key
    pub fn symbol_or_name(self) -> &'static str {
        // TODO(emilk): add support for more unicode symbols (see for instance https://wincent.com/wiki/Unicode_representations_of_modifier_keys).
        // Before we do we must first make sure they are supported in `Fonts` though,
        // so perhaps this functions needs to take a `supports_character: impl Fn(char) -> bool` or something.
        match self {
            Self::ArrowDown => "⏷",
            Self::ArrowLeft => "⏴",
            Self::ArrowRight => "⏵",
            Self::ArrowUp => "⏶",

            Self::Colon => ":",
            Self::Comma => ",",
            Self::Minus => crate::MINUS_CHAR_STR,
            Self::Period => ".",
            Self::Plus => "+",
            Self::Equals => "=",
            Self::Semicolon => ";",
            Self::Backslash => "\\",
            Self::Slash => "/",
            Self::Pipe => "|",
            Self::Questionmark => "?",
            Self::OpenBracket => "[",
            Self::CloseBracket => "]",
            Self::Backtick => "`",

            _ => self.name(),
        }
    }

    /// Human-readable English name.
    pub fn name(self) -> &'static str {
        match self {
            Self::ArrowDown => "Down",
            Self::ArrowLeft => "Left",
            Self::ArrowRight => "Right",
            Self::ArrowUp => "Up",

            Self::Escape => "Escape",
            Self::Tab => "Tab",
            Self::Backspace => "Backspace",
            Self::Enter => "Enter",

            Self::Insert => "Insert",
            Self::Delete => "Delete",
            Self::Home => "Home",
            Self::End => "End",
            Self::PageUp => "PageUp",
            Self::PageDown => "PageDown",

            Self::Copy => "Copy",
            Self::Cut => "Cut",
            Self::Paste => "Paste",

            Self::Space => "Space",
            Self::Colon => "Colon",
            Self::Comma => "Comma",
            Self::Minus => "Minus",
            Self::Period => "Period",
            Self::Plus => "Plus",
            Self::Equals => "Equals",
            Self::Semicolon => "Semicolon",
            Self::Backslash => "Backslash",
            Self::Slash => "Slash",
            Self::Pipe => "Pipe",
            Self::Questionmark => "Questionmark",
            Self::OpenBracket => "OpenBracket",
            Self::CloseBracket => "CloseBracket",
            Self::Backtick => "Backtick",

            Self::Num0 => "0",
            Self::Num1 => "1",
            Self::Num2 => "2",
            Self::Num3 => "3",
            Self::Num4 => "4",
            Self::Num5 => "5",
            Self::Num6 => "6",
            Self::Num7 => "7",
            Self::Num8 => "8",
            Self::Num9 => "9",

            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
            Self::D => "D",
            Self::E => "E",
            Self::F => "F",
            Self::G => "G",
            Self::H => "H",
            Self::I => "I",
            Self::J => "J",
            Self::K => "K",
            Self::L => "L",
            Self::M => "M",
            Self::N => "N",
            Self::O => "O",
            Self::P => "P",
            Self::Q => "Q",
            Self::R => "R",
            Self::S => "S",
            Self::T => "T",
            Self::U => "U",
            Self::V => "V",
            Self::W => "W",
            Self::X => "X",
            Self::Y => "Y",
            Self::Z => "Z",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
            Self::F11 => "F11",
            Self::F12 => "F12",
            Self::F13 => "F13",
            Self::F14 => "F14",
            Self::F15 => "F15",
            Self::F16 => "F16",
            Self::F17 => "F17",
            Self::F18 => "F18",
            Self::F19 => "F19",
            Self::F20 => "F20",
            Self::F21 => "F21",
            Self::F22 => "F22",
            Self::F23 => "F23",
            Self::F24 => "F24",
            Self::F25 => "F25",
            Self::F26 => "F26",
            Self::F27 => "F27",
            Self::F28 => "F28",
            Self::F29 => "F29",
            Self::F30 => "F30",
            Self::F31 => "F31",
            Self::F32 => "F32",
            Self::F33 => "F33",
            Self::F34 => "F34",
            Self::F35 => "F35",
        }
    }
}

#[test]
fn test_key_from_name() {
    assert_eq!(
        Key::ALL.len(),
        Key::F35 as usize + 1,
        "Some keys are missing in Key::ALL"
    );

    for &key in Key::ALL {
        let name = key.name();
        assert_eq!(
            Key::from_name(name),
            Some(key),
            "Failed to roundtrip {key:?} from name {name:?}"
        );

        let symbol = key.symbol_or_name();
        assert_eq!(
            Key::from_name(symbol),
            Some(key),
            "Failed to roundtrip {key:?} from symbol {symbol:?}"
        );
    }
}
