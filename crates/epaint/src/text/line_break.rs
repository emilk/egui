//! Where a row of text may end. See [`LineBreaker`].

use super::unicode::{is_cjk, is_cjk_break_allowed};

/// A place where a row of text may end: the next row starts at byte [`Self::offset`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LineBreak {
    /// Byte offset into the paragraph text where the next row starts.
    ///
    /// Must be on a char boundary. `0` and the length of the text mean nothing and are ignored.
    pub offset: usize,

    pub kind: LineBreakKind,
}

impl LineBreak {
    /// The row must end here.
    #[inline]
    pub fn hard(offset: usize) -> Self {
        Self {
            offset,
            kind: LineBreakKind::Hard,
        }
    }

    /// The row may end here.
    #[inline]
    pub fn soft(offset: usize) -> Self {
        Self::soft_with_penalty(offset, 0)
    }

    /// The row may end here, if no soft break with a lower `penalty` fits on the row.
    #[inline]
    pub fn soft_with_penalty(offset: usize, penalty: u8) -> Self {
        Self {
            offset,
            kind: LineBreakKind::Soft { penalty },
        }
    }
}

/// See [`LineBreak`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LineBreakKind {
    /// The row must end here, as after U+2028 LINE SEPARATOR (UAX #14 mandatory break).
    ///
    /// Honoured only right after a character of UAX #14 class BK, CR, LF or NL,
    /// and only when [`crate::text::LayoutJob::break_on_newline`] is set.
    /// Anywhere else it is a soft break with penalty `0`.
    Hard,

    /// The row may end here.
    ///
    /// A row that is too wide ends at the last soft break on it with the lowest `penalty`.
    /// UAX #14 break opportunities all have penalty `0`;
    /// higher penalties are for fallbacks such as breaking after a `-` in a URL.
    Soft { penalty: u8 },
}

/// Finds where rows of text may end: UAX #14 line breaking, dictionary or model-based word
/// breaking for Thai, Khmer and the like, kinsoku rules for Japanese, and so on.
///
/// When a word is wider than the wrap width and there is no break opportunity in it,
/// epaint breaks it between grapheme clusters (UAX #29), so an emoji ZWJ sequence or a letter
/// with its marks stays whole. With [`crate::text::TextWrapping::break_anywhere`], a row ends
/// at its last grapheme cluster boundary that fits, wherever the soft breaks are.
///
/// The default is [`SimpleLineBreaker`]. Install another with
/// [`crate::text::Fonts::set_line_breaker`] (`egui::Context::set_line_breaker` in egui).
///
/// ## Example
/// A line breaker on top of `craft_text::segment`, whose `Break::offset` is also the byte
/// offset where the next line starts:
///
/// ```ignore
/// use epaint::text::{LineBreak, LineBreaker};
///
/// struct CraftLineBreaker;
///
/// impl LineBreaker for CraftLineBreaker {
///     fn line_breaks(&self, text: &str, language: Option<&str>, out: &mut Vec<LineBreak>) {
///         let options = craft_text::segment::Options::for_language(language);
///         out.extend(
///             craft_text::segment::line_breaks(text, &options)
///                 .into_iter()
///                 .map(|b| {
///                     if b.mandatory {
///                         LineBreak::hard(b.offset)
///                     } else {
///                         LineBreak::soft(b.offset)
///                     }
///                 }),
///         );
///     }
/// }
///
/// ctx.set_line_breaker(std::sync::Arc::new(CraftLineBreaker));
/// ```
pub trait LineBreaker: Send + Sync {
    /// Append to `out` the places where a row of `text` may end.
    ///
    /// `text` is one paragraph: when [`crate::text::LayoutJob::break_on_newline`] is set
    /// (the default) it has no `\n`. `language` is [`crate::text::LayoutJob::language`],
    /// a BCP 47 tag such as `"th"` or `"ja-JP"`, if the job has one.
    ///
    /// The breaks may come in any order. A break that isn't on a char boundary is ignored.
    fn line_breaks(&self, text: &str, language: Option<&str>, out: &mut Vec<LineBreak>);
}

/// The default [`LineBreaker`]: a few rules that work for most European languages and
/// for Chinese and Japanese, with no language data.
///
/// * After whitespace other than a no-break space.
/// * Between two CJK ideographs or kana, unless the next char may not start a row
///   (closing brackets and punctuation, small kana, `ー` and the like).
/// * Before a CJK ideograph or kana.
/// * With penalty `1`, after a `-`.
/// * With penalty `2`, after other ASCII punctuation, e.g. in URLs.
///
/// It does not know where Thai, Lao, Khmer or Burmese words end, so text in those scripts
/// wraps between grapheme clusters.
#[derive(Clone, Copy, Debug, Default)]
pub struct SimpleLineBreaker;

impl LineBreaker for SimpleLineBreaker {
    fn line_breaks(&self, text: &str, _language: Option<&str>, out: &mut Vec<LineBreak>) {
        const NON_BREAKING_SPACE: char = '\u{A0}';
        let mut chars = text.char_indices().peekable();
        while let Some((_, chr)) = chars.next() {
            let Some(&(next_offset, next)) = chars.peek() else {
                break; // The end of the text is no break opportunity.
            };
            let penalty = if (chr.is_whitespace() && chr != NON_BREAKING_SPACE)
                || (is_cjk(chr) && is_cjk_break_allowed(next))
            {
                0
            } else if chr == '-' {
                1
            } else if chr.is_ascii_punctuation() {
                2
            } else if is_cjk(next) {
                0
            } else {
                continue;
            };
            out.push(LineBreak::soft_with_penalty(next_offset, penalty));
        }
    }
}

/// A character after which UAX #14 requires a line break (classes BK, CR, LF and NL).
///
/// `\r` is one only when it isn't followed by `\n`; that is up to the [`LineBreaker`].
pub(crate) fn is_mandatory_break(chr: char) -> bool {
    matches!(
        chr,
        '\n' | '\r' | '\u{0B}' | '\u{0C}' | '\u{85}' | '\u{2028}' | '\u{2029}'
    )
}
