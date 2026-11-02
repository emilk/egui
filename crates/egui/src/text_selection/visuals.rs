use alloc::sync::Arc;

use emath::Pos2;
use epaint::{
    Stroke,
    text::{
        CharIndex,
        cursor::{CCursor, LayoutCursor},
    },
};

use crate::{
    Galley, Painter, Rangef, Rect, Ui, Visuals, text_selection::text_cursor_state::cursor_rect,
    vec2,
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
        // Where the cursor goes on this row if it is empty, row-relative:
        let empty_row_x = galley
            .pos_from_layout_cursor(&LayoutCursor {
                row: ri,
                column: CharIndex::ZERO,
            })
            .min
            .x
            - galley.rows[ri].pos.x;
        let placed_row = &mut galley.rows[ri];
        let row = Arc::make_mut(&mut placed_row.row);

        let first_column = if ri == min.row {
            min.column
        } else {
            CharIndex::ZERO
        };
        let ranges = if ri == max.row {
            row.x_ranges(first_column..max.column)
        } else {
            let mut ranges = row.x_ranges(first_column..row.char_count_excluding_newline());
            let newline_size = if placed_row.ends_with_newline {
                row.height() * 0.5 // visualize that we select the newline
            } else {
                0.0
            };
            // The newline is at the end of the row in reading order:
            if row.rtl_paragraph {
                let left = row
                    .glyphs
                    .iter()
                    .map(|g| g.pos.x)
                    .reduce(f32::min)
                    .unwrap_or(empty_row_x);
                match ranges.first_mut() {
                    Some(range) if range.min <= left => range.min = left - newline_size,
                    _ if 0.0 < newline_size => {
                        ranges.insert(0, Rangef::new(left - newline_size, left));
                    }
                    _ => {}
                }
            } else if !row.glyphs.iter().any(|glyph| glyph.is_rtl()) {
                if let Some(range) = ranges.last_mut() {
                    range.max = row.size.x + newline_size;
                }
            } else if 0.0 < newline_size {
                ranges.push(Rangef::new(row.size.x, row.size.x + newline_size));
            }
            ranges
        };

        let mesh = &mut row.visuals.mesh;

        if !row.glyphs.is_empty() {
            // Change color of the selected text:
            let first_glyph_index = if ri == min.row { min.column.0 } else { 0 };
            let last_glyph_index = if ri == max.row {
                max.column.0
            } else {
                row.glyphs.len()
            };

            let first_vertex_index = row
                .glyphs
                .get(first_glyph_index)
                .map_or(row.visuals.glyph_vertex_range.end, |g| g.first_vertex as _);
            let last_vertex_index = row
                .glyphs
                .get(last_glyph_index)
                .map_or(row.visuals.glyph_vertex_range.end, |g| g.first_vertex as _);

            for vi in first_vertex_index..last_vertex_index {
                mesh.vertices[vi].color = text_color;
            }
        }

        // Time to insert the selection rectangles into the row mesh.
        // They should be on top (after) of any background in the galley,
        // but behind (before) any glyphs. The row visuals has this information:
        let glyph_index_start = row.visuals.glyph_index_start;

        for range in ranges {
            let rect = Rect::from_x_y_ranges(range, 0.0..=row.size.y);

            // Start by appending the selection rectangle to end of the mesh, as two triangles (= 6 indices):
            let num_indices_before = mesh.indices.len();
            mesh.add_colored_rect(rect, background_color);
            assert_eq!(
                num_indices_before + 6,
                mesh.indices.len(),
                "We expect exactly 6 new indices"
            );

            // Copy out the new triangles:
            let selection_triangles = [
                mesh.indices[num_indices_before],
                mesh.indices[num_indices_before + 1],
                mesh.indices[num_indices_before + 2],
                mesh.indices[num_indices_before + 3],
                mesh.indices[num_indices_before + 4],
                mesh.indices[num_indices_before + 5],
            ];

            // Move every old triangle forwards by 6 indices to make room for the new triangle:
            for i in (glyph_index_start..num_indices_before).rev() {
                mesh.indices.swap(i, i + 6);
            }
            // Put the new triangle in place:
            mesh.indices[glyph_index_start..glyph_index_start + 6]
                .clone_from_slice(&selection_triangles);

            if let Some(new_vertex_indices) = &mut new_vertex_indices {
                new_vertex_indices.push(RowVertexIndices {
                    row: ri,
                    vertex_indices: selection_triangles,
                });
            }
        }

        row.visuals.mesh_bounds = mesh.calc_bounds();
    }
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

        let first_column = if ri == min.row {
            min.column
        } else {
            CharIndex::ZERO
        };
        let last_column = if ri == max.row {
            max.column
        } else {
            row.char_count_excluding_newline()
        };

        let offset_y = placed_row.pos.y + row.size.y;

        for range in row.x_ranges(first_column..last_column) {
            painter.line_segment(
                [
                    pos + vec2(range.min, offset_y),
                    pos + vec2(range.max, offset_y),
                ],
                stroke,
            );
        }
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

#[cfg(test)]
mod tests {
    use epaint::{
        Color32, FontId,
        text::{FontDefinitions, Fonts, TextOptions},
    };

    use super::*;

    /// One rectangle per stretch of selected glyphs on screen.
    #[test]
    fn selection_across_a_direction_boundary_paints_each_stretch() {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::default());
        let galley = fonts.with_pixels_per_point(1.0).layout_no_wrap(
            "abc אבג def".to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
        );
        let rect_count = |range: CCursorRange| {
            let mut galley = Arc::clone(&galley);
            let mut rects = vec![];
            paint_text_selection(&mut galley, &Visuals::dark(), &range, Some(&mut rects));
            rects.len()
        };

        // "abc " on the left, "א" at the right end of the Hebrew word:
        assert_eq!(
            rect_count(CCursorRange::two(CCursor::new(0), CCursor::new(5))),
            2
        );
        // The Hebrew word, and all of the text:
        assert_eq!(
            rect_count(CCursorRange::two(CCursor::new(4), CCursor::new(7))),
            1
        );
        assert_eq!(
            rect_count(CCursorRange::two(CCursor::new(0), CCursor::new(11))),
            1
        );
    }

    /// A selected newline is drawn at the end of its row in reading order.
    #[test]
    fn selected_newline_follows_the_paragraph_direction() {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::default());
        for (text, rtl) in [("אבג\nדהו", true), ("abc אבג\ndef", false)] {
            let mut galley = fonts.with_pixels_per_point(1.0).layout_no_wrap(
                text.to_owned(),
                FontId::proportional(14.0),
                Color32::WHITE,
            );
            let end = galley.end();
            let mut rects = vec![];
            let range = CCursorRange::two(CCursor::new(0), end);
            paint_text_selection(&mut galley, &Visuals::dark(), &range, Some(&mut rects));

            let row = &galley.rows[0];
            let selected_x = rects
                .iter()
                .filter(|rect| rect.row == 0)
                .flat_map(|rect| rect.vertex_indices)
                .map(|i| row.visuals.mesh.vertices[i as usize].pos.x);
            let (min, max) = selected_x.fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), x| {
                (a.min(x), b.max(x))
            });
            let glyphs_min = row
                .glyphs
                .iter()
                .map(|g| g.pos.x)
                .fold(f32::INFINITY, f32::min);
            let glyphs_max = row.glyphs.iter().map(|g| g.max_x()).fold(0.0, f32::max);
            if rtl {
                assert!(min < glyphs_min && max <= glyphs_max, "{text:?}");
            } else {
                assert!(glyphs_min <= min && glyphs_max < max, "{text:?}");
            }
        }
    }
}
