//! Pure helpers for the web text agent's IME anchoring in right-to-left text.
//!
//! These live outside the wasm-gated [`crate::web`] module so their unit tests
//! actually run in `cargo test -p eframe --lib` on the host (upstream itself
//! flags wasm-gated tests as never-run, see `web/backend.rs`).
//!
//! No web-sys or DOM code here: pure `std` + [`egui`] only, so the module is
//! wasm-safe by construction.

/// The bidi base direction of a piece of text.
///
/// Used to decide which edge of the egui caret rect the hidden text-agent
/// input should be anchored to (the reading edge).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WritingDirection {
    /// Left-to-right base direction.
    Ltr,
    /// Right-to-left base direction.
    Rtl,
}

/// Resolve the bidi base direction of `text` with a simplified UAX #9 P2 scan:
/// the first strong directional character decides.
///
/// The RTL script blocks are checked **before** the
/// [`char::is_alphabetic`] fallback, because `is_alphabetic` is true for
/// Arabic and Hebrew too — the ordering is load-bearing.
///
/// Digits (European numbers, bidi class EN) and neutrals are weak under UAX #9
/// and are skipped. If no strong character is found, this returns `None`
/// instead of baking in UAX #9 P3's unconditional LTR default: the caller maps
/// `None` to the pre-existing center-point fallback, and the browser applies
/// P3 itself when resolving `dir="auto"` on the hidden input.
///
/// Note: rare RTL scripts outside the blocks listed below (e.g. Adlam,
/// U+1E900–U+1E95F, or Old Hungarian) degrade to LTR here.
pub(crate) fn bidi_base_direction(text: &str) -> Option<WritingDirection> {
    for ch in text.chars() {
        if is_rtl_char(ch) {
            return Some(WritingDirection::Rtl);
        }
        // Coarse complement of the RTL table: Latin, Greek, Cyrillic,
        // Han/Hangul/Kana etc. are all `is_alphabetic`, and all have bidi
        // class L (or resolve as strong L for this heuristic's purposes).
        if ch.is_alphabetic() {
            return Some(WritingDirection::Ltr);
        }
        // Everything else (digits, punctuation, whitespace, symbols) is weak
        // or neutral under UAX #9 — keep scanning.
    }
    None
}

/// Is `ch` a character from one of the common RTL script blocks?
///
/// Kept as a small table of blocks; operates on a `char`, never on bytes.
fn is_rtl_char(ch: char) -> bool {
    // (start, end) code points of each RTL script block.
    const RTL_RANGES: [(u32, u32); 12] = [
        (0x0590, 0x05FF), // Hebrew
        (0x0600, 0x06FF), // Arabic
        (0x0700, 0x074F), // Syriac
        (0x0750, 0x077F), // Arabic Supplement
        (0x0780, 0x07BF), // Thaana
        (0x07C0, 0x07FF), // NKo
        (0x0800, 0x083F), // Samaritan
        (0x0840, 0x085F), // Mandaic
        (0x08A0, 0x08FF), // Arabic Extended-A
        (0xFB50, 0xFDFF), // Arabic Presentation Forms-A
        (0xFB1D, 0xFB4F), // Hebrew presentation forms
        (0xFE70, 0xFEFF), // Arabic Presentation Forms-B
    ];

    let code = ch as u32;
    RTL_RANGES.iter().any(|&(lo, hi)| (lo..=hi).contains(&code))
}

/// The x position (in egui points) at which to anchor the hidden text-agent
/// input for the given caret rect and base direction.
///
/// * [`Some`]`(`[`WritingDirection::Rtl`]`)`: the caret's **right** edge —
///   the reading edge in RTL text (the candidate window extends leftward).
/// * [`Some`]`(`[`WritingDirection::Ltr`]`)`: the caret's **left** edge —
///   the reading edge in LTR text.
/// * `None`: the caret's center, which is the pre-existing fallback for
///   values with no strong character (identical to the old behavior).
///
/// All of these are pure math on [`egui::Rect`], so a zero- or
/// negative-width rect (hostile document data) can never panic here.
pub(crate) fn anchor_x(cursor_rect: egui::Rect, direction: Option<WritingDirection>) -> f32 {
    match direction {
        Some(WritingDirection::Rtl) => cursor_rect.right(),
        Some(WritingDirection::Ltr) => cursor_rect.left(),
        None => cursor_rect.center().x,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_is_rtl() {
        assert_eq!(bidi_base_direction("أهلا"), Some(WritingDirection::Rtl));
    }

    #[test]
    fn hebrew_is_rtl() {
        assert_eq!(bidi_base_direction("שלום"), Some(WritingDirection::Rtl));
    }

    #[test]
    fn latin_is_ltr() {
        assert_eq!(bidi_base_direction("hello"), Some(WritingDirection::Ltr));
    }

    #[test]
    fn cjk_is_ltr() {
        // Han/Kana/Hangul have bidi class L and are `is_alphabetic`.
        assert_eq!(
            bidi_base_direction("こんにちは"),
            Some(WritingDirection::Ltr)
        );
        assert_eq!(bidi_base_direction("한국어"), Some(WritingDirection::Ltr));
        assert_eq!(bidi_base_direction("中文"), Some(WritingDirection::Ltr));
    }

    #[test]
    fn digits_and_neutrals_are_weak() {
        assert_eq!(bidi_base_direction(""), None);
        assert_eq!(bidi_base_direction("123"), None);
        assert_eq!(bidi_base_direction(" .,!?- "), None);
        assert_eq!(bidi_base_direction("42 😄 🎉"), None);
    }

    #[test]
    fn digits_before_arabic_resolve_rtl() {
        // Digits are weak (EN) under UAX #9: the first *strong* character is
        // the Arabic letter, so the whole value resolves RTL.
        assert_eq!(bidi_base_direction("123أهلا"), Some(WritingDirection::Rtl));
    }

    #[test]
    fn mixed_latin_then_arabic_resolves_ltr() {
        // The whole input value is scanned: the first strong character is the
        // Latin `h`, so the value resolves LTR.
        assert_eq!(
            bidi_base_direction("hello مرحبا"),
            Some(WritingDirection::Ltr)
        );
    }

    #[test]
    fn multibyte_characters_are_safe() {
        // Four-byte chars before the first strong char: must scan on `chars`,
        // never bytes.
        assert_eq!(bidi_base_direction("😀שלום"), Some(WritingDirection::Rtl));
        assert_eq!(bidi_base_direction("😀أهلا"), Some(WritingDirection::Rtl));
        assert_eq!(bidi_base_direction("😀hello"), Some(WritingDirection::Ltr));
    }

    #[test]
    fn anchor_x_rtl_uses_right_edge() {
        let rect = egui::Rect::from_min_max(egui::pos2(10.0, 0.0), egui::pos2(11.0, 2.0));
        assert_eq!(anchor_x(rect, Some(WritingDirection::Rtl)), 11.0);
    }

    #[test]
    fn anchor_x_ltr_uses_left_edge() {
        let rect = egui::Rect::from_min_max(egui::pos2(10.0, 0.0), egui::pos2(11.0, 2.0));
        assert_eq!(anchor_x(rect, Some(WritingDirection::Ltr)), 10.0);
    }

    #[test]
    fn anchor_x_none_uses_center() {
        let rect = egui::Rect::from_min_max(egui::pos2(10.0, 0.0), egui::pos2(11.0, 2.0));
        assert_eq!(anchor_x(rect, None), 10.5);
    }

    #[test]
    fn anchor_x_zero_width_rect_does_not_panic() {
        let rect = egui::Rect::from_min_max(egui::pos2(7.5, 0.0), egui::pos2(7.5, 2.0));
        let width = 100.0;
        for direction in [
            None,
            Some(WritingDirection::Ltr),
            Some(WritingDirection::Rtl),
        ] {
            let x = anchor_x(rect, direction);
            assert_eq!(x, 7.5);
            // After the caller's canvas clamp the point must stay in bounds:
            let clamped = (x * 1.0).clamp(0.0, width);
            assert!((0.0..=width).contains(&clamped));
        }
    }
}
