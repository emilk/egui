//! The bundled Southeast-Asian line-break dictionaries.
//!
//! Word lists for Thai, Lao, Khmer and Myanmar, taken from the Unicode/ICU
//! line-break dictionaries. Each file keeps its upstream copyright/license
//! header verbatim; the header lines (and the leading BOM) are stripped at
//! load below. Only used with the `sea_dictionary` feature.

use alloc::{boxed::Box, vec::Vec};
use std::sync::OnceLock;

static THAI: &str = include_str!("dictionaries/thai.txt");
static LAO: &str = include_str!("dictionaries/lao.txt");
// khmer (2,031,114 B) and myanmar (1,164,408 B) are over the 1 MB
// `max-include-file-size` set in the repo's clippy.toml, so they carry the
// expectation below. thai (493,054 B) and lao (673,894 B) are UNDER it and must
// NOT carry it: an `#[expect]` on an under-threshold include fires
// `unfulfilled_lint_expectations`, a hard error under the gate's `-D warnings`.
#[expect(clippy::large_include_file)]
static KHMER: &str = include_str!("dictionaries/khmer.txt");
#[expect(clippy::large_include_file)]
static MYANMAR: &str = include_str!("dictionaries/myanmar.txt");

/// The parsed dictionary: all words, sorted and deduplicated, for binary
/// searching, plus the length in bytes of the longest word (so the matcher
/// never scans past it).
struct Dictionary {
    words: Box<[&'static str]>,
    max_word_len: usize,
}

static DICTIONARY: OnceLock<Dictionary> = OnceLock::new();

pub(super) fn words() -> &'static [&'static str] {
    &DICTIONARY.get_or_init(build).words
}

/// Length in bytes of the longest word in the dictionaries.
pub(super) fn max_word_len() -> usize {
    DICTIONARY.get_or_init(build).max_word_len
}

/// Longest dictionary word that is a prefix of `s`, in bytes.
///
/// Tries candidate lengths from `min(max_word_len, s.len())` down, only at
/// `char_boundary` offsets, with one `binary_search` each. Returns `None`
/// when no word matches.
pub(super) fn longest_word(s: &str) -> Option<usize> {
    let words = words();
    let mut end = max_word_len().min(s.len());
    while 0 < end {
        if s.is_char_boundary(end)
            && words
                .binary_search_by(|word| word.as_bytes().cmp(&s.as_bytes()[..end]))
                .is_ok()
        {
            return Some(end);
        }
        end -= 1;
    }
    None
}

/// Built lazily on the first Southeast-Asian layout (one `sort_unstable` over
/// ~179k `&'static str`, tens of milliseconds, once per process).
fn build() -> Dictionary {
    let mut words: Vec<&'static str> = [THAI, LAO, KHMER, MYANMAR]
        .iter()
        .flat_map(|dictionary| dictionary.lines())
        .map(|line| line.trim_start_matches('\u{FEFF}').trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    // The dictionaries are already sorted upstream, so the sort is cheap and
    // only guards against future data changes.
    words.sort_unstable();
    words.dedup();

    let max_word_len = words.iter().map(|word| word.len()).max().unwrap_or(0);
    Dictionary {
        words: words.into_boxed_slice(),
        max_word_len,
    }
}
