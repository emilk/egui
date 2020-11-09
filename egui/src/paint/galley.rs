use crate::math::{vec2, NumExt, Vec2};

/// Character cursor
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct CCursor {
    /// Character offset (NOT byte offset!).
    pub index: usize,

    /// If this cursors sits right at the border of a wrapped line (NOT `\n`),
    /// do we prefer the next line?
    /// For instance, consider this text, word wrapped:
    /// ``` text
    /// Hello_
    /// world!
    /// ```
    ///
    /// The offset `6` is both the end of the first line
    /// and the start of the second line.
    /// The `prefer_next_line` selects which.
    pub prefer_next_line: bool,
}

impl CCursor {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            prefer_next_line: false,
        }
    }
}

/// Two `CCursor`s are considered equal if they refer to the same character boundary,
/// even if one prefers the start of the next line.
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
            prefer_next_line: self.prefer_next_line,
        }
    }
}

impl std::ops::Sub<usize> for CCursor {
    type Output = CCursor;
    fn sub(self, rhs: usize) -> Self::Output {
        CCursor {
            index: self.index.saturating_sub(rhs),
            prefer_next_line: self.prefer_next_line,
        }
    }
}

/// Line Cursor
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct LCursor {
    /// 0 is first line, and so on.
    /// Note that a single paragraph can span multiple lines.
    /// (a paragraph is text separated by `\n`).
    pub line: usize,

    /// Character based (NOT bytes).
    /// It is fine if this points to something beyond the end of the current line.
    /// When moving up/down it may again be within the next line.
    pub column: usize,
}

/// Paragraph Cursor
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PCursor {
    /// 0 is first paragraph, and so on.
    /// Note that a single paragraph can span multiple lines.
    /// (a paragraph is text separated by `\n`).
    pub paragraph: usize,

    /// Character based (NOT bytes).
    /// It is fine if this points to something beyond the end of the current line.
    /// When moving up/down it may again be within the next line.
    pub offset: usize,

    /// If this cursors sits right at the border of a wrapped line (NOT `\n`),
    /// do we prefer the next line?
    /// For instance, consider this text, word wrapped:
    /// ``` text
    /// Hello_
    /// world!
    /// ```
    ///
    /// The offset `6` is both the end of the first line
    /// and the start of the second line.
    /// The `prefer_next_line` selects which.
    pub prefer_next_line: bool,
}

/// Two `PCursor`s are considered equal if they refer to the same character boundary,
/// even if one prefers the start of the next line.
impl PartialEq for PCursor {
    fn eq(&self, other: &PCursor) -> bool {
        self.paragraph == other.paragraph && self.offset == other.offset
    }
}

/// All different types of cursors together.
/// They all point to the same place, but in their own different ways.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Cursor {
    pub ccursor: CCursor,
    pub lcursor: LCursor,
    pub pcursor: PCursor,
}

/// A collection of text locked into place.
#[derive(Clone, Debug, Default)]
pub struct Galley {
    /// The full text, including any an all `\n`.
    pub text: String,

    /// Lines of text, from top to bottom.
    /// The number of chars in all lines sum up to text.chars().count().
    /// Note that each paragraph (pieces of text separated with `\n`)
    /// can be split up into multiple lines.
    pub lines: Vec<Line>,

    // Optimization: calculated once and reused.
    pub size: Vec2,
}

// TODO: should this maybe be renamed `Row` to avoid confusion with lines as 'that which is broken by \n'.
/// A typeset piece of text on a single line.
#[derive(Clone, Debug)]
pub struct Line {
    /// The start of each character, probably starting at zero.
    /// The last element is the end of the last character.
    /// This is never empty.
    /// Unit: points.
    ///
    /// `x_offsets.len() + (ends_with_newline as usize) == text.chars().count() + 1`
    pub x_offsets: Vec<f32>,

    /// Top of the line, offset within the Galley.
    /// Unit: points.
    pub y_min: f32,

    /// Bottom of the line, offset within the Galley.
    /// Unit: points.
    pub y_max: f32,

    /// If true, this Line came from a paragraph ending with a `\n`.
    /// The `\n` itself is omitted from `x_offsets`.
    /// A `\n` in the input text always creates a new `Line` below it,
    /// so that text that ends with `\n` has an empty `Line` last.
    /// This also implies that the last `Line` in a `Galley` always has `ends_with_newline == false`.
    pub ends_with_newline: bool,
}

impl Line {
    pub fn sanity_check(&self) {
        assert!(!self.x_offsets.is_empty());
    }

    /// Excludes the implicit `\n` after the `Line`, if any.
    pub fn char_count_excluding_newline(&self) -> usize {
        assert!(!self.x_offsets.is_empty());
        self.x_offsets.len() - 1
    }

    /// Includes the implicit `\n` after the `Line`, if any.
    pub fn char_count_including_newline(&self) -> usize {
        self.char_count_excluding_newline() + (self.ends_with_newline as usize)
    }

    pub fn min_x(&self) -> f32 {
        *self.x_offsets.first().unwrap()
    }

    pub fn max_x(&self) -> f32 {
        *self.x_offsets.last().unwrap()
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
}

impl Galley {
    pub fn sanity_check(&self) {
        let mut char_count = 0;
        for line in &self.lines {
            line.sanity_check();
            char_count += line.char_count_including_newline();
        }
        assert_eq!(char_count, self.text.chars().count());
        if let Some(last_line) = self.lines.last() {
            debug_assert!(
                !last_line.ends_with_newline,
                "If the text ends with '\\n', there would be an empty Line last.\n\
                Galley: {:#?}",
                self
            );
        }
    }
}

/// ## Physical positions
impl Galley {
    pub fn last_pos(&self) -> Vec2 {
        if let Some(last) = self.lines.last() {
            vec2(last.max_x(), last.y_min)
        } else {
            vec2(0.0, 0.0) // Empty galley
        }
    }

    pub fn pos_from_pcursor(&self, pcursor: PCursor) -> Vec2 {
        let mut it = PCursor::default();

        for line in &self.lines {
            if it.paragraph == pcursor.paragraph {
                // Right paragraph, but is it the right line in the paragraph?

                if it.offset <= pcursor.offset
                    && pcursor.offset <= it.offset + line.char_count_excluding_newline()
                {
                    let column = pcursor.offset - it.offset;
                    let column = column.at_most(line.char_count_excluding_newline());

                    let select_next_line_instead = pcursor.prefer_next_line
                        && !line.ends_with_newline
                        && column == line.char_count_excluding_newline();
                    if !select_next_line_instead {
                        return vec2(line.x_offsets[column], line.y_min);
                    }
                }
            }

            if line.ends_with_newline {
                it.paragraph += 1;
                it.offset = 0;
            } else {
                it.offset += line.char_count_including_newline();
            }
        }

        self.last_pos()
    }

    pub fn pos_from_cursor(&self, cursor: &Cursor) -> Vec2 {
        // self.pos_from_lcursor(cursor.lcursor)
        self.pos_from_pcursor(cursor.pcursor) // The one TextEdit stores
    }

    /// Cursor at the given position within the galley
    pub fn cursor_at(&self, pos: Vec2) -> Cursor {
        let mut best_y_dist = f32::INFINITY;
        let mut cursor = Cursor::default();

        let mut ccursor_index = 0;
        let mut pcursor_it = PCursor::default();

        for (line_nr, line) in self.lines.iter().enumerate() {
            let y_dist = (line.y_min - pos.y).abs().min((line.y_max - pos.y).abs());
            if y_dist < best_y_dist {
                best_y_dist = y_dist;
                let column = line.char_at(pos.x);
                cursor = Cursor {
                    ccursor: CCursor {
                        index: ccursor_index + column,
                        prefer_next_line: column == 0,
                    },
                    lcursor: LCursor {
                        line: line_nr,
                        column,
                    },
                    pcursor: PCursor {
                        paragraph: pcursor_it.paragraph,
                        offset: pcursor_it.offset + column,
                        prefer_next_line: column == 0,
                    },
                }
            }
            ccursor_index += line.char_count_including_newline();
            if line.ends_with_newline {
                pcursor_it.paragraph += 1;
                pcursor_it.offset = 0;
            } else {
                pcursor_it.offset += line.char_count_including_newline();
            }
        }
        cursor
    }
}

/// ## Cursor positions
impl Galley {
    /// Cursor to one-past last character.
    pub fn end(&self) -> Cursor {
        if self.lines.is_empty() {
            return Default::default();
        }
        let mut ccursor = CCursor {
            index: 0,
            prefer_next_line: true,
        };
        let mut pcursor = PCursor {
            paragraph: 0,
            offset: 0,
            prefer_next_line: true,
        };
        for line in &self.lines {
            let line_char_count = line.char_count_including_newline();
            ccursor.index += line_char_count;
            if line.ends_with_newline {
                pcursor.paragraph += 1;
                pcursor.offset = 0;
            } else {
                pcursor.offset += line_char_count;
            }
        }
        Cursor {
            ccursor,
            lcursor: self.end_lcursor(),
            pcursor,
        }
    }

    pub fn end_lcursor(&self) -> LCursor {
        if let Some(last_line) = self.lines.last() {
            debug_assert!(!last_line.ends_with_newline);
            LCursor {
                line: self.lines.len() - 1,
                column: last_line.char_count_excluding_newline(),
            }
        } else {
            Default::default()
        }
    }
}

/// ## Cursor conversions
impl Galley {
    // TODO: return identical cursor, or clamp?
    pub fn from_ccursor(&self, ccursor: CCursor) -> Cursor {
        let prefer_next_line = ccursor.prefer_next_line;
        let mut ccursor_it = CCursor {
            index: 0,
            prefer_next_line,
        };
        let mut pcursor_it = PCursor {
            paragraph: 0,
            offset: 0,
            prefer_next_line,
        };

        for (line_nr, line) in self.lines.iter().enumerate() {
            let line_char_count = line.char_count_excluding_newline();

            if ccursor_it.index <= ccursor.index
                && ccursor.index <= ccursor_it.index + line_char_count
            {
                let column = ccursor.index - ccursor_it.index;

                let select_next_line_instead = prefer_next_line
                    && !line.ends_with_newline
                    && column == line.char_count_excluding_newline();
                if !select_next_line_instead {
                    pcursor_it.offset += column;
                    return Cursor {
                        ccursor,
                        lcursor: LCursor {
                            line: line_nr,
                            column,
                        },
                        pcursor: pcursor_it,
                    };
                }
            }
            ccursor_it.index += line.char_count_including_newline();
            if line.ends_with_newline {
                pcursor_it.paragraph += 1;
                pcursor_it.offset = 0;
            } else {
                pcursor_it.offset += line.char_count_including_newline();
            }
        }
        debug_assert_eq!(ccursor_it, self.end().ccursor);
        Cursor {
            ccursor: ccursor_it, // clamp
            lcursor: self.end_lcursor(),
            pcursor: pcursor_it,
        }
    }

    // TODO: return identical cursor, or clamp?
    pub fn from_lcursor(&self, lcursor: LCursor) -> Cursor {
        if lcursor.line >= self.lines.len() {
            return self.end();
        }

        let prefer_next_line = lcursor.column == 0;
        let mut ccursor_it = CCursor {
            index: 0,
            prefer_next_line,
        };
        let mut pcursor_it = PCursor {
            paragraph: 0,
            offset: 0,
            prefer_next_line,
        };

        for (line_nr, line) in self.lines.iter().enumerate() {
            if line_nr == lcursor.line {
                let column = lcursor.column.at_most(line.char_count_excluding_newline());

                let select_next_line_instead = prefer_next_line
                    && !line.ends_with_newline
                    && column == line.char_count_excluding_newline();

                if !select_next_line_instead {
                    ccursor_it.index += column;
                    pcursor_it.offset += column;
                    return Cursor {
                        ccursor: ccursor_it,
                        lcursor,
                        pcursor: pcursor_it,
                    };
                }
            }
            ccursor_it.index += line.char_count_including_newline();
            if line.ends_with_newline {
                pcursor_it.paragraph += 1;
                pcursor_it.offset = 0;
            } else {
                pcursor_it.offset += line.char_count_including_newline();
            }
        }
        Cursor {
            ccursor: ccursor_it,
            lcursor: self.end_lcursor(),
            pcursor: pcursor_it,
        }
    }

    // TODO: return identical cursor, or clamp?
    pub fn from_pcursor(&self, pcursor: PCursor) -> Cursor {
        let prefer_next_line = pcursor.prefer_next_line;
        let mut ccursor_it = CCursor {
            index: 0,
            prefer_next_line,
        };
        let mut pcursor_it = PCursor {
            paragraph: 0,
            offset: 0,
            prefer_next_line,
        };

        for (line_nr, line) in self.lines.iter().enumerate() {
            if pcursor_it.paragraph == pcursor.paragraph {
                // Right paragraph, but is it the right line in the paragraph?

                if pcursor_it.offset <= pcursor.offset
                    && pcursor.offset <= pcursor_it.offset + line.char_count_excluding_newline()
                {
                    let column = pcursor.offset - pcursor_it.offset;
                    let column = column.at_most(line.char_count_excluding_newline());

                    let select_next_line_instead = pcursor.prefer_next_line
                        && !line.ends_with_newline
                        && column == line.char_count_excluding_newline();
                    if !select_next_line_instead {
                        ccursor_it.index += column;
                        return Cursor {
                            ccursor: ccursor_it,
                            lcursor: LCursor {
                                line: line_nr,
                                column,
                            },
                            pcursor,
                        };
                    }
                }
            }

            ccursor_it.index += line.char_count_including_newline();
            if line.ends_with_newline {
                pcursor_it.paragraph += 1;
                pcursor_it.offset = 0;
            } else {
                pcursor_it.offset += line.char_count_including_newline();
            }
        }
        Cursor {
            ccursor: ccursor_it,
            lcursor: self.end_lcursor(),
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
            self.from_ccursor(cursor.ccursor - 1)
        }
    }

    pub fn cursor_right_one_character(&self, cursor: &Cursor) -> Cursor {
        self.from_ccursor(cursor.ccursor + 1)
    }

    pub fn cursor_up_one_line(&self, cursor: &Cursor) -> Cursor {
        if cursor.lcursor.line == 0 {
            Cursor::default()
        } else {
            let x = self.pos_from_cursor(cursor).x;
            let line = cursor.lcursor.line - 1;
            let column = self.lines[line].char_at(x).max(cursor.lcursor.column);
            self.from_lcursor(LCursor { line, column })
        }
    }

    pub fn cursor_down_one_line(&self, cursor: &Cursor) -> Cursor {
        if cursor.lcursor.line + 1 < self.lines.len() {
            let x = self.pos_from_cursor(cursor).x;
            let line = cursor.lcursor.line + 1;
            let column = self.lines[line].char_at(x).max(cursor.lcursor.column);
            self.from_lcursor(LCursor { line, column })
        } else {
            self.end()
        }
    }

    pub fn cursor_begin_of_line(&self, cursor: &Cursor) -> Cursor {
        self.from_lcursor(LCursor {
            line: cursor.lcursor.line,
            column: 0,
        })
    }

    pub fn cursor_end_of_line(&self, cursor: &Cursor) -> Cursor {
        self.from_lcursor(LCursor {
            line: cursor.lcursor.line,
            column: self.lines[cursor.lcursor.line].char_count_excluding_newline(),
        })
    }
}

// ----------------------------------------------------------------------------

#[test]
fn test_text_layout() {
    use crate::mutex::Mutex;
    use crate::paint::{font::Font, *};

    let pixels_per_point = 1.0;
    let typeface_data = include_bytes!("../../fonts/ProggyClean.ttf");
    let atlas = TextureAtlas::new(512, 16);
    let atlas = std::sync::Arc::new(Mutex::new(atlas));
    let font = Font::new(atlas, typeface_data, 13.0, pixels_per_point);

    let galley = font.layout_multiline("".to_owned(), 1024.0);
    assert_eq!(galley.lines.len(), 1);
    assert_eq!(galley.lines[0].ends_with_newline, false);
    assert_eq!(galley.lines[0].x_offsets, vec![0.0]);

    let galley = font.layout_multiline("\n".to_owned(), 1024.0);
    assert_eq!(galley.lines.len(), 2);
    assert_eq!(galley.lines[0].ends_with_newline, true);
    assert_eq!(galley.lines[1].ends_with_newline, false);
    assert_eq!(galley.lines[1].x_offsets, vec![0.0]);

    let galley = font.layout_multiline("\n\n".to_owned(), 1024.0);
    assert_eq!(galley.lines.len(), 3);
    assert_eq!(galley.lines[0].ends_with_newline, true);
    assert_eq!(galley.lines[1].ends_with_newline, true);
    assert_eq!(galley.lines[2].ends_with_newline, false);
    assert_eq!(galley.lines[2].x_offsets, vec![0.0]);

    let galley = font.layout_multiline(" ".to_owned(), 1024.0);
    assert_eq!(galley.lines.len(), 1);
    assert_eq!(galley.lines[0].ends_with_newline, false);

    let galley = font.layout_multiline("One line".to_owned(), 1024.0);
    assert_eq!(galley.lines.len(), 1);
    assert_eq!(galley.lines[0].ends_with_newline, false);

    let galley = font.layout_multiline("First line\n".to_owned(), 1024.0);
    assert_eq!(galley.lines.len(), 2);
    assert_eq!(galley.lines[0].ends_with_newline, true);
    assert_eq!(galley.lines[1].ends_with_newline, false);
    assert_eq!(galley.lines[1].x_offsets, vec![0.0]);

    let galley = font.layout_multiline("line\nbreak".to_owned(), 10.0);
    assert_eq!(galley.lines.len(), 2);
    assert_eq!(galley.lines[0].ends_with_newline, true);
    assert_eq!(galley.lines[1].ends_with_newline, false);

    // Test wrapping:
    let galley = font.layout_multiline("line wrap".to_owned(), 10.0);
    assert_eq!(galley.lines.len(), 2);
    assert_eq!(galley.lines[0].ends_with_newline, false);
    assert_eq!(galley.lines[1].ends_with_newline, false);

    {
        // Test wrapping:
        let galley = font.layout_multiline("Line wrap.\nNew paragraph.".to_owned(), 10.0);
        assert_eq!(galley.lines.len(), 4);
        assert_eq!(galley.lines[0].ends_with_newline, false);
        assert_eq!(
            galley.lines[0].char_count_excluding_newline(),
            "Line ".len()
        );
        assert_eq!(
            galley.lines[0].char_count_including_newline(),
            "Line ".len()
        );
        assert_eq!(galley.lines[1].ends_with_newline, true);
        assert_eq!(
            galley.lines[1].char_count_excluding_newline(),
            "wrap.".len()
        );
        assert_eq!(
            galley.lines[1].char_count_including_newline(),
            "wrap.\n".len()
        );
        assert_eq!(galley.lines[2].ends_with_newline, false);
        assert_eq!(galley.lines[3].ends_with_newline, false);

        let cursor = Cursor::default();
        assert_eq!(cursor, galley.from_ccursor(cursor.ccursor));
        assert_eq!(cursor, galley.from_lcursor(cursor.lcursor));
        assert_eq!(cursor, galley.from_pcursor(cursor.pcursor));

        let cursor = galley.end();
        assert_eq!(cursor, galley.from_ccursor(cursor.ccursor));
        assert_eq!(cursor, galley.from_lcursor(cursor.lcursor));
        assert_eq!(cursor, galley.from_pcursor(cursor.pcursor));
        assert_eq!(
            cursor,
            Cursor {
                ccursor: CCursor::new(25),
                lcursor: LCursor {
                    line: 3,
                    column: 10
                },
                pcursor: PCursor {
                    paragraph: 1,
                    offset: 14,
                    prefer_next_line: false,
                }
            }
        );

        let cursor = galley.from_ccursor(CCursor::new(1));
        assert_eq!(cursor.lcursor, LCursor { line: 0, column: 1 });
        assert_eq!(
            cursor.pcursor,
            PCursor {
                paragraph: 0,
                offset: 1,
                prefer_next_line: false,
            }
        );
        assert_eq!(cursor, galley.from_ccursor(cursor.ccursor));
        assert_eq!(cursor, galley.from_lcursor(cursor.lcursor));
        assert_eq!(cursor, galley.from_pcursor(cursor.pcursor));

        let cursor = galley.from_pcursor(PCursor {
            paragraph: 1,
            offset: 2,
            prefer_next_line: false,
        });
        assert_eq!(cursor.lcursor, LCursor { line: 2, column: 2 });
        assert_eq!(cursor, galley.from_ccursor(cursor.ccursor));
        assert_eq!(cursor, galley.from_lcursor(cursor.lcursor));
        assert_eq!(cursor, galley.from_pcursor(cursor.pcursor));

        let cursor = galley.from_pcursor(PCursor {
            paragraph: 1,
            offset: 6,
            prefer_next_line: false,
        });
        assert_eq!(cursor.lcursor, LCursor { line: 3, column: 2 });
        assert_eq!(cursor, galley.from_ccursor(cursor.ccursor));
        assert_eq!(cursor, galley.from_lcursor(cursor.lcursor));
        assert_eq!(cursor, galley.from_pcursor(cursor.pcursor));

        // On the border between two lines within the same paragraph:
        let cursor = galley.from_lcursor(LCursor { line: 0, column: 5 });
        assert_eq!(
            cursor,
            Cursor {
                ccursor: CCursor::new(5),
                lcursor: LCursor { line: 0, column: 5 },
                pcursor: PCursor {
                    paragraph: 0,
                    offset: 5,
                    prefer_next_line: false,
                }
            }
        );
        assert_eq!(cursor, galley.from_lcursor(cursor.lcursor));

        let cursor = galley.from_lcursor(LCursor { line: 1, column: 0 });
        assert_eq!(
            cursor,
            Cursor {
                ccursor: CCursor::new(5),
                lcursor: LCursor { line: 1, column: 0 },
                pcursor: PCursor {
                    paragraph: 0,
                    offset: 5,
                    prefer_next_line: false,
                }
            }
        );
        assert_eq!(cursor, galley.from_lcursor(cursor.lcursor));
    }

    {
        // Test cursor movement:
        let galley = font.layout_multiline("Line wrap.\nNew paragraph.".to_owned(), 10.0);
        assert_eq!(galley.lines.len(), 4);
        assert_eq!(galley.lines[0].ends_with_newline, false);
        assert_eq!(galley.lines[1].ends_with_newline, true);
        assert_eq!(galley.lines[2].ends_with_newline, false);
        assert_eq!(galley.lines[3].ends_with_newline, false);

        let cursor = Cursor::default();

        assert_eq!(galley.cursor_up_one_line(&cursor), cursor);
        assert_eq!(galley.cursor_begin_of_line(&cursor), cursor);

        assert_eq!(
            galley.cursor_end_of_line(&cursor),
            Cursor {
                ccursor: CCursor::new(5),
                lcursor: LCursor { line: 0, column: 5 },
                pcursor: PCursor {
                    paragraph: 0,
                    offset: 5,
                    prefer_next_line: false,
                }
            }
        );

        assert_eq!(
            galley.cursor_down_one_line(&cursor),
            Cursor {
                ccursor: CCursor::new(5),
                lcursor: LCursor { line: 1, column: 0 },
                pcursor: PCursor {
                    paragraph: 0,
                    offset: 5,
                    prefer_next_line: false,
                }
            }
        );

        let cursor = Cursor::default();
        assert_eq!(
            galley.cursor_down_one_line(&galley.cursor_down_one_line(&cursor)),
            Cursor {
                ccursor: CCursor::new(11),
                lcursor: LCursor { line: 2, column: 0 },
                pcursor: PCursor {
                    paragraph: 1,
                    offset: 0,
                    prefer_next_line: false,
                }
            }
        );

        let cursor = galley.end();
        assert_eq!(galley.cursor_down_one_line(&cursor), cursor);

        let cursor = galley.end();
        assert!(galley.cursor_up_one_line(&galley.end()) != cursor);

        assert_eq!(
            galley.cursor_up_one_line(&galley.end()),
            Cursor {
                ccursor: CCursor::new(15),
                lcursor: LCursor {
                    line: 2,
                    column: 10
                },
                pcursor: PCursor {
                    paragraph: 1,
                    offset: 4,
                    prefer_next_line: false,
                }
            }
        );
    }
}
