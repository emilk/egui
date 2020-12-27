//! This is going to get complicated.
//!
//! To avoid confusion, we never use the word "line".
//! The `\n` character demarcates the split of text into "paragraphs".
//! Each paragraph is wrapped at some width onto one or more "rows".
//!
//! If this cursors sits right at the border of a wrapped row break (NOT paragraph break)
//! do we prefer the next row?
//! For instance, consider this single paragraph, word wrapped:
//! ``` text
//! Hello_
//! world!
//! ```
//!
//! The offset `6` is both the end of the first row
//! and the start of the second row.
//! The `prefer_next_row` selects which.

use crate::math::{pos2, NumExt, Rect, Vec2};

/// Character cursor
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct CCursor {
    /// Character offset (NOT byte offset!).
    pub index: usize,

    /// If this cursors sits right at the border of a wrapped row break (NOT paragraph break)
    /// do we prefer the next row?
    /// This is *almost* always what you want, *except* for when
    /// explicitly clicking the end of a row or pressing the end key.
    pub prefer_next_row: bool,
}

impl CCursor {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            prefer_next_row: false,
        }
    }
}

/// Two `CCursor`s are considered equal if they refer to the same character boundary,
/// even if one prefers the start of the next row.
impl PartialEq for CCursor {
    fn eq(&self, other: &CCursor) -> bool {
        self.index == other.index
    }
}

impl std::ops::Add<usize> for CCursor {
    type Output = CCursor;
    fn add(self, rhs: usize) -> Self::Output {
        CCursor {
            index: self.index.saturating_add(rhs),
            prefer_next_row: self.prefer_next_row,
        }
    }
}

impl std::ops::Sub<usize> for CCursor {
    type Output = CCursor;
    fn sub(self, rhs: usize) -> Self::Output {
        CCursor {
            index: self.index.saturating_sub(rhs),
            prefer_next_row: self.prefer_next_row,
        }
    }
}

/// Row Cursor
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RCursor {
    /// 0 is first row, and so on.
    /// Note that a single paragraph can span multiple rows.
    /// (a paragraph is text separated by `\n`).
    pub row: usize,

    /// Character based (NOT bytes).
    /// It is fine if this points to something beyond the end of the current row.
    /// When moving up/down it may again be within the next row.
    pub column: usize,
}

/// Paragraph Cursor
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PCursor {
    /// 0 is first paragraph, and so on.
    /// Note that a single paragraph can span multiple rows.
    /// (a paragraph is text separated by `\n`).
    pub paragraph: usize,

    /// Character based (NOT bytes).
    /// It is fine if this points to something beyond the end of the current paragraph.
    /// When moving up/down it may again be within the next paragraph.
    pub offset: usize,

    /// If this cursors sits right at the border of a wrapped row break (NOT paragraph break)
    /// do we prefer the next row?
    /// This is *almost* always what you want, *except* for when
    /// explicitly clicking the end of a row or pressing the end key.
    pub prefer_next_row: bool,
}

/// Two `PCursor`s are considered equal if they refer to the same character boundary,
/// even if one prefers the start of the next row.
impl PartialEq for PCursor {
    fn eq(&self, other: &PCursor) -> bool {
        self.paragraph == other.paragraph && self.offset == other.offset
    }
}

/// All different types of cursors together.
/// They all point to the same place, but in their own different ways.
/// pcursor/rcursor can also point to after the end of the paragraph/row.
/// Does not implement `PartialEq` because you must think which cursor should be equivalent.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Cursor {
    pub ccursor: CCursor,
    pub rcursor: RCursor,
    pub pcursor: PCursor,
}

/// A collection of text locked into place.
#[derive(Clone, Debug, Default)]
pub struct Galley {
    /// The full text, including any an all `\n`.
    pub text: String,

    /// Rows of text, from top to bottom.
    /// The number of chars in all rows sum up to text.chars().count().
    /// Note that each paragraph (pieces of text separated with `\n`)
    /// can be split up into multiple rows.
    pub rows: Vec<Row>,

    // Optimization: calculated once and reused.
    pub size: Vec2,
}

/// A typeset piece of text on a single row.
#[derive(Clone, Debug)]
pub struct Row {
    /// The start of each character, probably starting at zero.
    /// The last element is the end of the last character.
    /// This is never empty.
    /// Unit: points.
    ///
    /// `x_offsets.len() + (ends_with_newline as usize) == text.chars().count() + 1`
    pub x_offsets: Vec<f32>,

    /// Top of the row, offset within the Galley.
    /// Unit: points.
    pub y_min: f32,

    /// Bottom of the row, offset within the Galley.
    /// Unit: points.
    pub y_max: f32,

    /// If true, this `Row` came from a paragraph ending with a `\n`.
    /// The `\n` itself is omitted from `x_offsets`.
    /// A `\n` in the input text always creates a new `Row` below it,
    /// so that text that ends with `\n` has an empty `Row` last.
    /// This also implies that the last `Row` in a `Galley` always has `ends_with_newline == false`.
    pub ends_with_newline: bool,
}

impl Row {
    pub fn sanity_check(&self) {
        assert!(!self.x_offsets.is_empty());
    }

    /// Excludes the implicit `\n` after the `Row`, if any.
    pub fn char_count_excluding_newline(&self) -> usize {
        assert!(!self.x_offsets.is_empty());
        self.x_offsets.len() - 1
    }

    /// Includes the implicit `\n` after the `Row`, if any.
    pub fn char_count_including_newline(&self) -> usize {
        self.char_count_excluding_newline() + (self.ends_with_newline as usize)
    }

    pub fn min_x(&self) -> f32 {
        *self.x_offsets.first().unwrap()
    }

    pub fn max_x(&self) -> f32 {
        *self.x_offsets.last().unwrap()
    }

    pub fn height(&self) -> f32 {
        self.y_max - self.y_min
    }

    pub fn rect(&self) -> Rect {
        Rect::from_min_max(
            pos2(self.min_x(), self.y_min),
            pos2(self.max_x(), self.y_max),
        )
    }

    /// Closest char at the desired x coordinate.
    /// Returns something in the range `[0, char_count_excluding_newline()]`.
    pub fn char_at(&self, desired_x: f32) -> usize {
        for (i, char_x_bounds) in self.x_offsets.windows(2).enumerate() {
            let char_center_x = 0.5 * (char_x_bounds[0] + char_x_bounds[1]);
            if desired_x < char_center_x {
                return i;
            }
        }
        self.char_count_excluding_newline()
    }

    pub fn x_offset(&self, column: usize) -> f32 {
        self.x_offsets[column.min(self.x_offsets.len() - 1)]
    }
}

impl Galley {
    pub fn sanity_check(&self) {
        let mut char_count = 0;
        for row in &self.rows {
            row.sanity_check();
            char_count += row.char_count_including_newline();
        }
        assert_eq!(char_count, self.text.chars().count());
        if let Some(last_row) = self.rows.last() {
            debug_assert!(
                !last_row.ends_with_newline,
                "If the text ends with '\\n', there would be an empty row last.\n\
                Galley: {:#?}",
                self
            );
        }
    }
}

/// ## Physical positions
impl Galley {
    fn end_pos(&self) -> Rect {
        if let Some(row) = self.rows.last() {
            let x = row.max_x();
            Rect::from_min_max(pos2(x, row.y_min), pos2(x, row.y_max))
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
                        return Rect::from_min_max(pos2(x, row.y_min), pos2(x, row.y_max));
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
        self.pos_from_pcursor(cursor.pcursor) // The one TextEdit stores
    }

    /// Cursor at the given position within the galley
    pub fn cursor_from_pos(&self, pos: Vec2) -> Cursor {
        let mut best_y_dist = f32::INFINITY;
        let mut cursor = Cursor::default();

        let mut ccursor_index = 0;
        let mut pcursor_it = PCursor::default();

        for (row_nr, row) in self.rows.iter().enumerate() {
            let y_dist = (row.y_min - pos.y).abs().min((row.y_max - pos.y).abs());
            if y_dist < best_y_dist {
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
            debug_assert!(!last_row.ends_with_newline);
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
        debug_assert_eq!(ccursor_it, self.end().ccursor);
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

    // TODO: return identical cursor, or clamp?
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
                let column = if x > self.rows[new_row].max_x() {
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
                let column = if x > self.rows[new_row].max_x() {
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

// ----------------------------------------------------------------------------

#[test]
fn test_text_layout() {
    impl PartialEq for Cursor {
        fn eq(&self, other: &Cursor) -> bool {
            (self.ccursor, self.rcursor, self.pcursor)
                == (other.ccursor, other.rcursor, other.pcursor)
        }
    }

    use crate::paint::*;

    let pixels_per_point = 1.0;
    let fonts = Fonts::from_definitions(pixels_per_point, FontDefinitions::default());
    let font = &fonts[TextStyle::Monospace];

    let galley = font.layout_multiline("".to_owned(), 1024.0);
    assert_eq!(galley.rows.len(), 1);
    assert_eq!(galley.rows[0].ends_with_newline, false);
    assert_eq!(galley.rows[0].x_offsets, vec![0.0]);

    let galley = font.layout_multiline("\n".to_owned(), 1024.0);
    assert_eq!(galley.rows.len(), 2);
    assert_eq!(galley.rows[0].ends_with_newline, true);
    assert_eq!(galley.rows[1].ends_with_newline, false);
    assert_eq!(galley.rows[1].x_offsets, vec![0.0]);

    let galley = font.layout_multiline("\n\n".to_owned(), 1024.0);
    assert_eq!(galley.rows.len(), 3);
    assert_eq!(galley.rows[0].ends_with_newline, true);
    assert_eq!(galley.rows[1].ends_with_newline, true);
    assert_eq!(galley.rows[2].ends_with_newline, false);
    assert_eq!(galley.rows[2].x_offsets, vec![0.0]);

    let galley = font.layout_multiline(" ".to_owned(), 1024.0);
    assert_eq!(galley.rows.len(), 1);
    assert_eq!(galley.rows[0].ends_with_newline, false);

    let galley = font.layout_multiline("One row!".to_owned(), 1024.0);
    assert_eq!(galley.rows.len(), 1);
    assert_eq!(galley.rows[0].ends_with_newline, false);

    let galley = font.layout_multiline("First row!\n".to_owned(), 1024.0);
    assert_eq!(galley.rows.len(), 2);
    assert_eq!(galley.rows[0].ends_with_newline, true);
    assert_eq!(galley.rows[1].ends_with_newline, false);
    assert_eq!(galley.rows[1].x_offsets, vec![0.0]);

    let galley = font.layout_multiline("line\nbreak".to_owned(), 10.0);
    assert_eq!(galley.rows.len(), 2);
    assert_eq!(galley.rows[0].ends_with_newline, true);
    assert_eq!(galley.rows[1].ends_with_newline, false);

    // Test wrapping:
    let galley = font.layout_multiline("word wrap".to_owned(), 10.0);
    assert_eq!(galley.rows.len(), 2);
    assert_eq!(galley.rows[0].ends_with_newline, false);
    assert_eq!(galley.rows[1].ends_with_newline, false);

    {
        // Test wrapping:
        let galley = font.layout_multiline("word wrap.\nNew paragraph.".to_owned(), 10.0);
        assert_eq!(galley.rows.len(), 4);
        assert_eq!(galley.rows[0].ends_with_newline, false);
        assert_eq!(galley.rows[0].char_count_excluding_newline(), "word ".len());
        assert_eq!(galley.rows[0].char_count_including_newline(), "word ".len());
        assert_eq!(galley.rows[1].ends_with_newline, true);
        assert_eq!(galley.rows[1].char_count_excluding_newline(), "wrap.".len());
        assert_eq!(
            galley.rows[1].char_count_including_newline(),
            "wrap.\n".len()
        );
        assert_eq!(galley.rows[2].ends_with_newline, false);
        assert_eq!(galley.rows[3].ends_with_newline, false);

        let cursor = Cursor::default();
        assert_eq!(cursor, galley.from_ccursor(cursor.ccursor));
        assert_eq!(cursor, galley.from_rcursor(cursor.rcursor));
        assert_eq!(cursor, galley.from_pcursor(cursor.pcursor));

        let cursor = galley.end();
        assert_eq!(cursor, galley.from_ccursor(cursor.ccursor));
        assert_eq!(cursor, galley.from_rcursor(cursor.rcursor));
        assert_eq!(cursor, galley.from_pcursor(cursor.pcursor));
        assert_eq!(
            cursor,
            Cursor {
                ccursor: CCursor::new(25),
                rcursor: RCursor { row: 3, column: 10 },
                pcursor: PCursor {
                    paragraph: 1,
                    offset: 14,
                    prefer_next_row: false,
                }
            }
        );

        let cursor = galley.from_ccursor(CCursor::new(1));
        assert_eq!(cursor.rcursor, RCursor { row: 0, column: 1 });
        assert_eq!(
            cursor.pcursor,
            PCursor {
                paragraph: 0,
                offset: 1,
                prefer_next_row: false,
            }
        );
        assert_eq!(cursor, galley.from_ccursor(cursor.ccursor));
        assert_eq!(cursor, galley.from_rcursor(cursor.rcursor));
        assert_eq!(cursor, galley.from_pcursor(cursor.pcursor));

        let cursor = galley.from_pcursor(PCursor {
            paragraph: 1,
            offset: 2,
            prefer_next_row: false,
        });
        assert_eq!(cursor.rcursor, RCursor { row: 2, column: 2 });
        assert_eq!(cursor, galley.from_ccursor(cursor.ccursor));
        assert_eq!(cursor, galley.from_rcursor(cursor.rcursor));
        assert_eq!(cursor, galley.from_pcursor(cursor.pcursor));

        let cursor = galley.from_pcursor(PCursor {
            paragraph: 1,
            offset: 6,
            prefer_next_row: false,
        });
        assert_eq!(cursor.rcursor, RCursor { row: 3, column: 2 });
        assert_eq!(cursor, galley.from_ccursor(cursor.ccursor));
        assert_eq!(cursor, galley.from_rcursor(cursor.rcursor));
        assert_eq!(cursor, galley.from_pcursor(cursor.pcursor));

        // On the border between two rows within the same paragraph:
        let cursor = galley.from_rcursor(RCursor { row: 0, column: 5 });
        assert_eq!(
            cursor,
            Cursor {
                ccursor: CCursor::new(5),
                rcursor: RCursor { row: 0, column: 5 },
                pcursor: PCursor {
                    paragraph: 0,
                    offset: 5,
                    prefer_next_row: false,
                }
            }
        );
        assert_eq!(cursor, galley.from_rcursor(cursor.rcursor));

        let cursor = galley.from_rcursor(RCursor { row: 1, column: 0 });
        assert_eq!(
            cursor,
            Cursor {
                ccursor: CCursor::new(5),
                rcursor: RCursor { row: 1, column: 0 },
                pcursor: PCursor {
                    paragraph: 0,
                    offset: 5,
                    prefer_next_row: false,
                }
            }
        );
        assert_eq!(cursor, galley.from_rcursor(cursor.rcursor));
    }

    {
        // Test cursor movement:
        let galley = font.layout_multiline("word wrap.\nNew paragraph.".to_owned(), 10.0);
        assert_eq!(galley.rows.len(), 4);
        assert_eq!(galley.rows[0].ends_with_newline, false);
        assert_eq!(galley.rows[1].ends_with_newline, true);
        assert_eq!(galley.rows[2].ends_with_newline, false);
        assert_eq!(galley.rows[3].ends_with_newline, false);

        let cursor = Cursor::default();

        assert_eq!(galley.cursor_up_one_row(&cursor), cursor);
        assert_eq!(galley.cursor_begin_of_row(&cursor), cursor);

        assert_eq!(
            galley.cursor_end_of_row(&cursor),
            Cursor {
                ccursor: CCursor::new(5),
                rcursor: RCursor { row: 0, column: 5 },
                pcursor: PCursor {
                    paragraph: 0,
                    offset: 5,
                    prefer_next_row: false,
                }
            }
        );

        assert_eq!(
            galley.cursor_down_one_row(&cursor),
            Cursor {
                ccursor: CCursor::new(5),
                rcursor: RCursor { row: 1, column: 0 },
                pcursor: PCursor {
                    paragraph: 0,
                    offset: 5,
                    prefer_next_row: false,
                }
            }
        );

        let cursor = Cursor::default();
        assert_eq!(
            galley.cursor_down_one_row(&galley.cursor_down_one_row(&cursor)),
            Cursor {
                ccursor: CCursor::new(11),
                rcursor: RCursor { row: 2, column: 0 },
                pcursor: PCursor {
                    paragraph: 1,
                    offset: 0,
                    prefer_next_row: false,
                }
            }
        );

        let cursor = galley.end();
        assert_eq!(galley.cursor_down_one_row(&cursor), cursor);

        let cursor = galley.end();
        assert!(galley.cursor_up_one_row(&galley.end()) != cursor);

        assert_eq!(
            galley.cursor_up_one_row(&galley.end()),
            Cursor {
                ccursor: CCursor::new(15),
                rcursor: RCursor { row: 2, column: 10 },
                pcursor: PCursor {
                    paragraph: 1,
                    offset: 4,
                    prefer_next_row: false,
                }
            }
        );
    }
}
