#![allow(clippy::derive_hash_xor_eq)] // We need to impl Hash for f32, but we don't implement Eq, which is fine

use std::ops::Range;
use std::sync::Arc;

use super::{cursor::*, font::UvRect};
use crate::{Color32, FontId, Mesh, Stroke};
use emath::*;

/// Describes the task of laying out text.
///
/// This supports mixing different fonts, color and formats (underline etc).
///
/// Pass this to [`crate::Fonts::layout_job`] or [`crate::text::layout`].
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
    pub sections: Vec<LayoutSection>,

    pub wrap: TextWrapping,

    /// The first row must be at least this high.
    /// This is in case we lay out text that is the continuation
    /// of some earlier text (sharing the same row),
    /// in which case this will be the height of the earlier text.
    /// In other cases, set this to `0.0`.
    pub first_row_min_height: f32,

    /// If `false`, all newlines characters will be ignored
    /// and show up as the replacement character.
    /// Default: `true`.
    pub break_on_newline: bool,

    /// How to horizontally align the text (`Align::LEFT`, `Align::Center`, `Align::RIGHT`).
    pub halign: Align,

    /// Justify text so that word-wrapped rows fill the whole [`TextWrapping::max_width`]
    pub justify: bool,
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
        }
    }
}

impl LayoutJob {
    /// Break on `\n` and at the given wrap width.
    #[inline]
    pub fn simple(text: String, font_id: FontId, color: Color32, wrap_width: f32) -> Self {
        Self {
            sections: vec![LayoutSection {
                leading_space: 0.0,
                byte_range: 0..text.len(),
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

    /// Does not break on `\n`, but shows the replacement character instead.
    #[inline]
    pub fn simple_singleline(text: String, font_id: FontId, color: Color32) -> Self {
        Self {
            sections: vec![LayoutSection {
                leading_space: 0.0,
                byte_range: 0..text.len(),
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
                byte_range: 0..text.len(),
                format,
            }],
            text,
            wrap: Default::default(),
            break_on_newline: true,
            ..Default::default()
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    /// Helper for adding a new section when building a [`LayoutJob`].
    pub fn append(&mut self, text: &str, leading_space: f32, format: TextFormat) {
        let start = self.text.len();
        self.text += text;
        let byte_range = start..self.text.len();
        self.sections.push(LayoutSection {
            leading_space,
            byte_range,
            format,
        });
    }

    /// The height of the tallest used font in the job.
    pub fn font_height(&self, fonts: &crate::Fonts) -> f32 {
        let mut max_height = 0.0_f32;
        for section in &self.sections {
            max_height = max_height.max(fonts.row_height(&section.format.font_id));
        }
        max_height
    }
}

impl std::hash::Hash for LayoutJob {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self {
            text,
            sections,
            wrap,
            first_row_min_height,
            break_on_newline,
            halign,
            justify,
        } = self;

        text.hash(state);
        sections.hash(state);
        wrap.hash(state);
        crate::f32_hash(state, *first_row_min_height);
        break_on_newline.hash(state);
        halign.hash(state);
        justify.hash(state);
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct LayoutSection {
    /// Can be used for first row indentation.
    pub leading_space: f32,
    /// Range into the galley text
    pub byte_range: Range<usize>,
    pub format: TextFormat,
}

impl std::hash::Hash for LayoutSection {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self {
            leading_space,
            byte_range,
            format,
        } = self;
        crate::f32_hash(state, *leading_space);
        byte_range.hash(state);
        format.hash(state);
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Debug, Hash, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TextFormat {
    pub font_id: FontId,
    /// Text color
    pub color: Color32,
    pub background: Color32,
    pub italics: bool,
    pub underline: Stroke,
    pub strikethrough: Stroke,
    /// If you use a small font and [`Align::TOP`] you
    /// can get the effect of raised text.
    pub valign: Align,
    // TODO(emilk): lowered
}

impl Default for TextFormat {
    #[inline]
    fn default() -> Self {
        Self {
            font_id: FontId::default(),
            color: Color32::GRAY,
            background: Color32::TRANSPARENT,
            italics: false,
            underline: Stroke::NONE,
            strikethrough: Stroke::NONE,
            valign: Align::BOTTOM,
        }
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

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TextWrapping {
    /// Try to break text so that no row is wider than this.
    /// Set to [`f32::INFINITY`] to turn off wrapping.
    /// Note that `\n` always produces a new line.
    pub max_width: f32,

    /// Maximum amount of rows the text should have.
    /// Set to `0` to disable this.
    pub max_rows: usize,

    /// Don't try to break text at an appropriate place.
    pub break_anywhere: bool,

    /// Character to use to represent clipped text, `…` for example, which is the default.
    pub overflow_character: Option<char>,
}

impl std::hash::Hash for TextWrapping {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self {
            max_width,
            max_rows,
            break_anywhere,
            overflow_character,
        } = self;
        crate::f32_hash(state, *max_width);
        max_rows.hash(state);
        break_anywhere.hash(state);
        overflow_character.hash(state);
    }
}

impl Default for TextWrapping {
    fn default() -> Self {
        Self {
            max_width: f32::INFINITY,
            max_rows: 0,
            break_anywhere: false,
            overflow_character: Some('…'),
        }
    }
}

// ----------------------------------------------------------------------------

/// Text that has been laid out, ready for painting.
///
/// You can create a [`Galley`] using [`crate::Fonts::layout_job`];
///
/// This needs to be recreated if `pixels_per_point` (dpi scale) changes.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Galley {
    /// The job that this galley is the result of.
    /// Contains the original string and style sections.
    pub job: Arc<LayoutJob>,

    /// Rows of text, from top to bottom.
    /// The number of characters in all rows sum up to `job.text.chars().count()`.
    /// Note that each paragraph (pieces of text separated with `\n`)
    /// can be split up into multiple rows.
    pub rows: Vec<Row>,

    /// Bounding rect.
    ///
    /// `rect.top()` is always 0.0.
    ///
    /// With [`LayoutJob::halign`]:
    /// * [`Align::LEFT`]: rect.left() == 0.0
    /// * [`Align::Center`]: rect.center() == 0.0
    /// * [`Align::RIGHT`]: rect.right() == 0.0
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Row {
    /// One for each `char`.
    pub glyphs: Vec<Glyph>,

    /// Logical bounding rectangle based on font heights etc.
    /// Use this when drawing a selection or similar!
    /// Includes leading and trailing whitespace.
    pub rect: Rect,

    /// The mesh, ready to be rendered.
    pub visuals: RowVisuals,

    /// If true, this [`Row`] came from a paragraph ending with a `\n`.
    /// The `\n` itself is omitted from [`Self::glyphs`].
    /// A `\n` in the input text always creates a new [`Row`] below it,
    /// so that text that ends with `\n` has an empty [`Row`] last.
    /// This also implies that the last [`Row`] in a [`Galley`] always has `ends_with_newline == false`.
    pub ends_with_newline: bool,
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

    /// The range of vertices in the mesh the contain glyphs.
    /// Before comes backgrounds (if any), and after any underlines and strikethrough.
    pub glyph_vertex_range: Range<usize>,
}

impl Default for RowVisuals {
    fn default() -> Self {
        Self {
            mesh: Default::default(),
            mesh_bounds: Rect::NOTHING,
            glyph_vertex_range: 0..0,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Glyph {
    /// The character this glyph represents.
    pub chr: char,

    /// Relative to the galley position.
    /// Logical position: pos.y is the same for all chars of the same [`TextFormat`].
    pub pos: Pos2,

    /// Advance width and font row height.
    pub size: Vec2,

    /// Position of the glyph in the font texture, in texels.
    pub uv_rect: UvRect,

    /// Index into [`LayoutJob::sections`]. Decides color etc.
    pub section_index: u32,
}

impl Glyph {
    pub fn max_x(&self) -> f32 {
        self.pos.x + self.size.x
    }

    /// Same y range for all characters with the same [`TextFormat`].
    #[inline]
    pub fn logical_rect(&self) -> Rect {
        Rect::from_min_size(self.pos, self.size)
    }
}

// ----------------------------------------------------------------------------

impl Row {
    /// Excludes the implicit `\n` after the [`Row`], if any.
    #[inline]
    pub fn char_count_excluding_newline(&self) -> usize {
        self.glyphs.len()
    }

    /// Includes the implicit `\n` after the [`Row`], if any.
    #[inline]
    pub fn char_count_including_newline(&self) -> usize {
        self.glyphs.len() + (self.ends_with_newline as usize)
    }

    #[inline]
    pub fn min_y(&self) -> f32 {
        self.rect.top()
    }

    #[inline]
    pub fn max_y(&self) -> f32 {
        self.rect.bottom()
    }

    #[inline]
    pub fn height(&self) -> f32 {
        self.rect.height()
    }

    /// Closest char at the desired x coordinate.
    /// Returns something in the range `[0, char_count_excluding_newline()]`.
    pub fn char_at(&self, desired_x: f32) -> usize {
        for (i, glyph) in self.glyphs.iter().enumerate() {
            if desired_x < glyph.logical_rect().center().x {
                return i;
            }
        }
        self.char_count_excluding_newline()
    }

    pub fn x_offset(&self, column: usize) -> f32 {
        if let Some(glyph) = self.glyphs.get(column) {
            glyph.pos.x
        } else {
            self.rect.right()
        }
    }
}

impl Galley {
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.job.is_empty()
    }

    #[inline(always)]
    pub fn text(&self) -> &str {
        &self.job.text
    }

    pub fn size(&self) -> Vec2 {
        self.rect.size()
    }
}

// ----------------------------------------------------------------------------

/// ## Physical positions
impl Galley {
    /// Zero-width rect past the last character.
    fn end_pos(&self) -> Rect {
        if let Some(row) = self.rows.last() {
            let x = row.rect.right();
            Rect::from_min_max(pos2(x, row.min_y()), pos2(x, row.max_y()))
        } else {
            // Empty galley
            Rect::from_min_max(pos2(0.0, 0.0), pos2(0.0, 0.0))
        }
    }

    /// Returns a 0-width Rect.
    pub fn pos_from_pcursor(&self, pcursor: PCursor) -> Rect {
        let mut it = PCursor::default();

        for row in &self.rows {
            if it.paragraph == pcursor.paragraph {
                // Right paragraph, but is it the right row in the paragraph?

                if it.offset <= pcursor.offset
                    && (pcursor.offset <= it.offset + row.char_count_excluding_newline()
                        || row.ends_with_newline)
                {
                    let column = pcursor.offset - it.offset;

                    let select_next_row_instead = pcursor.prefer_next_row
                        && !row.ends_with_newline
                        && column >= row.char_count_excluding_newline();
                    if !select_next_row_instead {
                        let x = row.x_offset(column);
                        return Rect::from_min_max(pos2(x, row.min_y()), pos2(x, row.max_y()));
                    }
                }
            }

            if row.ends_with_newline {
                it.paragraph += 1;
                it.offset = 0;
            } else {
                it.offset += row.char_count_including_newline();
            }
        }

        self.end_pos()
    }

    /// Returns a 0-width Rect.
    pub fn pos_from_cursor(&self, cursor: &Cursor) -> Rect {
        self.pos_from_pcursor(cursor.pcursor) // pcursor is what TextEdit stores
    }

    /// Cursor at the given position within the galley
    pub fn cursor_from_pos(&self, pos: Vec2) -> Cursor {
        let mut best_y_dist = f32::INFINITY;
        let mut cursor = Cursor::default();

        let mut ccursor_index = 0;
        let mut pcursor_it = PCursor::default();

        for (row_nr, row) in self.rows.iter().enumerate() {
            let is_pos_within_row = pos.y >= row.min_y() && pos.y <= row.max_y();
            let y_dist = (row.min_y() - pos.y).abs().min((row.max_y() - pos.y).abs());
            if is_pos_within_row || y_dist < best_y_dist {
                best_y_dist = y_dist;
                let column = row.char_at(pos.x);
                let prefer_next_row = column < row.char_count_excluding_newline();
                cursor = Cursor {
                    ccursor: CCursor {
                        index: ccursor_index + column,
                        prefer_next_row,
                    },
                    rcursor: RCursor {
                        row: row_nr,
                        column,
                    },
                    pcursor: PCursor {
                        paragraph: pcursor_it.paragraph,
                        offset: pcursor_it.offset + column,
                        prefer_next_row,
                    },
                };

                if is_pos_within_row {
                    return cursor;
                }
            }
            ccursor_index += row.char_count_including_newline();
            if row.ends_with_newline {
                pcursor_it.paragraph += 1;
                pcursor_it.offset = 0;
            } else {
                pcursor_it.offset += row.char_count_including_newline();
            }
        }
        cursor
    }
}

/// ## Cursor positions
impl Galley {
    /// Cursor to one-past last character.
    pub fn end(&self) -> Cursor {
        if self.rows.is_empty() {
            return Default::default();
        }
        let mut ccursor = CCursor {
            index: 0,
            prefer_next_row: true,
        };
        let mut pcursor = PCursor {
            paragraph: 0,
            offset: 0,
            prefer_next_row: true,
        };
        for row in &self.rows {
            let row_char_count = row.char_count_including_newline();
            ccursor.index += row_char_count;
            if row.ends_with_newline {
                pcursor.paragraph += 1;
                pcursor.offset = 0;
            } else {
                pcursor.offset += row_char_count;
            }
        }
        Cursor {
            ccursor,
            rcursor: self.end_rcursor(),
            pcursor,
        }
    }

    pub fn end_rcursor(&self) -> RCursor {
        if let Some(last_row) = self.rows.last() {
            crate::epaint_assert!(!last_row.ends_with_newline);
            RCursor {
                row: self.rows.len() - 1,
                column: last_row.char_count_excluding_newline(),
            }
        } else {
            Default::default()
        }
    }
}

/// ## Cursor conversions
impl Galley {
    // The returned cursor is clamped.
    pub fn from_ccursor(&self, ccursor: CCursor) -> Cursor {
        let prefer_next_row = ccursor.prefer_next_row;
        let mut ccursor_it = CCursor {
            index: 0,
            prefer_next_row,
        };
        let mut pcursor_it = PCursor {
            paragraph: 0,
            offset: 0,
            prefer_next_row,
        };

        for (row_nr, row) in self.rows.iter().enumerate() {
            let row_char_count = row.char_count_excluding_newline();

            if ccursor_it.index <= ccursor.index
                && ccursor.index <= ccursor_it.index + row_char_count
            {
                let column = ccursor.index - ccursor_it.index;

                let select_next_row_instead = prefer_next_row
                    && !row.ends_with_newline
                    && column >= row.char_count_excluding_newline();
                if !select_next_row_instead {
                    pcursor_it.offset += column;
                    return Cursor {
                        ccursor,
                        rcursor: RCursor {
                            row: row_nr,
                            column,
                        },
                        pcursor: pcursor_it,
                    };
                }
            }
            ccursor_it.index += row.char_count_including_newline();
            if row.ends_with_newline {
                pcursor_it.paragraph += 1;
                pcursor_it.offset = 0;
            } else {
                pcursor_it.offset += row.char_count_including_newline();
            }
        }
        crate::epaint_assert!(ccursor_it == self.end().ccursor);
        Cursor {
            ccursor: ccursor_it, // clamp
            rcursor: self.end_rcursor(),
            pcursor: pcursor_it,
        }
    }

    pub fn from_rcursor(&self, rcursor: RCursor) -> Cursor {
        if rcursor.row >= self.rows.len() {
            return self.end();
        }

        let prefer_next_row =
            rcursor.column < self.rows[rcursor.row].char_count_excluding_newline();
        let mut ccursor_it = CCursor {
            index: 0,
            prefer_next_row,
        };
        let mut pcursor_it = PCursor {
            paragraph: 0,
            offset: 0,
            prefer_next_row,
        };

        for (row_nr, row) in self.rows.iter().enumerate() {
            if row_nr == rcursor.row {
                ccursor_it.index += rcursor.column.at_most(row.char_count_excluding_newline());

                if row.ends_with_newline {
                    // Allow offset to go beyond the end of the paragraph
                    pcursor_it.offset += rcursor.column;
                } else {
                    pcursor_it.offset += rcursor.column.at_most(row.char_count_excluding_newline());
                }
                return Cursor {
                    ccursor: ccursor_it,
                    rcursor,
                    pcursor: pcursor_it,
                };
            }
            ccursor_it.index += row.char_count_including_newline();
            if row.ends_with_newline {
                pcursor_it.paragraph += 1;
                pcursor_it.offset = 0;
            } else {
                pcursor_it.offset += row.char_count_including_newline();
            }
        }
        Cursor {
            ccursor: ccursor_it,
            rcursor: self.end_rcursor(),
            pcursor: pcursor_it,
        }
    }

    // TODO(emilk): return identical cursor, or clamp?
    pub fn from_pcursor(&self, pcursor: PCursor) -> Cursor {
        let prefer_next_row = pcursor.prefer_next_row;
        let mut ccursor_it = CCursor {
            index: 0,
            prefer_next_row,
        };
        let mut pcursor_it = PCursor {
            paragraph: 0,
            offset: 0,
            prefer_next_row,
        };

        for (row_nr, row) in self.rows.iter().enumerate() {
            if pcursor_it.paragraph == pcursor.paragraph {
                // Right paragraph, but is it the right row in the paragraph?

                if pcursor_it.offset <= pcursor.offset
                    && (pcursor.offset <= pcursor_it.offset + row.char_count_excluding_newline()
                        || row.ends_with_newline)
                {
                    let column = pcursor.offset - pcursor_it.offset;

                    let select_next_row_instead = pcursor.prefer_next_row
                        && !row.ends_with_newline
                        && column >= row.char_count_excluding_newline();

                    if !select_next_row_instead {
                        ccursor_it.index += column.at_most(row.char_count_excluding_newline());

                        return Cursor {
                            ccursor: ccursor_it,
                            rcursor: RCursor {
                                row: row_nr,
                                column,
                            },
                            pcursor,
                        };
                    }
                }
            }

            ccursor_it.index += row.char_count_including_newline();
            if row.ends_with_newline {
                pcursor_it.paragraph += 1;
                pcursor_it.offset = 0;
            } else {
                pcursor_it.offset += row.char_count_including_newline();
            }
        }
        Cursor {
            ccursor: ccursor_it,
            rcursor: self.end_rcursor(),
            pcursor,
        }
    }
}

/// ## Cursor positions
impl Galley {
    pub fn cursor_left_one_character(&self, cursor: &Cursor) -> Cursor {
        if cursor.ccursor.index == 0 {
            Default::default()
        } else {
            let ccursor = CCursor {
                index: cursor.ccursor.index,
                prefer_next_row: true, // default to this when navigating. It is more often useful to put cursor at the begging of a row than at the end.
            };
            self.from_ccursor(ccursor - 1)
        }
    }

    pub fn cursor_right_one_character(&self, cursor: &Cursor) -> Cursor {
        let ccursor = CCursor {
            index: cursor.ccursor.index,
            prefer_next_row: true, // default to this when navigating. It is more often useful to put cursor at the begging of a row than at the end.
        };
        self.from_ccursor(ccursor + 1)
    }

    pub fn cursor_up_one_row(&self, cursor: &Cursor) -> Cursor {
        if cursor.rcursor.row == 0 {
            Cursor::default()
        } else {
            let new_row = cursor.rcursor.row - 1;

            let cursor_is_beyond_end_of_current_row = cursor.rcursor.column
                >= self.rows[cursor.rcursor.row].char_count_excluding_newline();

            let new_rcursor = if cursor_is_beyond_end_of_current_row {
                // keep same column
                RCursor {
                    row: new_row,
                    column: cursor.rcursor.column,
                }
            } else {
                // keep same X coord
                let x = self.pos_from_cursor(cursor).center().x;
                let column = if x > self.rows[new_row].rect.right() {
                    // beyond the end of this row - keep same colum
                    cursor.rcursor.column
                } else {
                    self.rows[new_row].char_at(x)
                };
                RCursor {
                    row: new_row,
                    column,
                }
            };
            self.from_rcursor(new_rcursor)
        }
    }

    pub fn cursor_down_one_row(&self, cursor: &Cursor) -> Cursor {
        if cursor.rcursor.row + 1 < self.rows.len() {
            let new_row = cursor.rcursor.row + 1;

            let cursor_is_beyond_end_of_current_row = cursor.rcursor.column
                >= self.rows[cursor.rcursor.row].char_count_excluding_newline();

            let new_rcursor = if cursor_is_beyond_end_of_current_row {
                // keep same column
                RCursor {
                    row: new_row,
                    column: cursor.rcursor.column,
                }
            } else {
                // keep same X coord
                let x = self.pos_from_cursor(cursor).center().x;
                let column = if x > self.rows[new_row].rect.right() {
                    // beyond the end of the next row - keep same column
                    cursor.rcursor.column
                } else {
                    self.rows[new_row].char_at(x)
                };
                RCursor {
                    row: new_row,
                    column,
                }
            };

            self.from_rcursor(new_rcursor)
        } else {
            self.end()
        }
    }

    pub fn cursor_begin_of_row(&self, cursor: &Cursor) -> Cursor {
        self.from_rcursor(RCursor {
            row: cursor.rcursor.row,
            column: 0,
        })
    }

    pub fn cursor_end_of_row(&self, cursor: &Cursor) -> Cursor {
        self.from_rcursor(RCursor {
            row: cursor.rcursor.row,
            column: self.rows[cursor.rcursor.row].char_count_excluding_newline(),
        })
    }
}
