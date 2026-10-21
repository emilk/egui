//! Placing and moving the text cursor on screen, in text of mixed direction.
//!
//! Left and Right move the cursor on screen, one grapheme cluster or one word at a time,
//! as in Chromium, Safari, Firefox, GTK and the Cocoa text system (Qt alone moves in logical
//! order by default). A selection stays a logical range of chars, and may be drawn in pieces.
//!
//! Where the bidi level changes, one char offset has two places on screen: the edge of the
//! run before it and the edge of the run after it. [`CCursor::prefer_next_row`] picks one,
//! the same way it picks the end of a row or the start of the next one at a row wrap:
//! `true` (downstream) means the char after the cursor, `false` (upstream) the char before it.
//!
//! On a row, the cursor stops once at each boundary between two clusters on screen,
//! and at both ends of the row. A cursor that moves past a cluster sticks to it,
//! so moving back returns it to the same place.

use unicode_segmentation::UnicodeSegmentation as _;

use emath::Align;

use super::{CharIndex, Galley, Row, cursor::CCursor, grapheme};

/// A grapheme cluster of a [`Row`], with its row-relative char range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cluster {
    start: usize,
    end: usize,
    rtl: bool,
}

/// A cursor in a row: a row-relative column, and whether it sticks to the cluster
/// after it (downstream) or before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RowCaret {
    column: usize,
    downstream: bool,
}

impl Cluster {
    /// The cursor on the left edge of this cluster, sticking to it.
    fn left_caret(self) -> RowCaret {
        if self.rtl {
            RowCaret {
                column: self.end,
                downstream: false,
            }
        } else {
            RowCaret {
                column: self.start,
                downstream: true,
            }
        }
    }

    /// The cursor on the right edge of this cluster, sticking to it.
    fn right_caret(self) -> RowCaret {
        if self.rtl {
            RowCaret {
                column: self.start,
                downstream: true,
            }
        } else {
            RowCaret {
                column: self.end,
                downstream: false,
            }
        }
    }
}

/// The cursor stop that `caret` is at, in `clusters` (from left to right):
/// stop `k` is left of cluster `k`, and stop `clusters.len()` is the right end of the row.
fn stop_of(clusters: &[Cluster], caret: RowCaret) -> usize {
    let column = caret.column;
    let after = clusters
        .iter()
        .position(|c| c.start == column)
        .map(|i| (i, true));
    let before = clusters
        .iter()
        .position(|c| c.end == column)
        .map(|i| (i, false));
    let inside = || {
        clusters
            .iter()
            .position(|c| c.start < column && column < c.end)
            .map(|i| (i, true))
    };
    let attached = if caret.downstream {
        after.or(before)
    } else {
        before.or(after)
    };
    let Some((i, at_start)) = attached.or_else(inside) else {
        return 0;
    };
    // The logical start of a right-to-left cluster is its right edge.
    i + usize::from(at_start == clusters[i].rtl)
}

/// The cursor at stop `stop`, sticking to the cluster on its left if `stick_left` and there is one,
/// otherwise to the one on its right.
fn caret_at_stop(clusters: &[Cluster], stop: usize, stick_left: bool) -> RowCaret {
    let from_left = stop
        .checked_sub(1)
        .and_then(|i| clusters.get(i))
        .map(|c| c.right_caret());
    let from_right = clusters.get(stop).map(|c| c.left_caret());
    let caret = if stick_left {
        from_left.or(from_right)
    } else {
        from_right.or(from_left)
    };
    caret.unwrap_or(RowCaret {
        column: 0,
        downstream: true,
    })
}

impl Row {
    /// The grapheme clusters of this row, from left to right.
    fn visual_clusters(&self) -> Vec<Cluster> {
        let text = self.text();
        let mut cluster_of = vec![0; self.glyphs.len()];
        let mut ranges = Vec::new();
        let mut start = 0;
        for cluster in text.graphemes(true) {
            let end = start + cluster.chars().count();
            cluster_of[start..end].fill(ranges.len());
            ranges.push(start..end);
            start = end;
        }

        let mut seen = vec![false; ranges.len()];
        let mut clusters = Vec::with_capacity(ranges.len());
        for glyph in self.visual_order() {
            let i = cluster_of[glyph];
            if !seen[i] {
                seen[i] = true;
                let range = &ranges[i];
                clusters.push(Cluster {
                    start: range.start,
                    end: range.end,
                    rtl: self.glyphs[range.start].is_rtl(),
                });
            }
        }
        clusters
    }

    /// Like [`Self::x_offset`], for a cursor that sticks to the char after it (`downstream`)
    /// or the char before it.
    ///
    /// Where the bidi level changes at `column`, those two chars can be apart on screen,
    /// and the cursor goes on the edge of the one it sticks to.
    pub fn x_offset_with_affinity(&self, column: CharIndex, downstream: bool) -> f32 {
        let c = column.0;
        if !downstream
            && 0 < c
            && c < self.glyphs.len()
            && self.glyphs[c - 1].bidi_level != self.glyphs[c].bidi_level
        {
            let before = &self.glyphs[grapheme::previous_boundary(&self.text(), column).0];
            return if before.is_rtl() {
                before.pos.x
            } else {
                before.max_x()
            };
        }
        self.x_offset(column)
    }

    /// The column at `desired_x` (row-relative), as [`Self::char_at`],
    /// and whether the cursor sticks to the char after it: the cluster that was hit.
    pub(crate) fn caret_at(&self, desired_x: f32) -> (CharIndex, bool) {
        let column = self.char_at(desired_x);
        let c = column.0;
        if c == 0 || self.glyphs.len() <= c {
            return (column, c == 0);
        }
        if self.glyphs[c - 1].bidi_level == self.glyphs[c].bidi_level {
            return (column, true);
        }
        // Stick to whichever neighbor of `column` is nearer to `desired_x`.
        let distance = |glyph: &super::Glyph| {
            (glyph.pos.x - desired_x)
                .max(desired_x - glyph.max_x())
                .max(0.0)
        };
        let before = &self.glyphs[grapheme::previous_boundary(&self.text(), column).0];
        (column, distance(&self.glyphs[c]) <= distance(before))
    }
}

/// Word segments (UAX #29) of the text of a galley, for word motion.
struct Words {
    chars: Vec<char>,

    /// For each char: the index of its segment.
    segment: Vec<u32>,

    /// For each segment: is it all word chars (letters, digits, `_`)?
    is_word: Vec<bool>,
}

impl Words {
    fn new(galley: &Galley) -> Self {
        let mut text = String::new();
        for row in &galley.rows {
            text.extend(row.glyphs.iter().map(|g| g.chr));
            if row.ends_with_newline {
                text.push('\n');
            }
        }
        let mut segment = Vec::with_capacity(text.len());
        let mut is_word = Vec::new();
        for word in text.split_word_bounds() {
            let id = is_word.len() as u32;
            segment.extend(word.chars().map(|_| id));
            is_word.push(word.chars().all(|c| c.is_alphanumeric() || c == '_'));
        }
        Self {
            chars: text.chars().collect(),
            segment,
            is_word,
        }
    }

    /// Does word motion stop before the char at `inside`, coming from `across`
    /// (`None`: from a paragraph break)?
    ///
    /// It stops before each segment that is not a word, so it skips spaces and then a word,
    /// and before each `.`, as on macOS for `www.example.com`.
    fn stops_before(&self, inside: usize, across: Option<usize>) -> bool {
        let Some(&segment) = self.segment.get(inside) else {
            return true;
        };
        !self.is_word[segment as usize]
            && (self.chars[inside] == '.'
                || across.is_none_or(|across| self.segment.get(across) != Some(&segment)))
    }
}

/// ## Cursor movement on screen
impl Galley {
    fn row_start(&self, row: usize) -> usize {
        self.rows[..row]
            .iter()
            .map(|row| row.char_count_including_newline().0)
            .sum()
    }

    fn row_caret(&self, cursor: &CCursor) -> (usize, RowCaret) {
        let layout = self.layout_from_cursor(*cursor);
        let row = layout.row.min(self.rows.len() - 1);
        let caret = RowCaret {
            column: layout.column.0.min(self.rows[row].glyphs.len()),
            downstream: cursor.prefer_next_row,
        };
        (row, caret)
    }

    fn cursor_from_row_caret(&self, row: usize, caret: RowCaret) -> CCursor {
        CCursor {
            index: CharIndex(self.row_start(row) + caret.column),
            prefer_next_row: caret.downstream,
        }
    }

    /// Does `row` wrap onto the next row, rather than end its paragraph?
    fn wraps(&self, row: usize) -> bool {
        !self.rows[row].ends_with_newline && row + 1 < self.rows.len()
    }

    /// The cursor at the start (in reading order) of `row`, or at its end.
    fn row_edge(&self, row: usize, start: bool) -> RowCaret {
        let clusters = self.rows[row].visual_clusters();
        let at_left = start != self.rows[row].rtl_paragraph;
        let stop = if at_left { 0 } else { clusters.len() };
        caret_at_stop(&clusters, stop, !at_left)
    }

    /// The x coordinate, in galley coordinates, of a cursor at `column` of `row`.
    pub(crate) fn caret_x(&self, row: usize, column: CharIndex, downstream: bool) -> f32 {
        let placed_row = &self.rows[row];
        if placed_row.glyphs.is_empty()
            && placed_row.rtl_paragraph
            && self.job.halign == Align::Min
            && self.job.halign_min_is_start
        {
            // An empty right-to-left row starts where the other right-to-left rows do:
            return self
                .rows
                .iter()
                .map(|row| row.pos.x + row.size.x)
                .fold(0.0, f32::max);
        }
        placed_row.pos.x + placed_row.x_offset_with_affinity(column, downstream)
    }

    /// The cursor one grapheme cluster to the left on screen.
    ///
    /// Past the left end of a row it goes on to the previous row in a left-to-right paragraph,
    /// and to the next one in a right-to-left paragraph.
    pub fn cursor_left_visually(&self, cursor: &CCursor) -> CCursor {
        self.cursor_one_cluster(cursor, false)
    }

    /// The cursor one grapheme cluster to the right on screen.
    ///
    /// Past the right end of a row it goes on to the next row in a left-to-right paragraph,
    /// and to the previous one in a right-to-left paragraph.
    pub fn cursor_right_visually(&self, cursor: &CCursor) -> CCursor {
        self.cursor_one_cluster(cursor, true)
    }

    fn cursor_one_cluster(&self, cursor: &CCursor, rightward: bool) -> CCursor {
        if self.rows.is_empty() {
            return *cursor;
        }
        let (row, caret) = self.row_caret(cursor);
        let clusters = self.rows[row].visual_clusters();
        let stop = stop_of(&clusters, caret);
        let forward = rightward != self.rows[row].rtl_paragraph;

        let target = if rightward {
            (stop < clusters.len()).then_some(stop + 1)
        } else {
            stop.checked_sub(1)
        };
        if let Some(target) = target {
            let caret = caret_at_stop(&clusters, target, rightward);
            let end_of_row = if self.rows[row].rtl_paragraph {
                0
            } else {
                clusters.len()
            };
            if target == end_of_row
                && caret.column == self.rows[row].glyphs.len()
                && self.wraps(row)
            {
                // The end of a wrapped row is the start of the next row; show it there.
                return CCursor {
                    index: CharIndex(self.row_start(row) + caret.column),
                    prefer_next_row: true,
                };
            }
            return self.cursor_from_row_caret(row, caret);
        }

        let next_row = if forward {
            Some(row + 1).filter(|&r| r < self.rows.len())
        } else {
            row.checked_sub(1)
        };
        let Some(next_row) = next_row else {
            return *cursor;
        };
        let entry = self.cursor_from_row_caret(next_row, self.row_edge(next_row, forward));
        if entry.index == cursor.index {
            // Across a row wrap the end of one row is the start of the next; don't stop twice.
            self.cursor_one_cluster(&entry, rightward)
        } else {
            entry
        }
    }

    /// The cursor one word to the left on screen: to the start of the word
    /// on its left, in reading order of that word.
    ///
    /// It snaps to grapheme clusters and goes on to other rows like [`Self::cursor_left_visually`].
    pub fn cursor_word_left_visually(&self, cursor: &CCursor) -> CCursor {
        self.cursor_one_word(cursor, false)
    }

    /// The cursor one word to the right on screen. See [`Self::cursor_word_left_visually`].
    pub fn cursor_word_right_visually(&self, cursor: &CCursor) -> CCursor {
        self.cursor_one_word(cursor, true)
    }

    fn cursor_one_word(&self, cursor: &CCursor, rightward: bool) -> CCursor {
        if self.rows.is_empty() {
            return *cursor;
        }
        let words = Words::new(self);
        let (mut row, caret) = self.row_caret(cursor);
        let mut clusters = self.rows[row].visual_clusters();
        let mut stop = stop_of(&clusters, caret);

        // Each pass moves on to another row in one direction, or returns.
        for _ in 0..=self.rows.len() {
            let row_start = self.row_start(row);
            let placed_row = &self.rows[row];
            let forward = rightward != placed_row.rtl_paragraph;
            let edge = if rightward { clusters.len() } else { 0 };
            let at = |stop: usize| {
                self.cursor_from_row_caret(row, caret_at_stop(&clusters, stop, rightward))
            };

            let mut k = stop;
            while k != edge {
                k = if rightward { k + 1 } else { k - 1 };
                if k == edge {
                    break;
                }
                let (inside, across) = if rightward {
                    (clusters[k], clusters[k - 1])
                } else {
                    (clusters[k - 1], clusters[k])
                };
                if words.stops_before(row_start + inside.start, Some(row_start + across.start)) {
                    return at(k);
                }
            }

            // The edge of the row: a paragraph break, the end of the text, or a row wrap.
            let soft = if forward {
                self.wraps(row)
            } else {
                row > 0 && self.wraps(row - 1)
            };
            if stop != edge {
                if !soft {
                    return at(edge);
                }
                let boundary = if forward {
                    row_start + placed_row.char_count_including_newline().0
                } else {
                    row_start
                };
                let edge_cluster = if rightward {
                    clusters.last()
                } else {
                    clusters.first()
                };
                let (inside, across) = if forward {
                    (boundary, edge_cluster.map(|c| row_start + c.start))
                } else {
                    (boundary - 1, edge_cluster.map(|c| row_start + c.start))
                };
                if words.stops_before(inside, across) {
                    let cursor = at(edge);
                    // The end of a wrapped row is the start of the next row; show it there.
                    let prefer_next_row =
                        cursor.prefer_next_row || (forward && cursor.index.0 == boundary);
                    return CCursor {
                        prefer_next_row,
                        ..cursor
                    };
                }
            }

            let next_row = if forward {
                Some(row + 1).filter(|&r| r < self.rows.len())
            } else {
                row.checked_sub(1)
            };
            let Some(next_row) = next_row else {
                return at(edge);
            };
            row = next_row;
            clusters = self.rows[row].visual_clusters();
            let at_left = forward != self.rows[row].rtl_paragraph;
            stop = if at_left { 0 } else { clusters.len() };
            if soft {
                continue;
            }

            // Past a paragraph break:
            let entry = self.cursor_from_row_caret(row, self.row_edge(row, forward));
            if at_left != rightward {
                // A paragraph of the other direction, entered at the far side.
                return entry;
            }
            let inside = if rightward {
                clusters.first()
            } else {
                clusters.last()
            };
            match inside {
                Some(inside) if !words.stops_before(self.row_start(row) + inside.start, None) => {}
                _ => return entry,
            }
        }
        *cursor
    }

    /// The cursor at the left end of its row on screen.
    pub fn cursor_left_end_of_row(&self, cursor: &CCursor) -> CCursor {
        self.cursor_row_end_on_screen(cursor, false)
    }

    /// The cursor at the right end of its row on screen.
    pub fn cursor_right_end_of_row(&self, cursor: &CCursor) -> CCursor {
        self.cursor_row_end_on_screen(cursor, true)
    }

    fn cursor_row_end_on_screen(&self, cursor: &CCursor, right: bool) -> CCursor {
        if self.rows.is_empty() {
            return *cursor;
        }
        let (row, _) = self.row_caret(cursor);
        let start = right == self.rows[row].rtl_paragraph;
        self.cursor_from_row_caret(row, self.row_edge(row, start))
    }

    /// The cursor at the start of its row in reading order: the left end in a
    /// left-to-right paragraph, the right end in a right-to-left one.
    pub fn cursor_begin_of_row(&self, cursor: &CCursor) -> CCursor {
        if self.rows.is_empty() {
            return *cursor;
        }
        let (row, _) = self.row_caret(cursor);
        self.cursor_from_row_caret(row, self.row_edge(row, true))
    }

    /// The cursor at the end of its row in reading order: the right end in a
    /// left-to-right paragraph, the left end in a right-to-left one.
    pub fn cursor_end_of_row(&self, cursor: &CCursor) -> CCursor {
        if self.rows.is_empty() {
            return *cursor;
        }
        let (row, _) = self.row_caret(cursor);
        self.cursor_from_row_caret(row, self.row_edge(row, false))
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;

    use emath::vec2;

    use super::*;
    use crate::{
        Color32, FontId,
        text::{FontDefinitions, Fonts, LayoutJob, TextDirection, TextOptions},
    };

    fn layout(text: &str, direction: TextDirection, wrap_width: f32) -> Arc<Galley> {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::default());
        let mut job = LayoutJob::simple(
            text.to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
            wrap_width,
        );
        job.direction = direction;
        fonts.with_pixels_per_point(1.0).layout_job(job)
    }

    fn auto(text: &str) -> Arc<Galley> {
        layout(text, TextDirection::Auto, f32::INFINITY)
    }

    fn x(galley: &Galley, cursor: CCursor) -> f32 {
        galley.pos_from_cursor(cursor).min.x
    }

    /// The cursors from pressing a key until the cursor stops moving.
    fn presses(galley: &Galley, from: CCursor, step: impl Fn(&CCursor) -> CCursor) -> Vec<CCursor> {
        let mut cursors = vec![from];
        loop {
            let next = step(cursors.last().unwrap());
            let last = cursors.last().unwrap();
            if next.index == last.index && x(galley, next) == x(galley, *last) {
                return cursors;
            }
            assert!(cursors.len() < 100, "no end in sight: {cursors:?}");
            cursors.push(next);
        }
    }

    fn indices(cursors: &[CCursor]) -> Vec<usize> {
        cursors.iter().map(|c| c.index.0).collect()
    }

    /// Each press moves the cursor further in the same direction on screen.
    fn assert_moves_on_screen(galley: &Galley, cursors: &[CCursor], rightward: bool) {
        for pair in cursors.windows(2) {
            let (a, b) = (x(galley, pair[0]), x(galley, pair[1]));
            assert!(
                if rightward { a < b } else { b < a },
                "{:?}: {:?} at {a} to {:?} at {b}",
                galley.text(),
                pair[0],
                pair[1]
            );
        }
    }

    /// Visits every place on the row once, from one end to the other, both ways.
    fn assert_arrow_sequences(galley: &Galley, rightward: &[usize], leftward: &[usize]) {
        let row = &galley.rows[0];
        let left_end = galley.cursor_left_end_of_row(&galley.begin());
        let right_end = galley.cursor_right_end_of_row(&galley.begin());
        let left = row
            .glyphs
            .iter()
            .map(|g| g.pos.x)
            .fold(f32::INFINITY, f32::min);
        assert_eq!(x(galley, left_end), row.pos.x + left);

        let cursors = presses(galley, left_end, |c| galley.cursor_right_visually(c));
        assert_eq!(indices(&cursors), rightward, "{:?}", galley.text());
        assert_moves_on_screen(galley, &cursors, true);
        assert_eq!(x(galley, *cursors.last().unwrap()), x(galley, right_end));

        let cursors = presses(galley, right_end, |c| galley.cursor_left_visually(c));
        assert_eq!(indices(&cursors), leftward, "{:?}", galley.text());
        assert_moves_on_screen(galley, &cursors, false);
        assert_eq!(x(galley, *cursors.last().unwrap()), x(galley, left_end));
    }

    #[test]
    fn arrows_move_on_screen_in_mixed_text() {
        // On screen: "abc גבא def"
        assert_arrow_sequences(
            &auto("abc אבג def"),
            &[0, 1, 2, 3, 4, 6, 5, 4, 8, 9, 10, 11],
            &[11, 10, 9, 8, 7, 5, 6, 7, 3, 2, 1, 0],
        );
        // A right-to-left paragraph, on screen: "abc גבא"
        assert_arrow_sequences(
            &auto("אבג abc"),
            &[4, 5, 6, 7, 3, 2, 1, 0],
            &[0, 1, 2, 3, 4, 6, 5, 4],
        );
        // Digits are left-to-right within Arabic, on screen: "رانيد 123 نمث"
        assert_arrow_sequences(
            &auto("ثمن 123 دينار"),
            &[13, 12, 11, 10, 9, 8, 7, 5, 6, 7, 3, 2, 1, 0],
            &[0, 1, 2, 3, 4, 6, 5, 4, 8, 9, 10, 11, 12, 13],
        );
        // Three levels: digits (2) in Hebrew (1) in English (0), on screen: "abc והד 123 גבא def"
        assert_arrow_sequences(
            &auto("abc אבג 123 דהו def"),
            &[
                0, 1, 2, 3, 4, 14, 13, 12, 11, 9, 10, 11, 7, 6, 5, 4, 16, 17, 18, 19,
            ],
            &[
                19, 18, 17, 16, 15, 5, 6, 7, 8, 10, 9, 8, 12, 13, 14, 15, 3, 2, 1, 0,
            ],
        );
    }

    /// One char offset, two places: the cursor sticks to the run it moved past.
    #[test]
    fn affinity_picks_the_run() {
        let galley = auto("abc אבג def");
        let row = &galley.rows[0];
        let [space, alef, gimel, space2] = [3, 4, 6, 7].map(|i| row.glyphs[i]);

        let upstream = |i| CCursor::new(i);
        let downstream = |i| CCursor {
            prefer_next_row: true,
            ..CCursor::new(i)
        };
        // After "abc ", or before the alef at the right end of the Hebrew word:
        assert_eq!(x(&galley, upstream(4)), space.max_x());
        assert_eq!(x(&galley, downstream(4)), alef.max_x());
        // After the gimel at its left end, or before the second space:
        assert_eq!(x(&galley, upstream(7)), gimel.pos.x);
        assert_eq!(x(&galley, downstream(7)), space2.pos.x);
    }

    /// A click sticks the cursor to the char that was clicked.
    #[test]
    fn clicks_pick_the_affinity() {
        let galley = auto("abc אבג def");
        let row = &galley.rows[0];
        let y = row.size.y / 2.0;
        let click = |i: usize, fraction: f32| {
            let rect = row.glyphs[i].logical_rect();
            galley.cursor_from_pos(vec2(rect.lerp_inside([fraction, 0.5]).x, y))
        };
        let near = |cursor: CCursor, edge: f32| (x(&galley, cursor) - edge).abs() < 0.01;

        let alef = row.glyphs[4];
        let cursor = click(4, 0.75);
        assert_eq!((cursor.index.0, cursor.prefer_next_row), (4, true));
        assert!(near(cursor, alef.max_x()));

        let space = row.glyphs[3];
        let cursor = click(3, 0.75);
        assert_eq!((cursor.index.0, cursor.prefer_next_row), (4, false));
        assert!(near(cursor, space.max_x()));

        let gimel = row.glyphs[6];
        let cursor = click(6, 0.25);
        assert_eq!((cursor.index.0, cursor.prefer_next_row), (7, false));
        assert!(near(cursor, gimel.pos.x));

        let space2 = row.glyphs[7];
        let cursor = click(7, 0.25);
        assert_eq!((cursor.index.0, cursor.prefer_next_row), (7, true));
        assert!(near(cursor, space2.pos.x));
    }

    /// Wrapped left-to-right text: the end of a row and the start of the next are one stop.
    #[test]
    fn arrows_cross_a_row_wrap_once() {
        let unwrapped = auto("abc def");
        let width = unwrapped.rows[0].glyphs[4].pos.x + 1.0;
        let galley = layout("abc def", TextDirection::Auto, width);
        assert_eq!(galley.rows.len(), 2);

        let rightward = presses(&galley, galley.begin(), |c| galley.cursor_right_visually(c));
        assert_eq!(indices(&rightward), [0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(galley.layout_from_cursor(rightward[4]).row, 1);

        let leftward = presses(&galley, galley.end(), |c| galley.cursor_left_visually(c));
        assert_eq!(indices(&leftward), [7, 6, 5, 4, 3, 2, 1, 0]);
        assert_eq!(galley.layout_from_cursor(leftward[3]).row, 1);
    }

    /// Past the end of a row, the cursor goes on in the reading order of the paragraph.
    #[test]
    fn arrows_cross_rows_in_reading_order() {
        let galley = layout("אב\nגד", TextDirection::RightToLeft, f32::INFINITY);
        let leftward = presses(&galley, galley.begin(), |c| galley.cursor_left_visually(c));
        assert_eq!(indices(&leftward), [0, 1, 2, 3, 4, 5]);
        let rightward = presses(&galley, galley.end(), |c| galley.cursor_right_visually(c));
        assert_eq!(indices(&rightward), [5, 4, 3, 2, 1, 0]);
    }

    #[test]
    fn home_and_end_go_to_the_start_and_end_in_reading_order() {
        let ends = |galley: &Galley, from: usize| {
            let from = CCursor::new(from);
            let home = galley.cursor_begin_of_row(&from);
            let end = galley.cursor_end_of_row(&from);
            let row = &galley.rows[0];
            // Which end of the row on screen, to the nearest point:
            let side = |cursor| ((x(galley, cursor) - row.pos.x) / row.size.x).round();
            ((home.index.0, side(home)), (end.index.0, side(end)))
        };

        let (left, right) = (0.0, 1.0);
        let galley = auto("abc אבג def");
        assert_eq!(ends(&galley, 5), ((0, left), (11, right)));

        // The ends of the row on screen, not in logical order:
        let galley = auto("אבג abc");
        assert_eq!(ends(&galley, 5), ((0, right), (4, left)));
        let left_end = galley.cursor_left_end_of_row(&CCursor::new(1));
        let right_end = galley.cursor_right_end_of_row(&CCursor::new(5));
        assert_eq!((left_end.index.0, right_end.index.0), (4, 0));

        let galley = auto("abc אבג");
        assert_eq!(ends(&galley, 1), ((0, left), (4, right)));
    }

    /// An empty row of a right-to-left paragraph starts on the right, like the others.
    #[test]
    fn cursor_on_an_empty_rtl_row_is_on_the_right() {
        let galley = layout("אבג\n", TextDirection::RightToLeft, f32::INFINITY);
        let right = galley.rows[0].pos.x + galley.rows[0].size.x;
        assert_eq!(x(&galley, galley.end()), right);
        assert_eq!(x(&galley, CCursor::new(4)), right);

        let galley = layout("abc\n", TextDirection::LeftToRight, f32::INFINITY);
        assert_eq!(x(&galley, galley.end()), 0.0);
    }

    #[test]
    fn word_motion_moves_on_screen() {
        let words = |galley: &Galley, from: CCursor, rightward: bool| {
            let step = |c: &CCursor| {
                if rightward {
                    galley.cursor_word_right_visually(c)
                } else {
                    galley.cursor_word_left_visually(c)
                }
            };
            let cursors = presses(galley, from, step);
            assert_moves_on_screen(galley, &cursors, rightward);
            indices(&cursors)
        };

        // On screen: "abc גבא def". Rightward to the right end of each word, leftward to the left end:
        let galley = auto("abc אבג def");
        assert_eq!(words(&galley, galley.begin(), true), [0, 3, 4, 11]);
        assert_eq!(words(&galley, galley.end(), false), [11, 8, 7, 0]);

        // A right-to-left paragraph, on screen: "והד גבא"
        let galley = auto("אבג דהו");
        assert_eq!(words(&galley, galley.begin(), false), [0, 3, 7]);
        assert_eq!(words(&galley, galley.end(), true), [7, 4, 0]);

        // Across paragraphs, as in logical order:
        let galley = auto("abc def\nghi");
        let rightward = presses(&galley, galley.begin(), |c| {
            galley.cursor_word_right_visually(c)
        });
        assert_eq!(indices(&rightward), [0, 3, 7, 11]);
        let leftward = presses(&galley, galley.end(), |c| {
            galley.cursor_word_left_visually(c)
        });
        assert_eq!(indices(&leftward), [11, 8, 4, 0]);

        // A dot is a stop:
        let galley = auto("www.example.com");
        assert_eq!(words(&galley, galley.begin(), true), [0, 3, 11, 15]);
    }

    /// A combining mark moves with its base.
    #[test]
    fn arrows_skip_whole_clusters() {
        let galley = auto("אָב");
        let cursors = presses(&galley, galley.begin(), |c| galley.cursor_left_visually(c));
        assert_eq!(indices(&cursors), [0, 2, 3]);
        let galley = auto("a\u{301}b");
        let cursors = presses(&galley, galley.begin(), |c| galley.cursor_right_visually(c));
        assert_eq!(indices(&cursors), [0, 2, 3]);
    }
}
