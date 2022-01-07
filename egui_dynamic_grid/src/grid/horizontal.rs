use crate::{
    layout::{CellSize, LineDirection},
    sizing::Sizing,
    Layout, Padding, Size,
};
use egui::Ui;

use super::VerticalGridBuilder;

pub struct HorizontalGridBuilder<'a> {
    ui: &'a mut Ui,
    padding: Padding,
    sizing: Sizing,
}

impl<'a> HorizontalGridBuilder<'a> {
    pub(crate) fn new(ui: &'a mut Ui, padding: Padding) -> Self {
        let layouter = Sizing::new(
            ui.available_rect_before_wrap().width() - 2.0 * padding.outer,
            padding.inner,
        );

        Self {
            ui,
            padding,
            sizing: layouter,
        }
    }

    pub fn column(mut self, size: Size) -> Self {
        self.sizing.add_size(size);
        self
    }

    pub fn columns(mut self, size: Size, count: usize) -> Self {
        for _ in 0..count {
            self.sizing.add_size(size.clone());
        }
        self
    }

    pub fn build<F>(self, horizontal_grid: F)
    where
        F: for<'b> FnOnce(HorizontalGrid<'a, 'b>),
    {
        let widths = self.sizing.into_lengths();
        let mut layout = Layout::new(self.ui, self.padding.clone(), LineDirection::TopToBottom);
        let grid = HorizontalGrid {
            layout: &mut layout,
            padding: self.padding.clone(),
            widths,
        };
        horizontal_grid(grid);
    }
}

pub struct HorizontalGrid<'a, 'b> {
    layout: &'b mut Layout<'a>,
    padding: Padding,
    widths: Vec<f32>,
}

impl<'a, 'b> HorizontalGrid<'a, 'b> {
    pub fn empty(&mut self) {
        assert!(
            !self.widths.is_empty(),
            "Tried using more grid cells then available."
        );

        self.layout.empty(
            CellSize::Absolute(self.widths.remove(0)),
            CellSize::Remainder,
        );
    }

    pub fn _cell(&mut self, clip: bool, add_contents: impl FnOnce(&mut Ui)) {
        assert!(
            !self.widths.is_empty(),
            "Tried using more grid cells then available."
        );

        self.layout.add(
            CellSize::Absolute(self.widths.remove(0)),
            CellSize::Remainder,
            clip,
            add_contents,
        );
    }

    pub fn cell(&mut self, add_contents: impl FnOnce(&mut Ui)) {
        self._cell(true, add_contents);
    }

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

    pub fn horizontal(&mut self, horizontal_grid_builder: impl FnOnce(HorizontalGridBuilder)) {
        self._horizontal(true, horizontal_grid_builder)
    }

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

    pub fn vertical(&mut self, vertical_grid_builder: impl FnOnce(VerticalGridBuilder)) {
        self._vertical(true, vertical_grid_builder);
    }

    pub fn vertical_noclip(&mut self, vertical_grid_builder: impl FnOnce(VerticalGridBuilder)) {
        self._vertical(false, vertical_grid_builder);
    }
}

impl<'a, 'b> Drop for HorizontalGrid<'a, 'b> {
    fn drop(&mut self) {
        while !self.widths.is_empty() {
            self.empty();
        }
    }
}
