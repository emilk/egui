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
