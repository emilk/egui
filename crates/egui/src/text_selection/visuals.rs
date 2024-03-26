use crate::*;

use self::layers::ShapeIdx;

use super::CursorRange;

pub fn paint_text_selection(
    painter: &Painter,
    visuals: &Visuals,
    galley_pos: Pos2,
    galley: &Galley,
    cursor_range: &CursorRange,
    mut out_shaped_idx: Option<&mut Vec<ShapeIdx>>,
) {
    if cursor_range.is_empty() {
        return;
    }

    // We paint the cursor selection on top of the text, so make it transparent:
    let color = visuals.selection.bg_fill.linear_multiply(0.5);
    let [min, max] = cursor_range.sorted_cursors();
    let min = min.rcursor;
    let max = max.rcursor;

    for ri in min.row..=max.row {
        let row = &galley.rows[ri];
        let left = if ri == min.row {
            row.x_offset(min.column)
        } else {
            row.rect.left()
        };
        let right = if ri == max.row {
            row.x_offset(max.column)
        } else {
            let newline_size = if row.ends_with_newline {
                row.height() / 2.0 // visualize that we select the newline
            } else {
                0.0
            };
            row.rect.right() + newline_size
        };
        let rect = Rect::from_min_max(
            galley_pos + vec2(left, row.min_y()),
            galley_pos + vec2(right, row.max_y()),
        );
        let shape_idx = painter.rect_filled(rect, 0.0, color);
        if let Some(out_shaped_idx) = &mut out_shaped_idx {
            out_shaped_idx.push(shape_idx);
        }
    }
}

/// Paint one end of the selection, e.g. the primary cursor.
pub fn paint_cursor(painter: &Painter, visuals: &Visuals, cursor_rect: Rect) {
    let stroke = visuals.text_cursor;

    let top = cursor_rect.center_top();
    let bottom = cursor_rect.center_bottom();

    painter.line_segment([top, bottom], (stroke.width, stroke.color));

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
