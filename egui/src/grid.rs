use crate::*;

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub(crate) struct State {
    col_widths: Vec<f32>,
    row_heights: Vec<f32>,
}

impl State {
    fn set_min_col_width(&mut self, col: usize, width: f32) {
        self.col_widths
            .resize(self.col_widths.len().max(col + 1), 0.0);
        self.col_widths[col] = self.col_widths[col].max(width);
    }

    fn set_min_row_height(&mut self, row: usize, height: f32) {
        self.row_heights
            .resize(self.row_heights.len().max(row + 1), 0.0);
        self.row_heights[row] = self.row_heights[row].max(height);
    }

    fn col_width(&self, col: usize) -> Option<f32> {
        self.col_widths.get(col).copied()
    }

    fn row_height(&self, row: usize) -> Option<f32> {
        self.row_heights.get(row).copied()
    }

    fn full_width(&self, x_spacing: f32) -> f32 {
        self.col_widths.iter().sum::<f32>()
            + (self.col_widths.len().at_least(1) - 1) as f32 * x_spacing
    }
}

// ----------------------------------------------------------------------------

pub(crate) struct GridLayout {
    ctx: CtxRef,
    id: Id,

    /// State previous frame (if any).
    /// This can be used to predict future sizes of cells.
    prev_state: State,
    /// State accumulated during the current frame.
    curr_state: State,

    spacing: Vec2,

    striped: bool,
    initial_x: f32,
    min_row_height: f32,
    col: usize,
    row: usize,
}

impl GridLayout {
    pub(crate) fn new(ui: &Ui, id: Id) -> Self {
        let prev_state = ui.memory().grid.get(&id).cloned().unwrap_or_default();

        Self {
            ctx: ui.ctx().clone(),
            id,
            prev_state,
            curr_state: State::default(),
            spacing: ui.style().spacing.item_spacing,
            striped: false,
            initial_x: ui.cursor().x,
            min_row_height: ui.style().spacing.interact_size.y,
            col: 0,
            row: 0,
        }
    }
}

impl GridLayout {
    fn prev_row_height(&self, row: usize) -> f32 {
        self.prev_state
            .row_height(row)
            .unwrap_or(self.min_row_height)
    }

    pub(crate) fn available_rect(&self, region: &Region) -> Rect {
        let mut rect = Rect::from_min_max(region.cursor, region.max_rect.max);
        rect.set_height(rect.height().at_least(self.min_row_height));
        rect
    }

    pub(crate) fn available_rect_finite(&self, region: &Region) -> Rect {
        let mut rect = Rect::from_min_max(region.cursor, region.max_rect_finite().max);
        rect.set_height(rect.height().at_least(self.min_row_height));
        rect
    }

    pub(crate) fn next_cell(&self, cursor: Pos2, child_size: Vec2) -> Rect {
        let width = self.prev_state.col_width(self.col).unwrap_or(0.0);
        let height = self.prev_row_height(self.row);
        let size = child_size.max(vec2(width, height));
        Rect::from_min_size(cursor, size)
    }

    pub(crate) fn align_size_within_rect(&self, size: Vec2, frame: Rect) -> Rect {
        // TODO: allow this alignment to be customized
        Align2::LEFT_CENTER.align_size_within_rect(size, frame)
    }

    pub(crate) fn justify_or_align(&self, frame: Rect, size: Vec2) -> Rect {
        self.align_size_within_rect(size, frame)
    }

    pub(crate) fn advance(&mut self, cursor: &mut Pos2, frame_rect: Rect, widget_rect: Rect) {
        self.curr_state
            .set_min_col_width(self.col, widget_rect.width());
        self.curr_state
            .set_min_row_height(self.row, widget_rect.height().at_least(self.min_row_height));
        self.col += 1;
        cursor.x += frame_rect.width() + self.spacing.x;
    }

    pub(crate) fn end_row(&mut self, cursor: &mut Pos2, painter: &Painter) {
        let row_height = self.prev_row_height(self.row);

        cursor.x = self.initial_x;
        cursor.y += row_height + self.spacing.y;
        self.col = 0;
        self.row += 1;

        if self.striped && self.row % 2 == 1 {
            if let Some(height) = self.prev_state.row_height(self.row) {
                // Paint background for coming row:
                let size = Vec2::new(self.prev_state.full_width(self.spacing.x), height);
                let rect = Rect::from_min_size(*cursor, size);
                let rect = rect.expand2(0.5 * self.spacing.y * Vec2::Y);
                let color = Rgba::from_white_alpha(0.0075);
                // let color = Rgba::from_black_alpha(0.2);
                painter.rect_filled(rect, 2.0, color);
            }
        }
    }

    pub(crate) fn save(&self) {
        if self.curr_state != self.prev_state {
            self.ctx
                .memory()
                .grid
                .insert(self.id, self.curr_state.clone());
            self.ctx.request_repaint();
        }
    }
}

// ----------------------------------------------------------------------------

/// A simple `Grid` layout.
///
/// The contents of each cell be aligned to the left and center.
/// If you want to add multiple widgets to a cell you need to group them with
/// [`Ui::horizontal`], [`Ui::vertical`] etc.
///
/// ```
/// # let ui = &mut egui::Ui::__test();
/// egui::Grid::new("some_unique_id").show(ui, |ui| {
///     ui.label("First row, first column");
///     ui.label("First row, second column");
///     ui.end_row();
///
///     ui.label("Second row, first column");
///     ui.label("Second row, second column");
///     ui.label("Second row, third column");
/// });
/// ```
pub struct Grid {
    id_source: Id,
    striped: bool,
    min_row_height: Option<f32>,
    spacing: Option<Vec2>,
}

impl Grid {
    pub fn new(id_source: impl std::hash::Hash) -> Self {
        Self {
            id_source: Id::new(id_source),
            striped: false,
            min_row_height: None,
            spacing: None,
        }
    }

    /// If `true`, add a subtle background color to every other row.
    ///
    /// This can make a table easier to read.
    /// Default: `false`.
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// Set minimum height of each row. Default: [`Spacing::interact_size.y`].
    pub fn min_row_height(mut self, min_row_height: f32) -> Self {
        self.min_row_height = Some(min_row_height);
        self
    }

    /// Set spacing between columns/rows.
    /// Default: [`Spacing::item_spacing`].
    pub fn spacing(mut self, spacing: impl Into<Vec2>) -> Self {
        self.spacing = Some(spacing.into());
        self
    }
}

impl Grid {
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
        let Self {
            id_source,
            striped,
            min_row_height,
            spacing,
        } = self;
        let min_row_height = min_row_height.unwrap_or_else(|| ui.style().spacing.interact_size.y);
        let spacing = spacing.unwrap_or_else(|| ui.style().spacing.item_spacing);

        // Each grid cell is aligned LEFT_CENTER.
        // If somebody wants to wrap more things inside a cell,
        // then we should pick a default layout that matches that alignment,
        // which we do here:
        ui.horizontal(|ui| {
            let id = ui.make_persistent_id(id_source);
            let grid = GridLayout {
                striped,
                min_row_height,
                spacing,
                ..GridLayout::new(ui, id)
            };
            ui.set_grid(grid);
            let r = add_contents(ui);
            ui.save_grid();
            r
        })
        .0
    }
}
