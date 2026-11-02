//! Extended grapheme cluster boundaries (UAX #29) for cursor movement and editing.
//!
//! All indices are char indices, like [`super::cursor::CCursor::index`].

use core::ops::Range;

use unicode_segmentation::UnicodeSegmentation as _;

use super::CharIndex;

/// The char ranges of the grapheme clusters of `text`, in order.
fn clusters(text: &str) -> impl Iterator<Item = Range<usize>> + '_ {
    let mut start = 0;
    text.graphemes(true).map(move |cluster| {
        let end = start + cluster.chars().count();
        let range = start..end;
        start = end;
        range
    })
}

/// The closest cluster boundary before `index`, or zero.
pub fn previous_boundary(text: &str, index: CharIndex) -> CharIndex {
    let mut boundary = 0;
    for cluster in clusters(text) {
        if index.0 <= cluster.end {
            break;
        }
        boundary = cluster.end;
    }
    CharIndex(boundary)
}

/// The closest cluster boundary after `index`, or the end of `text`.
pub fn next_boundary(text: &str, index: CharIndex) -> CharIndex {
    let mut boundary = 0;
    for cluster in clusters(text) {
        boundary = cluster.end;
        if index.0 < boundary {
            break;
        }
    }
    CharIndex(boundary)
}

/// The cluster that `index` is inside of, if it is not on a boundary.
///
/// `None` on a boundary, including the start and end of `text` and anything past its end.
pub fn cluster_around(text: &str, index: CharIndex) -> Option<Range<CharIndex>> {
    clusters(text)
        .find(|cluster| index.0 < cluster.end)
        .filter(|cluster| cluster.start < index.0)
        .map(|cluster| CharIndex(cluster.start)..CharIndex(cluster.end))
}

/// `index`, moved back to the start of the cluster it is inside of, if any.
pub fn floor_boundary(text: &str, index: CharIndex) -> CharIndex {
    cluster_around(text, index).map_or(index, |cluster| cluster.start)
}

/// `index`, moved on to the end of the cluster it is inside of, if any.
pub fn ceil_boundary(text: &str, index: CharIndex) -> CharIndex {
    cluster_around(text, index).map_or(index, |cluster| cluster.end)
}

/// Where the text removed by Backspace at `index` starts.
///
/// Backspace removes the last code point of the cluster before the cursor, so a
/// mistyped accent, vowel sign or tone mark can be corrected without retyping its
/// base letter. This holds for Devanagari conjuncts and vowel signs, Thai vowel and
/// tone marks, Arabic harakat and Hangul syllables spelled with conjoining jamo,
/// as in macOS (`kCFStringBackwardDeletionCluster` does not combine U+0530..=U+194F),
/// Chromium and Android. A precomposed Hangul syllable is one code point.
///
/// The whole cluster goes at once when part of it is not something the user typed:
/// * emoji sequences: ZWJ sequences, flags, keycaps, skin tone modifiers,
///   tag sequences and emoji presentation selectors;
/// * `"\r\n"`.
///
/// A variation selector is removed together with the char it selects a glyph for.
pub fn backspace_start(text: &str, index: CharIndex) -> CharIndex {
    if index == CharIndex::ZERO {
        return index;
    }
    let start = previous_boundary(text, index);
    let cluster: String = text.chars().skip(start.0).take(index.0 - start.0).collect();

    if removes_whole_cluster(&cluster) {
        return start;
    }

    let mut chars = cluster.chars().rev();
    match (chars.next(), chars.next()) {
        (Some(last), Some(_)) if is_variation_selector(last) => index - 2,
        _ => index - 1,
    }
}

fn removes_whole_cluster(cluster: &str) -> bool {
    cluster == "\r\n" || is_emoji_sequence(cluster)
}

fn is_emoji_sequence(cluster: &str) -> bool {
    use unicode_properties::emoji::{EmojiStatus, UnicodeEmoji as _};

    let is_emoji_part = |c: char| {
        matches!(
            c,
            '\u{FE0F}' // emoji presentation selector
                | '\u{20E3}' // combining enclosing keycap
                | '\u{1F1E6}'..='\u{1F1FF}' // regional indicators (flags)
                | '\u{1F3FB}'..='\u{1F3FF}' // emoji modifiers (skin tones)
                | '\u{E0020}'..='\u{E007F}' // tags (subdivision flags)
        )
    };
    // Outside emoji, a zero width joiner is a mark that asks for a different glyph
    // (e.g. a Devanagari half form) and goes like one.
    let is_zwj_emoji = || {
        cluster.contains('\u{200D}')
            && cluster
                .chars()
                .next()
                .is_some_and(|c| !c.is_ascii() && c.emoji_status() != EmojiStatus::NonEmoji)
    };
    cluster.chars().any(is_emoji_part) || is_zwj_emoji()
}

fn is_variation_selector(c: char) -> bool {
    matches!(c, '\u{FE00}'..='\u{FE0F}' | '\u{E0100}'..='\u{E01EF}')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boundaries(text: &str) -> Vec<usize> {
        let mut out = vec![0];
        out.extend(clusters(text).map(|c| c.end));
        out
    }

    /// `(name, text, cluster boundaries, text left by Backspace at the end,
    /// text left by Delete before the last cluster)`.
    const CASES: &[(&str, &str, &[usize], &str, &str)] = &[
        ("combining acute", "ae\u{301}", &[0, 1, 3], "ae", "a"),
        (
            "family ZWJ emoji",
            "a👨\u{200D}👩\u{200D}👧",
            &[0, 1, 6],
            "a",
            "a",
        ),
        ("flags", "🇸🇪🇫🇷", &[0, 2, 4], "🇸🇪", "🇸🇪"),
        ("keycap", "a1\u{FE0F}\u{20E3}", &[0, 1, 4], "a", "a"),
        ("skin tone", "a👍🏽", &[0, 1, 3], "a", "a"),
        ("Devanagari conjunct", "क्ष", &[0, 3], "क्", ""),
        ("Devanagari vowel sign", "कि", &[0, 2], "क", ""),
        ("Thai with vowel and tone marks", "กี่", &[0, 3], "กี", ""),
        (
            "Hangul jamo",
            "\u{1100}\u{1161}\u{11A8}\u{1100}\u{1161}",
            &[0, 3, 5],
            "\u{1100}\u{1161}\u{11A8}\u{1100}",
            "\u{1100}\u{1161}\u{11A8}",
        ),
        (
            "Arabic with shadda",
            "\u{628}\u{627}\u{651}",
            &[0, 1, 3],
            "\u{628}\u{627}",
            "\u{628}",
        ),
        ("CRLF", "a\r\n", &[0, 1, 3], "a", "a"),
        ("variation selector", "a葛\u{E0100}", &[0, 1, 3], "a", "a"),
        ("text presentation", "a❤\u{FE0E}", &[0, 1, 3], "a", "a"),
        ("Devanagari half form", "क्\u{200D}", &[0, 3], "क्", ""),
    ];

    fn without(text: &str, range: Range<usize>) -> String {
        text.chars()
            .enumerate()
            .filter(|(i, _)| !range.contains(i))
            .map(|(_, c)| c)
            .collect()
    }

    #[test]
    fn movement_and_deletion() {
        for &(name, text, expected_boundaries, after_backspace, after_delete) in CASES {
            let end = text.chars().count();
            assert_eq!(boundaries(text), expected_boundaries, "{name}");

            let mut right = vec![0];
            let mut index = CharIndex(0);
            while index.0 < end {
                index = next_boundary(text, index);
                right.push(index.0);
            }
            assert_eq!(right, expected_boundaries, "{name}: Right");

            let mut left = vec![end];
            let mut index = CharIndex(end);
            while index.0 > 0 {
                index = previous_boundary(text, index);
                left.push(index.0);
            }
            left.reverse();
            assert_eq!(left, expected_boundaries, "{name}: Left");

            for i in 0..=end {
                let index = CharIndex(i);
                let on_boundary = expected_boundaries.contains(&i);
                assert_eq!(
                    cluster_around(text, index).is_none(),
                    on_boundary,
                    "{name} at {i}"
                );
                assert!(
                    expected_boundaries.contains(&floor_boundary(text, index).0)
                        && expected_boundaries.contains(&ceil_boundary(text, index).0),
                    "{name} at {i}"
                );
            }

            let start = backspace_start(text, CharIndex(end));
            assert_eq!(
                without(text, start.0..end),
                after_backspace,
                "{name}: Backspace"
            );

            let last = previous_boundary(text, CharIndex(end));
            let next = next_boundary(text, last);
            assert_eq!(
                without(text, last.0..next.0),
                after_delete,
                "{name}: Delete"
            );
        }
    }

    #[test]
    fn out_of_range() {
        assert_eq!(next_boundary("ab", CharIndex(5)), CharIndex(2));
        assert_eq!(previous_boundary("ab", CharIndex(5)), CharIndex(2));
        assert_eq!(previous_boundary("", CharIndex(0)), CharIndex(0));
        assert_eq!(next_boundary("", CharIndex(0)), CharIndex(0));
        assert_eq!(backspace_start("", CharIndex(0)), CharIndex(0));
        assert_eq!(cluster_around("ab", CharIndex(5)), None);
    }
}
