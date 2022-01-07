use crate::{layout::CellSize, sizing::Sizing, Layout, Padding, Size};
use egui::Ui;

use super::HorizontalGridBuilder;

pub struct VerticalGridBuilder<'a> {
    ui: &'a mut Ui,
    padding: Padding,
    sizing: Sizing,
}

impl<'a> VerticalGridBuilder<'a> {
    /// Create new grid builder for vertical grid
    /// After adding size hints with [Self::row]/[Self::rows] the grid can be build with [Self::build]
    pub(crate) fn new(ui: &'a mut Ui, padding: Padding) -> Self {
        let layouter = Sizing::new(
            ui.available_rect_before_wrap().height() - 2.0 * padding.outer,
            padding.inner,
        );

        Self {
            ui,
            padding,
            sizing: layouter,
        }
    }

    /// Add size hint for row
    pub fn row(mut self, size: Size) -> Self {
        self.sizing.add_size(size);
        self
    }

    /// Add size hint for rows [count] times
    pub fn rows(mut self, size: Size, count: usize) -> Self {
        for _ in 0..count {
            self.sizing.add_size(size.clone());
        }
        self
    }

    /// Build grid
    pub fn build<F>(self, vertical_grid: F)
    where
        F: for<'b> FnOnce(VerticalGrid<'a, 'b>),
    {
        let heights = self.sizing.into_lengths();
        let mut layout = Layout::new(
            self.ui,
            self.padding.clone(),
            crate::layout::LineDirection::LeftToRight,
        );
        let grid = VerticalGrid {
            layout: &mut layout,
            padding: self.padding.clone(),
            heights,
        };
        vertical_grid(grid);
    }
}

pub struct VerticalGrid<'a, 'b> {
    layout: &'b mut Layout<'a>,
    padding: Padding,
    heights: Vec<f32>,
}

impl<'a, 'b> VerticalGrid<'a, 'b> {
    /// Add empty cell
    pub fn empty(&mut self) {
        assert!(
            !self.heights.is_empty(),
            "Tried using more grid cells then available."
        );

        self.layout.empty(
            CellSize::Remainder,
            CellSize::Absolute(self.heights.remove(0)),
        );
    }

    pub fn _cell(&mut self, clip: bool, add_contents: impl FnOnce(&mut Ui)) {
        assert!(
            !self.heights.is_empty(),
            "Tried using more grid cells then available."
        );

        self.layout.add(
            CellSize::Remainder,
            CellSize::Absolute(self.heights.remove(0)),
            clip,
            add_contents,
        );
    }

    /// Add cell, content is clipped
    pub fn cell(&mut self, add_contents: impl FnOnce(&mut Ui)) {
        self._cell(true, add_contents);
    }

    /// Add cell, content is not clipped
    pub fn cell_noclip(&mut self, add_contents: impl FnOnce(&mut Ui)) {
        self._cell(false, add_contents);
    }

    pub fn _horizontal(
        &mut self,
        clip: bool,
        horizontal_grid_builder: impl FnOnce(HorizontalGridBuilder),
    ) {
        let padding = self.padding.clone();
        self._cell(clip, |ui| {
            horizontal_grid_builder(HorizontalGridBuilder::new(ui, padding));
        });
    }

    /// Add horizontal grid as cell, content is clipped
    pub fn horizontal(&mut self, horizontal_grid_builder: impl FnOnce(HorizontalGridBuilder)) {
        self._horizontal(true, horizontal_grid_builder)
    }

    /// Add horizontal grid as cell, content is not clipped
    pub fn horizontal_noclip(
        &mut self,
        horizontal_grid_builder: impl FnOnce(HorizontalGridBuilder),
    ) {
        self._horizontal(false, horizontal_grid_builder)
    }

    pub fn _vertical(
        &mut self,
        clip: bool,
        vertical_grid_builder: impl FnOnce(VerticalGridBuilder),
    ) {
        let padding = self.padding.clone();
        self._cell(clip, |ui| {
            vertical_grid_builder(VerticalGridBuilder::new(ui, padding));
        });
    }

    /// Add vertical grid as cell, content is clipped
    pub fn vertical(&mut self, vertical_grid_builder: impl FnOnce(VerticalGridBuilder)) {
        self._vertical(true, vertical_grid_builder);
    }

    /// Add vertical grid as cell, content is not clipped
    pub fn vertical_noclip(&mut self, vertical_grid_builder: impl FnOnce(VerticalGridBuilder)) {
        self._vertical(false, vertical_grid_builder);
    }
}

impl<'a, 'b> Drop for VerticalGrid<'a, 'b> {
    fn drop(&mut self) {
        while !self.heights.is_empty() {
            self.empty();
        }
    }
}
