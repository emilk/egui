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
}
