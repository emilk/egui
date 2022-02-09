use crate::{
    layout::{CellSize, Layout, LineDirection},
    sizing::Sizing,
    Size,
};
use egui::{Response, Ui};

/// The direction in which cells are positioned in the grid.
///
/// In a horizontal grid cells are positions from left to right.
/// In a vertical grid cells are positions from top to bottom.
enum GridDirection {
    Horizontal,
    Vertical,
}

pub struct GridBuilder<'a> {
    ui: &'a mut Ui,
    sizing: Sizing,
}

impl<'a> GridBuilder<'a> {
    /// Create new grid builder.
    ///
    /// In contrast to normal egui behavior, cells do *not* grow with its children!
    ///
    /// After adding size hints with `[Self::column]`/`[Self::columns]` the grid can be build with `[Self::horizontal]`/`[Self::vertical]`.
    ///
    /// ### Example
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// use egui_extras::{GridBuilder, Size};
    /// GridBuilder::new(ui)
    ///     .size(Size::RemainderMinimum(100.0))
    ///     .size(Size::Absolute(40.0))
    ///     .vertical(|mut grid| {
    ///         grid.grid(|builder| {
    ///             builder.sizes(Size::Remainder, 2).horizontal(|mut grid| {
    ///                 grid.cell(|ui| {
    ///                     ui.label("Top Left");
    ///                 });
    ///                 grid.cell(|ui| {
    ///                     ui.label("Top Right");
    ///                 });
    ///             });
    ///         });
    ///         grid.cell(|ui| {
    ///             ui.label("Fixed");
    ///         });
    ///     });
    /// # });
    /// ```
    pub fn new(ui: &'a mut Ui) -> Self {
        let sizing = Sizing::new();

        Self { ui, sizing }
    }

    /// Add size hint for column/row
    pub fn size(mut self, size: Size) -> Self {
        self.sizing.add(size);
        self
    }

    /// Add size hint for columns/rows [count] times
    pub fn sizes(mut self, size: Size, count: usize) -> Self {
        for _ in 0..count {
            self.sizing.add(size);
        }
        self
    }

    /// Build horizontal grid: Cells are positions from left to right.
    /// Takes the available horizontal width, so there can't be anything right of the grid or the container will grow slowly!
    ///
    /// Returns a `[egui::Response]` for hover events.
    pub fn horizontal<F>(self, grid: F) -> Response
    where
        F: for<'b> FnOnce(Grid<'a, 'b>),
    {
        let widths = self.sizing.into_lengths(
            self.ui.available_rect_before_wrap().width() - self.ui.spacing().item_spacing.x,
            self.ui.spacing().item_spacing.x,
        );
        let mut layout = Layout::new(self.ui, LineDirection::Vertical);
        grid(Grid {
            layout: &mut layout,
            direction: GridDirection::Horizontal,
            sizes: widths,
        });
        layout.set_rect()
    }

    /// Build vertical grid: Cells are positions from top to bottom.
    /// Takes the full available vertical height, so there can't be anything below of the grid or the container will grow slowly!
    ///
    /// Returns a `[egui::Response]` for hover events.
    pub fn vertical<F>(self, grid: F) -> Response
    where
        F: for<'b> FnOnce(Grid<'a, 'b>),
    {
        let heights = self.sizing.into_lengths(
            self.ui.available_rect_before_wrap().height() - self.ui.spacing().item_spacing.y,
            self.ui.spacing().item_spacing.y,
        );
        let mut layout = Layout::new(self.ui, LineDirection::Horizontal);
        grid(Grid {
            layout: &mut layout,
            direction: GridDirection::Vertical,
            sizes: heights,
        });
        layout.set_rect()
    }
}

pub struct Grid<'a, 'b> {
    layout: &'b mut Layout<'a>,
    direction: GridDirection,
    sizes: Vec<f32>,
}

impl<'a, 'b> Grid<'a, 'b> {
    fn next_cell_size(&mut self) -> (CellSize, CellSize) {
        match self.direction {
            GridDirection::Horizontal => (
                CellSize::Absolute(self.sizes.remove(0)),
                CellSize::Remainder,
            ),
            GridDirection::Vertical => (
                CellSize::Remainder,
                CellSize::Absolute(self.sizes.remove(0)),
            ),
        }
    }

    /// Add empty cell
    pub fn empty(&mut self) {
        assert!(
            !self.sizes.is_empty(),
            "Tried using more grid cells then available."
        );

        let (width, height) = self.next_cell_size();
        self.layout.empty(width, height);
    }

    fn _cell(&mut self, clip: bool, add_contents: impl FnOnce(&mut Ui)) {
        assert!(
            !self.sizes.is_empty(),
            "Tried using more grid cells then available."
        );

        let (width, height) = self.next_cell_size();
        self.layout.add(width, height, clip, add_contents);
    }

    /// Add cell, content is wrapped
    pub fn cell(&mut self, add_contents: impl FnOnce(&mut Ui)) {
        self._cell(false, add_contents);
    }

    /// Add cell, content is clipped
    pub fn cell_clip(&mut self, add_contents: impl FnOnce(&mut Ui)) {
        self._cell(true, add_contents);
    }

    fn _grid(&mut self, clip: bool, grid_builder: impl FnOnce(GridBuilder<'_>)) {
        self._cell(clip, |ui| {
            grid_builder(GridBuilder::new(ui));
        });
    }
    /// Add grid as cell
    pub fn grid(&mut self, grid_builder: impl FnOnce(GridBuilder<'_>)) {
        self._grid(false, grid_builder);
    }

    /// Add grid as cell, content is clipped
    pub fn grid_noclip(&mut self, grid_builder: impl FnOnce(GridBuilder<'_>)) {
        self._grid(true, grid_builder);
    }
}

impl<'a, 'b> Drop for Grid<'a, 'b> {
    fn drop(&mut self) {
        while !self.sizes.is_empty() {
            self.empty();
        }
    }
}
