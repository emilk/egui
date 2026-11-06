use alloc::sync::Arc;

use core::ops::Range;

use emath::Pos2;
use epaint::{
    Stroke,
    text::{
        CharIndex, Row,
        cursor::{CCursor, LayoutCursor},
    },
};

use crate::{
    Galley, Painter, Rect, Ui, Visuals, pos2, text_selection::text_cursor_state::cursor_rect, vec2,
};

use super::CCursorRange;

#[derive(Clone, Debug)]
pub struct RowVertexIndices {
    pub row: usize,
    pub vertex_indices: [u32; 6],
}

/// Adds text selection rectangles to the galley.
pub fn paint_text_selection(
    galley: &mut Arc<Galley>,
    visuals: &Visuals,
    cursor_range: &CCursorRange,
    mut new_vertex_indices: Option<&mut Vec<RowVertexIndices>>,
) {
    if cursor_range.is_empty() {
        return;
    }

    // We need to modify the galley (add text selection painting to it),
    // and so we need to clone it if it is shared:
    let galley: &mut Galley = Arc::make_mut(galley);

    let background_color = visuals.selection.bg_fill;
    let text_color = visuals.selection.stroke.color;

    let [min, max] = cursor_range.sorted_cursors();
    let min = galley.layout_from_cursor(min);
    let max = galley.layout_from_cursor(max);

    for ri in min.row..=max.row {
        let placed_row = &mut galley.rows[ri];
        let row = Arc::make_mut(&mut placed_row.row);
        let rtl = row.base_direction == epaint::text::TextDirection::RightToLeft;
        // The row's ink right edge in row-relative coordinates — the reading
        // start of a right-to-left row (0.0 for an empty row):
        let ink_max_x = row
            .glyphs
            .iter()
            .map(|glyph| glyph.max_x())
            .fold(0.0, f32::max);

        let newline_size = if ri != max.row && placed_row.ends_with_newline {
            row.height() * 0.5 // visualize that we select the newline
        } else {
            0.0
        };

        // The row's selected logical column range, clamped into the row (a
        // cursor can carry any char index):
        let glyph_count = row.glyphs.len();
        let row_lo = if ri == min.row {
            min.column.0.clamp(0, glyph_count)
        } else {
            0
        };
        let row_hi = if ri == max.row {
            max.column.0.clamp(0, glyph_count)
        } else {
            glyph_count
        };

        // The selection pieces of this row. On a level-uniform row (pure
        // left-to-right, pure right-to-left, or a legacy deserialized galley
        // whose glyphs all stamp level 0) the code below is the single-span
        // path; on a row mixing both directions the glyphs are stored in
        // visual order while the columns are logical, so the selection
        // decomposes into one piece per visually-contiguous run group.
        let pieces: Vec<SelectionPiece> = if row_has_mixed_levels(row) && row_lo < row_hi {
            per_run_selection_pieces(
                row,
                rtl,
                ri,
                min.row,
                max.row,
                row_lo,
                row_hi,
                ink_max_x,
                newline_size,
            )
        } else {
            // The selected x span of this row, in row-relative coordinates. The
            // reading direction decides which edge the caret side pairs with: on
            // a right-to-left row the selection runs from a caret rightward to
            // the row's reading START (the ink's visual right edge) when the
            // range begins at the row's first column, and leftward to the row's
            // reading END (the visual left, the allocated width and the newline
            // visualization extending reading-forward) when the range continues
            // past it.
            let (span_lo, span_hi) = if ri == min.row && ri == max.row {
                // Single-row selection: between the two carets.
                let a = row.x_offset(min.column);
                let b = row.x_offset(max.column);
                (a.min(b), a.max(b))
            } else if ri == min.row {
                // From the first selected column's caret to the row's reading end:
                let caret = row.x_offset(min.column);
                let end = if rtl {
                    ink_max_x - row.size.x - newline_size
                } else {
                    row.size.x + newline_size
                };
                (caret.min(end), caret.max(end))
            } else if ri == max.row {
                // From the row's reading start to the last selected column's caret:
                let caret = row.x_offset(max.column);
                let start = if rtl { ink_max_x } else { 0.0 };
                (caret.min(start), caret.max(start))
            } else if rtl {
                // Full row: from the reading end (visual left) to the reading
                // start (visual right):
                (ink_max_x - row.size.x - newline_size, ink_max_x)
            } else {
                (0.0, row.size.x + newline_size)
            };

            let rect = Rect::from_min_max(pos2(span_lo, 0.0), pos2(span_hi, row.size.y));

            if row.glyphs.is_empty() {
                vec![SelectionPiece {
                    rect,
                    recolor_vertices: 0..0,
                }]
            } else {
                // Change color of the selected text:
                let len = row.glyphs.len();
                let mut first_glyph_index = if ri == min.row { min.column.0 } else { 0 };
                let mut last_glyph_index = if ri == max.row { max.column.0 } else { len };
                if rtl {
                    // Glyphs are stored in visual order while the columns are
                    // logical: mirror the slot range into the visual glyph slots.
                    let start = first_glyph_index.min(len);
                    let end = last_glyph_index.min(len);
                    first_glyph_index = len - end;
                    last_glyph_index = len - start;
                }

                let first_vertex_index = row
                    .glyphs
                    .get(first_glyph_index)
                    .map_or(row.visuals.glyph_vertex_range.end, |g| g.first_vertex as _);
                let last_vertex_index = row
                    .glyphs
                    .get(last_glyph_index)
                    .map_or(row.visuals.glyph_vertex_range.end, |g| g.first_vertex as _);

                vec![SelectionPiece {
                    rect,
                    recolor_vertices: first_vertex_index..last_vertex_index,
                }]
            }
        };

        let mesh = &mut row.visuals.mesh;

        // Change the color of the selected text, per piece:
        for piece in &pieces {
            for vi in piece.recolor_vertices.clone() {
                mesh.vertices[vi].color = text_color;
            }
        }

        // Time to insert the selection rectangles into the row mesh.
        // They should be on top (after) of any background in the galley,
        // but behind (before) any glyphs. The row visuals has this information:
        let glyph_index_start = row.visuals.glyph_index_start;

        // Start by appending the selection rectangles to end of the mesh, as two triangles (= 6 indices) each:
        let num_indices_before = mesh.indices.len();
        for piece in &pieces {
            mesh.add_colored_rect(piece.rect, background_color);
        }
        assert_eq!(
            num_indices_before + 6 * pieces.len(),
            mesh.indices.len(),
            "We expect exactly 6 new indices per selection rect"
        );

        // Copy out the new triangles, one pair per selection rect:
        let selection_triangles: Vec<[u32; 6]> = mesh.indices[num_indices_before..]
            .chunks_exact(6)
            .map(|tri| [tri[0], tri[1], tri[2], tri[3], tri[4], tri[5]])
            .collect();

        // Move every old triangle forwards by 6 indices per selection rect to
        // make room for the new triangles:
        let shift = 6 * pieces.len();
        for i in (glyph_index_start..num_indices_before).rev() {
            mesh.indices.swap(i, i + shift);
        }
        // Put the new triangles in place:
        for (offset, triangle) in selection_triangles.iter().enumerate() {
            let base = glyph_index_start + 6 * offset;
            mesh.indices[base..base + 6].clone_from_slice(triangle);
        }

        row.visuals.mesh_bounds = mesh.calc_bounds();

        if let Some(new_vertex_indices) = &mut new_vertex_indices {
            for triangle in &selection_triangles {
                new_vertex_indices.push(RowVertexIndices {
                    row: ri,
                    vertex_indices: *triangle,
                });
            }
        }
    }
}

/// One visually-contiguous selection piece of a row: the highlight rectangle
/// and the half-open range of mesh vertices to recolor (the vertices of the
/// piece's glyph slots, which are contiguous in the mesh).
struct SelectionPiece {
    rect: Rect,
    recolor_vertices: Range<usize>,
}

/// Whether the row's glyphs carry more than one distinct bidi level. A
/// level-uniform row keeps the single-span selection path.
fn row_has_mixed_levels(row: &Row) -> bool {
    let Some(first) = row.glyphs.first() else {
        return false;
    };
    let first_level = first.bidi_level;
    row.glyphs
        .iter()
        .any(|glyph| glyph.bidi_level != first_level)
}

/// Clusters the row's visually-ordered glyphs into cluster cells: half-open
/// ranges into `Row::glyphs`, where a new cell starts at glyph 0 and at
/// every glyph with a positive advance (zero-advance glyphs are the
/// continuations of the cluster before them) — the same grouping epaint's
/// `bidi_cluster_cells` and the accessibility bridge's `logical_glyphs` use.
fn cluster_cells(row: &Row) -> Vec<Range<usize>> {
    let mut cells: Vec<Range<usize>> = Vec::new();
    let mut start = 0;
    for (i, glyph) in row.glyphs.iter().enumerate().skip(1) {
        if glyph.advance_width > 0.0 {
            cells.push(start..i);
            start = i;
        }
    }
    if !row.glyphs.is_empty() {
        cells.push(start..row.glyphs.len());
    }
    cells
}

/// The selected glyph-slot groups of one row: the maximal runs of contiguous
/// visual slots whose cluster cells intersect the selected logical column
/// range `[row_lo, row_hi)`, sorted by slot start.
///
/// The cells are walked in reading order (`Row::logical_cell_order`),
/// assigning logical columns in cell-size steps — glyph slots are 1:1 with
/// chars, so the cells tile the row's columns exactly — and a cell is
/// selected iff its column range intersects the selection (selection
/// granularity is the cluster). A selection covering every column of the
/// row degenerates to one group over all slots.
fn selected_slot_groups(row: &Row, row_lo: usize, row_hi: usize) -> Vec<Range<usize>> {
    let cells = cluster_cells(row);
    let order = row.logical_cell_order(&cells);

    let mut groups: Vec<Range<usize>> = Vec::new();
    let mut column = 0usize;
    for cell_index in order {
        let Some(cell) = cells.get(cell_index) else {
            continue;
        };
        let cell_end = column.saturating_add(cell.len());
        // Whole-cell rule: the cell is selected iff [column, cell_end)
        // intersects [row_lo, row_hi):
        if column < row_hi && cell_end > row_lo {
            groups.push(cell.clone());
        }
        column = cell_end;
    }
    groups.sort_by_key(|group| group.start);

    // Merge adjacent slot ranges into maximal visually-contiguous groups:
    let mut merged: Vec<Range<usize>> = Vec::new();
    for group in groups {
        match merged.last_mut() {
            Some(last) if group.start <= last.end => {
                last.end = last.end.max(group.end);
            }
            _ => merged.push(group),
        }
    }
    merged
}

/// The selection pieces of a mixed row: one per selected slot group, each a
/// rectangle over the group's glyph box (glyph slots chain, so the box edges
/// are the caret edges at the group's boundaries) and a recolor range over
/// the group's glyph vertices.
///
/// The anchor extensions mirror the single-span arms: a single-row selection
/// only extends when it reaches the row's reading end (to the row's advance
/// width under a left-to-right base, to the row's leftmost ink edge under a
/// right-to-left base); a selection that only starts or only ends on the row
/// extends its reading-end (resp. reading-start) extreme group to the same
/// row-kind anchor, with the newline visualization; a middle row selects the
/// whole row (one merged group) and extends on both sides. The
/// visually-rightmost group is the reading end under a left-to-right base,
/// the visually-leftmost under a right-to-left base (the logically-last run
/// sits there by the reordering); reading start is the opposite extreme.
#[expect(clippy::too_many_arguments)]
fn per_run_selection_pieces(
    row: &Row,
    rtl: bool,
    row_index: usize,
    min_row: usize,
    max_row: usize,
    row_lo: usize,
    row_hi: usize,
    ink_max_x: f32,
    newline_size: f32,
) -> Vec<SelectionPiece> {
    let glyph_count = row.glyphs.len();
    let vertex_range_end = row.visuals.glyph_vertex_range.end;
    let groups = selected_slot_groups(row, row_lo, row_hi);
    // The row's leftmost ink edge — the reading end anchor of a
    // right-to-left row:
    let min_pos_x = row
        .glyphs
        .iter()
        .map(|glyph| glyph.pos.x)
        .fold(f32::INFINITY, f32::min);

    let single_row = row_index == min_row && row_index == max_row;
    let group_count = groups.len();
    let mut pieces = Vec::with_capacity(group_count);
    for (group_index, group) in groups.iter().enumerate() {
        let Some(first_glyph) = row.glyphs.get(group.start) else {
            continue;
        };
        let Some(last_glyph) = row.glyphs.get(group.end.saturating_sub(1)) else {
            continue;
        };
        let mut x_lo = first_glyph.pos.x;
        let mut x_hi = last_glyph.max_x();

        let reading_end_extreme = if rtl {
            group_index == 0
        } else {
            group_index + 1 == group_count
        };
        let reading_start_extreme = if rtl {
            group_index + 1 == group_count
        } else {
            group_index == 0
        };

        if single_row {
            // No anchor extensions, except when the selection reaches the
            // row's reading end:
            if row_hi >= glyph_count && reading_end_extreme {
                if rtl {
                    x_lo = x_lo.min(min_pos_x);
                } else {
                    x_hi = x_hi.max(row.size.x);
                }
            }
        } else if row_index == min_row {
            // From the first selected group to the row's reading end:
            if reading_end_extreme {
                if rtl {
                    x_lo = x_lo.min(ink_max_x - row.size.x - newline_size);
                } else {
                    x_hi = x_hi.max(row.size.x + newline_size);
                }
            }
        } else if row_index == max_row {
            // From the row's reading start to the last selected group:
            if reading_start_extreme {
                if rtl {
                    x_hi = x_hi.max(ink_max_x);
                } else {
                    x_lo = x_lo.min(0.0);
                }
            }
        } else {
            // Full row: one merged group, extended on both sides:
            if rtl {
                x_lo = x_lo.min(ink_max_x - row.size.x - newline_size);
                x_hi = x_hi.max(ink_max_x);
            } else {
                x_lo = x_lo.min(0.0);
                x_hi = x_hi.max(row.size.x + newline_size);
            }
        }

        let rect = Rect::from_min_max(pos2(x_lo, 0.0), pos2(x_hi, row.size.y));
        let recolor_start = first_glyph.first_vertex as usize;
        let recolor_end = row
            .glyphs
            .get(group.end)
            .map_or(vertex_range_end, |glyph| glyph.first_vertex as usize);
        pieces.push(SelectionPiece {
            rect,
            recolor_vertices: recolor_start..recolor_end,
        });
    }
    pieces
}

#[expect(clippy::too_many_arguments)]
pub(crate) fn paint_ime_preedit_text_visuals(
    pos: Pos2,
    ui: &Ui,
    painter: &Painter,
    galley: &Arc<Galley>,
    row_height: f32,
    preedit_range: core::ops::Range<CCursor>,
    mut relative_active_range: Option<core::ops::Range<CCursor>>,
    time_since_last_interaction: f64,
) {
    /// Instead of implementing [`PartialOrd`] and [`Ord`] for [`CCursor`] to
    /// make [`std::ops::Range::is_empty`] available, we use this helper
    /// function instead.
    ///
    /// These traits are intentionally not implemented because
    /// [`CCursor::prefer_next_row`] makes it difficult to define a clear
    /// ordering between two [`CCursor`]s.
    fn is_cursor_range_empty(range: &core::ops::Range<CCursor>) -> bool {
        range.start.index == range.end.index
    }

    if is_cursor_range_empty(&preedit_range) {
        return;
    }

    if let Some(relative_active_range) = &mut relative_active_range
        && relative_active_range.end.index > preedit_range.end.index - preedit_range.start.index
    {
        relative_active_range.end.index = preedit_range.end.index - preedit_range.start.index;
    }

    let visuals = ui.visuals();
    let active_underline_stroke = visuals.ime_composition.active_underline_stroke;
    let inactive_underline_stroke = visuals.ime_composition.inactive_underline_stroke;

    if let Some(relative_active_range) = &relative_active_range
        && !is_cursor_range_empty(relative_active_range)
    {
        if relative_active_range.start.index > CharIndex::ZERO {
            paint_underlines(
                pos,
                painter,
                galley,
                galley.layout_from_cursor(preedit_range.start),
                galley.layout_from_cursor(preedit_range.start + relative_active_range.start.index),
                inactive_underline_stroke,
            );
        }

        paint_underlines(
            pos,
            painter,
            galley,
            galley.layout_from_cursor(preedit_range.start + relative_active_range.start.index),
            galley.layout_from_cursor(preedit_range.start + relative_active_range.end.index),
            active_underline_stroke,
        );

        if !is_cursor_range_empty(
            &(relative_active_range.end..(preedit_range.end - preedit_range.start.index)),
        ) {
            paint_underlines(
                pos,
                painter,
                galley,
                galley.layout_from_cursor(preedit_range.start + relative_active_range.end.index),
                galley.layout_from_cursor(preedit_range.end),
                inactive_underline_stroke,
            );
        }
    } else {
        paint_underlines(
            pos,
            painter,
            galley,
            galley.layout_from_cursor(preedit_range.start),
            galley.layout_from_cursor(preedit_range.end),
            inactive_underline_stroke,
        );
    }

    if let Some(relative_active_range) = relative_active_range
        && is_cursor_range_empty(&relative_active_range)
    {
        let active_cursor = preedit_range.start + relative_active_range.start.index;
        let cursor_rect = cursor_rect(galley, &active_cursor, row_height);

        paint_text_cursor(
            ui,
            painter,
            cursor_rect.translate(pos.to_vec2()),
            time_since_last_interaction,
        );
    }
}

fn paint_underlines(
    pos: Pos2,
    painter: &Painter,
    galley: &Arc<Galley>,
    min: LayoutCursor,
    max: LayoutCursor,
    stroke: Stroke,
) {
    for ri in min.row..=max.row {
        let placed_row = &galley.rows[ri];
        let row = &placed_row.row;

        let offset_y = placed_row.pos.y + row.size.y;

        for (left, right) in underline_segments(row, ri, min.row, max.row, min.column, max.column) {
            painter.line_segment(
                [pos + vec2(left, offset_y), pos + vec2(right, offset_y)],
                stroke,
            );
        }
    }
}

/// The underline segments of one row, as row-relative x-ranges.
///
/// On a level-uniform row this is the single-span formula: from the first
/// selected column's caret (or the row's left edge when the selection
/// continues from an earlier row) to the last selected column's caret (or
/// the row's advance width when it continues into a later one). On a mixed
/// row the glyphs are stored in visual order while the columns are logical,
/// so the selection decomposes into one segment per visually-contiguous run
/// group, each over the group's glyph box; a group stretches to the row's
/// left edge when the selection continues from an earlier row, and to the
/// row's advance width when it continues into a later one (the physical end
/// anchors — no newline visualization).
fn underline_segments(
    row: &Row,
    row_index: usize,
    min_row: usize,
    max_row: usize,
    min_column: CharIndex,
    max_column: CharIndex,
) -> Vec<(f32, f32)> {
    let glyph_count = row.glyphs.len();
    let row_lo = if row_index == min_row {
        min_column.0.clamp(0, glyph_count)
    } else {
        0
    };
    let row_hi = if row_index == max_row {
        max_column.0.clamp(0, glyph_count)
    } else {
        glyph_count
    };

    if row_has_mixed_levels(row) && row_lo < row_hi {
        let groups = selected_slot_groups(row, row_lo, row_hi);
        let group_count = groups.len();
        let mut segments = Vec::with_capacity(group_count);
        for (group_index, group) in groups.iter().enumerate() {
            let Some(first_glyph) = row.glyphs.get(group.start) else {
                continue;
            };
            let Some(last_glyph) = row.glyphs.get(group.end.saturating_sub(1)) else {
                continue;
            };
            let mut left = first_glyph.pos.x;
            let mut right = last_glyph.max_x();
            if row_index != min_row && group_index == 0 {
                left = left.min(0.0);
            }
            if row_index != max_row && group_index + 1 == group_count {
                right = right.max(row.size.x);
            }
            segments.push((left, right));
        }
        segments
    } else {
        let left = if row_index == min_row {
            row.x_offset(min_column)
        } else {
            0.0
        };
        let right = if row_index == max_row {
            row.x_offset(max_column)
        } else {
            row.size.x
        };
        vec![(left, right)]
    }
}

/// Paint one end of the selection, e.g. the primary cursor.
///
/// This will never blink.
pub fn paint_cursor_end(painter: &Painter, visuals: &Visuals, cursor_rect: Rect) {
    let stroke = visuals.text_cursor.stroke;

    let top = cursor_rect.center_top();
    let bottom = cursor_rect.center_bottom();

    painter.line_segment([top, bottom], stroke);

    // The dead roof/floor block below is the natural home for a bidi
    // caret-affinity indicator, should one ever be wanted: at a mixed-direction
    // run boundary a single visual edge hosts two logical caret positions
    // (logical-before vs logical-after), and roof/floor strokes are how such an
    // affinity choice could be made visible. Landing it first needs an exact
    // logical↔visual mapping inside runs that counteract the base direction —
    // i.e. the stored per-row run map documented as not-yet-implemented in
    // epaint's `Row` (see the epaint commit that made `Row::x_offset`
    // direction-aware). Visualizing an approximation now would change default
    // visuals for every user, so the block stays as scaffolded.
    if false {
        // Roof/floor:
        let extrusion = 3.0;
        let width = 1.0;
        painter.line_segment(
            [top - vec2(extrusion, 0.0), top + vec2(extrusion, 0.0)],
            (width, stroke.color),
        );
        painter.line_segment(
            [bottom - vec2(extrusion, 0.0), bottom + vec2(extrusion, 0.0)],
            (width, stroke.color),
        );
    }
}

/// Paint one end of the selection, e.g. the primary cursor, with blinking (if enabled).
pub fn paint_text_cursor(
    ui: &Ui,
    painter: &Painter,
    primary_cursor_rect: Rect,
    time_since_last_interaction: f64,
) {
    if ui.visuals().text_cursor.blink {
        let on_duration = ui.visuals().text_cursor.on_duration;
        let off_duration = ui.visuals().text_cursor.off_duration;
        let total_duration = on_duration + off_duration;

        let time_in_cycle = (time_since_last_interaction % (total_duration as f64)) as f32;

        let wake_in = if time_in_cycle < on_duration {
            // Cursor is visible
            paint_cursor_end(painter, ui.visuals(), primary_cursor_rect);
            on_duration - time_in_cycle
        } else {
            // Cursor is not visible
            total_duration - time_in_cycle
        };

        ui.request_repaint_after_secs(wake_in);
    } else {
        paint_cursor_end(painter, ui.visuals(), primary_cursor_rect);
    }
}
