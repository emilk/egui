use std::sync::Arc;

use epaint::text::cursor::Selection;

use crate::{vec2, Galley, Painter, Rect, Ui, Visuals};

/// Adds text selection rectangles to the galley.
/// Returns true if any selection rectangles were drawn.
pub fn paint_text_selection(
    galley: &mut Arc<Galley>,
    visuals: &Visuals,
    selection: &Selection,
) -> bool {
    if selection.is_empty() {
        return false;
    }

    // We need to modify the galley (add text selection painting to it),
    // and so we need to clone it if it is shared:
    let galley: &mut Galley = Arc::make_mut(galley);

    // TODO(valadaptive): implement selection stroke? the old code never did
    galley.paint_selection(visuals.selection.bg_fill, selection)
}

/// Paint one end of the selection, e.g. the primary cursor.
///
/// This will never blink.
pub fn paint_cursor_end(painter: &Painter, visuals: &Visuals, cursor_rect: Rect) {
    let stroke = visuals.text_cursor.stroke;

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

        ui.ctx().request_repaint_after_secs(wake_in);
    } else {
        paint_cursor_end(painter, ui.visuals(), primary_cursor_rect);
    }
}
