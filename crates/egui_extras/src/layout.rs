use egui::{
    IdSalt, Margin, Pos2, Rect, Response, Sense, StrokeKind, Ui, UiBuilder, emath::GuiRounding as _,
};

#[derive(Clone, Copy)]
pub(crate) enum CellSize {
    /// Absolute size in points
    Absolute(f32),

    /// Take all available space
    Remainder,
}

/// Cells are positioned in two dimensions, cells go in one direction and form lines.
///
/// In a strip there's only one line which goes in the direction of the strip:
///
/// In a horizontal strip, a [`StripLayout`] with horizontal [`CellDirection`] is used.
/// Its cells go from left to right inside this [`StripLayout`].
///
/// In a table there's a [`StripLayout`] for each table row with a horizontal [`CellDirection`].
/// Its cells go from left to right. And the lines go from top to bottom.
pub(crate) enum CellDirection {
    /// Cells go from left to right.
    Horizontal,

    /// Cells go from top to bottom.
    Vertical,
}

/// Where a cell is in a [`crate::Table`].
///
/// Decides which lines of [`egui::style::TableStyle`] the cell paints.
#[derive(Clone, Copy, Default)]
pub(crate) struct TableCellPosition {
    pub(crate) header: bool,
    pub(crate) first_row: bool,
    pub(crate) first_col: bool,
}

/// Flags used by [`StripLayout::add`].
#[derive(Clone, Copy, Default)]
pub(crate) struct StripLayoutFlags {
    pub(crate) clip: bool,
    pub(crate) striped: bool,
    pub(crate) hovered: bool,
    pub(crate) selected: bool,
    pub(crate) overline: bool,

    /// Set for table cells, which are styled with [`egui::style::TableStyle`].
    pub(crate) table_cell: Option<TableCellPosition>,

    /// Used when we want to accurately measure the size of this cell.
    pub(crate) sizing_pass: bool,
}

/// Positions cells in [`CellDirection`] and starts a new line on [`StripLayout::end_line`]
pub struct StripLayout<'l> {
    pub(crate) ui: &'l mut Ui,
    direction: CellDirection,
    pub(crate) rect: Rect,
    pub(crate) cursor: Pos2,

    /// Keeps track of the max used position,
    /// so we know how much space we used.
    max: Pos2,

    cell_layout: egui::Layout,
    sense: Sense,
}

impl<'l> StripLayout<'l> {
    pub(crate) fn new(
        ui: &'l mut Ui,
        direction: CellDirection,
        cell_layout: egui::Layout,
        sense: Sense,
    ) -> Self {
        let rect = ui.available_rect_before_wrap();
        let pos = rect.left_top();

        Self {
            ui,
            direction,
            rect,
            cursor: pos,
            max: pos,
            cell_layout,
            sense,
        }
    }

    fn cell_rect(&self, width: &CellSize, height: &CellSize) -> Rect {
        Rect {
            min: self.cursor,
            max: Pos2 {
                x: match width {
                    CellSize::Absolute(width) => self.cursor.x + width,
                    CellSize::Remainder => self.rect.right(),
                },
                y: match height {
                    CellSize::Absolute(height) => self.cursor.y + height,
                    CellSize::Remainder => self.rect.bottom(),
                },
            },
        }
    }

    fn set_pos(&mut self, rect: Rect) {
        self.max.x = self.max.x.max(rect.right());
        self.max.y = self.max.y.max(rect.bottom());

        match self.direction {
            CellDirection::Horizontal => {
                self.cursor.x = rect.right() + self.ui.spacing().item_spacing.x;
            }
            CellDirection::Vertical => {
                self.cursor.y = rect.bottom() + self.ui.spacing().item_spacing.y;
            }
        }
    }

    pub(crate) fn empty(&mut self, width: CellSize, height: CellSize) {
        self.set_pos(self.cell_rect(&width, &height));
    }

    /// This is the innermost part of [`crate::Table`] and [`crate::Strip`].
    ///
    /// Return the used space (`min_rect`) plus the [`Response`] of the whole cell.
    pub(crate) fn add(
        &mut self,
        flags: StripLayoutFlags,
        width: CellSize,
        height: CellSize,
        child_ui_id_salt: IdSalt,
        add_cell_contents: impl FnOnce(&mut Ui),
    ) -> (Rect, Response) {
        let max_rect = self.cell_rect(&width, &height);
        let table_style = self.ui.visuals().table;

        // Make sure we don't have a gap in the stripe/frame/selection background:
        let item_spacing = self.ui.spacing().item_spacing;
        let gapless_rect = max_rect.expand2(0.5 * item_spacing).round_ui();

        if flags.table_cell.is_some_and(|cell| cell.header) {
            self.ui.painter().rect_filled(
                gapless_rect,
                egui::CornerRadius::ZERO,
                table_style.header_fill,
            );
        }

        if flags.striped {
            self.ui.painter().rect_filled(
                gapless_rect,
                egui::CornerRadius::ZERO,
                table_style.stripe_fill,
            );
        }

        if flags.selected {
            self.ui.painter().rect_filled(
                gapless_rect,
                egui::CornerRadius::ZERO,
                table_style.selected_row_fill,
            );
        }

        if flags.hovered && !flags.selected && self.sense.interactive() {
            self.ui.painter().rect_filled(
                gapless_rect,
                egui::CornerRadius::ZERO,
                table_style.hovered_row_fill,
            );
        }

        let cell_margin = if let Some(cell) = flags.table_cell {
            self.paint_table_lines(cell, gapless_rect);
            table_style.cell_margin
        } else {
            Margin::ZERO
        };

        let mut child_ui = self.cell(
            flags,
            max_rect,
            cell_margin,
            child_ui_id_salt,
            add_cell_contents,
        );

        let used_rect = child_ui.min_rect() + cell_margin;

        // Make sure we catch clicks etc on the _whole_ cell, including its margin:
        child_ui.set_min_size(child_ui.max_rect().size());
        if cell_margin != Margin::ZERO {
            child_ui.expand_to_include_rect(max_rect);
        }

        let allocation_rect = if self.ui.is_sizing_pass() {
            if flags.clip {
                // Clipped content must not increase the parent's measured minimum size.
                used_rect.intersect(max_rect)
            } else {
                used_rect
            }
        } else if flags.clip {
            max_rect
        } else {
            max_rect | used_rect
        };

        self.set_pos(allocation_rect);

        self.ui.advance_cursor_after_rect(allocation_rect);

        let response = child_ui.response();

        (used_rect, response)
    }

    /// Paint the lines of [`egui::style::TableStyle`] on the edges of a table cell.
    ///
    /// Each cell paints the lines on its top and left edges,
    /// so that the fill of the next cell does not cover them.
    fn paint_table_lines(&self, cell: TableCellPosition, gapless_rect: Rect) {
        let table_style = &self.ui.visuals().table;
        let painter = self.ui.painter();

        if !cell.header && !cell.first_row {
            let y = painter.round_to_pixel_center(gapless_rect.top());
            painter.hline(gapless_rect.x_range(), y, table_style.row_stroke);
        }

        if !cell.first_col {
            let x = painter.round_to_pixel_center(gapless_rect.left());
            painter.vline(x, gapless_rect.y_range(), table_style.column_stroke);
        }
    }

    /// Paint [`egui::style::TableStyle::outer_stroke`] around the table,
    /// and [`egui::style::TableStyle::header_stroke`] under the header.
    ///
    /// Painted after the body, so that the body fills do not cover them.
    pub(crate) fn paint_table_outline(ui: &Ui, table_rect: Rect, body_top: Option<f32>) {
        let table_style = &ui.visuals().table;
        let painter = ui.painter();
        let rect = table_rect
            .expand2(0.5 * ui.spacing().item_spacing)
            .round_ui();

        if let Some(body_top) = body_top {
            let y = painter.round_to_pixel_center(body_top - 0.5 * ui.spacing().item_spacing.y);
            painter.hline(rect.x_range(), y, table_style.header_stroke);
        }

        painter.rect_stroke(
            rect,
            table_style.corner_radius,
            table_style.outer_stroke,
            StrokeKind::Middle,
        );
    }

    /// only needed for layouts with multiple lines, like [`Table`](crate::Table).
    pub fn end_line(&mut self) {
        match self.direction {
            CellDirection::Horizontal => {
                self.cursor.y = self.max.y + self.ui.spacing().item_spacing.y;
                self.cursor.x = self.rect.left();
            }
            CellDirection::Vertical => {
                self.cursor.x = self.max.x + self.ui.spacing().item_spacing.x;
                self.cursor.y = self.rect.top();
            }
        }
    }

    /// Skip a lot of space.
    pub(crate) fn skip_space(&mut self, delta: egui::Vec2) {
        let before = self.cursor;
        self.cursor += delta;
        let rect = Rect::from_two_pos(before, self.cursor);
        self.ui.expand_to_include_rect(rect);
    }

    /// Return the Ui to which the contents where added
    fn cell(
        &mut self,
        flags: StripLayoutFlags,
        max_rect: Rect,
        cell_margin: Margin,
        child_ui_id_salt: IdSalt,
        add_cell_contents: impl FnOnce(&mut Ui),
    ) -> Ui {
        let child_ui_id = self.ui.scope_id().with(child_ui_id_salt);
        let mut ui_builder = UiBuilder::new()
            .scope_id(child_ui_id)
            .ui_stack_info(egui::UiStackInfo::new(egui::UiKind::TableCell))
            .max_rect(max_rect - cell_margin)
            .layout(self.cell_layout)
            .sense(self.sense);
        if flags.sizing_pass {
            ui_builder = ui_builder.sizing_pass();
        }

        let mut child_ui = self.ui.new_child(ui_builder);

        if flags.clip {
            child_ui.shrink_clip_rect(max_rect);

            if !child_ui.is_sizing_pass() {
                // Better to truncate (if we can), rather than hard clipping:
                child_ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            }
        }

        if flags.selected {
            let stroke_color = child_ui.style().visuals.selection.stroke.color;
            child_ui.style_mut().visuals.override_text_color = Some(stroke_color);
        }

        if flags.overline {
            child_ui.painter().hline(
                max_rect.x_range(),
                max_rect.top(),
                child_ui.visuals().widgets.noninteractive.bg_stroke,
            );
        }

        add_cell_contents(&mut child_ui);

        child_ui
    }

    /// Allocate the rect in [`Self::ui`] so that the scrollview knows about our size
    pub fn allocate_rect(&mut self) -> Response {
        let mut rect = self.rect;
        rect.set_right(self.max.x);
        rect.set_bottom(self.max.y);

        self.ui.allocate_rect(rect, Sense::hover())
    }
}

#[cfg(test)]
mod tests {
    use super::{CellDirection, StripLayout};

    #[test]
    fn skip_space_expands_ui_without_consuming_widget_id() {
        egui::__run_test_ui(|ui| {
            let mut layout = StripLayout::new(
                ui,
                CellDirection::Horizontal,
                egui::Layout::left_to_right(egui::Align::Center),
                egui::Sense::hover(),
            );
            let next_auto_id = layout.ui.next_auto_id();
            let expected_bottom = layout.cursor.y + 100.0;

            layout.skip_space(egui::vec2(0.0, 100.0));

            assert_eq!(layout.ui.next_auto_id(), next_auto_id);
            assert_eq!(layout.ui.min_rect().bottom(), expected_bottom);
        });
    }
}
