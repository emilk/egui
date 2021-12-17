mod horizontal;
mod vertical;

use crate::Padding;
use egui::Ui;
pub use horizontal::*;
pub use vertical::*;

pub struct GridBuilder<'a> {
    ui: &'a mut Ui,
    padding: Padding,
}

impl<'a> GridBuilder<'a> {
    pub fn new(ui: &'a mut Ui, padding: Padding) -> Self {
        Self { ui, padding }
    }

    pub fn horizontal(self, horizontal_grid_builder: impl FnOnce(HorizontalGridBuilder)) {
        horizontal_grid_builder(HorizontalGridBuilder::new(self.ui, self.padding));
    }

    pub fn vertical(self, vertical_grid_builder: impl FnOnce(VerticalGridBuilder)) {
        vertical_grid_builder(VerticalGridBuilder::new(self.ui, self.padding));
    }
}
