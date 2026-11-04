//! Paragraph-level bidirectional run segmentation (UAX #9).
//!
//! Mixed-direction text (e.g. a filename like `تقرير ٢٠٢٦.pdf` in an RTL label) must be
//! laid out in *visual* order: runs of characters that share one resolved embedding
//! level are shaped in their own direction and then placed left-to-right in the order
//! given by UAX #9 rule L2. Shaping the runs in logical order (the historic behaviour)
//! renders the line backwards.
//!
//! The resolution itself (X/W/N/I rules, including bracket pairing) is delegated to the
//! `unicode-bidi` crate; this module turns its resolved per-byte levels into maximal
//! runs in visual order for the shaper.

use alloc::vec::Vec;
use core::ops::Range;

use unicode_bidi::{Level, ParagraphBidiInfo};

/// One maximal directional run of a paragraph segment.
///
/// A byte range whose characters share one resolved embedding level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BidiRun {
    /// The resolved embedding level of the run (odd = right-to-left).
    pub level: Level,

    /// Byte range within the segment text.
    pub byte_range: Range<usize>,
}

impl BidiRun {
    /// The direction the shaper should set for this run: characters at an odd
    /// embedding level are laid out right-to-left.
    #[inline]
    pub fn is_rtl(&self) -> bool {
        self.level.is_rtl()
    }
}

/// Segment one paragraph segment (text without newlines) into maximal directional
/// runs, in *visual* (left-to-right) order, seeded by the paragraph's base level.
///
/// The runs partition the whole segment and are appended to `out` (which is cleared
/// first) so the caller can reuse the allocation across segments.
///
/// The base level comes from the job's resolved direction ([`super::TextDirection`]),
/// not from re-inferring it here: UAX #9 P2/P3 first-strong inference already happened
/// (or the direction was pinned explicitly) at the galley-cache boundary.
pub(crate) fn segment_into_bidi_runs(segment: &str, base_level: Level, out: &mut Vec<BidiRun>) {
    out.clear();

    if segment.is_empty() {
        return;
    }

    // Fast path: ASCII text at an LTR base level can only ever resolve to a single
    // level-0 run. ASCII contains no R, AL or AN characters, so no odd level and no
    // reordering can arise (numbers stay at even levels, which shape identically).
    // This keeps the overwhelmingly common pure-LTR case allocation- and scan-free
    // and byte-identical to the pre-bidi behaviour.
    if base_level == Level::ltr() && segment.is_ascii() {
        out.push(BidiRun {
            level: base_level,
            byte_range: 0..segment.len(),
        });
        return;
    }

    let info = ParagraphBidiInfo::new(segment, Some(base_level));
    let (_, runs) = info.visual_runs(0..segment.len());

    out.extend(runs.into_iter().map(|byte_range| {
        // Each run is maximal in the (L1-adjusted) levels, so the level at its first
        // byte is the level of the whole run. The range is always within bounds.
        let level = info
            .levels
            .get(byte_range.start)
            .copied()
            .unwrap_or(base_level);
        BidiRun { level, byte_range }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runs_of(segment: &str, base_level: Level) -> Vec<(u8, &str)> {
        let mut out = Vec::new();
        segment_into_bidi_runs(segment, base_level, &mut out);
        out.iter()
            .map(|run| {
                (
                    run.level.number(),
                    segment.get(run.byte_range.clone()).unwrap_or_default(),
                )
            })
            .collect()
    }

    #[test]
    fn pure_ltr_is_one_run_in_logical_order() {
        assert_eq!(
            runs_of("Hello World 123", Level::ltr()),
            vec![(0, "Hello World 123")]
        );
    }

    #[test]
    fn rtl_base_reverses_run_order() {
        // The rename-ar oracle shape: Arabic word, Arabic-Indic digits, dot, latin
        // extension. Base RTL: Arabic + separating dot at level 1, the digits (AN,
        // after AL per W2) and the latin tail at level 2. Visual order puts the latin
        // extension at the far left.
        let arabic = "تقرير";
        let digits = "٢٠٢٦";
        let segment = format!("{arabic} {digits}.pdf");
        assert_eq!(
            runs_of(&segment, Level::rtl()),
            vec![
                (2, "pdf"),
                (1, "."),
                (2, digits),
                (1, &format!("{arabic} "))
            ]
        );
    }

    #[test]
    fn ltr_base_reverses_only_embedded_rtl_runs() {
        // Embedded Hebrew run at level 1 between two LTR runs; visual order is
        // unchanged (single level-1 sequence), the run is just isolated.
        assert_eq!(
            runs_of("Hello שלום world", Level::ltr()),
            vec![(0, "Hello "), (1, "שלום"), (0, " world")]
        );
    }

    #[test]
    fn digits_keep_their_uax9_level_in_rtl_context() {
        // European numbers in an RTL base sit at level 2 (LTR) per rule I2:
        assert_eq!(
            runs_of("عربي 123", Level::rtl()),
            vec![(2, "123"), (1, "عربي ")]
        );
    }

    #[test]
    fn empty_segment_yields_no_runs() {
        let mut out = Vec::new();
        segment_into_bidi_runs("", Level::rtl(), &mut out);
        assert!(out.is_empty());
    }
}
