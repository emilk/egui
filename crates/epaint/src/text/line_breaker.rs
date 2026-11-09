//! Word-boundary lookup for line-breaking scripts that are written without spaces.
//!
//! Thai, Lao, Khmer and Myanmar are written as unbroken runs of syllables:
//! Unicode line breaking ([UAX #14] class SA) needs dictionary analysis to find
//! where one word ends and the next begins. There are two ways to get word
//! boundaries:
//!
//! * the bundled dictionaries behind the `sea_dictionary` feature, or
//! * an application-installed [`LineBreaker`] via
//!   [`crate::text::TextWrapping::line_breaker`].
//!
//! [UAX #14]: https://www.unicode.org/reports/tr14/#SA

use alloc::{sync::Arc, vec::Vec};
use core::ops::Range;

/// Finds word boundaries in text, for line-breaking scripts that are written
/// without spaces.
///
/// Thai, Lao, Khmer and Myanmar are written as unbroken runs of syllables;
/// Unicode line breaking ([UAX #14] class SA) requires dictionary analysis to
/// find where one word ends and the next begins. epaint consults a
/// [`LineBreaker`] once per [`crate::text::LayoutJob`] with the full
/// [`crate::text::LayoutJob::text`].
///
/// The default breaker is `SeaLineBreaker` (behind the `sea_dictionary`
/// feature); applications with a better segmenter (ICU, `HarfBuzz`, OS text
/// services) install their own implementation via
/// [`crate::text::TextWrapping::line_breaker`].
///
/// # Example
///
/// ```
/// use epaint::text::LineBreaker;
///
/// struct SpaceBreaker;
/// impl LineBreaker for SpaceBreaker {
///     fn word_boundaries(&self, text: &str) -> Vec<core::ops::Range<usize>> {
///         let mut ranges = Vec::new();
///         let mut start = 0;
///         for (i, ch) in text.char_indices() {
///             if ch == ' ' {
///                 if start < i {
///                     ranges.push(start..i);
///                 }
///                 start = i + 1;
///             }
///         }
///         if start < text.len() {
///             ranges.push(start..text.len());
///         }
///         ranges
///     }
/// }
///
/// assert_eq!(SpaceBreaker.word_boundaries("hi there"), vec![0..2, 3..8]);
/// ```
///
/// [UAX #14]: https://www.unicode.org/reports/tr14/#SA
pub trait LineBreaker: Send + Sync + 'static {
    /// Byte ranges of words in `text`, ascending and non-overlapping.
    ///
    /// Ranges must start and end on `char` boundaries; ranges that are empty,
    /// inverted, out of bounds or out of order are ignored safely (the layout
    /// never slices `text` by these offsets). Ranges outside the scripts the
    /// breaker handles are simply not reported.
    fn word_boundaries(&self, text: &str) -> Vec<Range<usize>>;
}

/// A breaker that reports no word boundaries.
///
/// This is the built-in breaker when the `sea_dictionary` feature is disabled:
/// text in scripts that are written without spaces then wraps exactly as it
/// did before word-boundary support was added.
#[derive(Default)]
pub struct NoopLineBreaker;

impl LineBreaker for NoopLineBreaker {
    fn word_boundaries(&self, _text: &str) -> Vec<Range<usize>> {
        Vec::new()
    }
}

/// The breaker used when a job doesn't name one.
pub(crate) fn default_breaker() -> Arc<dyn LineBreaker> {
    use std::sync::OnceLock;
    static BREAKER: OnceLock<Arc<dyn LineBreaker>> = OnceLock::new();
    Arc::clone(BREAKER.get_or_init(built_in_breaker))
}

/// The feature switch is `#[cfg]`, NOT a `cfg!()` switch: `cfg!` expands to a
/// bool but does not strip code, so a `cfg!`-switched `SeaLineBreaker::default()`
/// arm is still type-checked on the feature-off build, where `SeaLineBreaker`
/// does not exist (E0425). Gating whole helper fns keeps each arm a plain tail
/// expression. (A `cfg!` switch is fine in `may_have_sea_text` only because its
/// arms touch just `text`/bools, which compile in both configurations; here the
/// feature-on arm names `SeaLineBreaker`, which does not exist without the
/// feature.)
#[cfg(not(feature = "sea_dictionary"))]
fn built_in_breaker() -> Arc<dyn LineBreaker> {
    Arc::new(NoopLineBreaker)
}

#[cfg(feature = "sea_dictionary")]
fn built_in_breaker() -> Arc<dyn LineBreaker> {
    Arc::new(SeaLineBreaker)
}

// The predicates below are `sea_dictionary`-gated because their only callers
// are the gated breaker and the gated tests; ungated they would be dead code
// and fail the gate's `-D warnings`.

/// Characters from the script blocks that need dictionary analysis to find
/// word boundaries: Thai, Lao, Khmer, Myanmar (UAX #14 class SA).
#[cfg(feature = "sea_dictionary")]
pub(crate) fn is_sea_script(c: char) -> bool {
    matches!(
        c as u32,
        0x0E00..=0x0E7F // Thai
            | 0x0E80..=0x0EFF // Lao
            | 0x1780..=0x17FF // Khmer
            | 0x1000..=0x109F // Myanmar
            | 0xAA60..=0xAA7F // Myanmar Extended-A
            | 0xA9E0..=0xA9FF // Myanmar Extended-B
            | 0x19E0..=0x19FF // Khmer Symbols
    )
}

/// Characters that join their base character's run: combining marks
/// (categories Mn/Mc/Me), zero-width joiners, and Thai/Lao SARA AM (category
/// Lo, so not covered by the mark test, but glued to its base all the same).
///
/// The standalone spacing vowel signs (Thai U+0E30/U+0E32, Lao U+0EB0/U+0EB2,
/// also Lo) deliberately stay OUT: they are word-start-worthy letters, not
/// joiners.
#[cfg(feature = "sea_dictionary")]
pub(crate) fn is_sea_extend(c: char) -> bool {
    matches!(c, '\u{200C}' | '\u{200D}') // ZWNJ, ZWJ
        || matches!(c, '\u{0E33}' | '\u{0EB3}') // Thai/Lao SARA AM
        || super::unicode::is_combining_mark(c)
}

/// Closing punctuation of the Southeast-Asian scripts: kinsoku, so it must
/// never start a row. It joins the preceding word instead.
#[cfg(feature = "sea_dictionary")]
pub(crate) fn is_sea_closing_punctuation(c: char) -> bool {
    matches!(
        c as u32,
        0x0E2F | 0x0E46 | 0x0E4F | 0x0E5A | 0x0E5B // Thai
            | 0x0EAF | 0x0EC6 // Lao
            | 0x17D4..=0x17D7 | 0x17D9 | 0x17DA // Khmer
            | 0x104A..=0x104C | 0x104F // Myanmar
    )
}

/// Splits `text` into runs of (byte range, is-SEA-script), extending
/// `char_indices`-style. Never splits a char.
///
/// A mark/joiner char (see [`is_sea_extend`]) inherits the kind of the run it
/// follows — so coeng U+17D2 (both a combining mark and in the Khmer block)
/// stays with its base — and a leading mark/joiner (malformed input) starts
/// its own non-SEA run.
#[cfg(feature = "sea_dictionary")]
pub(crate) fn sea_script_runs(text: &str) -> Vec<(Range<usize>, bool)> {
    let mut runs = Vec::new();
    let mut current: Option<bool> = None;
    let mut start = 0;
    for (i, c) in text.char_indices() {
        let is_sea = if is_sea_extend(c) {
            current.unwrap_or(false)
        } else {
            is_sea_script(c)
        };
        if current != Some(is_sea) {
            if let Some(previous) = current {
                runs.push((start..i, previous));
            }
            start = i;
            current = Some(is_sea);
        }
    }
    if let Some(previous) = current {
        runs.push((start..text.len(), previous));
    }
    runs
}

/// Maximum-matching word breaker over the bundled dictionaries.
///
/// Available with the `sea_dictionary` feature. Splits each Southeast-Asian
/// run by the longest dictionary word at every position; a position that
/// starts no known word advances one grapheme cluster (base + combining
/// marks), so marks and coeng subscripts stay with their base. Closing
/// punctuation joins the preceding word (kinsoku), so it can never start a
/// row.
#[cfg(feature = "sea_dictionary")]
#[derive(Default)]
pub struct SeaLineBreaker;

#[cfg(feature = "sea_dictionary")]
impl LineBreaker for SeaLineBreaker {
    fn word_boundaries(&self, text: &str) -> Vec<Range<usize>> {
        let mut words = Vec::new();
        for (range, is_sea) in sea_script_runs(text) {
            if !is_sea {
                continue;
            }
            // `range` is char-boundary aligned by construction, so slicing is safe.
            let run = &text[range.clone()];
            let run_start = range.start;
            let mut pos = 0;
            while pos < run.len() {
                let word_len =
                    if let Some(len) = super::line_breaker_dict::longest_word(&run[pos..]) {
                        len
                    } else {
                        // No dictionary word starts here: advance one grapheme
                        // unit (first char + following joiners) — a grapheme
                        // cluster by construction, so marks and coeng
                        // subscripts stay with their base.
                        let Some(first) = run[pos..].chars().next() else {
                            break;
                        };
                        let mut end = pos + first.len_utf8();
                        while let Some(next) = run[end..].chars().next()
                            && is_sea_extend(next)
                        {
                            end += next.len_utf8();
                        }
                        end - pos
                    };
                let mut end = pos + word_len;
                // Kinsoku: closing punctuation joins the preceding word, so
                // the boundary lands after the punctuation.
                while let Some(next) = run[end..].chars().next()
                    && is_sea_closing_punctuation(next)
                {
                    end += next.len_utf8();
                }
                words.push(run_start + pos..run_start + end);
                pos = end;
            }
        }
        words
    }
}

#[cfg(all(test, feature = "sea_dictionary"))]
mod tests {
    use alloc::{vec, vec::Vec};
    use unicode_segmentation::UnicodeSegmentation as _;

    use super::*;

    /// Assert the structural contract of [`LineBreaker::word_boundaries`] on
    /// the given text: ascending, non-overlapping, non-empty, char-boundary
    /// aligned, and every boundary a grapheme boundary.
    fn assert_ranges_wellformed(text: &str, ranges: &[Range<usize>]) {
        let mut previous_end = 0;
        for range in ranges {
            assert!(
                previous_end <= range.start && range.start < range.end,
                "ranges must be ascending, non-overlapping and non-empty"
            );
            assert!(
                text.is_char_boundary(range.start) && text.is_char_boundary(range.end),
                "range {range:?} of {text:?} is not char-boundary aligned"
            );
            // `grapheme_indices` yields the start of every cluster but never
            // `text.len()`, which is a valid boundary (end of text).
            let boundaries = |needle: usize| {
                core::iter::once(text.len())
                    .chain(text.grapheme_indices(true).map(|(i, _)| i))
                    .any(|i| i == needle)
            };
            assert!(
                boundaries(range.start) && boundaries(range.end),
                "range {range:?} of {text:?} splits a grapheme cluster"
            );
            previous_end = range.end;
        }
    }

    fn words(text: &str) -> Vec<&str> {
        let ranges = SeaLineBreaker.word_boundaries(text);
        assert_ranges_wellformed(text, &ranges);
        ranges.iter().map(|r| &text[r.start..r.end]).collect()
    }

    #[test]
    fn sea_word_boundaries() {
        assert_eq!(words("สวัสดีครับ"), vec!["สวัสดี", "ครับ"]);
        assert_eq!(words("ພາສາລາວສະບາຍດີ"), vec!["ພາສາ", "ລາວ", "ສະບາຍດີ"]);
        // The `ម្ព` coeng cluster stays whole.
        assert_eq!(words("ភាសាខ្មែរកម្ពុជា"), vec!["ភាសាខ្មែរ", "កម្ពុជា"]);
        assert_eq!(words("မြန်မာနိုင်ငံသည်"), vec!["မြန်မာနိုင်ငံ", "သည်"]);
        // The issue's repro sentence.
        assert_eq!(
            words("ภาษาไทยใช้การเขียนต่อเนื่องโดยไม่คั่นคำด้วยช่องว่าง"),
            vec![
                "ภาษา",
                "ไทย",
                "ใช้",
                "การ",
                "เขียน",
                "ต่อ",
                "เนื่อง",
                "โดย",
                "ไม่",
                "คั่น",
                "คำ",
                "ด้วย",
                "ช่อง",
                "ว่าง",
            ]
        );
    }

    #[test]
    fn closing_punctuation_joins_the_preceding_word() {
        let text = "កម្ពុជាមានប្រជាជនច្រើន។ខ្មែរជាភាសា។";
        let ranges = SeaLineBreaker.word_boundaries(text);
        assert_ranges_wellformed(text, &ranges);
        for range in &ranges {
            assert_ne!(
                &text[range.start..range.end],
                "។",
                "no word range may consist only of closing punctuation"
            );
        }
        let ends: Vec<&str> = ranges
            .iter()
            .filter(|r| text[r.start..r.end].ends_with('។'))
            .map(|r| &text[r.start..r.end])
            .collect();
        assert_eq!(ends, vec!["ច្រើន។", "ភាសា។"]);
    }

    #[test]
    fn latin_and_cjk_text_is_untouched() {
        assert!(SeaLineBreaker.word_boundaries("hello world").is_empty());
        assert!(
            SeaLineBreaker
                .word_boundaries("日本語のテキスト")
                .is_empty()
        );
        assert!(SeaLineBreaker.word_boundaries("").is_empty());
    }
}
