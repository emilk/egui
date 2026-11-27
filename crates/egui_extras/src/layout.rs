use egui::{
    IdSalt, Margin, Pos2, Rect, Response, Sense, StrokeKind, Ui, UiBuilder, Vec2,
    emath::GuiRounding as _,
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

/// The widths of the lines on the edges of a table cell.
///
/// The lines are inside the cell, outside its margin, so they take up space.
#[derive(Clone, Copy, Default)]
pub(crate) struct CellLines {
    /// [`egui::style::TableStyle::column_stroke`]
    pub(crate) left: f32,

    /// [`egui::style::TableStyle::row_stroke`]
    pub(crate) top: f32,

    /// [`egui::style::TableStyle::header_stroke`]
    pub(crate) bottom: f32,
}

impl CellLines {
    fn sum(self) -> Vec2 {
        Vec2::new(self.left, self.top + self.bottom)
    }

    /// Shrink a cell rect to the part inside the lines.
    fn inside(self, rect: Rect) -> Rect {
        Rect::from_min_max(
            rect.min + Vec2::new(self.left, self.top),
            rect.max - Vec2::new(0.0, self.bottom),
        )
    }

    /// Grow a rect inside the lines to the whole cell.
    fn outside(self, rect: Rect) -> Rect {
        Rect::from_min_max(
            rect.min - Vec2::new(self.left, self.top),
            rect.max + Vec2::new(0.0, self.bottom),
        )
    }
}

/// Where a cell is in a [`crate::Table`].
///
/// Decides which lines of [`egui::style::TableStyle`] the cell paints.
#[derive(Clone, Copy, Default)]
pub(crate) struct TableCellPosition {
    pub(crate) header: bool,
    pub(crate) first_col: bool,
    pub(crate) last_col: bool,
    pub(crate) lines: CellLines,
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

    /// Space between cells. Zero in tables, which use [`Self::cell_margin`] instead.
    spacing: Vec2,

    /// Space inside each cell, around its contents. Only used in tables.
    cell_margin: Margin,

    /// The margins of the edge cells that stick out of the table.
    /// Nothing is painted there. Only used in tables.
    outside: Margin,
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
        let spacing = ui.spacing().item_spacing;

        Self {
            ui,
            direction,
            rect,
            cursor: pos,
            max: pos,
            cell_layout,
            sense,
            spacing,
            cell_margin: Margin::ZERO,
            outside: Margin::ZERO,
        }
    }

    /// A row of a [`crate::Table`]: the cells touch,
    /// and [`egui::style::TableStyle::cell_margin`] is added around the size of each cell.
    ///
    /// `outside` is the part of the margins of the edge cells that sticks out of the table.
    pub(crate) fn new_table_row(
        ui: &'l mut Ui,
        cell_layout: egui::Layout,
        sense: Sense,
        outside: Margin,
    ) -> Self {
        let cell_margin = ui.visuals().table.cell_margin;
        Self {
            spacing: Vec2::ZERO,
            cell_margin,
            outside,
            ..Self::new(ui, CellDirection::Horizontal, cell_layout, sense)
        }
    }

    /// The rect of the next cell, including [`Self::cell_margin`] and `lines`.
    fn cell_rect(&self, width: &CellSize, height: &CellSize, lines: CellLines) -> Rect {
        let margin = self.cell_margin.sum() + lines.sum();
        Rect {
            min: self.cursor,
            max: Pos2 {
                x: match width {
                    CellSize::Absolute(width) => self.cursor.x + width + margin.x,
                    CellSize::Remainder => self.rect.right(),
                },
                y: match height {
                    CellSize::Absolute(height) => self.cursor.y + height + margin.y,
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
                self.cursor.x = rect.right() + self.spacing.x;
            }
            CellDirection::Vertical => {
                self.cursor.y = rect.bottom() + self.spacing.y;
            }
        }
    }

    pub(crate) fn empty(&mut self, width: CellSize, height: CellSize) {
        self.set_pos(self.cell_rect(&width, &height, CellLines::default()));
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
        let lines = flags
            .table_cell
            .map_or_else(CellLines::default, |cell| cell.lines);
        let max_rect = self.cell_rect(&width, &height, lines);
        let table_style = self.ui.visuals().table;

        // Make sure we don't have a gap in the stripe/frame/selection background:
        let mut gapless_rect = max_rect.expand2(0.5 * self.spacing).round_ui();

        // Don't paint the margins that stick out of the table:
        if let Some(cell) = flags.table_cell {
            if cell.first_col {
                gapless_rect.min.x += f32::from(self.outside.left);
            }
            if cell.last_col {
                gapless_rect.max.x -= f32::from(self.outside.right);
            }
            if cell.header {
                gapless_rect.min.y += f32::from(self.outside.top);
            }
        }

        if let Some(cell) = flags.table_cell.filter(|cell| cell.header) {
            // Round the top corners of the table, inside the outline:
            let radius = table_style.corner_radius;
            let corner_radius = egui::CornerRadius {
                nw: if cell.first_col { radius.nw } else { 0 },
                ne: if cell.last_col { radius.ne } else { 0 },
                sw: 0,
                se: 0,
            };
            self.ui
                .painter()
                .rect_filled(gapless_rect, corner_radius, table_style.header_fill);
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

        self.paint_table_lines(lines, gapless_rect);
        let cell_margin = self.cell_margin;

        let content_rect = lines.inside(max_rect) - cell_margin;
        let mut child_ui = self.cell(
            flags,
            max_rect,
            content_rect,
            child_ui_id_salt,
            add_cell_contents,
        );

        let used_rect = child_ui.min_rect();
        let used_rect_with_margin = lines.outside(used_rect + cell_margin);

        // Make sure we catch clicks etc on the _whole_ cell, including its margin:
        child_ui.set_min_size(child_ui.max_rect().size());
        if content_rect != max_rect {
            child_ui.expand_to_include_rect(max_rect);
        }

        let allocation_rect = if self.ui.is_sizing_pass() {
            if flags.clip {
                // Clipped content must not increase the parent's measured minimum size.
                used_rect_with_margin.intersect(max_rect)
            } else {
                used_rect_with_margin
            }
        } else if flags.clip {
            max_rect
        } else {
            max_rect | used_rect_with_margin
        };

        self.set_pos(allocation_rect);

        self.ui.advance_cursor_after_rect(allocation_rect);

        let response = child_ui.response();

        (used_rect, response)
    }

    /// Paint the lines of [`egui::style::TableStyle`] on the edges of a table cell,
    /// inside the cell.
    fn paint_table_lines(&self, lines: CellLines, rect: Rect) {
        let table_style = &self.ui.visuals().table;
        let painter = self.ui.painter();

        if 0.0 < lines.top {
            let y = rect.top() + 0.5 * lines.top;
            painter.hline(rect.x_range(), y, table_style.row_stroke);
        }
        if 0.0 < lines.bottom {
            let y = rect.bottom() - 0.5 * lines.bottom;
            painter.hline(rect.x_range(), y, table_style.header_stroke);
        }
        if 0.0 < lines.left {
            let x = rect.left() + 0.5 * lines.left;
            painter.vline(x, rect.y_range(), table_style.column_stroke);
        }
    }

    /// Paint [`egui::style::TableStyle::outer_stroke`] around the table.
    ///
    /// Painted after the body, so that the body fills do not cover it.
    pub(crate) fn paint_table_outline(ui: &Ui, table_rect: Rect) {
        let table_style = &ui.visuals().table;
        let painter = ui.painter();
        let rect = table_rect.round_ui();

        // Outside the cells, so that the lines between them stop at the outline:
        painter.rect_stroke(
            rect,
            table_style.corner_radius,
            table_style.outer_stroke,
            StrokeKind::Outside,
        );
    }

    /// only needed for layouts with multiple lines, like [`Table`](crate::Table).
    pub fn end_line(&mut self) {
        match self.direction {
            CellDirection::Horizontal => {
                self.cursor.y = self.max.y + self.spacing.y;
                self.cursor.x = self.rect.left();
            }
            CellDirection::Vertical => {
                self.cursor.x = self.max.x + self.spacing.x;
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
        content_rect: Rect,
        child_ui_id_salt: IdSalt,
        add_cell_contents: impl FnOnce(&mut Ui),
    ) -> Ui {
        let child_ui_id = self.ui.scope_id().with(child_ui_id_salt);
        let mut ui_builder = UiBuilder::new()
            .scope_id(child_ui_id)
            .ui_stack_info(egui::UiStackInfo::new(egui::UiKind::TableCell))
            .max_rect(content_rect)
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
