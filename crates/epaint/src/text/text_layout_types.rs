use alloc::sync::Arc;
use core::{ops::Range, str::FromStr as _};

use super::{
    cursor::{CCursor, LayoutCursor},
    glyph_atlas::UvRect,
    index::{ByteIndex, ByteRange, ByteRangeExt as _, CharIndex},
};
use crate::{Color32, FontId, Mesh, Stroke, text::FontsView};
use emath::{Align, GuiRounding as _, NumExt as _, OrderedFloat, Pos2, Rect, Vec2, pos2, vec2};
pub use font_types::Tag;
use smallvec::SmallVec;
use unicode_segmentation::UnicodeSegmentation as _;

/// The base reading direction of a paragraph of text.
///
/// Used by [`LayoutJob`] to anchor and elide text that reads
/// right-to-left (Arabic, Hebrew, …).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum TextDirection {
    /// The text reads left-to-right (the historic default).
    #[default]
    LeftToRight,
    /// The text reads right-to-left.
    RightToLeft,
}

impl TextDirection {
    /// First-strong-character inference (UAX #9 `dir=auto`): the first character with a
    /// strong bidi class decides; `None` if the text has no strong character.
    pub fn infer(text: &str) -> Option<Self> {
        text.chars()
            .find_map(|c| match unicode_bidi::bidi_class(c) {
                unicode_bidi::BidiClass::L => Some(Self::LeftToRight),
                unicode_bidi::BidiClass::R | unicode_bidi::BidiClass::AL => Some(Self::RightToLeft),
                _ => None,
            })
    }
}

/// Describes the task of laying out text.
///
/// This supports mixing different fonts, color and formats (underline etc).
///
/// Pass this to [`crate::FontsView::layout_job`].
///
/// ## Example:
/// ```
/// use epaint::{Color32, text::{LayoutJob, TextFormat}, FontFamily, FontId};
///
/// let mut job = LayoutJob::default();
/// job.append(
///     "Hello ",
///     0.0,
///     TextFormat {
///         font_id: FontId::new(14.0, FontFamily::Proportional),
///         color: Color32::WHITE,
///         ..Default::default()
///     },
/// );
/// job.append(
///     "World!",
///     0.0,
///     TextFormat {
///         font_id: FontId::new(14.0, FontFamily::Monospace),
///         color: Color32::BLACK,
///         ..Default::default()
///     },
/// );
/// ```
///
/// As you can see, constructing a [`LayoutJob`] is currently a lot of work.
/// It would be nice to have a helper macro for it!
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct LayoutJob {
    /// The complete text of this job, referenced by [`LayoutSection`].
    pub text: String,

    /// The different section, which can have different fonts, colors, etc.
    ///
    /// Invariant: the sections are ordered by their `byte_range`,
    /// and together cover the whole of [`Self::text`] with no gaps and no overlaps.
    /// That is: the first section starts at byte 0, the last section ends at `text.len()`,
    /// and each section starts exactly where the previous one ended.
    /// This is checked by [`Self::debug_sanity_check`].
    pub sections: Vec<LayoutSection>,

    /// Controls the text wrapping and elision.
    pub wrap: TextWrapping,

    /// The first row must be at least this high.
    /// This is in case we lay out text that is the continuation
    /// of some earlier text (sharing the same row),
    /// in which case this will be the height of the earlier text.
    /// In other cases, set this to `0.0`.
    pub first_row_min_height: f32,

    /// If `true`, all `\n` characters will result in a new _paragraph_,
    /// starting on a new row.
    ///
    /// If `false`, all `\n` characters will be ignored
    /// and show up as the `.notdef` glyph ("tofu").
    ///
    /// Default: `true`.
    pub break_on_newline: bool,

    /// How to horizontally align the text (`Align::LEFT`, `Align::Center`, `Align::RIGHT`).
    pub halign: Align,

    /// The paragraph's base reading direction.
    ///
    /// If `None`, the direction is inferred from the first strong character
    /// (UAX #9 `dir=auto`), falling back to [`TextDirection::LeftToRight`] —
    /// the historic behaviour for LTR-resolving text.
    /// Pin `Some(TextDirection::LeftToRight)` to force the old anchoring for RTL content.
    ///
    /// Note that layout normalizes the job: a returned galley's job carries the
    /// resolved direction and the reading-start `halign`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub direction: Option<TextDirection>,

    /// Justify text so that word-wrapped rows fill the whole [`TextWrapping::max_width`].
    pub justify: bool,

    /// Round output sizes using [`emath::GuiRounding`], to avoid rounding errors in layout code.
    pub round_output_to_gui: bool,

    /// If `false` (default), trailing whitespace is ignored when computing
    /// horizontal alignment ([`Self::halign`]).
    /// This is desirable for labels so that e.g. "Hello " centers the same as "Hello".
    ///
    /// If `true`, trailing whitespace is included in the row width used for alignment.
    /// This is desirable for text editors where the user expects to see their spaces.
    pub keep_trailing_whitespace: bool,

    /// BCP 47 language tag of the text, e.g. `"en"` or `"ar-EG"`.
    ///
    /// Pure metadata for line-breaking and shaping backends:
    /// it does not change layout on its own.
    /// egui widgets fill it in from the ambient UI language (`Context::language`) when unset.
    #[cfg_attr(feature = "serde", serde(default))]
    pub language: Option<String>,
}

impl Default for LayoutJob {
    #[inline]
    fn default() -> Self {
        Self {
            text: Default::default(),
            sections: Default::default(),
            wrap: Default::default(),
            first_row_min_height: 0.0,
            break_on_newline: true,
            halign: Align::LEFT,
            justify: false,
            round_output_to_gui: true,
            keep_trailing_whitespace: false,
            direction: None,
            language: None,
        }
    }
}

impl LayoutJob {
    /// Clear the text and sections while preserving the layout settings.
    #[inline]
    pub fn clear(&mut self) {
        self.text.clear();
        self.sections.clear();
    }

    /// Break on `\n` and at the given wrap width.
    #[inline]
    pub fn simple(text: String, font_id: FontId, color: Color32, wrap_width: f32) -> Self {
        Self {
            sections: vec![LayoutSection {
                leading_space: 0.0,
                byte_range: ByteRange::full(&text),
                format: TextFormat::simple(font_id, color),
            }],
            text,
            wrap: TextWrapping {
                max_width: wrap_width,
                ..Default::default()
            },
            break_on_newline: true,
            ..Default::default()
        }
    }

    /// Break on `\n`
    #[inline]
    pub fn simple_format(text: String, format: TextFormat) -> Self {
        Self {
            sections: vec![LayoutSection {
                leading_space: 0.0,
                byte_range: ByteRange::full(&text),
                format,
            }],
            text,
            break_on_newline: true,
            ..Default::default()
        }
    }

    /// Does not break on `\n`, but shows the `.notdef` glyph ("tofu") instead.
    #[inline]
    pub fn simple_singleline(text: String, font_id: FontId, color: Color32) -> Self {
        Self {
            sections: vec![LayoutSection {
                leading_space: 0.0,
                byte_range: ByteRange::full(&text),
                format: TextFormat::simple(font_id, color),
            }],
            text,
            wrap: Default::default(),
            break_on_newline: false,
            ..Default::default()
        }
    }

    #[inline]
    pub fn single_section(text: String, format: TextFormat) -> Self {
        Self {
            sections: vec![LayoutSection {
                leading_space: 0.0,
                byte_range: ByteRange::full(&text),
                format,
            }],
            text,
            wrap: Default::default(),
            break_on_newline: true,
            ..Default::default()
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    /// Helper for adding a new section when building a [`LayoutJob`].
    ///
    /// If the appended text has the same [`TextFormat`] as the last section and no
    /// `leading_space`, it is merged into that section instead of adding a new one.
    /// This keeps the section count down and lets text shaping (e.g. kerning) work
    /// across the appended text, since shaping is done per [`LayoutSection`].
    pub fn append(&mut self, text: &str, leading_space: f32, format: TextFormat) {
        let start = self.text.len();
        self.text += text;
        let byte_range = ByteIndex(start)..ByteIndex(self.text.len());

        // Optimization: merge into the previous section if it has the same format
        // and this one adds no leading space.
        if leading_space == 0.0
            && let Some(last) = self.sections.last_mut()
            && last.format == format
        {
            last.byte_range.end = byte_range.end;
            return;
        }

        self.sections.push(LayoutSection {
            leading_space,
            byte_range,
            format,
        });
    }

    /// The [`TextFormat`] of the section containing the character starting at the given byte index.
    ///
    /// If the index is past the end, the format of the last section is returned.
    ///
    /// Panics if the job has no sections.
    /// Assumes [`LayoutJob::sections`] are ordered by increasing `byte_range` (as produced by [`Self::append`]).
    pub fn format_at_byte(&self, byte_idx: ByteIndex) -> &TextFormat {
        self.debug_sanity_check();
        let last = self.sections.last().expect("LayoutJob has no sections");
        let idx = self
            .sections
            .partition_point(|section| section.byte_range.end <= byte_idx);
        let section = self.sections.get(idx).unwrap_or(last);
        &section.format
    }

    /// Check the [`Self::sections`] invariant: the sections are ordered and together
    /// cover the whole of [`Self::text`] with no gaps and no overlaps.
    ///
    /// Only does anything in debug builds.
    #[cfg_attr(not(debug_assertions), expect(clippy::unused_self))]
    pub fn debug_sanity_check(&self) {
        #[cfg(debug_assertions)]
        {
            if self.sections.is_empty() {
                assert!(
                    self.text.is_empty(),
                    "LayoutJob has text but no sections: {:?}",
                    self.text
                );
                return;
            }

            assert_eq!(
                self.sections
                    .first()
                    .expect("checked above")
                    .byte_range
                    .start,
                ByteIndex::ZERO,
                "First LayoutSection must start at byte 0"
            );
            assert_eq!(
                self.sections.last().expect("checked above").byte_range.end,
                ByteIndex(self.text.len()),
                "Last LayoutSection must end at the end of the text"
            );

            for section in &self.sections {
                let Range { start, end } = section.byte_range;
                assert!(start <= end, "LayoutSection has a reversed byte_range");
            }
            for (prev, next) in core::iter::zip(&self.sections, self.sections.iter().skip(1)) {
                assert_eq!(
                    prev.byte_range.end, next.byte_range.start,
                    "LayoutSections must be ordered with no gaps and no overlaps"
                );
            }
        }
    }

    /// The height of the tallest font used in the job.
    ///
    /// Returns a value rounded to [`emath::GUI_ROUNDING`].
    pub fn font_height(&self, fonts: &mut FontsView<'_>) -> f32 {
        let mut max_height = 0.0_f32;
        for section in &self.sections {
            max_height = max_height.max(fonts.row_height(&section.format.font_id));
        }
        max_height
    }

    /// The wrap with, with a small margin in some cases.
    pub fn effective_wrap_width(&self) -> f32 {
        if self.round_output_to_gui {
            // On a previous pass we may have rounded down by at most 0.5 and reported that as a width.
            // egui may then set that width as the max width for subsequent frames, and it is important
            // that we then don't wrap earlier.
            self.wrap.max_width + 0.5
        } else {
            self.wrap.max_width
        }
    }
}

impl core::hash::Hash for LayoutJob {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        let Self {
            text,
            sections,
            wrap,
            first_row_min_height,
            break_on_newline,
            halign,
            justify,
            round_output_to_gui,
            keep_trailing_whitespace,
            direction,
            language,
        } = self;

        text.hash(state);
        sections.hash(state);
        wrap.hash(state);
        emath::OrderedFloat(*first_row_min_height).hash(state);
        break_on_newline.hash(state);
        halign.hash(state);
        justify.hash(state);
        round_output_to_gui.hash(state);
        keep_trailing_whitespace.hash(state);
        direction.hash(state);
        language.hash(state);
    }
}

// ----------------------------------------------------------------------------

/// A contiguous range of [`LayoutJob::text`] that shares the same [`TextFormat`].
///
/// The sections of a [`LayoutJob`] are ordered and together cover the whole text
/// with no gaps and no overlaps. See [`LayoutJob::sections`] for the full invariant.
///
/// Text is shaped on a per-section basis: each section is an independent shaping run.
/// This means kerning (and ligatures) are only correct _within_ a single section,
/// and not across the boundary between two adjacent sections.
/// For this reason [`LayoutJob::append`] merges consecutive sections when possible.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct LayoutSection {
    /// Can be used for first row indentation.
    pub leading_space: f32,

    /// Range into [`LayoutJob::text`].
    pub byte_range: ByteRange,

    /// How to format the text in this section (font, color, etc).
    pub format: TextFormat,
}

impl core::hash::Hash for LayoutSection {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        let Self {
            leading_space,
            byte_range,
            format,
        } = self;
        OrderedFloat(*leading_space).hash(state);
        byte_range.hash(state);
        format.hash(state);
    }
}

// ----------------------------------------------------------------------------

/// Helper trait for all types that can be parsed as a [`font_types::Tag`].
pub trait IntoTag {
    fn into_tag(self) -> font_types::Tag;
}

impl IntoTag for font_types::Tag {
    #[inline(always)]
    fn into_tag(self) -> font_types::Tag {
        self
    }
}

impl IntoTag for u32 {
    #[inline(always)]
    fn into_tag(self) -> font_types::Tag {
        font_types::Tag::from_u32(self)
    }
}

impl IntoTag for [u8; 4] {
    #[inline(always)]
    fn into_tag(self) -> font_types::Tag {
        font_types::Tag::new_checked(&self).expect("Invalid variation axis tag")
    }
}

impl IntoTag for &[u8; 4] {
    #[inline(always)]
    fn into_tag(self) -> font_types::Tag {
        font_types::Tag::new_checked(self).expect("Invalid variation axis tag")
    }
}

impl IntoTag for &str {
    #[inline(always)]
    fn into_tag(self) -> font_types::Tag {
        font_types::Tag::from_str(self).expect("Invalid variation axis tag")
    }
}

/// List of font variation coordinates by axis tag. If more than one coordinate for a given axis is provided, the last
/// one added is used.
#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct VariationCoords(SmallVec<[(font_types::Tag, f32); 2]>);

impl VariationCoords {
    /// Create a list of variation coordinates from a sequence of (tag, value) pairs.
    ///
    /// ## Example:
    /// ```
    /// use epaint::text::VariationCoords;
    ///
    /// let coords = VariationCoords::new([
    ///     (b"wght", 500.0),
    ///     (b"wdth", 75.0),
    /// ]);
    /// ```
    pub fn new<T: IntoTag>(values: impl IntoIterator<Item = (T, f32)>) -> Self {
        Self(values.into_iter().map(|(t, c)| (t.into_tag(), c)).collect())
    }

    /// Add a variation coordinate to the list.
    #[inline(always)]
    pub fn push(&mut self, tag: impl IntoTag, coord: f32) {
        self.0.push((tag.into_tag(), coord));
    }

    /// Remove the coordinate at the given index.
    pub fn remove(&mut self, index: usize) {
        self.0.remove(index);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

impl AsRef<[(font_types::Tag, f32)]> for VariationCoords {
    #[inline(always)]
    fn as_ref(&self) -> &[(font_types::Tag, f32)] {
        &self.0
    }
}

impl AsMut<[(font_types::Tag, f32)]> for VariationCoords {
    fn as_mut(&mut self) -> &mut [(font_types::Tag, f32)] {
        &mut self.0
    }
}

impl core::hash::Hash for VariationCoords {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for (tag, coord) in &self.0 {
            tag.hash(state);
            OrderedFloat(*coord).hash(state);
        }
    }
}

/// Formatting option for a section of text.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TextFormat {
    pub font_id: FontId,

    /// Extra spacing between letters, in points.
    ///
    /// Default: 0.0.
    pub extra_letter_spacing: f32,

    /// Explicit line height of the text in points.
    ///
    /// This is the distance between the bottom row of two subsequent lines of text.
    ///
    /// If `None` (the default), the line height is determined by the font.
    ///
    /// If the line height differs from the height of the font,
    /// the text is vertically centered within the line height (like in CSS),
    /// regardless of [`Self::valign`].
    ///
    /// For even text it is recommended you round this to an even number of _pixels_.
    pub line_height: Option<f32>,

    /// Text color
    pub color: Color32,

    pub background: Color32,

    /// Amount to expand background fill by.
    ///
    /// Default: 1.0
    pub expand_bg: f32,

    pub coords: VariationCoords,

    pub italics: bool,

    pub underline: Stroke,

    pub strikethrough: Stroke,

    /// If you use a small font and [`Align::TOP`] you
    /// can get the effect of raised text.
    ///
    /// If you use a small font and [`Align::BOTTOM`]
    /// you get the effect of a subscript.
    ///
    /// If you use [`Align::Center`], you get text that is centered
    /// around a common center-line, which is nice when mixining emojis
    /// and normal text in e.g. a button.
    ///
    /// The alignment is applied to the difference between the height of the row
    /// and the [`Self::line_height`] of this section.
    /// Any extra space from an explicit [`Self::line_height`] is always split evenly
    /// above and below the text.
    ///
    /// Default: [`Align::BOTTOM`].
    pub valign: Align,
}

impl Default for TextFormat {
    #[inline]
    fn default() -> Self {
        Self {
            font_id: FontId::default(),
            extra_letter_spacing: 0.0,
            line_height: None,
            color: Color32::GRAY,
            background: Color32::TRANSPARENT,
            expand_bg: 1.0,
            coords: VariationCoords::default(),
            italics: false,
            underline: Stroke::NONE,
            strikethrough: Stroke::NONE,
            valign: Align::BOTTOM,
        }
    }
}

impl core::hash::Hash for TextFormat {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        let Self {
            font_id,
            extra_letter_spacing,
            line_height,
            color,
            background,
            expand_bg,
            coords,
            italics,
            underline,
            strikethrough,
            valign,
        } = self;
        font_id.hash(state);
        emath::OrderedFloat(*extra_letter_spacing).hash(state);
        if let Some(line_height) = *line_height {
            emath::OrderedFloat(line_height).hash(state);
        }
        color.hash(state);
        background.hash(state);
        emath::OrderedFloat(*expand_bg).hash(state);
        coords.hash(state);
        italics.hash(state);
        underline.hash(state);
        strikethrough.hash(state);
        valign.hash(state);
    }
}

impl TextFormat {
    #[inline]
    pub fn simple(font_id: FontId, color: Color32) -> Self {
        Self {
            font_id,
            color,
            ..Default::default()
        }
    }
}

// ----------------------------------------------------------------------------

/// How to wrap and elide text.
///
/// This enum is used in high-level APIs where providing a [`TextWrapping`] is too verbose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum TextWrapMode {
    /// The text should expand the `Ui` size when reaching its boundary.
    Extend,

    /// The text should wrap to the next line when reaching the `Ui` boundary.
    Wrap,

    /// The text should be elided using "…" when reaching the `Ui` boundary.
    ///
    /// Note that using [`TextWrapping`] and [`LayoutJob`] offers more control over the elision.
    Truncate,
}

/// Controls the text wrapping and elision of a [`LayoutJob`].
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TextWrapping {
    /// Wrap text so that no row is wider than this.
    ///
    /// If you would rather truncate text that doesn't fit, set [`Self::max_rows`] to `1`.
    ///
    /// Set `max_width` to [`f32::INFINITY`] to turn off wrapping and elision.
    ///
    /// Note that `\n` always produces a new row
    /// if [`LayoutJob::break_on_newline`] is `true`.
    pub max_width: f32,

    /// Maximum amount of rows the text galley should have.
    ///
    /// If this limit is reached, text will be truncated
    /// and [`Self::overflow_character`] appended to the final row.
    /// You can detect this by checking [`Galley::elided`].
    ///
    /// If set to `0`, no text will be outputted.
    ///
    /// If set to `1`, a single row will be outputted,
    /// eliding the text after [`Self::max_width`] is reached.
    /// When you set `max_rows = 1`, it is recommended you also set [`Self::break_anywhere`] to `true`.
    ///
    /// Default value: `usize::MAX`.
    pub max_rows: usize,

    /// If `true`: Allow breaking between any characters.
    /// If `false` (default): prefer breaking between words, etc.
    ///
    /// NOTE: Due to limitations in the current implementation,
    /// when truncating text using [`Self::max_rows`] the text may be truncated
    /// in the middle of a word even if [`Self::break_anywhere`] is `false`.
    /// Therefore it is recommended to set [`Self::break_anywhere`] to `true`
    /// whenever [`Self::max_rows`] is set to `1`.
    pub break_anywhere: bool,

    /// Character to use to represent elided text.
    ///
    /// The default is `…`.
    ///
    /// If not set, no character will be used (but the text will still be elided).
    pub overflow_character: Option<char>,

    /// Source of word boundaries for scripts that are written without spaces
    /// (Thai, Lao, Khmer, Myanmar).
    ///
    /// These scripts need dictionary-based line breaking ([UAX #14] class SA).
    /// `None` (the default) uses the built-in breaker: the bundled dictionary
    /// when the `sea_dictionary` feature is enabled, otherwise no extra break
    /// opportunities (behaviour identical to egui 0.37). Applications with their
    /// own segmenter (ICU, `HarfBuzz`, OS text services) install it here.
    ///
    /// The breaker is consulted once per layout with the full
    /// [`LayoutJob::text`]; see [`crate::text::LineBreaker`] for the range contract.
    ///
    /// The breaker takes part in the galley-cache key by identity, so install it
    /// once and keep reusing the same `Arc` — recreating it every frame forfeits
    /// galley caching.
    ///
    /// [UAX #14]: https://www.unicode.org/reports/tr14/#SA
    #[cfg_attr(feature = "serde", serde(skip))]
    pub line_breaker: Option<Arc<dyn crate::text::LineBreaker>>,
}

impl core::fmt::Debug for TextWrapping {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TextWrapping")
            .field("max_width", &self.max_width)
            .field("max_rows", &self.max_rows)
            .field("break_anywhere", &self.break_anywhere)
            .field("overflow_character", &self.overflow_character)
            .field("line_breaker", &self.line_breaker.is_some())
            .finish()
    }
}

impl PartialEq for TextWrapping {
    fn eq(&self, other: &Self) -> bool {
        self.max_width == other.max_width
            && self.max_rows == other.max_rows
            && self.break_anywhere == other.break_anywhere
            && self.overflow_character == other.overflow_character
            && match (&self.line_breaker, &other.line_breaker) {
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                (None, None) => true,
                (Some(_), None) | (None, Some(_)) => false,
            }
    }
}

impl core::hash::Hash for TextWrapping {
    #[inline]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        let Self {
            max_width,
            max_rows,
            break_anywhere,
            overflow_character,
            line_breaker,
        } = self;
        emath::OrderedFloat(*max_width).hash(state);
        max_rows.hash(state);
        break_anywhere.hash(state);
        overflow_character.hash(state);
        match line_breaker {
            None => 0_u8.hash(state),
            Some(breaker) => {
                1_u8.hash(state);
                // Hash the breaker by identity: two different breakers with
                // otherwise-identical jobs must not share a cache key. The
                // galley cache additionally pins and verifies this identity
                // (see `CachedGalley::breaker` in `galley_cache.rs`).
                (Arc::as_ptr(breaker).cast::<u8>() as usize).hash(state);
            }
        }
    }
}

impl Default for TextWrapping {
    fn default() -> Self {
        Self {
            max_width: f32::INFINITY,
            max_rows: usize::MAX,
            break_anywhere: false,
            overflow_character: Some('…'),
            line_breaker: None,
        }
    }
}

impl TextWrapping {
    /// Create a [`TextWrapping`] from a [`TextWrapMode`] and an available width.
    pub fn from_wrap_mode_and_width(mode: TextWrapMode, max_width: f32) -> Self {
        match mode {
            TextWrapMode::Extend => Self::no_max_width(),
            TextWrapMode::Wrap => Self::wrap_at_width(max_width),
            TextWrapMode::Truncate => Self::truncate_at_width(max_width),
        }
    }

    /// A row can be as long as it need to be.
    pub fn no_max_width() -> Self {
        Self {
            max_width: f32::INFINITY,
            ..Default::default()
        }
    }

    /// A row can be at most `max_width` wide but can wrap in any number of lines.
    pub fn wrap_at_width(max_width: f32) -> Self {
        Self {
            max_width,
            ..Default::default()
        }
    }

    /// Elide text that doesn't fit within the given width, replaced with `…`.
    pub fn truncate_at_width(max_width: f32) -> Self {
        Self {
            max_width,
            max_rows: 1,
            break_anywhere: true,
            ..Default::default()
        }
    }
}

// ----------------------------------------------------------------------------

/// Text that has been laid out, ready for painting.
///
/// You can create a [`Galley`] using [`crate::FontsView::layout_job`];
///
/// Needs to be recreated if the underlying font atlas texture changes, which
/// happens under the following conditions:
/// - `pixels_per_point` or `max_texture_size` change. These parameters are set
///   in [`crate::text::Fonts::begin_pass`]. When using `egui` they are set
///   from `egui::InputState` and can change at any time.
/// - The atlas has become full. This can happen any time a new glyph is added
///   to the atlas, which in turn can happen any time new text is laid out.
///
/// The name comes from typography, where a "galley" is a metal tray
/// containing a column of set type, usually the size of a page of text.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Galley {
    /// The job that this galley is the result of.
    /// Contains the original string and style sections.
    pub job: Arc<LayoutJob>,

    /// Rows of text, from top to bottom, and their offsets.
    ///
    /// The number of characters in all rows sum up to `job.text.chars().count()`
    /// unless [`Self::elided`] is `true`.
    ///
    /// Note that a paragraph (a piece of text separated with `\n`)
    /// can be split up into multiple rows.
    pub rows: Vec<PlacedRow>,

    /// Set to true the text was truncated due to [`TextWrapping::max_rows`].
    pub elided: bool,

    /// Bounding rect.
    ///
    /// `rect.top()` is always 0.0.
    ///
    /// With [`LayoutJob::halign`]:
    /// * [`Align::LEFT`]: `rect.left() == 0.0`
    /// * [`Align::Center`]: `rect.center() == 0.0`
    /// * [`Align::RIGHT`]: `rect.right() == 0.0`
    ///
    /// The anchors hold for the aligned ink; whitespace that the alignment
    /// ignores ([`LayoutJob::keep_trailing_whitespace`] is `false`) extends
    /// the rect reading-forward from the anchored ink. Reading-forward
    /// follows the row's base direction: leftward on a right-to-left row.
    pub rect: Rect,

    /// Tight bounding box around all the meshes in all the rows.
    /// Can be used for culling.
    pub mesh_bounds: Rect,

    /// Total number of vertices in all the row meshes.
    pub num_vertices: usize,

    /// Total number of indices in all the row meshes.
    pub num_indices: usize,

    /// The number of physical pixels for each logical point.
    /// Since this affects the layout, we keep track of it
    /// so that we can warn if this has changed once we get to
    /// tessellation.
    pub pixels_per_point: f32,

    pub(crate) intrinsic_size: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PlacedRow {
    /// The position of this [`Row`] relative to the galley.
    ///
    /// This is rounded to the closest _pixel_ in order to produce crisp, pixel-perfect text.
    pub pos: Pos2,

    /// The underlying unpositioned [`Row`].
    pub row: Arc<Row>,

    /// If true, this [`PlacedRow`] came from a paragraph ending with a `\n`.
    /// The `\n` itself is omitted from row's [`Row::glyphs`].
    /// A `\n` in the input text always creates a new [`PlacedRow`] below it,
    /// so that text that ends with `\n` has an empty [`PlacedRow`] last.
    /// This also implies that the last [`PlacedRow`] in a [`Galley`] always has `ends_with_newline == false`.
    pub ends_with_newline: bool,
}

impl PlacedRow {
    /// Logical bounding rectangle on font heights etc.
    ///
    /// This ignores / includes the `LayoutSection::leading_space`.
    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.pos, self.row.size)
    }

    /// Same as [`Self::rect`] but excluding the `LayoutSection::leading_space`.
    pub fn rect_without_leading_space(&self) -> Rect {
        let x = self.pos.x + self.glyphs.first().map_or(0.0, |g| g.pos.x);
        let right = self.pos.x + self.size.x;
        Rect::from_min_max(
            Pos2::new(x, self.pos.y),
            Pos2::new(right, self.pos.y + self.size.y),
        )
    }
}

impl core::ops::Deref for PlacedRow {
    type Target = Row;

    fn deref(&self) -> &Self::Target {
        &self.row
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Row {
    /// This is included in case there are no glyphs.
    ///
    /// Only used during layout, then set to an invalid value in order to
    /// enable the paragraph-concat optimization path without having to
    /// adjust `section_index` when concatting.
    pub(crate) section_index_at_start: u32,

    /// The paragraph's resolved base reading direction (UAX #9 P2/P3).
    ///
    /// The glyphs are stored in *visual* order (see [`Self::glyphs`]); this
    /// direction is what lets cursor columns — which are *logical* character
    /// offsets — be mapped onto those glyphs ([`Self::x_offset`] /
    /// [`Self::char_at`]).
    #[cfg_attr(feature = "serde", serde(default))]
    pub base_direction: TextDirection,

    /// The `LayoutSection::leading_space` advance consumed before this row's
    /// paragraph began, when this row carries the paragraph's first glyph.
    /// Kept in the ink at the reading start for right-anchored rows.
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) leading_advance: f32,

    /// One for each `char`.
    ///
    /// The glyphs are laid out left-to-right (their `pos.x` increases along
    /// the slice). For paragraphs containing right-to-left text this is the
    /// *visual* order per UAX #9: rows with bidirectionally reordered runs
    /// do not list their glyphs in logical string order. Consequence for
    /// cursors: a cursor column is a *logical* character offset, and mapping
    /// it onto the visual glyphs needs the row's base direction
    /// ([`Self::base_direction`]). That mapping is exact for every row via
    /// the per-level caret-boundary tables ([`Self::visual_caret_boundaries`],
    /// computed on demand from the stamped `Glyph::bidi_level` — no stored
    /// run map needed): each column's caret sits at the visual boundary the
    /// run displaying it assigns, and only degenerate streams (where two
    /// reading positions share one visual boundary) degrade to the historic
    /// base-parity mapping via the keep-first collision rule.
    ///
    /// Wrapped right-to-left text used to break UAX #9 L1/L2 per rendered
    /// line (words could end up on the wrong rows, e.g. an RTL-base
    /// `أول ثاني ثالث` wrapping with the logically last word `ثالث` on the
    /// first row): wrapping now walks the logical stream and reorders each
    /// produced row per UAX #9, so wrapped rows follow the reading order.
    /// The in-run cursor mapping is exact on wrapped rows too (the same
    /// caret-boundary tables, per row). Single-row text is unaffected.
    pub glyphs: Vec<Glyph>,

    /// Logical size based on font heights etc.
    /// Includes leading and trailing whitespace.
    pub size: Vec2,

    /// The mesh, ready to be rendered.
    pub visuals: RowVisuals,
}

/// The tessellated output of a row.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RowVisuals {
    /// The tessellated text, using non-normalized (texel) UV coordinates.
    /// That is, you need to divide the uv coordinates by the texture size.
    pub mesh: Mesh,

    /// Bounds of the mesh, and can be used for culling.
    /// Does NOT include leading or trailing whitespace glyphs!!
    pub mesh_bounds: Rect,

    /// The number of triangle indices added before the first glyph triangle.
    ///
    /// This can be used to insert more triangles after the background but before the glyphs,
    /// i.e. for text selection visualization.
    pub glyph_index_start: usize,

    /// The range of vertices in the mesh that contain glyphs (as opposed to background, underlines, strikethorugh, etc).
    ///
    /// The glyph vertices comes after backgrounds (if any), but before any underlines and strikethrough.
    pub glyph_vertex_range: Range<usize>,
}

impl Default for RowVisuals {
    fn default() -> Self {
        Self {
            mesh: Default::default(),
            mesh_bounds: Rect::NOTHING,
            glyph_index_start: 0,
            glyph_vertex_range: 0..0,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Glyph {
    /// The character this glyph represents.
    pub chr: char,

    /// Baseline position, relative to the row.
    /// Logical position: pos.y is the same for all chars of the same [`TextFormat`].
    pub pos: Pos2,

    /// Logical width of the glyph.
    pub advance_width: f32,

    /// Height of this row of text.
    ///
    /// Usually same as [`Self::font_height`],
    /// unless explicitly overridden by [`TextFormat::line_height`].
    pub line_height: f32,

    /// The ascent of this font.
    pub font_ascent: f32,

    /// The row/line height of this font.
    pub font_height: f32,

    /// The ascent of the sub-font within the font (`FontFace`).
    pub font_face_ascent: f32,

    /// The row/line height of the sub-font within the font (`FontFace`).
    pub font_face_height: f32,

    /// Position and size of the glyph in the font texture, in texels.
    pub uv_rect: UvRect,

    /// Whether this glyph carries its own color, e.g. a color emoji.
    pub is_color: bool,

    /// The resolved UAX #9 embedding level of the directional run this glyph
    /// was laid out for (odd = right-to-left; the row's base direction is
    /// [`Row::base_direction`]).
    ///
    /// Pure left-to-right rows stamp level 0 on every glyph. The glyphs of a
    /// row are in visual order, so splitting the slice at `bidi_level` changes
    /// recovers the directional runs in stream order (see e.g. the
    /// accessibility `TextRun` builder). Serialized galleys from before this
    /// field existed degrade to all-level-0: such rows read and step as
    /// left-to-right content. In a left-to-right base that is the historic
    /// behavior; in a right-to-left base the caret table treats them as
    /// embedded LTR runs (stepping by the local level) instead of mirroring
    /// the step by the base direction.
    #[cfg_attr(feature = "serde", serde(default))]
    pub bidi_level: u8,

    /// Whether the shaper emitted this glyph as one of a shaping cluster's
    /// surplus glyphs: the cluster shapes to more glyphs than characters (a
    /// decomposing font splitting one character into a component glyph plus
    /// its base), so this glyph must not own a cursor column — galley
    /// assembly folds it into the cluster's remaining glyph after
    /// tessellation, keeping one glyph per character. Glyphs of clusters
    /// that shape to one glyph per character or fewer — shaped combining
    /// marks among them, whose glyph draws ink but stands for its own
    /// character — are never stamped. Serialized galleys from before this
    /// field existed degrade to `false`: their decomposed rows keep one
    /// phantom cursor column per surplus glyph.
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_cluster_surplus: bool,

    /// Index into [`LayoutJob::sections`]. Decides color etc.
    ///
    /// Only used during layout, then set to an invalid value in order to
    /// enable the paragraph-concat optimization path without having to
    /// adjust `section_index` when concatting.
    pub(crate) section_index: u32,

    /// Which is our first vertex in [`RowVisuals::mesh`].
    pub first_vertex: u32,
}

/// Pin the glyph size: galleys are cached per layout, so a silently growing
/// [`Glyph`] would bloat every galley in every cache. rustc packs the
/// one-byte fields (`is_color`, `bidi_level`, `is_cluster_surplus`) into
/// existing padding, keeping the struct at 72 bytes. If this assert fires,
/// your field made every stored glyph grow — re-check the layout before
/// shipping.
const _: () = assert!(
    core::mem::size_of::<Glyph>() == 72,
    "Glyph grew beyond 72 bytes: every cached galley holds one Glyph per char",
);

impl Glyph {
    /// True if the glyph's directional run was laid out right-to-left (odd
    /// UAX #9 embedding level).
    #[inline]
    pub fn is_rtl(&self) -> bool {
        self.bidi_level % 2 == 1
    }

    #[inline]
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.advance_width, self.line_height)
    }

    #[inline]
    pub fn max_x(&self) -> f32 {
        self.pos.x + self.advance_width
    }

    /// Same y range for all characters with the same [`TextFormat`].
    #[inline]
    pub fn logical_rect(&self) -> Rect {
        Rect::from_min_size(self.pos - vec2(0.0, self.font_ascent), self.size())
    }
}

// ----------------------------------------------------------------------------

impl Row {
    /// The text on this row, excluding the implicit `\n` if any.
    pub fn text(&self) -> String {
        self.glyphs.iter().map(|g| g.chr).collect()
    }

    /// Excludes the implicit `\n` after the [`Row`], if any.
    #[inline]
    pub fn char_count_excluding_newline(&self) -> CharIndex {
        CharIndex(self.glyphs.len())
    }

    /// Closest char at the desired x coordinate in row-relative coordinates.
    /// Returns something in the range `[0, char_count_excluding_newline()]`.
    pub fn char_at(&self, desired_x: f32) -> CharIndex {
        if self.is_bidi_homogeneous_with_base() {
            // The historic base-parity mapping is exact for rows whose
            // content shares the base direction (pure LTR in an LTR base,
            // pure RTL in an RTL base, empty, elision-only):
            if self.base_direction == TextDirection::RightToLeft {
                return self.char_at_rtl(desired_x);
            }
            for (i, glyph) in self.glyphs.iter().enumerate() {
                if desired_x < glyph.logical_rect().center().x {
                    return CharIndex(i);
                }
            }
            return self.char_count_excluding_newline();
        }
        self.char_at_mixed(desired_x)
    }

    /// The closest *logical* column to `desired_x` in a mixed-direction row
    /// (a row that is not [`Self::is_bidi_homogeneous_with_base`]).
    ///
    /// The mirror of [`Self::x_offset`]'s table path, so a click resolves to
    /// the column whose caret renders nearest it: each cluster cell
    /// ([`Self::bidi_cluster_cells`]) contributes candidate columns by its
    /// *own* parity (not the row's base parity) — an odd cell reads
    /// right-to-left across its box, an even cell left-to-right — with the
    /// cell's first-read column taken from the caret-boundary tables
    /// ([`Self::visual_caret_boundaries`]). Among the candidates the click
    /// selects the one with the nearest caret x (ties keep the last,
    /// deterministic like the historic keep-last scan); a click no cell
    /// catches falls back to the reading exit, [`CharIndex`] `num_glyphs` —
    /// the same fallback the historic arms use.
    fn char_at_mixed(&self, desired_x: f32) -> CharIndex {
        let num_glyphs = self.glyphs.len();
        let (boundary_of_column, column_of_boundary) = self.visual_caret_boundaries();
        let caret_x_of_column = |column: usize| -> Option<f32> {
            boundary_of_column
                .get(column)
                .copied()
                .flatten()
                .map(|boundary| self.boundary_x(boundary))
        };

        let mut best: Option<CharIndex> = None;
        let mut best_dist = f32::INFINITY;

        for cell in self.bidi_cluster_cells() {
            let Some(leading) = self.glyphs.get(cell.start) else {
                continue;
            };
            let odd = leading.bidi_level % 2 == 1;
            // The cell's first-read column: an odd cell is entered at its
            // right edge, an even cell at its left edge. `None` (a degenerate
            // stream where two reading positions share one visual boundary):
            // this cell has no table column — skip it.
            let entry_boundary = if odd { cell.end } else { cell.start };
            let Some(col_lo) = column_of_boundary
                .get(entry_boundary)
                .copied()
                .flatten()
                .map(|column| column.0)
            else {
                continue;
            };

            // The cell's ink box and its equal sub-slot subdivision (the same
            // box math and documented approximation [`Self::char_at_rtl`]
            // makes; cells are never empty, so `len >= 1`):
            let Range { start, end } = cell;
            let box_lo = self.glyphs[start..end]
                .iter()
                .map(|glyph| glyph.pos.x)
                .fold(f32::INFINITY, f32::min);
            let box_hi = self.glyphs[start..end]
                .iter()
                .map(|glyph| glyph.max_x())
                .fold(f32::NEG_INFINITY, f32::max);
            let len = end.saturating_sub(start);

            // The sub-slot whose center the click is reading-side of, in the
            // cell's own reading direction (right-to-left for odd, left-to-
            // right for even cells). NaN/±inf clicks match nothing and fall
            // through to the fallback below.
            let candidate = if odd {
                (0..len).find_map(|t| {
                    let center = box_hi - (t as f32 + 0.5) * (box_hi - box_lo) / len as f32;
                    (desired_x > center).then(|| col_lo.saturating_add(t))
                })
            } else {
                (0..len).find_map(|t| {
                    let center = box_lo + (t as f32 + 0.5) * (box_hi - box_lo) / len as f32;
                    (desired_x < center).then(|| col_lo.saturating_add(t))
                })
            };
            let Some(candidate) = candidate else {
                continue;
            };
            // Nearest-caret selection across cells: cells to the click's
            // reading-side all match, so the winner is the candidate whose
            // caret is painted nearest the click (equivalent to comparing
            // against `boundary_x(boc[cand])`, from the already-built table).
            let Some(caret_x) = caret_x_of_column(candidate) else {
                continue;
            };
            let dist = (desired_x - caret_x).abs();
            if dist <= best_dist {
                best = Some(CharIndex(candidate));
                best_dist = dist;
            }
        }

        best.unwrap_or(CharIndex(num_glyphs))
    }

    /// The closest *logical* column to `desired_x` in a right-to-left row.
    ///
    /// The mirror image of the left-to-right [`Self::char_at`]: glyphs are
    /// stored in visual order while columns count from the reading start (the
    /// visual right edge), so scan the glyphs from right to left in logical
    /// order and stop at the first cluster cell whose center the click is
    /// reading-side (right) of.
    fn char_at_rtl(&self, desired_x: f32) -> CharIndex {
        let num_glyphs = self.glyphs.len();
        let mut best: Option<CharIndex> = None;

        // Walk the glyphs as cluster groups (a base glyph plus the zero-advance
        // continuation glyphs of its cluster), from the visually left end to
        // the right; each group covers the next-highest unassigned logical
        // columns. Keeping the *last* group with a match therefore keeps the
        // rightmost one — the first in logical order, which is the one the
        // mirrored left-to-right scan would stop at.
        let mut remaining = num_glyphs;
        let mut i = 0;
        while i < num_glyphs {
            let mut j = i + 1;
            while j < num_glyphs && self.glyphs[j].advance_width <= 0.0 {
                j += 1;
            }
            let cells = j - i;
            let group_lo = remaining.saturating_sub(cells);

            let box_lo = self.glyphs[i..j]
                .iter()
                .map(|glyph| glyph.pos.x)
                .fold(f32::INFINITY, f32::min);
            let box_hi = self.glyphs[i..j]
                .iter()
                .map(|glyph| glyph.max_x())
                .fold(f32::NEG_INFINITY, f32::max);

            for t in 0..cells {
                let center = box_hi - (t as f32 + 0.5) * (box_hi - box_lo) / cells as f32;
                if desired_x > center {
                    best = Some(CharIndex(group_lo + t));
                    break;
                }
            }

            remaining = group_lo;
            i = j;
        }

        best.unwrap_or(CharIndex(num_glyphs))
    }

    pub fn x_offset(&self, column: CharIndex) -> f32 {
        if !self.is_bidi_homogeneous_with_base() {
            // Mixed row: the caret for a logical column sits at the visual
            // boundary the per-level caret-boundary tables assign it, so the
            // painted caret lands where the local-level stepping and the hit
            // test place it (see [`Self::visual_caret_boundaries`]):
            let (boundary_of_column, _) = self.visual_caret_boundaries();
            if let Some(boundary) = boundary_of_column.get(column.0).copied().flatten() {
                return self.boundary_x(boundary);
            }
            // None (collision loser / degenerate stream) -> historic arm
            // below.
        }
        if self.base_direction == TextDirection::RightToLeft {
            return self.x_offset_rtl(column);
        }
        if let Some(glyph) = self.glyphs.get(column.0) {
            glyph.pos.x
        } else {
            self.size.x
        }
    }

    /// The painted x of a visual caret boundary `k` (`0..=n`, the row's
    /// visual slot edges): glyph `k`'s painted anchor for `k < n`, the row
    /// width (the visual right edge, the same anchor the historic
    /// left-to-right end-of-row caret uses) for `k = n` — and for any other
    /// out-of-range boundary, so a hostile index degrades to the row edge
    /// instead of panicking.
    fn boundary_x(&self, boundary: usize) -> f32 {
        match self.glyphs.get(boundary) {
            Some(glyph) => glyph.pos.x,
            None => self.size.x,
        }
    }

    /// The caret x for a *logical* column of a right-to-left row, in
    /// row-relative coordinates.
    ///
    /// Glyphs are stored in visual order while columns count from the reading
    /// start (the visual right edge): the caret for a column sits at the
    /// reading-side (right) edge of the cluster box displaying it, stepping
    /// leftward through multi-glyph clusters. Past the text end the caret sits
    /// at the visual left edge of the row.
    fn x_offset_rtl(&self, column: CharIndex) -> f32 {
        let num_glyphs = self.glyphs.len();
        if num_glyphs == 0 {
            return self.size.x;
        }
        if num_glyphs <= column.0 {
            // Past the text end: the reading end is the visual left edge.
            return self
                .glyphs
                .iter()
                .map(|glyph| glyph.pos.x)
                .fold(f32::INFINITY, f32::min);
        }

        // Walk the glyphs as cluster groups, from the visually left end to the
        // right; each group covers the next-highest unassigned logical columns.
        let mut remaining = num_glyphs;
        let mut i = 0;
        while i < num_glyphs {
            let mut j = i + 1;
            while j < num_glyphs && self.glyphs[j].advance_width <= 0.0 {
                j += 1;
            }
            let cells = j - i;
            let group_lo = remaining.saturating_sub(cells);

            if group_lo <= column.0 {
                let t = column.0 - group_lo;
                let box_lo = self.glyphs[i..j]
                    .iter()
                    .map(|glyph| glyph.pos.x)
                    .fold(f32::INFINITY, f32::min);
                let box_hi = self.glyphs[i..j]
                    .iter()
                    .map(|glyph| glyph.max_x())
                    .fold(f32::NEG_INFINITY, f32::max);
                return box_hi - t as f32 * (box_hi - box_lo) / cells as f32;
            }

            remaining = group_lo;
            i = j;
        }

        // `column < num_glyphs` always lands in some group; fall back to the
        // reading end rather than panicking on unexpected row states.
        self.glyphs[0].pos.x
    }

    /// Groups the row's visually-ordered glyphs into cluster cells: half-open
    /// ranges into [`Self::glyphs`], where a new cell starts at glyph 0 and at
    /// every glyph with a positive advance (zero-advance glyphs are the
    /// continuations of the cluster before them; a genuinely zero-advance
    /// *base* merges into its neighbour — the same grouping and the same
    /// documented approximation the row-level mapping ([`Self::char_at_rtl`])
    /// and the accessibility `TextRun` builder make). An empty row yields no
    /// cells. The level of a cell is its first glyph's `bidi_level`.
    fn bidi_cluster_cells(&self) -> Vec<Range<usize>> {
        let mut cells: Vec<Range<usize>> = Vec::new();
        let mut start = 0;
        for (i, glyph) in self.glyphs.iter().enumerate().skip(1) {
            if glyph.advance_width > 0.0 {
                cells.push(start..i);
                start = i;
            }
        }
        if !self.glyphs.is_empty() {
            cells.push(start..self.glyphs.len());
        }
        cells
    }

    /// The reading-order permutation of `cells`: the cells' indices in the
    /// order the row's text reads, by re-applying the UAX #9 L2 reversal
    /// schedule to the visual cell sequence (for `level` from
    /// `(min_level | 1)..=max_level` descending, reverse each maximal
    /// contiguous subsequence of cells at `level >= level`;
    /// `min_level | 1` is exactly unicode-bidi's `new_lowest_ge_rtl`,
    /// `Level(self.0 | 1)`).
    ///
    /// This is the single shared copy of the schedule: the items may be any
    /// non-empty ranges into `Self::glyphs` whose level is the first glyph's
    /// `bidi_level` stamp — the row's cluster cells here, whole same-level
    /// text runs in egui's accessibility bridge
    /// (`accesskit_text.rs::logical_run_order`), which feeds this the row's
    /// non-empty runs and re-appends its glyph-less newline-only runs last,
    /// because empty items carry no level of their own. All-level-0 rows
    /// (legacy deserialized galleys) skip the replay entirely (`0 | 1 > 0`),
    /// so their cells stay in visual order: the content reads as
    /// left-to-right.
    pub fn logical_cell_order(&self, cells: &[Range<usize>]) -> Vec<usize> {
        let level_of = |i: usize| -> u8 {
            cells
                .get(i)
                .and_then(|cell| self.glyphs.get(cell.start))
                .map_or(0, |glyph| glyph.bidi_level)
        };
        let min_level = (0..cells.len()).map(level_of).min().unwrap_or(0);
        let max_level = (0..cells.len()).map(level_of).max().unwrap_or(0);
        let bound = min_level | 1;

        let mut order: Vec<usize> = (0..cells.len()).collect();
        for level in (bound..=max_level).rev() {
            let mut i = 0;
            while i < order.len() {
                if level_of(order[i]) >= level {
                    let start = i;
                    while i < order.len() && level_of(order[i]) >= level {
                        i += 1;
                    }
                    order[start..i].reverse();
                } else {
                    i += 1;
                }
            }
        }
        order
    }

    /// The caret boundary table of a mixed-direction row.
    ///
    /// Returns `(boundary_of_column, column_of_boundary)`, both with
    /// `glyphs.len() + 1` entries: the caret boundaries `k = 0..=n` are the
    /// row's visual edges (`k = 0` the visual left edge, `k = n` the visual
    /// right), `boundary_of_column[c]` is the visual boundary the caret for
    /// *logical* column `c` sits at, and `column_of_boundary[k]` is the
    /// logical column whose caret sits at visual boundary `k`. A missing
    /// entry (`None`) means the mapping is undefined for that column or
    /// boundary (an empty row, or a degenerate stream where two reading
    /// positions share one visual boundary) — the caller falls back to the
    /// historic step.
    ///
    /// Construction, over the cluster cells ([`Self::bidi_cluster_cells`]) in
    /// reading order ([`Self::logical_cell_order`]):
    ///
    /// 1. The reading walk's anchors: the first-read cell's entry caret (an
    ///    even cell's left edge, an odd cell's right edge), then every
    ///    reading transition's caret — at the shared visual boundary of
    ///    visually-adjacent cells, or at the entering cell's own entry edge
    ///    across a bidi reading "teleport" (e.g. عربي↔abc). These are
    ///    written first so they win the keep-first race below against the
    ///    neighbouring cells' own walks.
    /// 2. Within each cell, its caret columns walk across the cell's box in
    ///    its parity direction (even: `s..=e` ascending; odd: `e..=s`
    ///    descending), from its entry edge to its exit edge — the last-read
    ///    cell's exit edge is the row's reading-exit caret (column `n`).
    ///    Sub-columns follow the cell's own parity, exactly reproducing
    ///    [`Self::x_offset_rtl`]'s in-cluster sub-columns for odd cells and
    ///    the left-to-right glyph-left rule for even cells.
    ///
    /// Writes are keep-first: where two reading positions claim one visual
    /// boundary (degenerate streams), the first (reading-order) claim wins
    /// and the loser looks up to `None` — the caller falls back. No panics
    /// are possible: every index comes from `saturating` arithmetic bounded
    /// by `glyphs.len()`, both vecs are pre-sized `n + 1`, and every read and
    /// write goes through `get`/`get_mut`.
    ///
    /// `pub(super)`: the caret-geometry tests in the sibling `text_layout`
    /// module pin the painted walk against these tables directly.
    pub(super) fn visual_caret_boundaries(&self) -> (Vec<Option<usize>>, Vec<Option<CharIndex>>) {
        let n = self.glyphs.len();
        let mut boundary_of_column: Vec<Option<usize>> = Vec::new();
        boundary_of_column.resize(n + 1, None);
        let mut column_of_boundary: Vec<Option<CharIndex>> = Vec::new();
        column_of_boundary.resize(n + 1, None);

        let cells = self.bidi_cluster_cells();
        let order = self.logical_cell_order(&cells);

        // The read cells in reading order: (first caret column, slot span,
        // odd embedding level). The first caret column of a cell is shared
        // with the previous-read cell's exit column; the cursor advances by
        // the cell's glyph count, so the columns tile `0..=n`.
        let mut read_cells: Vec<(usize, Range<usize>, bool)> = Vec::new();
        let mut column_cursor = 0usize;
        for &cell_index in &order {
            let Some(cell) = cells.get(cell_index).cloned() else {
                continue;
            };
            let Some(leading) = self.glyphs.get(cell.start) else {
                continue;
            };
            let len = cell.end.saturating_sub(cell.start);
            read_cells.push((column_cursor, cell, leading.bidi_level % 2 == 1));
            column_cursor = column_cursor.saturating_add(len);
        }

        // A keep-first claim is atomic across the two tables: it lands only
        // when BOTH of its slots are still free, so a column whose mapping
        // was already decided (by an anchor) can never steal a boundary in
        // the inverse table.
        fn claim(
            boundary_of_column: &mut [Option<usize>],
            column_of_boundary: &mut [Option<CharIndex>],
            column: usize,
            boundary: usize,
        ) {
            let column_free = boundary_of_column
                .get(column)
                .is_none_or(|slot| slot.is_none());
            let boundary_free = column_of_boundary
                .get(boundary)
                .is_none_or(|slot| slot.is_none());
            if !(column_free && boundary_free) {
                return;
            }
            if let Some(slot) = boundary_of_column.get_mut(column) {
                *slot = Some(boundary);
            }
            if let Some(slot) = column_of_boundary.get_mut(boundary) {
                *slot = Some(CharIndex(column));
            }
        }

        // Pass 1 — the anchors (rules 1-2): the first-read cell's entry
        // caret, then every reading transition's caret. Written before the
        // walks so a transition value wins the keep-first race against both
        // neighbours' parity walks.
        let mut previous: Option<Range<usize>> = None;
        for (column, cell, odd) in &read_cells {
            let entry_edge = if *odd { cell.end } else { cell.start };
            let value = match previous {
                None => entry_edge,
                Some(prev_cell) => {
                    if prev_cell.end == cell.start {
                        // Visually adjacent (the previous-read cell ends
                        // where this one begins): the caret sits at their
                        // shared boundary.
                        prev_cell.end
                    } else if cell.end == prev_cell.start {
                        // Visually adjacent the other way (this cell sits
                        // visually left of the previous-read one): again a
                        // shared boundary.
                        cell.end
                    } else {
                        // Bidi reading teleport: land on this cell's own
                        // entry edge.
                        entry_edge
                    }
                }
            };
            claim(
                &mut boundary_of_column,
                &mut column_of_boundary,
                *column,
                value,
            );
            previous = Some(cell.clone());
        }

        // Pass 2 — within-cell columns (rule 3): each cell's caret columns
        // walk across its own box in its parity direction (even: `s..=e`
        // ascending; odd: `e..=s` descending), from its entry edge to its
        // exit edge. The last-read cell's exit lands on column `n`, the
        // row's reading-exit caret. Keep-first: the anchors above win where
        // they coincide.
        for (column, cell, odd) in &read_cells {
            let len = cell.end.saturating_sub(cell.start);
            let anchor = if *odd { cell.end } else { cell.start };
            for j in 0..=len {
                let boundary = if *odd {
                    anchor.saturating_sub(j)
                } else {
                    anchor.saturating_add(j)
                };
                let walked = column.saturating_add(j);
                claim(
                    &mut boundary_of_column,
                    &mut column_of_boundary,
                    walked,
                    boundary,
                );
            }
        }
        (boundary_of_column, column_of_boundary)
    }

    /// Fast path for caret stepping: true when the historic base-mirrored
    /// step already moves the caret one visual position per keypress, i.e.
    /// the row is empty or every glyph shares one `bidi_level` whose parity
    /// equals the row's base direction. Such rows (pure LTR in an LTR base,
    /// pure RTL in an RTL base, empty, elision-only) never need the caret
    /// table.
    fn is_bidi_homogeneous_with_base(&self) -> bool {
        let base_parity = u8::from(self.base_direction == TextDirection::RightToLeft);
        match self.glyphs.first() {
            None => true,
            Some(leading) => {
                let level = leading.bidi_level;
                level % 2 == base_parity
                    && self.glyphs.iter().all(|glyph| glyph.bidi_level == level)
            }
        }
    }

    /// Fast path for word stepping: true when the whole-text logical word walk
    /// already moves the caret the way the visual word arrows must, i.e. the
    /// row is empty or every glyph sits at embedding level 0 under a
    /// left-to-right base direction. Strictly narrower than
    /// [`Self::is_bidi_homogeneous_with_base`]: uniform right-to-left rows are
    /// deliberately excluded (their visual word edges run the other way, which
    /// is exactly what the level-aware walk corrects), while an all-even row
    /// under a right-to-left base reads left-to-right and stays on the
    /// level-aware walk.
    fn is_ltr_homogeneous(&self) -> bool {
        self.base_direction == TextDirection::LeftToRight
            && self.glyphs.iter().all(|glyph| glyph.bidi_level == 0)
    }

    /// Whether the row's first reading-order cell (the reading walk of
    /// [`Self::bidi_cluster_cells`] through [`Self::logical_cell_order`])
    /// carries an odd `bidi_level`, i.e. reads right-to-left. An empty row has
    /// no cell and reads left-to-right.
    fn first_read_cell_is_odd(&self) -> bool {
        let cells = self.bidi_cluster_cells();
        let order = self.logical_cell_order(&cells);
        order
            .first()
            .and_then(|&cell_index| cells.get(cell_index))
            .and_then(|cell| self.glyphs.get(cell.start))
            .is_some_and(|glyph| glyph.bidi_level % 2 == 1)
    }

    #[inline]
    pub fn height(&self) -> f32 {
        self.size.y
    }
}

impl PlacedRow {
    #[inline]
    pub fn min_y(&self) -> f32 {
        self.rect().top()
    }

    #[inline]
    pub fn max_y(&self) -> f32 {
        self.rect().bottom()
    }

    /// Includes the implicit `\n` after the [`PlacedRow`], if any.
    #[inline]
    pub fn char_count_including_newline(&self) -> CharIndex {
        CharIndex(self.row.glyphs.len() + (self.ends_with_newline as usize))
    }
}

impl Galley {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.job.is_empty()
    }

    /// The full, non-elided text of the input job.
    #[inline]
    pub fn text(&self) -> &str {
        &self.job.text
    }

    #[inline]
    pub fn size(&self) -> Vec2 {
        self.rect.size()
    }

    /// This is the size that a non-wrapped, non-truncated, non-justified version of the text
    /// would have.
    ///
    /// Useful for advanced layouting.
    #[inline]
    pub fn intrinsic_size(&self) -> Vec2 {
        // We do the rounding here instead of in `round_output_to_gui` so that rounding
        // errors don't accumulate when concatenating multiple galleys.
        if self.job.round_output_to_gui {
            self.intrinsic_size.round_ui()
        } else {
            self.intrinsic_size
        }
    }

    pub(crate) fn round_output_to_gui(&mut self) {
        for placed_row in &mut self.rows {
            // Optimization: only call `make_mut` if necessary (can cause a deep clone)
            let rounded_size = placed_row.row.size.round_ui();
            if placed_row.row.size != rounded_size {
                Arc::make_mut(&mut placed_row.row).size = rounded_size;
            }
        }

        let rect = &mut self.rect;

        let did_exceed_wrap_width_by_a_lot = rect.width() > self.job.wrap.max_width + 1.0;

        *rect = rect.round_ui();

        if did_exceed_wrap_width_by_a_lot {
            // If the user picked a too aggressive wrap width (e.g. more narrow than any individual glyph),
            // we should let the user know by reporting that our width is wider than the wrap width.
        } else {
            // Make sure we don't report being wider than the wrap width the user picked:
            rect.max.x = rect
                .max
                .x
                .at_most(rect.min.x + self.job.wrap.max_width)
                .floor_ui();
        }
    }

    /// Append each galley under the previous one.
    pub fn concat(job: Arc<LayoutJob>, galleys: &[Arc<Self>], pixels_per_point: f32) -> Self {
        profiling::function_scope!();

        let mut merged_galley = Self {
            job,
            rows: Vec::new(),
            elided: false,
            rect: Rect::ZERO,
            mesh_bounds: Rect::NOTHING,
            num_vertices: 0,
            num_indices: 0,
            pixels_per_point,
            intrinsic_size: Vec2::ZERO,
        };

        for (i, galley) in galleys.iter().enumerate() {
            let current_y_offset = merged_galley.rect.height();
            let is_last_galley = i + 1 == galleys.len();

            merged_galley
                .rows
                .extend(galley.rows.iter().enumerate().map(|(row_idx, placed_row)| {
                    let new_pos = placed_row.pos + current_y_offset * Vec2::Y;
                    let new_pos = new_pos.round_to_pixels(pixels_per_point);
                    merged_galley.mesh_bounds |=
                        placed_row.visuals.mesh_bounds.translate(new_pos.to_vec2());
                    merged_galley.rect |= Rect::from_min_size(new_pos, placed_row.size);

                    let mut ends_with_newline = placed_row.ends_with_newline;
                    let is_last_row_in_galley = row_idx + 1 == galley.rows.len();
                    // Since we remove the `\n` when splitting rows, we need to add it back here
                    ends_with_newline |= !is_last_galley && is_last_row_in_galley;
                    super::PlacedRow {
                        pos: new_pos,
                        row: Arc::clone(&placed_row.row),
                        ends_with_newline,
                    }
                }));

            merged_galley.num_vertices += galley.num_vertices;
            merged_galley.num_indices += galley.num_indices;
            // Note that if `galley.elided` is true this will be the last `Galley` in
            // the vector and the loop will end.
            merged_galley.elided |= galley.elided;
            merged_galley.intrinsic_size.x =
                f32::max(merged_galley.intrinsic_size.x, galley.intrinsic_size.x);
            merged_galley.intrinsic_size.y += galley.intrinsic_size.y;
        }

        if merged_galley.job.round_output_to_gui {
            merged_galley.round_output_to_gui();
        }

        merged_galley
    }
}

impl AsRef<str> for Galley {
    #[inline]
    fn as_ref(&self) -> &str {
        self.text()
    }
}

impl core::borrow::Borrow<str> for Galley {
    #[inline]
    fn borrow(&self) -> &str {
        self.text()
    }
}

impl core::ops::Deref for Galley {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        self.text()
    }
}

// ----------------------------------------------------------------------------

/// ## Physical positions
impl Galley {
    /// Zero-width rect past the last character.
    ///
    /// The past-the-end caret sits at the reading end: the row's visual right
    /// edge for a left-to-right row, its visual left edge for a right-to-left
    /// one.
    fn end_pos(&self) -> Rect {
        if let Some(row) = self.rows.last() {
            let x = if row.base_direction == TextDirection::RightToLeft {
                row.rect().left()
            } else {
                row.rect().right()
            };
            Rect::from_min_max(pos2(x, row.min_y()), pos2(x, row.max_y()))
        } else {
            // Empty galley
            Rect::from_min_max(pos2(0.0, 0.0), pos2(0.0, 0.0))
        }
    }

    /// Returns a 0-width Rect.
    pub fn pos_from_layout_cursor(&self, layout_cursor: &LayoutCursor) -> Rect {
        let Some(row) = self.rows.get(layout_cursor.row) else {
            return self.end_pos();
        };

        let x = row.x_offset(layout_cursor.column) + row.pos.x;
        Rect::from_min_max(pos2(x, row.min_y()), pos2(x, row.max_y()))
    }

    /// Returns a 0-width Rect.
    pub fn pos_from_cursor(&self, cursor: CCursor) -> Rect {
        self.pos_from_layout_cursor(&self.layout_from_cursor(cursor))
    }

    /// Cursor at the given position within the galley.
    ///
    /// A cursor above the galley is considered
    /// same as a cursor at the start,
    /// and a cursor below the galley is considered
    /// same as a cursor at the end.
    /// This allows implementing text-selection by dragging above/below the galley.
    pub fn cursor_from_pos(&self, pos: Vec2) -> CCursor {
        // Vertical margin around galley improves text selection UX
        const VMARGIN: f32 = 5.0;

        if let Some(first_row) = self.rows.first()
            && pos.y < first_row.min_y() - VMARGIN
        {
            return self.begin();
        }
        if let Some(last_row) = self.rows.last()
            && last_row.max_y() + VMARGIN < pos.y
        {
            return self.end();
        }

        let mut best_y_dist = f32::INFINITY;
        let mut cursor = CCursor::default();

        let mut ccursor_index = CharIndex::ZERO;

        for row in &self.rows {
            let min_y = row.min_y();
            let max_y = row.max_y();

            let is_pos_within_row = min_y <= pos.y && pos.y <= max_y;
            let y_dist = (min_y - pos.y).abs().min((max_y - pos.y).abs());
            if is_pos_within_row || y_dist < best_y_dist {
                best_y_dist = y_dist;
                // char_at is `Row` not `PlacedRow` relative which means we have to subtract the pos.
                let column = row.char_at(pos.x - row.pos.x);
                let prefer_next_row = column < row.char_count_excluding_newline();
                cursor = CCursor {
                    index: ccursor_index + column,
                    prefer_next_row,
                };

                if is_pos_within_row {
                    return cursor;
                }
            }
            ccursor_index += row.char_count_including_newline();
        }

        cursor
    }
}

/// ## Cursor positions
impl Galley {
    /// Cursor to the first character.
    ///
    /// This is the same as [`CCursor::default`].
    #[inline]
    #[expect(clippy::unused_self)]
    pub fn begin(&self) -> CCursor {
        CCursor::default()
    }

    /// Cursor to one-past last character.
    pub fn end(&self) -> CCursor {
        if self.rows.is_empty() {
            return Default::default();
        }
        let mut ccursor = CCursor {
            index: CharIndex::ZERO,
            prefer_next_row: true,
        };
        for row in &self.rows {
            let row_char_count = row.char_count_including_newline();
            ccursor.index += row_char_count;
        }
        ccursor
    }
}

/// ## Cursor conversions
impl Galley {
    // The returned cursor is clamped.
    pub fn layout_from_cursor(&self, cursor: CCursor) -> LayoutCursor {
        let prefer_next_row = cursor.prefer_next_row;
        let mut ccursor_it = CCursor {
            index: CharIndex::ZERO,
            prefer_next_row,
        };

        for (row_nr, row) in self.rows.iter().enumerate() {
            let row_char_count = row.char_count_excluding_newline();

            if ccursor_it.index <= cursor.index && cursor.index <= ccursor_it.index + row_char_count
            {
                let column = cursor.index - ccursor_it.index;

                let select_next_row_instead = prefer_next_row
                    && !row.ends_with_newline
                    && column >= row.char_count_excluding_newline();
                if !select_next_row_instead {
                    return LayoutCursor {
                        row: row_nr,
                        column,
                    };
                }
            }
            ccursor_it.index += row.char_count_including_newline();
        }
        debug_assert!(ccursor_it == self.end(), "Cursor out of bounds");

        if let Some(last_row) = self.rows.last() {
            LayoutCursor {
                row: self.rows.len() - 1,
                column: last_row.char_count_including_newline(),
            }
        } else {
            Default::default()
        }
    }

    fn cursor_from_layout(&self, layout_cursor: LayoutCursor) -> CCursor {
        if layout_cursor.row >= self.rows.len() {
            return self.end();
        }

        let prefer_next_row =
            layout_cursor.column < self.rows[layout_cursor.row].char_count_excluding_newline();
        let mut cursor_it = CCursor {
            index: CharIndex::ZERO,
            prefer_next_row,
        };

        for (row_nr, row) in self.rows.iter().enumerate() {
            if row_nr == layout_cursor.row {
                cursor_it.index += layout_cursor.column.min(row.char_count_excluding_newline());

                return cursor_it;
            }
            cursor_it.index += row.char_count_including_newline();
        }
        cursor_it
    }
}

/// ## Cursor positions
impl Galley {
    /// Is this galley laid out with a right-to-left base reading direction?
    ///
    /// Every row of a galley shares the job's resolved base direction, so the
    /// first row decides.
    pub fn is_rtl(&self) -> bool {
        self.rows
            .first()
            .is_some_and(|row| row.base_direction == TextDirection::RightToLeft)
    }

    /// Step the cursor one character in the given *logical* direction:
    /// forward (toward the next character) or backward.
    fn step_one_character(&self, cursor: &CCursor, forward: bool) -> CCursor {
        if forward {
            CCursor {
                index: (cursor.index + 1).min(self.end().index),
                prefer_next_row: true, // default to this when navigating. It is more often useful to put cursor at the beginning of a row than at the end.
            }
        } else if cursor.index == CharIndex::ZERO {
            CCursor::default()
        } else {
            CCursor {
                index: cursor.index - 1,
                prefer_next_row: true, // default to this when navigating. It is more often useful to put cursor at the beginning of a row than at the end.
            }
        }
    }

    /// Step the cursor one *visual* character position, by the local bidi
    /// levels of the row under the cursor (the standard editor caret model
    /// for bidirectional text).
    ///
    /// Returns `None` when the historic base-mirrored step should decide
    /// instead: on rows where every glyph shares one embedding level whose
    /// parity equals the base direction (pure LTR, pure RTL, empty,
    /// elision-only — the byte-identity fast path), on row edges (crossing
    /// rows / clamping), and on any degenerate or out-of-range case.
    fn step_one_character_visually(&self, cursor: &CCursor, left: bool) -> Option<CCursor> {
        // Clamp the incoming index to the galley's end first: the historic
        // step clamped silently, and an out-of-range cursor with
        // `prefer_next_row: false` would otherwise trip the pre-existing
        // `debug_assert` in `layout_from_cursor` in debug builds. A no-op for
        // every in-range cursor.
        let cursor = CCursor {
            index: cursor.index.min(self.end().index),
            prefer_next_row: cursor.prefer_next_row,
        };
        let layout = self.layout_from_cursor(cursor);
        let row = self.rows.get(layout.row)?;
        if row.is_bidi_homogeneous_with_base() {
            return None;
        }
        let (boundary_of_column, column_of_boundary) = row.visual_caret_boundaries();
        let boundary = boundary_of_column.get(layout.column.0).copied().flatten()?;
        let target = if left {
            boundary.checked_sub(1)?
        } else {
            boundary.checked_add(1)?
        };
        let column = column_of_boundary.get(target).copied().flatten()?;
        // The row-local column becomes a global index through the same
        // per-row prefix accumulation `cursor_from_layout` uses:
        let mut index = CharIndex::ZERO;
        for placed in self.rows.iter().take(layout.row) {
            index += placed.char_count_including_newline().0;
        }
        Some(CCursor {
            index: index + column.0,
            prefer_next_row: true,
        })
    }

    /// Move the cursor one character toward the visual **left**.
    ///
    /// Steps by each embedded run's *local* bidi level: inside a maximal run
    /// of one embedding level the caret moves one visual position in the
    /// run's own direction, and crossing into the visually-adjacent run lands
    /// on the column of the shared visual boundary. On base-parity-matching
    /// uniform rows, at row edges and on any degenerate case this falls back
    /// to the historic step (under a right-to-left base direction the
    /// logically *next* character; under a left-to-right one the logically
    /// previous one).
    pub fn cursor_left_one_character(&self, cursor: &CCursor) -> CCursor {
        self.step_one_character_visually(cursor, true)
            .unwrap_or_else(|| self.step_one_character(cursor, self.is_rtl()))
    }

    /// Move the cursor one character toward the visual **right**.
    ///
    /// The mirror of [`Self::cursor_left_one_character`]: steps by the local
    /// bidi levels, falling back to the historic step (under a right-to-left
    /// base direction the logically *previous* character; under a
    /// left-to-right one the logically next one).
    pub fn cursor_right_one_character(&self, cursor: &CCursor) -> CCursor {
        self.step_one_character_visually(cursor, false)
            .unwrap_or_else(|| self.step_one_character(cursor, !self.is_rtl()))
    }

    pub fn clamp_cursor(&self, cursor: &CCursor) -> CCursor {
        self.cursor_from_layout(self.layout_from_cursor(*cursor))
    }

    pub fn cursor_up_one_row(
        &self,
        cursor: &CCursor,
        h_pos: Option<f32>,
    ) -> (CCursor, Option<f32>) {
        let layout_cursor = self.layout_from_cursor(*cursor);
        let h_pos = h_pos.unwrap_or_else(|| self.pos_from_layout_cursor(&layout_cursor).center().x);
        if layout_cursor.row == 0 {
            (CCursor::default(), None)
        } else {
            let new_row = layout_cursor.row - 1;

            let new_layout_cursor = {
                // keep same X coord
                // char_at is Row-relative, so subtract the row's position
                let column = self.rows[new_row].char_at(h_pos - self.rows[new_row].pos.x);
                LayoutCursor {
                    row: new_row,
                    column,
                }
            };
            (self.cursor_from_layout(new_layout_cursor), Some(h_pos))
        }
    }

    pub fn cursor_down_one_row(
        &self,
        cursor: &CCursor,
        h_pos: Option<f32>,
    ) -> (CCursor, Option<f32>) {
        let layout_cursor = self.layout_from_cursor(*cursor);
        let h_pos = h_pos.unwrap_or_else(|| self.pos_from_layout_cursor(&layout_cursor).center().x);
        if layout_cursor.row + 1 < self.rows.len() {
            let new_row = layout_cursor.row + 1;

            let new_layout_cursor = {
                // keep same X coord
                // char_at is Row-relative, so subtract the row's position
                let column = self.rows[new_row].char_at(h_pos - self.rows[new_row].pos.x);
                LayoutCursor {
                    row: new_row,
                    column,
                }
            };

            (self.cursor_from_layout(new_layout_cursor), Some(h_pos))
        } else {
            (self.end(), None)
        }
    }

    pub fn cursor_begin_of_row(&self, cursor: &CCursor) -> CCursor {
        let layout_cursor = self.layout_from_cursor(*cursor);
        self.cursor_from_layout(LayoutCursor {
            row: layout_cursor.row,
            column: CharIndex::ZERO,
        })
    }

    pub fn cursor_end_of_row(&self, cursor: &CCursor) -> CCursor {
        let layout_cursor = self.layout_from_cursor(*cursor);
        self.cursor_from_layout(LayoutCursor {
            row: layout_cursor.row,
            column: self.rows[layout_cursor.row].char_count_excluding_newline(),
        })
    }

    pub fn cursor_begin_of_paragraph(&self, cursor: &CCursor) -> CCursor {
        let mut layout_cursor = self.layout_from_cursor(*cursor);
        layout_cursor.column = CharIndex::ZERO;

        loop {
            let prev_row = layout_cursor
                .row
                .checked_sub(1)
                .and_then(|row| self.rows.get(row));

            let Some(prev_row) = prev_row else {
                // This is the first row
                break;
            };

            if prev_row.ends_with_newline {
                break;
            }

            layout_cursor.row -= 1;
        }

        self.cursor_from_layout(layout_cursor)
    }

    pub fn cursor_end_of_paragraph(&self, cursor: &CCursor) -> CCursor {
        let mut layout_cursor = self.layout_from_cursor(*cursor);
        loop {
            let row = &self.rows[layout_cursor.row];
            if row.ends_with_newline || layout_cursor.row == self.rows.len() - 1 {
                layout_cursor.column = row.char_count_excluding_newline();
                break;
            }

            layout_cursor.row += 1;
        }

        self.cursor_from_layout(layout_cursor)
    }
}

/// The outcome of one visual word step ([`Galley::step_one_word_visually`]).
pub enum WordStep {
    /// A word-edge landing within the row.
    Within(CCursor),
    /// No word edge in the stepping direction within the row: the caller
    /// continues with the whole-text logical word step — forward (toward the
    /// reading end) when `reading_forward`, backward otherwise — and clamps
    /// the result when it lands inside `row_range`.
    ///
    /// The natural walk exit (the caret sits at the row's visual edge) carries
    /// the exiting row's own caret columns, so a logical step landing back
    /// inside the row (it would move the caret visually *backward*,
    /// oscillating across the edge) clamps in place, while a landing on the
    /// next row's first column passes through as the legitimate reading-order
    /// crossing. An empty range means the caller's result always passes
    /// through unclamped: the pure left-to-right fast path and the degenerate
    /// exits must reproduce the historic behavior exactly.
    Exit {
        /// Which whole-text logical word step to continue with.
        reading_forward: bool,
        /// The clamp window for the logical step's result; empty means never
        /// clamp.
        row_range: Range<CharIndex>,
    },
}

/// Whether the char counts as a word character for word-edge stops:
/// alphanumeric or `'_'`, the same class the logical word walk uses.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Byte offset of the `index`-th char of `text`, saturating at the text's end
/// (always a char boundary: a char start, or the end).
fn byte_offset_from_char_index(text: &str, index: CharIndex) -> usize {
    if index.0 == 0 {
        return 0;
    }
    let mut remaining = index.0;
    for (byte_i, _) in text.char_indices() {
        if remaining == 0 {
            return byte_i;
        }
        remaining -= 1;
    }
    text.len()
}

/// Merge adjacent segments whose chars are all word chars into one segment:
/// the logical word walk never lands between two such segments (each is
/// skipped in favor of the run's far edge), so a run whose characters the
/// word-boundary iterator leaves unjoined — it defers scripts written without
/// spaces to dictionaries — steps as one word, which a line breaker's
/// dictionary ranges then subdivide. Segments separated by transparent
/// material (whitespace, punctuation, a dot) stay apart. Ascending and
/// non-overlapping in, ascending and non-overlapping out; a segment that is
/// not char-boundary aligned (a hostile breaker range) passes through
/// unmerged.
fn merge_adjacent_word_runs(row_text: &str, segments: Vec<Range<usize>>) -> Vec<Range<usize>> {
    let mut merged: Vec<Range<usize>> = Vec::with_capacity(segments.len());
    let mut previous_is_word_run = false;
    for segment in segments {
        let all_word = row_text
            .get(segment.clone())
            .is_some_and(|slice| slice.chars().all(is_word_char));
        let joins = all_word
            && previous_is_word_run
            && merged
                .last()
                .is_some_and(|previous| previous.end == segment.start);
        if joins && let Some(previous) = merged.last_mut() {
            previous.end = previous.end.max(segment.end);
        } else {
            merged.push(segment);
        }
        previous_is_word_run = all_word;
    }
    merged
}

/// Cut ascending, non-overlapping segments at the given byte positions: a
/// position strictly inside a segment splits it in two, and the pieces stay
/// ascending and non-overlapping.
fn cut_segments(segments: Vec<Range<usize>>, cuts: &[usize]) -> Vec<Range<usize>> {
    let mut result = Vec::with_capacity(segments.len());
    for segment in segments {
        let mut start = segment.start;
        for &cut in cuts {
            if start < cut && cut < segment.end {
                result.push(start..cut);
                start = cut;
            }
        }
        result.push(start..segment.end);
    }
    result
}

/// Row-local char ranges of ascending, non-overlapping byte segments, in one
/// walk over the row text. A segment whose endpoints are not char boundaries
/// (a hostile breaker range) contributes a degraded range and is dropped by
/// the caller's slice guard; nothing here panics.
fn char_ranges_of_segments(row_text: &str, segments: &[Range<usize>]) -> Vec<Range<usize>> {
    let mut result = Vec::with_capacity(segments.len());
    let mut byte_cursor = 0usize;
    let mut char_cursor = 0usize;
    for segment in segments {
        while byte_cursor < segment.start {
            let Some(c) = row_text.get(byte_cursor..).and_then(|s| s.chars().next()) else {
                break;
            };
            byte_cursor += c.len_utf8();
            char_cursor += 1;
        }
        let c0 = char_cursor;
        while byte_cursor < segment.end {
            let Some(c) = row_text.get(byte_cursor..).and_then(|s| s.chars().next()) else {
                break;
            };
            byte_cursor += c.len_utf8();
            char_cursor += 1;
        }
        if c0 < char_cursor {
            result.push(c0..char_cursor);
        }
    }
    result
}

/// ## Visual word stepping
impl Galley {
    /// The word segments of one row's text slice, as byte ranges into
    /// `row_text` (which spans `row_bytes`, a byte range of the job text):
    /// ascending and non-overlapping, each segment char-boundary aligned
    /// unless a custom breaker misbehaves, in which case callers must re-check
    /// before slicing.
    ///
    /// The segmentation composes the same way the layout consults the breaker
    /// for wrapping: UAX #29 words first, adjacent all-word segments merged
    /// (the logical word walk never lands between them — a run whose
    /// characters the iterator leaves unjoined, deferring to dictionaries,
    /// steps as one word), then a refinement at `'.'` chars
    /// (`www.example.com` navigates as three words — the dots stay
    /// segment-less), then — when a breaker is engaged — the breaker's
    /// dictionary words cut the still-unbroken runs they cover. Engaged
    /// exactly when the layout can engage it (a custom breaker installed, or
    /// the bundled dictionary compiled in and the text may contain such
    /// characters), minus the wrap-width half: navigation needs the boundaries
    /// whether or not the row wraps, and as a separate call site it never
    /// changes the layout's break candidates. Without the feature and without
    /// a custom breaker this consults the no-op breaker: empty ranges, zero
    /// cost.
    fn word_segments_of_row(&self, row_text: &str, row_bytes: Range<usize>) -> Vec<Range<usize>> {
        let base: Vec<Range<usize>> = row_text
            .split_word_bound_indices()
            .map(|(word_byte, word)| word_byte..word_byte + word.len())
            .collect();
        let merged = merge_adjacent_word_runs(row_text, base);

        // The '.' boundary refinement: `www.example.com` walks as three words
        // (the mac convention), so each segment splits at its '.' chars and
        // the dots stay segment-less (transparent, like whitespace).
        let mut segments: Vec<Range<usize>> = Vec::new();
        for word in merged {
            let Some(word_text) = row_text.get(word.clone()) else {
                continue; // not char-boundary aligned (hostile breaker): skip
            };
            let mut segment_start = word.start;
            for (char_byte, c) in word_text.char_indices() {
                if c == '.' {
                    if segment_start < word.start + char_byte {
                        segments.push(segment_start..word.start + char_byte);
                    }
                    segment_start = word.start + char_byte + c.len_utf8();
                }
            }
            if segment_start < word.end {
                segments.push(segment_start..word.end);
            }
        }

        let may_consult_breaker = self.job.wrap.line_breaker.is_some()
            || crate::text::text_layout::may_have_sea_text(&self.job.text);
        if may_consult_breaker {
            let breaker = self
                .job
                .wrap
                .line_breaker
                .clone()
                .unwrap_or_else(crate::text::line_breaker::default_breaker);
            let word_ranges = breaker.word_boundaries(&self.job.text);
            let mut cuts: Vec<usize> = Vec::new();
            for range in &word_ranges {
                // Intersect the job-global byte range with this row's byte
                // range; inverted or disjoint ranges drop out here.
                let start = range.start.max(row_bytes.start);
                let end = range.end.min(row_bytes.end);
                if start < end {
                    cuts.push(start - row_bytes.start);
                    cuts.push(end - row_bytes.start);
                }
            }
            cuts.sort_unstable();
            segments = cut_segments(segments, &cuts);
        }

        segments
    }

    /// Move the cursor one **word** toward the visual `left` edge (or, with
    /// `left == false`, the visual right edge), stepping between word-edge
    /// stops in the row's *visual* frame.
    ///
    /// Every maximal word segment of the row (whitespace- and
    /// punctuation-only runs are transparent) contributes two caret stops: at
    /// its visually-left and visually-right edge, in the row's visual boundary
    /// coordinate (see [`Row::visual_caret_boundaries`]). The key walks to the
    /// nearest stop strictly beyond the caret's own boundary in the stepping
    /// direction — for an odd-level word the two edges swap their logical
    /// columns, which the edge lookup handles uniformly. Row-local word
    /// segmentation comes from [`Self::word_segments_of_row`].
    ///
    /// When no stop lies beyond, the walk exits through the row's visual edge
    /// and the caller continues with the whole-text logical word step (see
    /// [`WordStep::Exit`]). On a pure left-to-right row the logical walk *is*
    /// the visual walk, so those rows and any degenerate caret exit straight
    /// back to the caller with no clamp window.
    ///
    /// Columns stay logical (0 = the reading start, anchored by the row's base
    /// direction); the incoming index is clamped to the galley's end, and any
    /// hostile index degrades to the [`WordStep::Exit`] passthrough.
    pub fn step_one_word_visually(&self, cursor: &CCursor, left: bool) -> WordStep {
        // Clamp the incoming index to the galley's end first: an out-of-range
        // cursor with `prefer_next_row: false` would otherwise trip the
        // pre-existing `debug_assert` in `layout_from_cursor` in debug builds.
        // A no-op for every in-range cursor.
        let cursor = CCursor {
            index: cursor.index.min(self.end().index),
            prefer_next_row: cursor.prefer_next_row,
        };
        let layout = self.layout_from_cursor(cursor);
        let Some(placed) = self.rows.get(layout.row) else {
            // No row to walk on: pass the caller's logical step through
            // unclamped.
            return WordStep::Exit {
                reading_forward: !left,
                row_range: CharIndex::ZERO..CharIndex::ZERO,
            };
        };
        let row = &placed.row;

        // Global char prefix of this row (the same per-row accumulation
        // `cursor_from_layout` uses).
        let mut prefix = 0usize;
        for before in self.rows.iter().take(layout.row) {
            prefix = prefix.saturating_add(before.char_count_including_newline().0);
        }
        let prefix = CharIndex(prefix);

        // A pure left-to-right row reads identically in both frames: hand the
        // step straight back to the caller's logical walk with no clamp
        // window, so its behavior stays exactly as it was.
        if row.is_ltr_homogeneous() {
            return WordStep::Exit {
                reading_forward: !left,
                row_range: prefix..prefix,
            };
        }

        let reading_forward = row.first_read_cell_is_odd() == left;
        let (boundary_of_column, column_of_boundary) = row.visual_caret_boundaries();

        let Some(current_k) = boundary_of_column.get(layout.column.0).copied().flatten() else {
            // The caret column has no visual boundary (a degenerate stream, or
            // an elided tail): pass the caller's logical step through
            // unclamped.
            return WordStep::Exit {
                reading_forward,
                row_range: prefix..prefix,
            };
        };

        // The row's own text: `char_count_excluding_newline` chars starting at
        // the global char prefix. Rows are logical slices of the job text
        // (their char counts sum to the text's, unless elided).
        let text = self.text();
        let row_char_count = placed.char_count_excluding_newline().0;
        let row_byte_start = byte_offset_from_char_index(text, prefix);
        let row_byte_end = byte_offset_from_char_index(text, prefix.saturating_add(row_char_count));
        let Some(row_text) = text.get(row_byte_start..row_byte_end) else {
            return WordStep::Exit {
                reading_forward,
                row_range: prefix..prefix,
            };
        };

        // Word segments, then the visual stops: each segment containing at
        // least one word char contributes its two visual edges. Whitespace and
        // punctuation-only segments are transparent — their edges belong to
        // the adjacent word segments.
        let segments = self.word_segments_of_row(row_text, row_byte_start..row_byte_end);
        let word_segments: Vec<Range<usize>> = segments
            .into_iter()
            .filter(|segment| {
                row_text
                    .get(segment.start..segment.end)
                    .is_some_and(|slice| slice.chars().any(is_word_char))
            })
            .collect();
        let char_ranges = char_ranges_of_segments(row_text, &word_segments);
        let mut left_stops: Vec<(usize, CharIndex)> = Vec::new();
        let mut right_stops: Vec<(usize, CharIndex)> = Vec::new();
        for char_range in &char_ranges {
            // One glyph slot per char is pinned for non-elided rows; a
            // mismatch (elided tail) degrades to a skipped stop.
            let (Some(k0), Some(k1)) = (
                boundary_of_column.get(char_range.start).copied().flatten(),
                boundary_of_column.get(char_range.end).copied().flatten(),
            ) else {
                continue;
            };
            if let Some(column) = column_of_boundary.get(k0.min(k1)).copied().flatten() {
                left_stops.push((k0.min(k1), column));
            }
            if let Some(column) = column_of_boundary.get(k0.max(k1)).copied().flatten() {
                right_stops.push((k0.max(k1), column));
            }
        }

        // The nearest stop strictly beyond the caret's own visual boundary.
        let target = if left {
            left_stops
                .iter()
                .filter(|(k, _)| *k < current_k)
                .max_by_key(|(k, _)| *k)
        } else {
            right_stops
                .iter()
                .filter(|(k, _)| *k > current_k)
                .min_by_key(|(k, _)| *k)
        };
        let Some((_, column)) = target else {
            // The walk leaves the row through its visual edge: continue with
            // the whole-text logical word step, clamped to this row's own
            // caret columns — the reading-end column included, so a historic
            // landing there clamps in place instead of moving the caret
            // visually backward across the edge.
            return WordStep::Exit {
                reading_forward,
                row_range: prefix..prefix.saturating_add(row_char_count.saturating_add(1)),
            };
        };
        WordStep::Within(CCursor {
            index: prefix + *column,
            prefer_next_row: left,
        })
    }
}
