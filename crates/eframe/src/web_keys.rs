//! Turning DOM `KeyboardEvent` values into egui keys and text.
//!
//! Kept free of `web-sys` so the logic can be tested natively.

/// Whether `key`, a `KeyboardEvent.key` value, names a key rather than the text it types.
///
/// Named key values (`Enter`, `AltGraph`, `Dead`, `Unidentified`, `F1`, `Soft1`, …) are ASCII
/// alphanumerics that start with an uppercase letter and are longer than one character.
/// Length alone doesn't decide: some keys type several characters (Arabic keyboards type `لا`
/// with one key).
pub fn is_named_key(key: &str) -> bool {
    key.len() > 1
        && key.starts_with(|c: char| c.is_ascii_uppercase())
        && key.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// The text typed by a key press with this `KeyboardEvent.key` value, if any.
pub fn text_from_key(key: &str) -> Option<&str> {
    (!key.is_empty() && !is_named_key(key)).then_some(key)
}

/// The letter key for a `KeyboardEvent.code` value `KeyA` … `KeyZ`.
fn letter_for_code(code: &str) -> Option<egui::Key> {
    let letter = code.strip_prefix("Key")?;
    if letter.len() == 1 && letter.bytes().all(|b| b.is_ascii_uppercase()) {
        egui::Key::from_name(letter)
    } else {
        None
    }
}

/// Whether a layout is Latin, from the characters its keys `KeyA` … `KeyZ` type.
///
/// Latin layouts put punctuation on a few of those keys (Dvorak's `'`, AZERTY's `,`), and
/// non-Latin ones put ASCII punctuation on some (Hebrew `/`, Greek `;`), so the majority decides.
/// `None` when there are no characters to judge by.
pub fn layout_is_latin<'a>(letter_key_chars: impl IntoIterator<Item = &'a str>) -> Option<bool> {
    let mut total = 0;
    let mut ascii_letters = 0;
    for key in letter_key_chars {
        total += 1;
        if key.len() == 1 && key.bytes().all(|b| b.is_ascii_alphabetic()) {
            ascii_letters += 1;
        }
    }
    (total > 0).then_some(2 * ascii_letters > total)
}

/// What one plain press (no modifiers) of a letter key says about the layout:
/// `Some(true)` for an ASCII letter, `Some(false)` for any other letter, `None` for punctuation.
fn press_evidence(key: &str, code: &str) -> Option<bool> {
    letter_for_code(code)?;
    let first = key.chars().next()?;
    if key.len() == 1 && first.is_ascii_alphabetic() {
        Some(true)
    } else if first.is_alphabetic() && !first.is_ascii() {
        Some(false)
    } else {
        None
    }
}

/// How many non-Latin letters in a row it takes to decide the layout isn't Latin, when there is
/// no layout map. More than one, so a Latin layout with a few non-ASCII letters on its letter
/// keys (Turkish `ı`, Northern Sami `á`) doesn't flip on every one of them.
const NON_LATIN_RUN: u8 = 3;

/// The keys of one `keydown` event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyDown {
    /// The logical key, for [`egui::Event::Key::key`].
    pub key: Option<egui::Key>,

    /// The key at this position on a US layout, for [`egui::Event::Key::physical_key`].
    pub physical_key: Option<egui::Key>,

    /// The press contradicts the layout map: the user may have switched layouts since it was read.
    pub query_layout_map: bool,
}

/// Turns `KeyboardEvent.key` and `KeyboardEvent.code` into egui keys.
///
/// On a non-Latin layout (Russian, Arabic, Hebrew, Greek, …) the letter keys `KeyA` … `KeyZ`
/// resolve from the key code, so letter shortcuts work whatever the key types. On a Latin layout
/// (QWERTY, Dvorak, AZERTY, …) every key resolves from the character it types.
///
/// Whether the layout is Latin is decided for the whole layout, not per key, so that Hebrew's
/// `KeyQ` (which types `/`) still gives [`egui::Key::Q`]. It comes from the layout map where the
/// browser has one (`navigator.keyboard.getLayoutMap()`, Chromium), and otherwise from the letters
/// the user types. Until either says otherwise, the layout counts as Latin.
#[derive(Debug, Default)]
pub struct KeyResolver {
    latin: Option<bool>,

    /// [`Self::latin`] came from a layout map, so presses only ask for it to be read again.
    from_layout_map: bool,

    /// Non-Latin letters typed in a row, without a layout map.
    non_latin_run: u8,

    /// The logical key each held key code resolved to on `keydown`, so its `keyup` releases the
    /// same key even if the layout changed in between.
    pressed: std::collections::HashMap<String, egui::Key>,
}

impl KeyResolver {
    /// Whether the letter keys resolve from the characters they type.
    pub fn is_latin(&self) -> bool {
        self.latin.unwrap_or(true)
    }

    /// Set the layout from the characters a layout map gives for `KeyA` … `KeyZ`.
    pub fn set_layout_map<'a>(&mut self, letter_key_chars: impl IntoIterator<Item = &'a str>) {
        if let Some(latin) = layout_is_latin(letter_key_chars) {
            self.latin = Some(latin);
            self.from_layout_map = true;
            self.non_latin_run = 0;
        }
    }

    /// Resolve a `keydown`. `plain` is true when no modifier (Shift, Ctrl, Alt, Meta, `AltGr`) is
    /// held, the only presses that tell us about the layout.
    pub fn key_down(&mut self, key: &str, code: &str, plain: bool) -> KeyDown {
        let mut query_layout_map = false;
        if let Some(latin) = plain.then(|| press_evidence(key, code)).flatten() {
            if self.from_layout_map {
                query_layout_map = latin != self.is_latin();
            } else if latin {
                self.latin = Some(true);
                self.non_latin_run = 0;
            } else {
                self.non_latin_run = self.non_latin_run.saturating_add(1);
                if NON_LATIN_RUN <= self.non_latin_run {
                    self.latin = Some(false);
                }
            }
        }

        let physical_key = egui::Key::from_name(code);
        // Modifier keys, and keys that type something egui has no key for, fall back to the
        // physical key.
        let logical_key = self.resolve(key, code).or(physical_key);
        if let Some(logical_key) = logical_key
            && !code.is_empty()
        {
            self.pressed.insert(code.to_owned(), logical_key);
        }
        KeyDown {
            key: logical_key,
            physical_key,
            query_layout_map,
        }
    }

    /// Resolve a `keyup`: the logical key and the physical key.
    pub fn key_up(&mut self, key: &str, code: &str) -> (Option<egui::Key>, Option<egui::Key>) {
        let physical_key = egui::Key::from_name(code);
        let logical_key = self
            .pressed
            .remove(code)
            .or_else(|| self.resolve(key, code))
            .or(physical_key);
        (logical_key, physical_key)
    }

    /// Forget the held keys, when focus is lost and their `keyup` events go elsewhere.
    pub fn clear_pressed(&mut self) {
        self.pressed.clear();
    }

    fn resolve(&self, key: &str, code: &str) -> Option<egui::Key> {
        if !self.is_latin()
            && let Some(letter) = letter_for_code(code)
        {
            return Some(letter);
        }
        egui::Key::from_name(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_keys_are_not_text() {
        for key in [
            "AltGraph",
            "Dead",
            "Unidentified",
            "Process",
            "Compose",
            "Enter",
            "Escape",
            "Esc",
            "F1",
            "F12",
            "Soft1",
            "GroupNext",
            "KanaMode",
            "HangulMode",
            "Shift",
            "ArrowLeft",
            "AudioVolumeUp",
            "",
        ] {
            assert_eq!(text_from_key(key), None, "{key:?}");
        }
    }

    #[test]
    fn printable_keys_are_text() {
        for key in [
            "a", "A", "F", "1", " ", "/", "é", "ф", "Ж", "ש", "ب", "لا", "λ", "ß", "😀",
        ] {
            assert_eq!(text_from_key(key), Some(key), "{key:?}");
        }
    }

    /// What `KeyA` … `KeyZ` type on each layout, without modifiers.
    const RUSSIAN: [&str; 26] = [
        "ф", "и", "с", "в", "у", "а", "п", "р", "ш", "о", "л", "д", "ь", "т", "щ", "з", "й", "к",
        "ы", "е", "г", "м", "ц", "ч", "н", "я",
    ];
    const ARABIC: [&str; 26] = [
        "ش", "لا", "ؤ", "ي", "ث", "ب", "ل", "ا", "ه", "ت", "ن", "م", "ة", "ى", "خ", "ح", "ض", "ق",
        "س", "ف", "ع", "ر", "ص", "ء", "غ", "ئ",
    ];
    const HEBREW: [&str; 26] = [
        "ש", "נ", "ב", "ג", "ק", "כ", "ע", "י", "ן", "ח", "ל", "ך", "צ", "מ", "ם", "פ", "/", "ר",
        "ד", "א", "ו", "ה", "'", "ס", "ט", "ז",
    ];
    const GREEK: [&str; 26] = [
        "α", "β", "ψ", "δ", "ε", "φ", "γ", "η", "ι", "ξ", "κ", "λ", "μ", "ν", "ο", "π", ";", "ρ",
        "σ", "τ", "θ", "ω", "ς", "χ", "υ", "ζ",
    ];
    const DVORAK: [&str; 26] = [
        "a", "x", "j", "e", ".", "u", "i", "d", "c", "h", "t", "n", "m", "b", "r", "l", "'", "p",
        "o", "y", "g", "k", ",", "q", "f", ";",
    ];
    const AZERTY: [&str; 26] = [
        "q", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", ",", "n", "o", "p", "a", "r",
        "s", "t", "u", "v", "z", "x", "y", "w",
    ];

    const LAYOUTS: [(&str, [&str; 26], bool); 6] = [
        ("Russian", RUSSIAN, false),
        ("Arabic", ARABIC, false),
        ("Hebrew", HEBREW, false),
        ("Greek", GREEK, false),
        ("Dvorak", DVORAK, true),
        ("AZERTY", AZERTY, true),
    ];

    fn code(i: usize) -> String {
        format!("Key{}", char::from(b'A' + i as u8))
    }

    /// The key each letter position resolves to.
    fn resolved(resolver: &mut KeyResolver, layout: &[&str; 26]) -> Vec<Option<egui::Key>> {
        (0..26)
            .map(|i| {
                let key = resolver.key_down(layout[i], &code(i), false).key;
                resolver.key_up(layout[i], &code(i));
                key
            })
            .collect()
    }

    fn assert_resolves_like(
        name: &str,
        resolver: &mut KeyResolver,
        layout: &[&str; 26],
        latin: bool,
    ) {
        for (i, key) in resolved(resolver, layout).into_iter().enumerate() {
            let expected = if latin {
                // Letters as typed; punctuation as typed (Dvorak `KeyQ` is `'`).
                egui::Key::from_name(layout[i])
            } else {
                egui::Key::from_name(&code(i))
            };
            assert_eq!(key, expected, "{name} {} types {:?}", code(i), layout[i]);
        }
    }

    #[test]
    fn layout_map_decides_per_layout() {
        for (name, layout, latin) in LAYOUTS {
            assert_eq!(layout_is_latin(layout), Some(latin), "{name}");

            let mut resolver = KeyResolver::default();
            resolver.set_layout_map(layout);
            assert_resolves_like(name, &mut resolver, &layout, latin);
        }
        assert_eq!(layout_is_latin([]), None);
    }

    #[test]
    fn typed_letters_decide_without_layout_map() {
        // A word typed on the layout, as indices into its table.
        let word = [5, 14, 17, 12, 0]; // KeyF KeyO KeyR KeyM KeyA
        for (name, layout, latin) in LAYOUTS {
            let mut resolver = KeyResolver::default();
            for &i in &word {
                resolver.key_down(layout[i], &code(i), true);
                resolver.key_up(layout[i], &code(i));
            }
            assert_eq!(resolver.is_latin(), latin, "{name}");
            assert_resolves_like(name, &mut resolver, &layout, latin);
        }
    }

    #[test]
    fn hebrew_shortcut_on_punctuation_letter_key() {
        let mut resolver = KeyResolver::default();
        resolver.set_layout_map(HEBREW);
        // Ctrl+Q types `/` on Hebrew.
        let down = resolver.key_down("/", "KeyQ", false);
        assert_eq!(down.key, Some(egui::Key::Q));
        assert_eq!(down.physical_key, Some(egui::Key::Q));
        // Digits and punctuation keys still resolve as typed.
        assert_eq!(
            resolver.key_down(".", "Slash", false).key,
            Some(egui::Key::Period)
        );
    }

    #[test]
    fn switching_layouts_without_layout_map() {
        let mut resolver = KeyResolver::default();
        let type_key = |resolver: &mut KeyResolver, key: &str, code: &str| {
            resolver.key_down(key, code, true);
            resolver.key_up(key, code);
        };

        // Fewer than `NON_LATIN_RUN` letters (Turkish `ı`) don't flip the layout.
        type_key(&mut resolver, "ı", "KeyI");
        assert!(resolver.is_latin());

        for (key, code) in [("п", "KeyG"), ("р", "KeyH"), ("и", "KeyB")] {
            type_key(&mut resolver, key, code);
        }
        assert!(!resolver.is_latin());

        // Spaces and punctuation don't count either way.
        type_key(&mut resolver, " ", "Space");
        type_key(&mut resolver, ".", "Slash");
        assert!(!resolver.is_latin());

        // Shifted or modified presses don't count: Hebrew types Latin capitals with Shift.
        resolver.key_down("E", "KeyE", false);
        assert!(!resolver.is_latin());

        type_key(&mut resolver, "q", "KeyQ");
        assert!(resolver.is_latin());
    }

    #[test]
    fn contradicting_press_asks_for_layout_map() {
        let mut resolver = KeyResolver::default();
        resolver.set_layout_map(DVORAK);
        assert!(!resolver.key_down("o", "KeyS", true).query_layout_map);
        assert!(!resolver.key_down("'", "KeyQ", true).query_layout_map);
        assert!(resolver.key_down("ы", "KeyS", true).query_layout_map);
        // The layout map stays in charge until it is read again.
        assert!(resolver.is_latin());
    }

    #[test]
    fn key_up_releases_the_key_pressed() {
        let mut resolver = KeyResolver::default();
        resolver.set_layout_map(HEBREW);
        assert_eq!(
            resolver.key_down("/", "KeyQ", false).key,
            Some(egui::Key::Q)
        );

        // The user switches to a Latin layout while holding the key.
        resolver.set_layout_map(DVORAK);
        assert_eq!(resolver.key_up("/", "KeyQ").0, Some(egui::Key::Q));
        assert_eq!(resolver.key_up("'", "KeyQ").0, Some(egui::Key::Quote));

        resolver.key_down("'", "KeyQ", false);
        resolver.clear_pressed();
        assert_eq!(resolver.key_up("/", "KeyQ").0, Some(egui::Key::Slash));
    }
}
