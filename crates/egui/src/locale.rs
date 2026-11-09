//! Locale → reading-direction resolution.
//!
//! The only locale logic egui needs: which reading direction a locale's
//! primary language subtag implies. Used to resolve
//! [`DirectionPreference::System`](crate::DirectionPreference::System) from
//! [`RawInput::system_locale`](crate::RawInput::system_locale).

use crate::emath::ReadingDirection;

/// The primary language subtags whose CLDR default script is right-to-left
/// (`Arab`, `Hebr` or `Thaa`) — the same set Chromium's `base/i18n/rtl.cc`
/// ships.
///
/// Deliberately excluded: `ku` (CLDR likely-subtags resolve it to Latin;
/// Sorani Kurdish is `ckb`) and other borderline subtags — a wrong guess would
/// degrade to a mirrored interface, while an unmirrored one is today's
/// behavior.
const RTL_LANGUAGES: &[&str] = &[
    "ar", "ckb", "dv", "fa", "he", "iw", "ps", "sd", "ug", "ur", "yi",
];

/// The reading direction implied by the primary language subtag of a locale
/// tag (BCP 47 or a common locale id, e.g. `"ar-EG"`).
///
/// Locale tags are matched case-insensitively; anything unknown, malformed or
/// empty resolves to [`ReadingDirection::Ltr`] (i.e. today's behavior).
pub(crate) fn locale_direction(locale: &str) -> ReadingDirection {
    // The primary language subtag is everything before the first `-` or `_`
    // separator. Comparing against the fixed RTL set is case-insensitive and
    // allocation-free, and every non-matching input (garbage, digits, empty)
    // falls through to Ltr.
    let primary = locale.split(['-', '_']).next().unwrap_or_default();

    if RTL_LANGUAGES
        .iter()
        .any(|rtl| primary.eq_ignore_ascii_case(rtl))
    {
        ReadingDirection::Rtl
    } else {
        ReadingDirection::Ltr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtl_languages() {
        for locale in ["ar", "ar-EG", "he-IL", "iw", "fa", "ur-PK", "ckb", "ps_AF"] {
            assert_eq!(
                locale_direction(locale),
                ReadingDirection::Rtl,
                "{locale} should resolve right-to-left"
            );
        }
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(locale_direction("AR-eg"), ReadingDirection::Rtl);
        assert_eq!(locale_direction("He"), ReadingDirection::Rtl);
    }

    #[test]
    fn ltr_languages() {
        for locale in ["en", "en-US", "zh-Hans", "ku", "de-DE"] {
            assert_eq!(
                locale_direction(locale),
                ReadingDirection::Ltr,
                "{locale} should resolve left-to-right"
            );
        }
    }

    #[test]
    fn unknown_and_garbage_fall_back_to_ltr() {
        for locale in ["", "-", "_", "123", "xx-YY", "ku-Arab", "🙂"] {
            assert_eq!(
                locale_direction(locale),
                ReadingDirection::Ltr,
                "{locale:?} should fall back to left-to-right"
            );
        }
    }
}
