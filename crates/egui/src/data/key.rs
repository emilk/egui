use core::str::FromStr;

pub use keyboard_types::NamedKey;
use smol_str::SmolStr;

/// A logical keyboard key.
///
/// Character keys depend on the integration and platform:
///
/// - On supported desktop platforms, `egui-winit` reports the logical key without modifiers.
///   For example, `Shift+A` reports `"a"` and `Shift+1` reports `"1"` on a US layout.
/// - `eframe` on the web uses [`KeyboardEvent.key`](https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key).
///   Browsers do not generally provide the logical key without modifiers. Character values are
///   lowercased, so `Shift+A` reports `"a"`, but `Shift+1` reports `"!"` on a US layout,
///   and `Option+A` on macOS reports `"å"`.
/// - Other integrations and platforms may report different values, including modifier effects.
///
/// This affects logical key queries and shortcuts; lowercasing cannot undo every modifier.
/// The text produced by a key press is sent separately as [`crate::Event::Text`].
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Key<Str = SmolStr> {
    Character(Str),
    Named(NamedKey),
}

/// Parses a W3C key value, as reported by e.g. [`KeyboardEvent.key`](https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key):
/// a [`NamedKey`] name (including `"Dead"` and `"Unidentified"`), or else the produced character as-is.
///
/// Fails only for the empty string. See [`Key::from_name`] for parsing human-readable key names.
impl<S> FromStr for Key<S>
where
    S: for<'a> From<&'a str>,
{
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<NamedKey>() {
            Ok(named_key) => Ok(Self::Named(named_key)),
            Err(_) if s.is_empty() => Err(()),
            Err(_) => Ok(Self::Character(s.into())),
        }
    }
}

impl<S: From<String>> From<keyboard_types::Key> for Key<S> {
    fn from(value: keyboard_types::Key) -> Self {
        match value {
            keyboard_types::Key::Character(c) => Self::Character(c.into()),
            keyboard_types::Key::Named(named_key) => Self::Named(named_key),
        }
    }
}

impl<S> From<NamedKey> for Key<S> {
    #[inline]
    fn from(named_key: NamedKey) -> Self {
        Self::Named(named_key)
    }
}

impl<Str> PartialEq<NamedKey> for Key<Str> {
    #[inline]
    fn eq(&self, rhs: &NamedKey) -> bool {
        match self {
            Self::Named(a) => a == rhs,
            Self::Character(_) => false,
        }
    }
}

impl<Str: PartialEq<str>> PartialEq<str> for Key<Str> {
    #[inline]
    fn eq(&self, rhs: &str) -> bool {
        match self {
            Self::Character(s) => s == rhs,
            Self::Named(_) => false,
        }
    }
}

impl<Str: PartialEq<str>> PartialEq<&str> for Key<Str> {
    #[inline]
    fn eq(&self, rhs: &&str) -> bool {
        self == *rhs
    }
}

impl<S: AsRef<str>> Key<S> {
    /// Borrow the character string, making it easier to match on a `Key`.
    /// Named keys remain unchanged.
    pub fn as_ref(&self) -> Key<&str> {
        match self {
            Self::Named(a) => Key::Named(*a),
            Self::Character(ch) => Key::Character(ch.as_ref()),
        }
    }
}

impl Key<SmolStr> {
    /// A [`Key::Character`] key, usable in `const` contexts.
    ///
    /// ```
    /// # use egui::Key;
    /// const SPACE: Key = Key::character(" ");
    /// ```
    #[inline]
    pub const fn character(s: &'static str) -> Self {
        Self::Character(SmolStr::new_static(s))
    }
}

impl Key {
    /// Compare two keys, ignoring the ASCII letter case of [`Key::Character`] keys.
    ///
    /// Useful for keys that keep their letter case, e.g. parsed from user configuration.
    ///
    /// ```
    /// # use egui::Key;
    /// assert!(Key::character("z").eq_ignore_ascii_case(&Key::character("Z")));
    /// assert!(!Key::character("z").eq_ignore_ascii_case(&Key::character("y")));
    /// ```
    pub fn eq_ignore_ascii_case(&self, other: &Self) -> bool {
        match (self.as_ref(), other.as_ref()) {
            (Key::Character(a), Key::Character(b)) => a.eq_ignore_ascii_case(b),
            _ => self == other,
        }
    }

    /// Parses a logical key name, e.g. `"A"` as `Key::character("a")`
    /// or `"PageUp"` as `Key::Named(NamedKey::PageUp)`.
    ///
    /// This will parse the output of both [`Self::name`] and [`Self::symbol_or_name`],
    /// as well as the W3C names of all [`NamedKey`]s (as reported by web browsers and winit),
    /// and any single character. Single ASCII letters are parsed as lowercase.
    ///
    /// For backwards compatibility it also parses the names of the old `egui::Key` enum
    /// (from before egui used [`keyboard_types`]), e.g. `"Esc"`, `"ShiftLeft"`, `"KeyA"`, `"Numpad1"`.
    /// Keys that used to be distinct are mapped to their logical equivalent,
    /// e.g. `"ShiftLeft"` and `"ShiftRight"` both become [`NamedKey::Shift`].
    ///
    /// Returns `None` for empty or unknown multi-character strings,
    /// and for [`NamedKey::Dead`] and [`NamedKey::Unidentified`].
    ///
    /// ```
    /// # use egui::{Key, NamedKey};
    /// assert_eq!(Key::from_name("A"), Some(Key::character("a")));
    /// assert_eq!(Key::from_name("-"), Some(Key::character("-")));
    /// assert_eq!(Key::from_name("Minus"), Some(Key::character("-")));
    /// assert_eq!(Key::from_name("⏶"), Some(Key::Named(NamedKey::ArrowUp)));
    /// assert_eq!(Key::from_name("Up"), Some(Key::Named(NamedKey::ArrowUp)));
    /// assert_eq!(Key::from_name("ArrowUp"), Some(Key::Named(NamedKey::ArrowUp)));
    /// assert_eq!(Key::from_name("Esc"), Some(Key::Named(NamedKey::Escape)));
    /// assert_eq!(Key::from_name("KeyA"), Some(Key::character("a")));
    /// assert_eq!(Key::from_name("Foo"), None);
    /// ```
    pub fn from_name(key: &str) -> Option<Self> {
        let arrow = match key {
            "⏷" | "Down" => Some(NamedKey::ArrowDown),
            "⏴" | "Left" => Some(NamedKey::ArrowLeft),
            "⏵" | "Right" => Some(NamedKey::ArrowRight),
            "⏶" | "Up" => Some(NamedKey::ArrowUp),
            _ => None,
        };
        if let Some(arrow) = arrow {
            return Some(Self::Named(arrow));
        }

        if key == crate::MINUS_CHAR_STR {
            return Some(Self::character("-"));
        }

        if let Some(&(character, _)) = CHARACTER_NAMES.iter().find(|(_, name)| *name == key) {
            return Some(Self::character(character));
        }

        match key.parse::<NamedKey>() {
            Ok(NamedKey::Dead | NamedKey::Unidentified) => None,
            Ok(named_key) => Some(Self::Named(named_key)),
            Err(_) => single_character_key(key).or_else(|| from_legacy_name(key)),
        }
    }

    /// Emoji or name representing the key.
    ///
    /// Same as [`Self::name`], except arrow keys and punctuation are shown as symbols.
    ///
    /// ```
    /// # use egui::{Key, NamedKey};
    /// assert_eq!(Key::Named(NamedKey::ArrowUp).symbol_or_name(), "⏶");
    /// assert_eq!(Key::character("[").symbol_or_name(), "[");
    /// assert_eq!(Key::character("a").symbol_or_name(), "A");
    /// assert_eq!(Key::character(" ").symbol_or_name(), "Space");
    /// ```
    pub fn symbol_or_name(&self) -> &str {
        // TODO(emilk): add support for more unicode symbols (see for instance https://wincent.com/wiki/Unicode_representations_of_modifier_keys).
        // Before we do we must first make sure they are supported in `Fonts` though,
        // so perhaps this functions needs to take a `supports_character: impl Fn(char) -> bool` or something.
        match self.as_ref() {
            Key::Named(NamedKey::ArrowDown) => "⏷",
            Key::Named(NamedKey::ArrowLeft) => "⏴",
            Key::Named(NamedKey::ArrowRight) => "⏵",
            Key::Named(NamedKey::ArrowUp) => "⏶",
            Key::Character("-") => crate::MINUS_CHAR_STR,
            Key::Character(s) if matches!(s.as_bytes(), [b] if b.is_ascii_punctuation()) => s,
            _ => self.name(),
        }
    }

    /// Human-readable English name.
    ///
    /// Single ASCII letters are upper-cased, and space and ASCII punctuation are spelled out.
    /// Other characters are returned as-is.
    ///
    /// ```
    /// # use egui::{Key, NamedKey};
    /// assert_eq!(Key::Named(NamedKey::ArrowUp).name(), "Up");
    /// assert_eq!(Key::Named(NamedKey::PageDown).name(), "PageDown");
    /// assert_eq!(Key::character("[").name(), "OpenBracket");
    /// assert_eq!(Key::character("a").name(), "A");
    /// assert_eq!(Key::character("é").name(), "é");
    /// ```
    pub fn name(&self) -> &str {
        match self.as_ref() {
            Key::Named(NamedKey::ArrowDown) => "Down",
            Key::Named(NamedKey::ArrowLeft) => "Left",
            Key::Named(NamedKey::ArrowRight) => "Right",
            Key::Named(NamedKey::ArrowUp) => "Up",
            Key::Named(named_key) => named_key_str(named_key),
            Key::Character(s) => character_name(s),
        }
    }
}

/// Parses a single character as a [`Key::Character`], lowercasing ASCII letters.
fn single_character_key(s: &str) -> Option<Key> {
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => {
            let mut buf = [0; 4];
            let lowercase = c.to_ascii_lowercase().encode_utf8(&mut buf);
            Some(Key::Character(SmolStr::new(lowercase)))
        }
        _ => None,
    }
}

/// Parses names of the old `egui::Key` enum (and the aliases it accepted)
/// that are not otherwise understood by [`Key::from_name`].
fn from_legacy_name(name: &str) -> Option<Key> {
    let named_key = match name {
        "Esc" => Some(NamedKey::Escape),
        "Return" | "NumpadEnter" => Some(NamedKey::Enter),
        "LaunchApp1" => Some(NamedKey::LaunchApplication1),
        "LaunchApp2" => Some(NamedKey::LaunchApplication2),
        "ShiftLeft" | "ShiftRight" => Some(NamedKey::Shift),
        "ControlLeft" | "ControlRight" => Some(NamedKey::Control),
        "AltLeft" | "AltRight" => Some(NamedKey::Alt),
        "SuperLeft" | "SuperRight" | "MetaLeft" | "MetaRight" | "OSLeft" | "OSRight" => {
            Some(NamedKey::Meta)
        }
        "Menu" | "Apps" => Some(NamedKey::ContextMenu),
        "Globe" => Some(NamedKey::Fn),
        "Sleep" => Some(NamedKey::Standby),
        _ => None,
    };
    if let Some(named_key) = named_key {
        return Some(Key::Named(named_key));
    }

    let character = match name {
        "NumpadComma" => Some(","),
        "NumpadSubtract" => Some("-"),
        "NumpadDecimal" => Some("."),
        "NumpadAdd" => Some("+"),
        "Equal" | "NumpadEqual" => Some("="),
        "NumpadDivide" => Some("/"),
        "BracketLeft" => Some("["),
        "BracketRight" => Some("]"),
        "Backquote" | "Grave" => Some("`"),
        "IntlBackslash" => Some("\\"),
        _ => None,
    };
    if let Some(character) = character {
        return Some(Key::character(character));
    }

    // Physical key code names, e.g. `KeyA`, `Digit1`, `Numpad1`.
    let letter = name
        .strip_prefix("Key")
        .filter(|s| matches!(s.as_bytes(), [b'A'..=b'Z']));
    let digit = name
        .strip_prefix("Digit")
        .or_else(|| name.strip_prefix("Numpad"))
        .filter(|s| matches!(s.as_bytes(), [b'0'..=b'9']));
    letter.or(digit).and_then(single_character_key)
}

/// Human-readable English names of space and ASCII punctuation [`Key::Character`]s.
const CHARACTER_NAMES: [(&str, &str); 19] = [
    (" ", "Space"),
    (":", "Colon"),
    (",", "Comma"),
    ("-", "Minus"),
    (".", "Period"),
    ("+", "Plus"),
    ("=", "Equals"),
    (";", "Semicolon"),
    ("\\", "Backslash"),
    ("/", "Slash"),
    ("|", "Pipe"),
    ("?", "Questionmark"),
    ("!", "Exclamationmark"),
    ("[", "OpenBracket"),
    ("]", "CloseBracket"),
    ("{", "OpenCurlyBracket"),
    ("}", "CloseCurlyBracket"),
    ("`", "Backtick"),
    ("'", "Quote"),
];

/// Human-readable English name of the text produced by a [`Key::Character`].
fn character_name(s: &str) -> &str {
    const ASCII_UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";

    if let Some(&(_, name)) = CHARACTER_NAMES
        .iter()
        .find(|(character, _)| *character == s)
    {
        return name;
    }

    match s.as_bytes() {
        [b @ b'a'..=b'z'] => {
            let i = usize::from(b - b'a');
            &ASCII_UPPERCASE[i..=i]
        }
        _ => s,
    }
}

/// The W3C name of a [`NamedKey`], e.g. `"PageUp"`.
///
/// Same as the `Display` output of [`NamedKey`], but without allocating.
#[expect(
    deprecated,
    reason = "NamedKey::Hyper and NamedKey::Super are deprecated, but can still be reported by backends"
)]
fn named_key_str(named_key: NamedKey) -> &'static str {
    macro_rules! names {
        ($($variant:ident)*) => {
            match named_key {
                $(NamedKey::$variant => stringify!($variant),)*
                // `NamedKey` is `#[non_exhaustive]`.
                _ => "Unidentified",
            }
        };
    }

    names! {
        Unidentified Alt AltGraph CapsLock Control Fn FnLock Meta NumLock ScrollLock Shift Symbol
        SymbolLock Hyper Super Enter Tab ArrowDown ArrowLeft ArrowRight ArrowUp End Home PageDown
        PageUp Backspace Clear Copy CrSel Cut Delete EraseEof ExSel Insert Paste Redo Undo Accept
        Again Attn Cancel ContextMenu Escape Execute Find Help Pause Play Props Select ZoomIn
        ZoomOut BrightnessDown BrightnessUp Eject LogOff Power PowerOff PrintScreen Hibernate
        Standby WakeUp AllCandidates Alphanumeric CodeInput Compose Convert Dead FinalMode
        GroupFirst GroupLast GroupNext GroupPrevious ModeChange NextCandidate NonConvert
        PreviousCandidate Process SingleCandidate HangulMode HanjaMode JunjaMode Eisu Hankaku
        Hiragana HiraganaKatakana KanaMode KanjiMode Katakana Romaji Zenkaku ZenkakuHankaku Soft1
        Soft2 Soft3 Soft4 ChannelDown ChannelUp Close MailForward MailReply MailSend MediaClose
        MediaFastForward MediaPause MediaPlay MediaPlayPause MediaRecord MediaRewind MediaStop
        MediaTrackNext MediaTrackPrevious New Open Print Save SpellCheck Key11 Key12
        AudioBalanceLeft AudioBalanceRight AudioBassBoostDown AudioBassBoostToggle
        AudioBassBoostUp AudioFaderFront AudioFaderRear AudioSurroundModeNext AudioTrebleDown
        AudioTrebleUp AudioVolumeDown AudioVolumeUp AudioVolumeMute MicrophoneToggle
        MicrophoneVolumeDown MicrophoneVolumeUp MicrophoneVolumeMute SpeechCorrectionList
        SpeechInputToggle LaunchApplication1 LaunchApplication2 LaunchCalendar LaunchContacts
        LaunchMail LaunchMediaPlayer LaunchMusicPlayer LaunchPhone LaunchScreenSaver
        LaunchSpreadsheet LaunchWebBrowser LaunchWebCam LaunchWordProcessor BrowserBack
        BrowserFavorites BrowserForward BrowserHome BrowserRefresh BrowserSearch BrowserStop
        AppSwitch Call Camera CameraFocus EndCall GoBack GoHome HeadsetHook LastNumberRedial
        Notification MannerMode VoiceDial TV TV3DMode TVAntennaCable TVAudioDescription
        TVAudioDescriptionMixDown TVAudioDescriptionMixUp TVContentsMenu TVDataService TVInput
        TVInputComponent1 TVInputComponent2 TVInputComposite1 TVInputComposite2 TVInputHDMI1
        TVInputHDMI2 TVInputHDMI3 TVInputHDMI4 TVInputVGA1 TVMediaContext TVNetwork
        TVNumberEntry TVPower TVRadioService TVSatellite TVSatelliteBS TVSatelliteCS
        TVSatelliteToggle TVTerrestrialAnalog TVTerrestrialDigital TVTimer AVRInput AVRPower
        ColorF0Red ColorF1Green ColorF2Yellow ColorF3Blue ColorF4Grey ColorF5Brown
        ClosedCaptionToggle Dimmer DisplaySwap DVR Exit FavoriteClear0 FavoriteClear1
        FavoriteClear2 FavoriteClear3 FavoriteRecall0 FavoriteRecall1 FavoriteRecall2
        FavoriteRecall3 FavoriteStore0 FavoriteStore1 FavoriteStore2 FavoriteStore3 Guide
        GuideNextDay GuidePreviousDay Info InstantReplay Link ListProgram LiveContent Lock
        MediaApps MediaAudioTrack MediaLast MediaSkipBackward MediaSkipForward MediaStepBackward
        MediaStepForward MediaTopMenu NavigateIn NavigateNext NavigateOut NavigatePrevious
        NextFavoriteChannel NextUserProfile OnDemand Pairing PinPDown PinPMove PinPToggle PinPUp
        PlaySpeedDown PlaySpeedReset PlaySpeedUp RandomToggle RcLowBattery RecordSpeedNext
        RfBypass ScanChannelsToggle ScreenModeNext Settings SplitScreenToggle STBInput STBPower
        Subtitle Teletext VideoModeNext Wink ZoomToggle F1 F2 F3 F4 F5 F6 F7 F8 F9 F10 F11 F12
        F13 F14 F15 F16 F17 F18 F19 F20 F21 F22 F23 F24 F25 F26 F27 F28 F29 F30 F31 F32 F33 F34
        F35
    }
}

#[test]
fn test_key_from_name_roundtrip() {
    let mut keys: Vec<Key> = [
        NamedKey::ArrowDown,
        NamedKey::ArrowLeft,
        NamedKey::ArrowRight,
        NamedKey::ArrowUp,
        NamedKey::Escape,
        NamedKey::Enter,
        NamedKey::PageUp,
        NamedKey::Shift,
        NamedKey::MediaPlayPause,
        NamedKey::TV3DMode,
        NamedKey::F35,
    ]
    .into_iter()
    .map(Key::Named)
    .collect();
    keys.extend(CHARACTER_NAMES.iter().map(|&(c, _)| Key::character(c)));
    keys.extend(
        ('a'..='z')
            .chain('0'..='9')
            .map(|c| Key::Character(c.to_string().into())),
    );
    keys.push(Key::character("é"));

    for key in keys {
        let name = key.name();
        assert_eq!(
            Key::from_name(name).as_ref(),
            Some(&key),
            "Failed to roundtrip {key:?} from name {name:?}"
        );

        let symbol = key.symbol_or_name();
        assert_eq!(
            Key::from_name(symbol).as_ref(),
            Some(&key),
            "Failed to roundtrip {key:?} from symbol {symbol:?}"
        );
    }
}

#[test]
fn test_key_from_legacy_name() {
    // Every name returned by the old `egui::Key::name` must still parse.
    let legacy_names = [
        "Down",
        "Left",
        "Right",
        "Up",
        "Escape",
        "Tab",
        "Backspace",
        "Enter",
        "Insert",
        "Delete",
        "Home",
        "End",
        "PageUp",
        "PageDown",
        "Copy",
        "Cut",
        "Paste",
        "Space",
        "Colon",
        "Comma",
        "Minus",
        "Period",
        "Plus",
        "Equals",
        "Semicolon",
        "Backslash",
        "Slash",
        "Pipe",
        "Questionmark",
        "Exclamationmark",
        "OpenBracket",
        "CloseBracket",
        "OpenCurlyBracket",
        "CloseCurlyBracket",
        "Backtick",
        "Quote",
        "BrowserBack",
        "BrowserForward",
        "BrowserRefresh",
        "BrowserSearch",
        "BrowserHome",
        "BrowserFavorites",
        "BrowserStop",
        "MediaPlayPause",
        "MediaTrackNext",
        "MediaTrackPrevious",
        "MediaStop",
        "AudioVolumeMute",
        "AudioVolumeDown",
        "AudioVolumeUp",
        "LaunchMail",
        "LaunchApp1",
        "LaunchApp2",
        "ShiftLeft",
        "ShiftRight",
        "ControlLeft",
        "ControlRight",
        "AltLeft",
        "AltRight",
        "SuperLeft",
        "SuperRight",
        "IntlBackslash",
        "CapsLock",
        "NumLock",
        "ScrollLock",
        "PrintScreen",
        "Pause",
        "Menu",
        "Fn",
        "Eject",
        "Help",
        "Power",
        "Sleep",
        "Clear",
    ];
    let generated = ('0'..='9')
        .chain('A'..='Z')
        .map(|c| c.to_string())
        .chain((1..=35).map(|n| format!("F{n}")));
    for name in legacy_names
        .iter()
        .map(|s| (*s).to_owned())
        .chain(generated)
    {
        assert!(
            Key::from_name(&name).is_some(),
            "Failed to parse legacy name {name:?}"
        );
    }

    // Aliases that map onto a different logical key.
    for (name, expected) in [
        ("Esc", Key::Named(NamedKey::Escape)),
        ("Return", Key::Named(NamedKey::Enter)),
        ("NumpadEnter", Key::Named(NamedKey::Enter)),
        ("ShiftRight", Key::Named(NamedKey::Shift)),
        ("OSLeft", Key::Named(NamedKey::Meta)),
        ("Menu", Key::Named(NamedKey::ContextMenu)),
        ("Sleep", Key::Named(NamedKey::Standby)),
        ("LaunchApp1", Key::Named(NamedKey::LaunchApplication1)),
        ("NumpadSubtract", Key::character("-")),
        ("Equal", Key::character("=")),
        ("BracketLeft", Key::character("[")),
        ("Grave", Key::character("`")),
        ("KeyQ", Key::character("q")),
        ("Digit7", Key::character("7")),
        ("Numpad0", Key::character("0")),
    ] {
        assert_eq!(Key::from_name(name), Some(expected), "{name:?}");
    }

    for name in ["Keya", "KeyAB", "Digit", "Numpad10"] {
        assert_eq!(Key::from_name(name), None, "{name:?}");
    }
}
