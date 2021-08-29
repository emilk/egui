use std::ops::{Range, RangeInclusive};
use std::sync::Arc;

use super::{font::*, *};
use crate::Color32;
use emath::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct TextFormat {
    pub style: TextStyle,
    pub color: Color32,
    pub italics: bool,
    // TODO: underline, background, strikethrough, raised, lowered, …
}

impl Default for TextFormat {
    fn default() -> Self {
        Self {
            style: TextStyle::Body,
            color: Color32::GRAY,
            italics: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    /// Can be used for first row indentation.
    pub leading_space: f32,
    /// Range into the galley text
    pub byte_range: Range<usize>,
    pub format: TextFormat,
}

/// Temporary storage before line-wrapping.
#[derive(Default, Clone)]
struct Paragraph {
    /// Start of the next glyph to be added.
    pub cursor_x: f32,
    pub glyphs: Vec<Glyph>,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Glyph {
    pub chr: char,
    pub pos: Vec2,
    pub uv_rect: UvRect,
    /// Index into [`Galley::section`]. Decides color etc
    pub section_index: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Galley2 {
    /// The job that this galley is the result of.
    /// Contains the original string and style sections.
    pub job: Arc<LayoutJob>,

    /// Rows of text, from top to bottom.
    /// The number of chars in all rows sum up to text.chars().count().
    /// Note that each paragraph (pieces of text separated with `\n`)
    /// can be split up into multiple rows.
    pub rows: Vec<Row2>,

    /// Bounding size (min is always `[0,0]`)
    pub size: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row2 {
    // TODO: pre-tesselate each row into a `Mesh`.
    // Per-row, so we later can do per-row culling.
    // PROBLEM: we need to know texture size.
    // or we still do the UV normalization in `tesselator.rs`.
    /// One for each `char`.
    pub glyphs: Vec<Glyph>,

    // /// The start of each character, probably starting at zero.
    // /// The last element is the end of the last character.
    // /// This is never empty.
    // /// Unit: points.
    // ///
    // /// `x_offsets.len() + (ends_with_newline as usize) == text.chars().count() + 1`
    // pub x_offsets: Vec<f32>,
    /// Bounding rectangle
    pub rect: Rect,

    /// If true, this `Row` came from a paragraph ending with a `\n`.
    /// The `\n` itself is omitted from [`Self::glyphs`].
    /// A `\n` in the input text always creates a new `Row` below it,
    /// so that text that ends with `\n` has an empty `Row` last.
    /// This also implies that the last `Row` in a `Galley` always has `ends_with_newline == false`.
    pub ends_with_newline: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutJob {
    pub text: String, // TODO: Cow<'static, str>
    pub sections: Vec<Section>,

    /// Try to break text so that no row is wider than this.
    /// Set to [`f32::INFINITY`] to turn off wrapping.
    /// Note that `\n` always produces a new line.
    pub wrap_width: f32,

    /// The first row must be at least this high.
    /// This is in case we lay out text that is the continuation
    /// of some earlier text (sharing the same row),
    /// in which case this will be the height of the earlier text.
    /// In other cases, set this to `0.0`.
    pub first_row_min_height: f32,
}

impl Default for LayoutJob {
    fn default() -> Self {
        Self {
            text: Default::default(),
            sections: Default::default(),
            wrap_width: f32::INFINITY,
            first_row_min_height: 0.0,
        }
    }
}

impl LayoutJob {
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }
    pub fn append(&mut self, text: &str, leading_space: f32, format: TextFormat) {
        let start = self.text.len();
        self.text += text;
        let byte_range = start..self.text.len();
        self.sections.push(Section {
            leading_space,
            byte_range,
            format,
        });
    }
}

impl Glyph {
    pub fn max_x(&self) -> f32 {
        self.pos.x + self.uv_rect.size.x
    }
}

impl Galley2 {
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.job.is_empty()
    }

    pub fn text(&self) -> &str {
        &self.job.text
    }
}

// ----------------------------------------------------------------------------

pub fn layout(fonts: &Fonts, job: Arc<LayoutJob>) -> Galley2 {
    let mut paragraphs = vec![Paragraph::default()];
    for (section_index, section) in job.sections.iter().enumerate() {
        layout_section(
            fonts,
            &job.text,
            section_index as u32,
            section,
            &mut paragraphs,
        );
    }

    let rows = rows_from_paragraphs(paragraphs, job.wrap_width);

    galley_from_rows(fonts, job, rows)
}

fn layout_section(
    fonts: &Fonts,
    text: &str,
    section_index: u32,
    section: &Section,
    out_paragraphs: &mut Vec<Paragraph>,
) {
    let mut paragraph = out_paragraphs.last_mut().unwrap();

    let Section {
        leading_space,
        byte_range,
        format,
    } = section;

    paragraph.cursor_x += leading_space;

    let font = &fonts[format.style];
    let font_height = font.row_height();

    let mut last_glyph_id = None;

    for chr in text[byte_range.clone()].chars() {
        if chr == '\n' {
            out_paragraphs.push(Paragraph::default());
            paragraph = out_paragraphs.last_mut().unwrap();
        } else {
            let (font_impl, glyph_info) = font.glyph_info_and_font_impl(chr);
            if let Some(last_glyph_id) = last_glyph_id {
                paragraph.cursor_x += font_impl.pair_kerning(last_glyph_id, glyph_info.id)
            }

            paragraph.glyphs.push(Glyph {
                chr,
                pos: vec2(paragraph.cursor_x, font_height), // we use pos.y for height until the entire paragraph is done.
                uv_rect: glyph_info.uv_rect,
                section_index,
            });

            paragraph.cursor_x += glyph_info.advance_width;
            paragraph.cursor_x = font.round_to_pixel(paragraph.cursor_x);
            last_glyph_id = Some(glyph_info.id);
        }
    }
}

/// We ignore y at this stage
fn rect_from_x_range(x_range: RangeInclusive<f32>) -> Rect {
    Rect::from_x_y_ranges(x_range, f32::NAN..=f32::NAN)
}

fn rows_from_paragraphs(paragraphs: Vec<Paragraph>, wrap_width: f32) -> Vec<Row2> {
    let num_paragraphs = paragraphs.len();

    let mut rows = vec![];

    for (i, paragraph) in paragraphs.into_iter().enumerate() {
        let is_last_paragraph = (i + 1) == num_paragraphs;

        if paragraph.glyphs.is_empty() {
            rows.push(Row2 {
                glyphs: vec![],
                rect: rect_from_x_range(paragraph.cursor_x..=paragraph.cursor_x),
                ends_with_newline: !is_last_paragraph,
            });
        } else {
            let paragraph_max_x = paragraph.glyphs.last().unwrap().max_x();
            if paragraph_max_x <= wrap_width {
                // early-out optimization
                let paragraph_min_x = paragraph.glyphs[0].pos.x;
                rows.push(Row2 {
                    glyphs: paragraph.glyphs,
                    rect: rect_from_x_range(paragraph_min_x..=paragraph_max_x),
                    ends_with_newline: !is_last_paragraph,
                });
            } else {
                line_break(paragraph, wrap_width, &mut rows);
            }
        }
    }

    rows
}

fn line_break(paragraph: Paragraph, wrap_width: f32, out_rows: &mut Vec<Row2>) {
    // Keeps track of good places to insert row break if we exceed `wrap_width`.
    let mut row_break_candidates = RowBreakCandidates::default();

    let mut first_row_indentation = paragraph.glyphs[0].pos.x;
    let mut row_start_x = 0.0;
    let mut row_start_idx = 0;

    for (i, glyph) in paragraph.glyphs.iter().enumerate() {
        let potential_row_width = glyph.pos.x - row_start_x;

        if potential_row_width > wrap_width {
            if first_row_indentation > 0.0 && !row_break_candidates.has_word_boundary() {
                // Allow the first row to be completely empty, because we know there will be more space on the next row:
                assert_eq!(row_start_idx, 0);
                out_rows.push(Row2 {
                    glyphs: vec![],
                    rect: rect_from_x_range(first_row_indentation..=first_row_indentation),
                    ends_with_newline: false,
                });
                first_row_indentation = 0.0;
            } else if let Some(last_kept_index) = row_break_candidates.get() {
                let glyphs: Vec<Glyph> = paragraph.glyphs[row_start_idx..=last_kept_index]
                    .iter()
                    .copied()
                    .map(|mut glyph| {
                        glyph.pos.x -= row_start_x;
                        glyph
                    })
                    .collect();

                let paragraph_min_x = glyphs[0].pos.x;
                let paragraph_max_x = glyphs.last().unwrap().max_x();

                out_rows.push(Row2 {
                    glyphs,
                    rect: rect_from_x_range(paragraph_min_x..=paragraph_max_x),
                    ends_with_newline: false,
                });

                row_start_idx = last_kept_index + 1;
                row_start_x = paragraph.glyphs[row_start_idx].pos.x;
                row_break_candidates = Default::default();
            } else {
                // Can't break, so don't!
            }
        }

        row_break_candidates.add(i, glyph.chr);
    }

    if row_start_idx < paragraph.glyphs.len() {
        let glyphs: Vec<Glyph> = paragraph.glyphs[row_start_idx..]
            .iter()
            .copied()
            .map(|mut glyph| {
                glyph.pos.x -= row_start_x;
                glyph
            })
            .collect();

        let paragraph_min_x = glyphs[0].pos.x;
        let paragraph_max_x = glyphs.last().unwrap().max_x();

        out_rows.push(Row2 {
            glyphs,
            rect: rect_from_x_range(paragraph_min_x..=paragraph_max_x),
            ends_with_newline: false,
        });
    }
}

fn galley_from_rows(fonts: &Fonts, job: Arc<LayoutJob>, mut rows: Vec<Row2>) -> Galley2 {
    let mut first_row_min_height = job.first_row_min_height;
    let mut cursor_y = 0.0;
    let mut max_x: f32 = 0.0;
    for row in &mut rows {
        let mut row_height = first_row_min_height;
        first_row_min_height = 0.0;
        for glyph in &row.glyphs {
            // We store font heights in pos.y temporarily.
            row_height = row_height.max(glyph.pos.y);
        }

        // Now positions each glyph:
        for glyph in &mut row.glyphs {
            // Align down. TODO: adjustable with e.g. raised text
            glyph.pos.y = cursor_y + row_height - glyph.pos.y;
            glyph.pos.y = fonts.round_to_pixel(glyph.pos.y);
        }

        row.rect.min.y = cursor_y;
        row.rect.max.y = cursor_y + row_height;

        max_x = max_x.max(row.rect.right());
        cursor_y += row_height;
        cursor_y = fonts.round_to_pixel(cursor_y);
    }

    let size = vec2(max_x, cursor_y);

    Galley2 { job, rows, size }
}

// ----------------------------------------------------------------------------

/// Keeps track of good places to break a long row of text.
/// Will focus primarily on spaces, secondarily on things like `-`
#[derive(Clone, Copy, Default)]
struct RowBreakCandidates {
    /// Breaking at ` ` or other whitespace
    /// is always the primary candidate.
    space: Option<usize>,
    /// Logogram (single character representing a whole word) are good candidates for line break.
    logogram: Option<usize>,
    /// Breaking at a dash is super-
    /// good idea.
    dash: Option<usize>,
    /// This is nicer for things like URLs, e.g. www.
    /// example.com.
    punctuation: Option<usize>,
    /// Breaking after just random character is some
    /// times necessary.
    any: Option<usize>,
}

impl RowBreakCandidates {
    fn add(&mut self, index: usize, chr: char) {
        const NON_BREAKING_SPACE: char = '\u{A0}';
        if chr.is_whitespace() && chr != NON_BREAKING_SPACE {
            self.space = Some(index);
        } else if is_chinese(chr) {
            self.logogram = Some(index);
        } else if chr == '-' {
            self.dash = Some(index);
        } else if chr.is_ascii_punctuation() {
            self.punctuation = Some(index);
        } else {
            self.any = Some(index);
        }
    }

    fn has_word_boundary(&self) -> bool {
        self.space.is_some() || self.logogram.is_some()
    }

    fn get(&self) -> Option<usize> {
        self.space
            .or(self.logogram)
            .or(self.dash)
            .or(self.punctuation)
            .or(self.any)
    }
}

#[inline]
fn is_chinese(c: char) -> bool {
    ('\u{4E00}' <= c && c <= '\u{9FFF}')
        || ('\u{3400}' <= c && c <= '\u{4DBF}')
        || ('\u{2B740}' <= c && c <= '\u{2B81F}')
}
