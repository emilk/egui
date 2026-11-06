use core::ops::Range;

use emath::TSTransform;
use epaint::text::cursor::CCursor;
use epaint::text::{CharIndex, PlacedRow, Row};

use crate::{Context, Galley, Id};

use super::{
    CCursorRange,
    text_cursor_state::{byte_index_from_char_index, is_word_char},
};

/// AccessKit's `word_starts` uses `u8` indices, so text runs cannot exceed this length.
pub(crate) const MAX_CHARS_PER_TEXT_RUN: usize = 255;

/// One text run of a row: a maximal stretch of glyphs that share one UAX #9
/// embedding level (so it reads in one direction and is contiguous on screen),
/// capped at [`MAX_CHARS_PER_TEXT_RUN`] glyphs.
///
/// `glyphs` indexes [`Row::glyphs`] — the *visual* glyph stream, stored
/// left-to-right (see that field's docs). A run's glyphs all carry the run's
/// [`epaint::text::Glyph::bidi_level`].
#[derive(Debug)]
pub(crate) struct TextRunInfo {
    /// Glyph range in stream (visual) order, excluding the row's trailing
    /// newline (tracked separately, see [`Self::newline`]).
    pub glyphs: Range<usize>,

    /// The run's UAX #9 embedding level (odd = right-to-left).
    pub level: u8,

    /// Whether the row's trailing newline is appended to this run's value.
    ///
    /// The newline logically terminates the row's text, so it belongs to the
    /// run that comes last in *logical* order — which for a right-to-left row
    /// is not the visually-last one (the visually-last run of an RTL row is
    /// the logically *first*).
    pub newline: bool,
}

impl TextRunInfo {
    /// Char count of the run's value (glyphs, plus the newline if carried).
    pub fn len(&self) -> usize {
        self.glyphs.len() + usize::from(self.newline)
    }

    /// The run's glyphs in *logical* character order.
    ///
    /// Even-level runs read in stream order. An odd-level run reads
    /// right-to-left while the stream lists its glyphs left-to-right, so its
    /// logical order reverses the *clusters*: a glyph continues the previous
    /// cluster while its advance width is <= 0 (the same grouping
    /// [`Row::char_at_rtl`] / [`Row::x_offset_rtl`] use), and the stream order
    /// is kept inside a cluster. Every glyph of a cluster carries the
    /// cluster's first char ([`Row::text`] builds on the same fact), so the
    /// value is invariant to the order *within* a cluster; multi-glyph
    /// clusters still duplicate that char in the value (a pre-existing,
    /// direction-independent limitation of building values from `glyph.chr`).
    ///
    /// Returns glyph indices into `row.glyphs`, in logical char order.
    fn logical_glyphs(&self, row: &Row) -> Vec<usize> {
        let glyphs = &row.glyphs[self.glyphs.clone()];

        // Cluster boundaries: the first glyph of the run starts a cluster, and
        // a new cluster starts at every glyph with a positive advance
        // (zero-advance glyphs are continuations of the cluster before them;
        // a genuinely zero-advance *base* merges into its neighbour — the same
        // approximation the row-level mapping makes).
        let mut clusters: Vec<Range<usize>> = Vec::new();
        let mut start = 0;
        for (i, glyph) in glyphs.iter().enumerate().skip(1) {
            if glyph.advance_width > 0.0 {
                clusters.push(start..i);
                start = i;
            }
        }
        if !glyphs.is_empty() {
            clusters.push(start..glyphs.len());
        }

        if self.level % 2 == 1 {
            clusters.reverse();
        }

        clusters
            .into_iter()
            .flatten()
            .map(|i| self.glyphs.start + i)
            .collect()
    }
}

/// The text runs of a row, as ranges of its columns, in stream (visual) order.
///
/// A run is a stretch of chars with the same bidi level, so it reads in one
/// direction and is contiguous on screen, at most [`MAX_CHARS_PER_TEXT_RUN`]
/// long. A row always has at least one run, even when empty. (The trailing
/// newline is folded into the visually-last range here; [`RowTextRuns`]
/// re-assigns it to the logically-last run.)
pub(crate) fn row_text_runs(row: &PlacedRow) -> Vec<Range<usize>> {
    let glyph_count = row.glyphs.len();
    let total_chars = glyph_count + usize::from(row.ends_with_newline);
    let mut runs = Vec::new();
    let mut start = 0;
    for i in 1..glyph_count {
        if row.glyphs[i].bidi_level != row.glyphs[i - 1].bidi_level
            || i - start == MAX_CHARS_PER_TEXT_RUN
        {
            runs.push(start..i);
            start = i;
        }
    }
    if total_chars - start > MAX_CHARS_PER_TEXT_RUN {
        runs.push(start..glyph_count);
        start = glyph_count;
    }
    runs.push(start..total_chars);
    runs
}

/// The text runs of a row, with their logical order resolved.
///
/// The glyph stream is in *visual* order, so the runs ([`Self::runs`], in
/// stream order — the order the run nodes are created in) are generally not in
/// logical order: for a right-to-left row the visually-leftmost run holds the
/// logically *first* text. [`Self::logical_order`] maps the stream to the
/// logical order, and [`Self::logical_range`] gives each run's char range
/// within the row's logical text.
pub(crate) struct RowTextRuns {
    /// Per run, in stream (visual) order.
    pub runs: Vec<TextRunInfo>,

    /// Run indices (into [`Self::runs`]), in logical order.
    pub logical_order: Vec<usize>,
}

impl RowTextRuns {
    pub fn new(row: &PlacedRow) -> Self {
        let glyph_count = row.glyphs.len();
        let mut runs: Vec<TextRunInfo> = row_text_runs(row)
            .into_iter()
            .map(|range| TextRunInfo {
                glyphs: range.start..range.end.min(glyph_count),
                level: row
                    .glyphs
                    .get(range.start)
                    .map_or(0, |glyph| glyph.bidi_level),
                newline: false,
            })
            .collect();

        let logical_order = logical_run_order(&row.row, &runs);

        // The row's trailing newline terminates its text, so it goes on the
        // logically-last run. It adds one char to that run's value; the char
        // is never a word start, so AccessKit's u8 word-start indices stay
        // valid even when the run already holds `MAX_CHARS_PER_TEXT_RUN`
        // glyphs (the newline is then the last char, at index 255).
        if row.ends_with_newline
            && let Some(&last) = logical_order.last()
        {
            runs[last].newline = true;
        }

        Self {
            runs,
            logical_order,
        }
    }

    /// The run's char range `[start, end)` within the row's logical text
    /// (the run values joined in [`Self::logical_order`]).
    pub fn logical_range(&self, stream_index: usize) -> Range<usize> {
        let mut start = 0;
        for &idx in &self.logical_order {
            if idx == stream_index {
                break;
            }
            start += self.runs[idx].len();
        }
        let len = self.runs[stream_index].len();
        start..start + len
    }

    /// The run that comes last in logical order, if any.
    pub fn logically_last(&self) -> Option<usize> {
        self.logical_order.last().copied()
    }
}

/// The logical order of a row's runs, as indices into `runs` (which are in
/// stream / visual order).
///
/// The order comes from [`Row::logical_cell_order`] in epaint — the shared
/// inverse-UAX #9-L2 replay over ranges into `Row::glyphs` (each run's level
/// is its first glyph's stamp, exactly what [`TextRunInfo::level`]
/// records). This function feeds it the row's non-empty runs and pins the
/// glyph-less runs last: a newline-only run from the
/// [`MAX_CHARS_PER_TEXT_RUN`] split — the newline terminates the row's text.
///
/// Known fence: wrapping walks the logical stream (`line_break_logically`)
/// and places each produced row per UAX #9 L1/L2 (`RowReorder::place_row`,
/// re-resolving the levels per row over the paragraph's whole byte span), and
/// each placed glyph is re-stamped with its row-resolved level — so this
/// replay is the exact inverse of the placement for every wrapped row. Past
/// the stamps remain: hostile byte drift, where the placement itself degrades
/// (clamped ranges, unclaimed blocks kept in logical order at the visual end)
/// and glyphs without a resolvable byte keep their emission level; and a
/// paragraph spanning several `LayoutSection`s whose single-row emission
/// split a run across the section boundary keeps the per-part visual order
/// (no re-placement runs on single-row text), which this replay then reads
/// with the whole-run schedule — the merged run reconstructs the parts in
/// reverse. Everything else reconstructs exactly.
fn logical_run_order(row: &Row, runs: &[TextRunInfo]) -> Vec<usize> {
    let non_empty: Vec<usize> = (0..runs.len())
        .filter(|&i| !runs[i].glyphs.is_empty())
        .collect();
    let ranges: Vec<Range<usize>> = non_empty.iter().map(|&i| runs[i].glyphs.clone()).collect();
    let mut order: Vec<usize> = row
        .logical_cell_order(&ranges)
        .into_iter()
        .map(|j| non_empty[j]) // the helper returns a permutation of 0..ranges.len()
        .collect();
    order.extend((0..runs.len()).filter(|&i| runs[i].glyphs.is_empty()));
    order
}

/// Convert a (row, column) layout cursor position to a text run node ID and character index,
/// accounting for rows that are split into multiple text runs.
///
/// `column` is a *logical* character offset within the row (what cursor
/// columns count since the glyphs' cluster ranges were fixed). A column on
/// the boundary between two runs is the start of the second run when the
/// cursor sticks to the char after it (`prefer_next_row`), and the end of the
/// first one otherwise, so the position names the run edge the cursor is
/// drawn at. The run is located by its *logical* char range (assigned over
/// [`RowTextRuns::logical_order`]) — never by its stream position, which
/// differs from the logical order inside right-to-left rows.
fn text_run_position(
    parent_id: Id,
    galley: &Galley,
    row: usize,
    column: CharIndex,
    prefer_next_row: bool,
) -> accesskit::TextPosition {
    let column = column.0;
    let fallback = || accesskit::TextPosition {
        node: parent_id.with(row).with(0usize).accesskit_id(),
        character_index: column,
    };
    let Some(placed_row) = galley.rows.get(row) else {
        return fallback();
    };
    let runs = RowTextRuns::new(placed_row);

    // Walk the runs in logical order, accumulating char counts:
    let mut start = 0usize;
    for &stream_index in &runs.logical_order {
        let end = start + runs.runs[stream_index].len();
        let matches = if prefer_next_row {
            column < end
        } else {
            column <= end
        };
        if matches {
            return accesskit::TextPosition {
                node: parent_id.with(row).with(stream_index).accesskit_id(),
                character_index: column - start,
            };
        }
        start = end;
    }

    // Past the row's end (the cursor clamps, so this is a degenerate galley):
    // clamp to the logically-last run.
    match runs.logically_last() {
        Some(stream_index) => {
            let mut start = 0usize;
            for &idx in &runs.logical_order {
                if idx == stream_index {
                    break;
                }
                start += runs.runs[idx].len();
            }
            accesskit::TextPosition {
                node: parent_id.with(row).with(stream_index).accesskit_id(),
                character_index: column.saturating_sub(start),
            }
        }
        None => fallback(),
    }
}

/// Update accesskit with the current text state.
///
/// This adds the [`accesskit::Role::TextRun`] children of the widget, and the
/// current text selection. It never touches the role of the widget itself: that
/// is set from the [`crate::WidgetInfo`] the widget reports, so a link stays a
/// link and a button stays a button.
///
/// While an IME composition is active on the widget (`composition` holds the
/// preedit's character range in the galley text — the preedit is already
/// reinserted into the buffer), the widget node is marked as a live region and
/// an [`accesskit::Role::ImeCandidate`] child announces the composing text with
/// its on-screen bounds. Both marks are rebuilt every pass: when composition
/// ends the child node is simply not built again, so adapters drop it.
///
/// Each run holds the *logical* text of one bidi-level stretch of a row, with
/// that stretch's direction (the row's glyph stream is in visual order, so the
/// logical text is reconstructed per run — cluster order reversed for
/// right-to-left runs — and the runs' logical order is reconstructed for
/// selection mapping and word starts). Character positions run from the run's
/// left edge for left-to-right runs and from its right edge for right-to-left
/// runs, as AccessKit expects: they grow in logical order and stay inside the
/// run.
pub fn update_accesskit_for_text_widget(
    ctx: &Context,
    widget_id: Id,
    cursor_range: Option<CCursorRange>,
    global_from_galley: TSTransform,
    galley: &Galley,
    composition: Option<Range<CharIndex>>,
) {
    let parent_id = ctx.accesskit_node_builder(widget_id, |builder| {
        let parent_id = widget_id;

        // Announce the composition's text changes to assistive technology
        // while it is active (cleared by simply not setting it next pass —
        // an AccessKit node replaces its previous serialization wholesale):
        if composition.is_some() {
            builder.set_live(accesskit::Live::Polite);
        }

        if let Some(cursor_range) = &cursor_range {
            let anchor = galley.layout_from_cursor(cursor_range.secondary);
            let focus = galley.layout_from_cursor(cursor_range.primary);
            builder.set_text_selection(accesskit::TextSelection {
                anchor: text_run_position(
                    parent_id,
                    galley,
                    anchor.row,
                    anchor.column,
                    cursor_range.secondary.prefer_next_row,
                ),
                focus: text_run_position(
                    parent_id,
                    galley,
                    focus.row,
                    focus.column,
                    cursor_range.primary.prefer_next_row,
                ),
            });
        }

        parent_id
    });

    let Some(parent_id) = parent_id else {
        return;
    };

    // The UI language is stamped on the AccessKit root and inherited by every
    // node (see `Context::begin_pass`); a text run whose content language is
    // known AND differs from it re-stamps the property here.
    let ui_language = ctx.language();

    let mut prev_row_ended_with_newline = true;

    for (row_index, placed_row) in galley.rows.iter().enumerate() {
        let runs = RowTextRuns::new(placed_row);

        // For soft-wrapped continuation rows, treat the start as a word
        // boundary so the first word character gets a `word_starts` entry.
        // Paragraph-starting runs (first row or after a newline) get an
        // implicit word start from AccessKit, so they don't need this.
        let mut was_at_word_end = !prev_row_ended_with_newline;

        // Word starts, as indices into the row's *logical* text (the run
        // values joined in logical order — the only order that matches the
        // `value` strings AccessKit indexes them against):
        let mut word_starts = Vec::<usize>::new();
        let mut logical_char = 0usize;
        for &stream_index in &runs.logical_order {
            let run = &runs.runs[stream_index];
            for glyph_index in run.logical_glyphs(&placed_row.row) {
                let is_word = is_word_char(placed_row.row.glyphs[glyph_index].chr);
                if is_word && was_at_word_end {
                    word_starts.push(logical_char);
                }
                was_at_word_end = !is_word;
                logical_char += 1;
            }
            if run.newline {
                // The '\n' occupies a value slot but is never a word start:
                logical_char += 1;
            }
        }

        // Per-run node data, computed before building nodes (which need the
        // context mutably):
        let row_rect = placed_row.rect_without_leading_space();
        let run_data: Vec<RunNodeData> = runs
            .runs
            .iter()
            .map(|run| {
                let logical_glyphs = run.logical_glyphs(&placed_row.row);
                let is_rtl = run.level % 2 == 1 && !logical_glyphs.is_empty();

                // The run's horizontal extent, row-relative:
                let (x0, x1) = if logical_glyphs.is_empty() {
                    (
                        row_rect.min.x - placed_row.pos.x,
                        row_rect.max.x - placed_row.pos.x,
                    )
                } else {
                    logical_glyphs.iter().fold(
                        (f32::INFINITY, f32::NEG_INFINITY),
                        |(x0, x1), &glyph_index| {
                            let glyph = &placed_row.row.glyphs[glyph_index];
                            (x0.min(glyph.pos.x), x1.max(glyph.max_x()))
                        },
                    )
                };

                let mut value = String::with_capacity(run.len());
                let mut character_lengths = Vec::<u8>::with_capacity(run.len());
                let mut character_positions = Vec::<f32>::with_capacity(run.len());
                let mut character_widths = Vec::<f32>::with_capacity(run.len());
                for &glyph_index in &logical_glyphs {
                    let glyph = &placed_row.row.glyphs[glyph_index];
                    let old_len = value.len();
                    value.push(glyph.chr);
                    character_lengths.push((value.len() - old_len) as _);
                    // Right-to-left runs measure from their right edge, so
                    // positions grow in logical order:
                    character_positions.push(if is_rtl {
                        x1 - glyph.max_x()
                    } else {
                        glyph.pos.x - x0
                    });
                    character_widths.push(glyph.advance_width);
                }
                if run.newline {
                    // The newline, at the end of the run:
                    value.push('\n');
                    character_lengths.push(1);
                    character_positions.push(x1 - x0);
                    character_widths.push(0.0);
                }

                RunNodeData {
                    value,
                    character_lengths,
                    character_positions,
                    character_widths,
                    is_rtl,
                    x0,
                    x1,
                }
            })
            .collect();

        for (run_index, data) in run_data.iter().enumerate() {
            let run_id = parent_id.with(row_index).with(run_index);
            ctx.register_accesskit_parent(run_id, parent_id);

            let logical_range = runs.logical_range(run_index);
            let run_word_starts: Vec<u8> = word_starts
                .iter()
                .filter(|&&ws| ws >= logical_range.start && ws < logical_range.end)
                .map(|&ws| (ws - logical_range.start) as u8)
                .collect();

            ctx.accesskit_node_builder(run_id, |builder| {
                builder.set_role(accesskit::Role::TextRun);
                builder.set_text_direction(if data.is_rtl {
                    accesskit::TextDirection::RightToLeft
                } else {
                    accesskit::TextDirection::LeftToRight
                });
                if let Some(language) = &galley.job.language
                    && ui_language.as_deref() != Some(language.as_str())
                {
                    builder.set_language(language.clone());
                }
                // TODO(mwcampbell): Set more node fields for the row
                // once AccessKit adapters expose text formatting info.

                // Neighbors link in stream (visual) order — the order the run
                // nodes are created (and traversed) in:
                if run_index > 0 {
                    let prev_id = parent_id.with(row_index).with(run_index - 1);
                    builder.set_previous_on_line(prev_id.accesskit_id());
                }
                if run_index + 1 < runs.runs.len() {
                    let next_id = parent_id.with(row_index).with(run_index + 1);
                    builder.set_next_on_line(next_id.accesskit_id());
                }

                let run_rect = emath::Rect::from_min_max(
                    emath::pos2(placed_row.pos.x + data.x0, row_rect.min.y),
                    emath::pos2(placed_row.pos.x + data.x1, row_rect.max.y),
                );
                let rect = global_from_galley * run_rect;
                builder.set_bounds(accesskit::Rect {
                    x0: rect.min.x.into(),
                    y0: rect.min.y.into(),
                    x1: rect.max.x.into(),
                    y1: rect.max.y.into(),
                });
                builder.set_value(data.value.clone());
                builder.set_character_lengths(data.character_lengths.clone());

                builder.set_character_positions(data.character_positions.clone());
                builder.set_character_widths(data.character_widths.clone());

                builder.set_word_starts(run_word_starts);
            });
        }

        prev_row_ended_with_newline = placed_row.ends_with_newline;
    }

    // While an IME composition is active, expose the composing text as its own
    // node so assistive technology has something concrete to announce even
    // without live-region support. The node goes through the same per-pass
    // machinery as the run nodes: when the composition ends it is simply not
    // built again, so it disappears from the tree without explicit removal.
    if let Some(composition) = composition {
        let ime_id = parent_id.with("ime_composition");
        ctx.register_accesskit_parent(ime_id, parent_id);

        // The preedit occupies `composition` in the laid-out text (for a
        // password field that text is the masked buffer, never the plaintext):
        let text = &galley.job.text;
        let byte_range = byte_index_from_char_index(text, composition.start).0
            ..byte_index_from_char_index(text, composition.end).0;
        let value = text.get(byte_range).unwrap_or_default().to_owned();

        // The preedit's on-screen bounds, mapped to screen space by the same
        // transform the run nodes use. `from_two_pos` normalizes: on a
        // right-to-left row the reading-start caret sits right of the end.
        let start_rect = galley.pos_from_cursor(CCursor {
            index: composition.start,
            prefer_next_row: true,
        });
        let end_rect = galley.pos_from_cursor(CCursor {
            index: composition.end,
            prefer_next_row: false,
        });
        let rect = global_from_galley * emath::Rect::from_two_pos(start_rect.min, end_rect.max);

        ctx.accesskit_node_builder(ime_id, |builder| {
            builder.set_role(accesskit::Role::ImeCandidate);
            builder.set_value(value);
            builder.set_bounds(accesskit::Rect {
                x0: rect.min.x.into(),
                y0: rect.min.y.into(),
                x1: rect.max.x.into(),
                y1: rect.max.y.into(),
            });
        });
    }
}

/// Everything a run node needs, precomputed per row pass.
struct RunNodeData {
    value: String,
    character_lengths: Vec<u8>,
    character_positions: Vec<f32>,
    character_widths: Vec<f32>,
    is_rtl: bool,
    /// The run's horizontal extent, row-relative.
    x0: f32,
    x1: f32,
}

#[cfg(test)]
mod tests {
    use accesskit::{NodeId, Role, TextDirection, TreeUpdate};
    use epaint::text::cursor::CCursor;

    use super::{MAX_CHARS_PER_TEXT_RUN, RowTextRuns, row_text_runs, text_run_position};
    use crate::text_selection::cursor_range::ccursor_from_accesskit_text_position;
    use crate::{Color32, Context, FontId, Id, RawInput, Ui};

    fn accesskit_update(ctx: &Context, mut add_contents: impl FnMut(&mut Ui)) -> TreeUpdate {
        ctx.enable_accesskit();
        let mut output = ctx.run_ui(RawInput::default(), |ui| add_contents(ui));
        output.textures_delta.clear();
        output
            .platform_output
            .accesskit_update
            .expect("Missing accesskit update")
    }

    fn node(update: &TreeUpdate, id: NodeId) -> &accesskit::Node {
        &update
            .nodes
            .iter()
            .find(|(node_id, _)| *node_id == id)
            .expect("node in update")
            .1
    }

    fn label<'a>(update: &'a TreeUpdate, text: &str) -> &'a accesskit::Node {
        &update
            .nodes
            .iter()
            .find(|(_, node)| node.role() == Role::Label && node.value() == Some(text))
            .unwrap_or_else(|| panic!("No label {text:?}"))
            .1
    }

    /// `(value, direction)` of each text run of a label, in tree (stream) order.
    fn runs(update: &TreeUpdate, text: &str) -> Vec<(String, TextDirection)> {
        label(update, text)
            .children()
            .iter()
            .map(|&id| node(update, id))
            .filter(|run| run.role() == Role::TextRun)
            .map(|run| {
                (
                    run.value().expect("run value").to_owned(),
                    run.text_direction().expect("run direction"),
                )
            })
            .collect()
    }

    /// Runs of one label in tree order, as `(id, value, direction)`:
    fn run_ids(update: &TreeUpdate, text: &str) -> Vec<(NodeId, String, TextDirection)> {
        label(update, text)
            .children()
            .iter()
            .map(|&id| {
                let run = node(update, id);
                (
                    id,
                    run.value().expect("run value").to_owned(),
                    run.text_direction().expect("run direction"),
                )
            })
            .collect()
    }

    #[test]
    fn text_runs_have_their_direction_and_logical_text() {
        let ctx = Context::default();
        let arabic = "مرحبا بالعالم";
        let mixed = "Hello שלום world";
        let update = accesskit_update(&ctx, |ui| {
            ui.label(arabic);
            ui.label(mixed);
        });

        // An RTL run's value is its logical text (not the visual glyph stream,
        // which lists the letters left-to-right), with no spurious glyphs:
        assert_eq!(
            runs(&update, arabic),
            [(arabic.to_owned(), TextDirection::RightToLeft)]
        );
        assert_eq!(
            runs(&update, mixed),
            [
                ("Hello ".to_owned(), TextDirection::LeftToRight),
                ("שלום".to_owned(), TextDirection::RightToLeft),
                (" world".to_owned(), TextDirection::LeftToRight),
            ]
        );

        // Positions of a right-to-left run count from its right edge,
        // so they grow in logical order and stay inside the run.
        let run = node(&update, label(&update, arabic).children()[0]);
        let positions = run.character_positions().expect("positions");
        let widths = run.character_widths().expect("widths");
        let bounds = run.bounds().expect("bounds");
        assert_eq!(positions.len(), arabic.chars().count());
        assert!(positions[0].abs() < 0.01, "{positions:?}");
        assert!(positions.is_sorted(), "{positions:?}");
        let end = positions.last().expect("non-empty") + widths.last().expect("non-empty");
        assert!(
            (f64::from(end) - bounds.width()).abs() < 0.01,
            "{end} vs {}",
            bounds.width()
        );

        // Runs on one line link to each other in tree order:
        let ids = label(&update, mixed).children();
        assert_eq!(node(&update, ids[0]).next_on_line(), Some(ids[1]));
        assert_eq!(node(&update, ids[2]).previous_on_line(), Some(ids[1]));
    }

    /// Word starts index the run's *logical* value: in a mixed row the second
    /// run's word starts at its own char 0 even though the row's glyph stream
    /// lists it elsewhere.
    #[test]
    fn word_starts_follow_the_logical_text() {
        let ctx = Context::default();
        let mixed = "Hello שלום world";
        let update = accesskit_update(&ctx, |ui| {
            ui.label(mixed);
        });

        let ids = label(&update, mixed).children();
        assert_eq!(ids.len(), 3, "{:?}", runs(&update, mixed));

        // Logical text "Hello שלום world": words at logical chars 6 and 11
        // (the paragraph's first word start is implicit in AccessKit, see the
        // `was_at_word_end` seeding). Run 0 = logical [0..6) = "Hello " (no
        // word starts), run 1 = [6..10) = "שלום" (word at its local 0), run
        // 2 = [10..16) = " world" (the word starts after its leading space,
        // at local 1):
        let word_starts = |id: NodeId| node(&update, id).word_starts();
        assert_eq!(word_starts(ids[0]), &[][..]);
        assert_eq!(
            word_starts(ids[1]),
            &[0u8][..],
            "שלום starts its own run value"
        );
        assert_eq!(word_starts(ids[2]), &[1u8][..], "'world' follows the space");
    }

    /// A right-to-left row stores its glyphs in visual order: the tree lists
    /// the runs visually (digits leftmost for `عربي 123`), but every run's
    /// value is its logical text and the run values joined in *logical* order
    /// read back the original string. Selection columns (logical offsets)
    /// map onto the right run and character index, and round-trip.
    #[test]
    fn rtl_base_row_maps_logical_columns_through_the_right_runs() {
        let ctx = Context::default();
        let id = Id::unique("rtl_text");
        let arabic = "عربي 123";
        let mut galley_slot = None;
        let update = accesskit_update(&ctx, |ui| {
            let galley = ui.painter().layout_no_wrap(
                arabic.to_owned(),
                FontId::proportional(14.0),
                Color32::WHITE,
            );
            assert!(galley.is_rtl(), "the label must resolve RTL");
            assert_eq!(galley.rows.len(), 1);
            // Visual run order: the digits at the visual left come first:
            let stream_runs = row_text_runs(&galley.rows[0]);
            assert_eq!(stream_runs.len(), 2, "digits and arabic split");
            galley_slot = Some(galley);
            ui.label(arabic);
        });

        let tree_runs = run_ids(&update, arabic);
        assert_eq!(
            tree_runs
                .iter()
                .map(|(_, value, _)| value.as_str())
                .collect::<Vec<_>>(),
            // Visual (stream) order: digits run first (visual left), then the
            // RTL run. Each value is the run's logical text:
            ["123", "عربي "],
            "{tree_runs:?}"
        );
        assert_eq!(tree_runs[0].2, TextDirection::LeftToRight); // level 2 digits
        assert_eq!(tree_runs[1].2, TextDirection::RightToLeft); // level 1 arabic

        // The values joined in LOGICAL order (arabic run first, digits second)
        // read back the original string:
        assert_eq!(
            format!("{}{}", tree_runs[1].1, tree_runs[0].1),
            arabic,
            "the logical run order reconstructs the text"
        );

        // Every logical column of the row maps to a run node + character
        // index, and round-trips back to the same column through the
        // reverse mapping (what an AT-driven `SetTextSelection` uses):
        let galley = galley_slot.expect("galley");
        let n = arabic.chars().count();
        for column in 0..=n {
            for prefer_next_row in [false, true] {
                let cursor = CCursor {
                    index: epaint::text::CharIndex(column),
                    prefer_next_row,
                };
                let layout = galley.layout_from_cursor(cursor);
                let position =
                    text_run_position(id, &galley, layout.row, layout.column, prefer_next_row);
                let back = ccursor_from_accesskit_text_position(id, &galley, &position)
                    .expect("position maps back");
                assert_eq!(back.index, cursor.index, "{cursor:?} -> {position:?}");
                // The caret stays put (a TextPosition cannot carry the
                // `prefer_next_row` flag; the reverse mapping re-derives it):
                assert_eq!(
                    galley.pos_from_cursor(back),
                    galley.pos_from_cursor(cursor),
                    "{cursor:?} {position:?}"
                );
            }
        }

        // Column 0 (logical start) is the START of the arabic run (stream
        // index 1); the row end is the END of the digits run (stream 0):
        let start = text_run_position(id, &galley, 0, epaint::text::CharIndex(0), false);
        assert_eq!(
            start.node,
            id.with(0usize).with(1usize).accesskit_id(),
            "column 0 must map to the visually-second (logically-first) run"
        );
        assert_eq!(start.character_index, 0);
        let end = text_run_position(id, &galley, 0, epaint::text::CharIndex(n), false);
        assert_eq!(end.node, id.with(0usize).with(0usize).accesskit_id());
        assert_eq!(end.character_index, 3, "the digits run holds 3 chars");
    }

    /// A wrapped right-to-left label whose row break lands inside the
    /// embedded latin island: the row's break space resets to the base level
    /// on the rendered row, so it is its own right-to-left run instead of
    /// folding into the island's run as a leading character, the island's run
    /// value is exactly its own text, and the run values joined in logical
    /// order read back the row's logical text. Selection columns round-trip
    /// through the run mapping with the caret staying put.
    #[test]
    fn wrapped_rtl_base_island_break_reconstructs_the_row_text() {
        let ctx = Context::default();
        let id = Id::unique("rtl_wrapped");
        let text = "عربي alpha beta gamma";
        let mut galley_slot = None;
        let update = accesskit_update(&ctx, |ui| {
            // Wrap width measured from an unwrapped twin: row 0 keeps
            // everything but the island's last word, whose glyphs lead the
            // twin's visual stream (a right-to-left row reads from its right
            // edge).
            let mut job = epaint::text::LayoutJob::simple(
                text.to_owned(),
                FontId::proportional(14.0),
                Color32::WHITE,
                f32::INFINITY,
            );
            job.direction = Some(epaint::text::TextDirection::RightToLeft);
            let twin = ui.painter().layout_job(job.clone());
            assert_eq!(twin.rows.len(), 1);
            let last_word: f32 = twin.rows[0]
                .glyphs
                .iter()
                .rev()
                .filter(|glyph| glyph.advance_width > 0.0)
                .take_while(|glyph| !glyph.chr.is_whitespace())
                .map(|glyph| glyph.advance_width)
                .sum();
            let row0_width = twin.rows[0].row.size.x - last_word;
            let wrap_width = row0_width + 0.5;
            assert!(
                row0_width > 0.0 && row0_width + last_word > wrap_width && last_word <= wrap_width,
                "the break must fall before the island's last word: \
                 row0 {row0_width} word {last_word} W {wrap_width}"
            );

            let mut wrapped = job;
            wrapped.wrap.max_width = wrap_width;
            let galley = ui.painter().layout_job(wrapped.clone());
            // FIRST, with a clear message — any wrap or shaping deviation
            // makes the run assertions below meaningless:
            assert_eq!(
                galley
                    .rows
                    .iter()
                    .map(|row| row.glyphs.len())
                    .collect::<Vec<_>>(),
                [16, 5],
                "rows must split `عربي alpha beta ` / `gamma`, one glyph slot per char"
            );
            galley_slot = Some(galley);
            ui.set_max_width(wrap_width);
            ui.add(crate::Label::new(wrapped).wrap_mode(crate::TextWrapMode::Wrap));
        });

        // Row 0's runs in tree (stream) order — the break space first (the
        // visual reading end of a right-to-left row), then the island, then
        // the arabic word; row 1 adds the island's last word:
        let tree_runs = run_ids(&update, text);
        let values: Vec<&str> = tree_runs
            .iter()
            .map(|(_, value, _)| value.as_str())
            .collect();
        assert_eq!(
            values,
            [" ", "alpha beta", "عربي ", "gamma"],
            "{tree_runs:?}"
        );
        assert_eq!(
            tree_runs[0].2,
            TextDirection::RightToLeft,
            "the break space is its own base-level run"
        );
        assert_eq!(tree_runs[1].2, TextDirection::LeftToRight);
        assert_eq!(tree_runs[2].2, TextDirection::RightToLeft);

        // The values joined in LOGICAL order read back the row's logical
        // text `عربي alpha beta ` — the break space at its logical end, not
        // inside the island's run:
        assert_eq!(
            format!("{}{}{}", tree_runs[2].1, tree_runs[1].1, tree_runs[0].1),
            "عربي alpha beta ",
            "the logical run order reconstructs the row's text"
        );

        // Every logical column of the text maps to a run node + character
        // index, and round-trips back to the same column through the reverse
        // mapping, with the caret staying put:
        let galley = galley_slot.expect("galley");
        let n = text.chars().count();
        for column in 0..=n {
            for prefer_next_row in [false, true] {
                let cursor = CCursor {
                    index: epaint::text::CharIndex(column),
                    prefer_next_row,
                };
                let layout = galley.layout_from_cursor(cursor);
                let position =
                    text_run_position(id, &galley, layout.row, layout.column, prefer_next_row);
                let back = ccursor_from_accesskit_text_position(id, &galley, &position)
                    .expect("position maps back");
                assert_eq!(back.index, cursor.index, "{cursor:?} -> {position:?}");
                assert_eq!(
                    galley.pos_from_cursor(back),
                    galley.pos_from_cursor(cursor),
                    "{cursor:?} {position:?}"
                );
            }
        }
    }

    /// A paragraph spanning two `LayoutSection`s wraps as ONE paragraph: the
    /// per-row reorder resolves the section-boundary space into the Hebrew
    /// run and the digits following it to their own level, so the row's tree
    /// runs join to the row's logical text — the row is NOT announced
    /// backwards (replaying the per-section emission stamps would read the
    /// L2 schedule over the wrong levels).
    #[test]
    fn wrapped_two_section_row_is_not_announced_backwards() {
        let ctx = Context::default();
        let id = Id::unique("two_section_wrapped");
        let text = "עברית 123 abcdef";
        let mut galley_slot = None;
        let update = accesskit_update(&ctx, |ui| {
            // Two differently-colored sections; the paragraph spans the
            // boundary. The pure-ASCII section's part stamps everything at
            // the base level at emission:
            let mut job = epaint::text::LayoutJob::default();
            job.append(
                "עברית",
                0.0,
                epaint::text::TextFormat::simple(FontId::proportional(14.0), Color32::RED),
            );
            job.append(
                " 123 abcdef",
                0.0,
                epaint::text::TextFormat::simple(FontId::proportional(14.0), Color32::WHITE),
            );
            job.direction = Some(epaint::text::TextDirection::LeftToRight);

            // Wrap width measured from an unwrapped twin: row 0 keeps
            // everything but the trailing latin word.
            let twin = ui.painter().layout_job(job.clone());
            assert_eq!(twin.rows.len(), 1);
            let last_word: f32 = twin.rows[0]
                .glyphs
                .iter()
                .rev()
                .filter(|glyph| glyph.advance_width > 0.0)
                .take_while(|glyph| !glyph.chr.is_whitespace())
                .map(|glyph| glyph.advance_width)
                .sum();
            let row0_width = twin.rows[0].row.size.x - last_word;
            let wrap_width = row0_width + 0.5;
            assert!(
                row0_width > 0.0 && row0_width + last_word > wrap_width && last_word <= wrap_width,
                "the break must fall after the boundary space: \
                 row0 {row0_width} word {last_word} W {wrap_width}"
            );

            let mut wrapped = job.clone();
            wrapped.wrap.max_width = wrap_width;
            let galley = ui.painter().layout_job(wrapped.clone());
            // FIRST, with a clear message:
            assert_eq!(
                galley
                    .rows
                    .iter()
                    .map(|row| row.glyphs.len())
                    .collect::<Vec<_>>(),
                [10, 6],
                "rows must split `עברית 123 ` / `abcdef`, one glyph slot per char"
            );
            galley_slot = Some(galley);
            ui.set_max_width(wrap_width);
            ui.add(crate::Label::new(job).wrap_mode(crate::TextWrapMode::Wrap));
        });

        // Row 0's runs in tree (stream) order — the digits at the visual
        // left, then the Hebrew run with the boundary space joined to it,
        // then the kept break space; row 1's latin word last:
        let tree_runs = run_ids(&update, text);
        let values: Vec<&str> = tree_runs
            .iter()
            .map(|(_, value, _)| value.as_str())
            .collect();
        assert_eq!(values, ["123", "עברית ", " ", "abcdef"], "{tree_runs:?}");
        assert_eq!(tree_runs[0].2, TextDirection::LeftToRight);
        assert_eq!(tree_runs[1].2, TextDirection::RightToLeft);
        assert_eq!(tree_runs[2].2, TextDirection::LeftToRight);

        // The values joined in LOGICAL order read back the row's logical
        // text `עברית 123 ` — not the backwards announcement the emission
        // stamps would produce:
        assert_eq!(
            format!("{}{}{}", tree_runs[1].1, tree_runs[0].1, tree_runs[2].1),
            "עברית 123 ",
            "the logical run order reconstructs the row's text"
        );

        // Selection round-trip for every column of row 0:
        let galley = galley_slot.expect("galley");
        for column in 0..=10usize {
            for prefer_next_row in [false, true] {
                let cursor = CCursor {
                    index: epaint::text::CharIndex(column),
                    prefer_next_row,
                };
                let layout = galley.layout_from_cursor(cursor);
                let position =
                    text_run_position(id, &galley, layout.row, layout.column, prefer_next_row);
                let back = ccursor_from_accesskit_text_position(id, &galley, &position)
                    .expect("position maps back");
                assert_eq!(back.index, cursor.index, "{cursor:?} -> {position:?}");
                assert_eq!(
                    galley.pos_from_cursor(back),
                    galley.pos_from_cursor(cursor),
                    "{cursor:?} {position:?}"
                );
            }
        }
    }

    /// Selection positions round-trip through the run mapping on mixed
    /// multi-row text (like an AT-driven selection would).
    #[test]
    fn text_positions_round_trip_through_runs() {
        let ctx = Context::default();
        let id = Id::unique("text");
        accesskit_update(&ctx, |ui| {
            let text = "abc אבג 12 def\nשלום";
            let galley = ui.painter().layout_no_wrap(
                text.to_owned(),
                FontId::proportional(14.0),
                Color32::WHITE,
            );
            assert!(row_text_runs(&galley.rows[0]).len() >= 3);
            for index in 0..=text.chars().count() {
                for prefer_next_row in [false, true] {
                    let cursor = CCursor {
                        index: epaint::text::CharIndex(index),
                        prefer_next_row,
                    };
                    let layout = galley.layout_from_cursor(cursor);
                    let position =
                        text_run_position(id, &galley, layout.row, layout.column, prefer_next_row);
                    let back = ccursor_from_accesskit_text_position(id, &galley, &position)
                        .expect("position maps back");
                    assert_eq!(back.index, cursor.index, "{position:?}");
                    // Where the direction changes, the caret stays at the same run edge:
                    assert_eq!(
                        galley.pos_from_cursor(back),
                        galley.pos_from_cursor(cursor),
                        "{cursor:?} {position:?}"
                    );
                }
            }
        });
    }

    /// Long pure-LTR text keeps today's chunking: 255-char chunks, ids
    /// `with(row).with(chunk)`, values that concatenate back to the text, and
    /// chunk-to-chunk links.
    #[test]
    fn long_ltr_rows_chunk_every_255_chars() {
        let ctx = Context::default();
        let text = "ab ".repeat(100); // 300 chars, words at every multiple of 3
        let update = accesskit_update(&ctx, |ui| {
            ui.add(crate::Label::new(text.clone()).wrap_mode(crate::TextWrapMode::Extend));
        });

        let ids = label(&update, &text).children();
        assert_eq!(
            ids.len(),
            300usize.div_ceil(MAX_CHARS_PER_TEXT_RUN),
            "two 255-capped chunks"
        );
        let values: Vec<String> = ids
            .iter()
            .map(|&id| node(&update, id).value().expect("value").to_owned())
            .collect();
        assert_eq!(values.concat(), text);
        assert_eq!(values[0].chars().count(), MAX_CHARS_PER_TEXT_RUN);
        assert_eq!(node(&update, ids[0]).next_on_line(), Some(ids[1]));
        assert_eq!(node(&update, ids[1]).previous_on_line(), Some(ids[0]));
        // Word starts are indices into each chunk's value. The paragraph's
        // first word start is implicit in AccessKit (not pushed), so chunk 0
        // holds the words at 3, 6, …, 252 (84 of them); the word at logical
        // char 255 opens chunk 1 at its own char 0 (15 more to the end):
        let chunk0 = node(&update, ids[0]).word_starts();
        let chunk1 = node(&update, ids[1]).word_starts();
        assert_eq!(chunk0.len(), 84);
        assert_eq!(&chunk0[..3], &[3u8, 6, 9]);
        assert_eq!(chunk1.len(), 15);
        assert_eq!(&chunk1[..3], &[0u8, 3, 6]);
    }

    /// A >255-char right-to-left row with a trailing newline: the 255-char
    /// split yields a glyph-less newline-only second run, which
    /// [`RowTextRuns`] clamps empty, keeps out of the logical-order replay,
    /// and pins logically last (the newline terminates the row's text). The
    /// LTR sibling (`long_ltr_rows_chunk_every_255_chars`) covers only the
    /// identity permutation; this walks the RTL path through the real
    /// pipeline.
    #[test]
    fn long_rtl_row_pins_the_glyph_less_newline_run_last() {
        let ctx = Context::default();
        let text = format!("{}\nx", "عربي ".repeat(51)); // 255 chars, then '\n', then 'x'
        let mut galley_slot = None;
        let update = accesskit_update(&ctx, |ui| {
            let galley = ui.painter().layout(
                text.clone(),
                FontId::proportional(14.0),
                Color32::WHITE,
                f32::INFINITY,
            );
            galley_slot = Some(galley);
            ui.add(crate::Label::new(text.clone()).wrap_mode(crate::TextWrapMode::Extend));
        });

        let galley = galley_slot.expect("galley");
        assert!(galley.is_rtl(), "the text must resolve RTL (first-strong)");
        assert_eq!(galley.rows.len(), 2, "the newline splits the text");
        // FIRST, with a clear message: the exact two-run shape below IS the
        // 255-split proof. Any deviation (shaping ≠ 1 glyph per char, an
        // unexpected wrap) makes every later assertion meaningless:
        assert_eq!(
            galley.rows[0].glyphs.len(),
            MAX_CHARS_PER_TEXT_RUN,
            "row 0 must hold exactly 255 glyphs"
        );
        assert!(galley.rows[0].ends_with_newline);
        assert_eq!(
            row_text_runs(&galley.rows[0]),
            [0..MAX_CHARS_PER_TEXT_RUN, 255..256],
            "the 255-split: one full run plus a glyph-less newline-only tail"
        );

        let row_runs = RowTextRuns::new(&galley.rows[0]);
        let last = row_runs.logically_last().expect("a logically last run");
        assert!(
            row_runs.runs[last].glyphs.is_empty(),
            "the glyph-less run must be pinned logically last"
        );
        assert_eq!(
            row_runs.logical_range(last),
            255..256,
            "the newline terminates the row's logical text"
        );

        // The row's tree runs, joined in logical order, read back the row's
        // text with the newline last (label children: row 0's two runs, then
        // row 1's single run):
        let ids = label(&update, &text).children();
        let values: Vec<String> = ids
            .iter()
            .map(|&id| node(&update, id).value().expect("value").to_owned())
            .collect();
        assert_eq!(values.len(), 3, "two runs on row 0, one on row 1");
        assert_eq!(values[2], "x", "row 1 holds the char after the newline");
        assert_eq!(values[1], "\n", "the glyph-less run's value is the newline");
        let row0_logical_text: String = row_runs
            .logical_order
            .iter()
            .map(|&stream_index| values[stream_index].as_str())
            .collect();
        assert_eq!(row0_logical_text, format!("{}\n", "عربي ".repeat(51)));
    }

    /// The UI language is stamped on the AccessKit root (inherited by every
    /// node); a text run whose content language is known and different
    /// re-stamps it on its run nodes, content in the UI's language inherits.
    #[test]
    fn ui_language_on_the_root_and_content_language_on_text_runs() {
        let ctx = Context::default();
        ctx.set_language("ar");

        // A LayoutJob carrying its own language (what per-Ui language plumbing
        // fills in; plain labels leave it unset):
        let update = accesskit_update(&ctx, |ui| {
            ui.label("مرحبا");
            let mut job = epaint::text::LayoutJob::simple(
                "Bonjour".to_owned(),
                FontId::proportional(14.0),
                Color32::WHITE,
                f32::INFINITY,
            );
            job.language = Some("fr".to_owned());
            ui.add(crate::Label::new(crate::WidgetText::from(job)));
        });

        let root = node(&update, update.tree.as_ref().expect("tree").root);
        assert_eq!(root.language(), Some("ar"));

        // Plain label: its job has no language → the run inherits the root's:
        let arabic = label(&update, "مرحبا");
        let run = node(&update, arabic.children()[0]);
        assert_eq!(run.role(), Role::TextRun);
        assert_eq!(
            run.language(),
            None,
            "the run must inherit the root's language"
        );

        // Content in another language re-stamps the property on the run:
        let french = label(&update, "Bonjour");
        let run = node(&update, french.children()[0]);
        assert_eq!(run.role(), Role::TextRun);
        assert_eq!(run.language(), Some("fr"));
    }

    /// While an IME composition is active on a focused text edit, the widget
    /// is marked as a live region and an `ImeCandidate` child announces the
    /// composing text with its bounds. When the composition ends (here: a
    /// commit), both are gone again — the node is not built next pass.
    #[test]
    fn ime_composition_is_exposed_to_the_accesskit_tree() {
        use accesskit::Live;

        use crate::{Event, ImeEvent, TextEdit};

        let ctx = Context::default();
        ctx.enable_accesskit();
        let id = Id::unique("ime_a11y_test");
        let mut text = String::from("hello ");

        let run_frame = |text: &mut String, events: Vec<Event>| {
            let mut output = ctx.run_ui(
                RawInput {
                    time: Some(0.0),
                    events,
                    ..RawInput::default()
                },
                |ui| {
                    ui.memory_mut(|mem| mem.request_focus(id));
                    let out = TextEdit::singleline(text).id(id).show(ui);
                    out.state.store(ui.ctx(), id);
                },
            );
            output.textures_delta.clear();
            output
                .platform_output
                .accesskit_update
                .expect("accesskit update")
        };

        // Focus first (a frame without events), then compose:
        let _setup = run_frame(&mut text, vec![]);
        let composing = run_frame(
            &mut text,
            vec![Event::Ime(ImeEvent::Preedit {
                text: "مرحبا".to_owned(),
                active_range_chars: None,
            })],
        );

        // The text widget (its role comes from `WidgetInfo`, refined in the
        // bridge) carries the live flag while composing:
        let text_widget = &composing
            .nodes
            .iter()
            .find(|(_, node)| node.role() == Role::TextInput)
            .expect("text input node")
            .1;
        assert_eq!(text_widget.live(), Some(Live::Polite));

        // The composition text has its own child node:
        let candidate = &composing
            .nodes
            .iter()
            .find(|(_, node)| node.role() == Role::ImeCandidate)
            .expect("ImeCandidate node while composing")
            .1;
        assert_eq!(candidate.value(), Some("مرحبا"));
        assert!(candidate.bounds().is_some(), "the candidate has bounds");

        // Committing ends the composition: no candidate node and no live flag
        // in the next update:
        let committed = run_frame(
            &mut text,
            vec![Event::Ime(ImeEvent::Commit("مرحبا".to_owned()))],
        );
        assert!(
            !committed
                .nodes
                .iter()
                .any(|(_, node)| node.role() == Role::ImeCandidate),
            "the ImeCandidate node must be dropped when the composition ends"
        );
        let text_widget = &committed
            .nodes
            .iter()
            .find(|(_, node)| node.role() == Role::TextInput)
            .expect("text input node")
            .1;
        assert_eq!(
            text_widget.live(),
            None,
            "the live mark must be cleared when the composition ends"
        );
    }
}
