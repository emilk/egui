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

/// Paint text cursor.
pub fn paint_text_cursor(
    ui: &mut Ui,
    painter: &Painter,
    primary_cursor_rect: Rect,
    is_stay_cursor: bool,
) {
    let i_time = ui.input(|i| i.time);
    let blink_mode = ui.visuals().text_cursor_blink;
    let is_blink_mode = blink_mode && is_stay_cursor;

    let mut is_cursor_visible = true;

    let on_duration = ui.visuals().text_cursor_on_duration;
    let off_duration = ui.visuals().text_cursor_off_duratio;
    let total_duration = on_duration + off_duration;

    if is_blink_mode {
        if (i_time % total_duration as f64) < on_duration as f64 {
            is_cursor_visible = true;
        } else {
            is_cursor_visible = false;
        }
    }

    if is_cursor_visible {
        paint_cursor(&painter, ui.visuals(), primary_cursor_rect);
    }

    if is_blink_mode {
        if is_cursor_visible {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis((on_duration * 1000.0) as u64));
        }
        if !is_cursor_visible {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis((off_duration * 1000.0) as u64));
        }
    }
}
