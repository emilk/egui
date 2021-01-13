use crate::*;

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub(crate) struct State {
    col_widths: Vec<f32>,
    row_heights: Vec<f32>,
}

impl State {
    /// Returns `true` if this made the column wider.
    fn set_min_col_width(&mut self, col: usize, width: f32) -> bool {
        self.col_widths
            .resize(self.col_widths.len().max(col + 1), 0.0);
        if self.col_widths[col] < width {
            self.col_widths[col] = width;
            true
        } else {
            false
        }
    }

    /// Returns `true` if this made the row higher.
    fn set_min_row_height(&mut self, row: usize, height: f32) -> bool {
        self.row_heights
            .resize(self.row_heights.len().max(row + 1), 0.0);
        if self.row_heights[row] < height {
            self.row_heights[row] = height;
            true
        } else {
            false
        }
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
    default_row_height: f32,
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
            default_row_height: 0.0,
            col: 0,
            row: 0,
        }
    }
}

impl GridLayout {
    fn prev_row_height(&self, row: usize) -> f32 {
        self.prev_state
            .row_height(row)
            .unwrap_or(self.default_row_height)
    }

    pub(crate) fn available_rect(&self, region: &Region) -> Rect {
        Rect::from_min_max(region.cursor, region.max_rect.max)
    }

    pub(crate) fn available_rect_finite(&self, region: &Region) -> Rect {
        Rect::from_min_max(region.cursor, region.max_rect_finite().max)
    }

    pub(crate) fn next_cell(&self, cursor: Pos2, child_size: Vec2) -> Rect {
        let width = self.prev_state.col_width(self.col).unwrap_or(0.0);
        let height = self.prev_row_height(self.row);
        let size = child_size.max(vec2(width, height));
        Rect::from_min_size(cursor, size)
    }

    pub(crate) fn advance(&mut self, cursor: &mut Pos2, frame_rect: Rect, widget_rect: Rect) {
        let dirty = self
            .curr_state
            .set_min_col_width(self.col, widget_rect.width());
        let dirty = self
            .curr_state
            .set_min_row_height(self.row, widget_rect.height())
            || dirty;
        if dirty {
            self.ctx
                .memory()
                .grid
                .insert(self.id, self.curr_state.clone());
            self.ctx.request_repaint();
        }
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
                let color = Rgba::from_white_alpha(0.0075);
                // let color = Rgba::from_black_alpha(0.2);
                painter.rect_filled(rect, 2.0, color);
            }
        }
    }
}

// ----------------------------------------------------------------------------

/// A simple `Grid` layout.
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
}

impl Grid {
    pub fn new(id_source: impl std::hash::Hash) -> Self {
        Self {
            id_source: Id::new(id_source),
            striped: false,
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
}

impl Grid {
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
        let Self { id_source, striped } = self;

        ui.wrap(|ui| {
            let id = ui.make_persistent_id(id_source);
            let grid = GridLayout {
                striped,
                ..GridLayout::new(ui, id)
            };
            ui.set_grid(grid);
            add_contents(ui)
        })
        .0
    }
}
