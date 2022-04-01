use crate::{
    layout::{CellDirection, CellSize, StripLayout},
    sizing::Sizing,
    Size,
};
use egui::{Response, Ui};

/// Builder for creating a new [`Strip`].
///
/// This can be used to do dynamic layouts.
///
/// In contrast to normal egui behavior, strip cells do *not* grow with its children!
///
/// After adding size hints with `[Self::column]`/`[Self::columns]` the strip can be build with `[Self::horizontal]`/`[Self::vertical]`.
///
/// ### Example
/// ```
/// # egui::__run_test_ui(|ui| {
/// use egui_extras::{StripBuilder, Size};
/// StripBuilder::new(ui)
///     .size(Size::remainder().at_least(100.0))
///     .size(Size::exact(40.0))
///     .vertical(|mut strip| {
///         strip.strip(|builder| {
///             builder.sizes(Size::remainder(), 2).horizontal(|mut strip| {
///                 strip.cell(|ui| {
///                     ui.label("Top Left");
///                 });
///                 strip.cell(|ui| {
///                     ui.label("Top Right");
///                 });
///             });
///         });
///         strip.cell(|ui| {
///             ui.label("Fixed");
///         });
///     });
/// # });
/// ```
pub struct StripBuilder<'a> {
    ui: &'a mut Ui,
    sizing: Sizing,
    clip: bool,
}

impl<'a> StripBuilder<'a> {
    /// Create new strip builder.
    pub fn new(ui: &'a mut Ui) -> Self {
        let sizing = Sizing::new();

        Self {
            ui,
            sizing,
            clip: true,
        }
    }

    /// Should we clip the contents of each cell? Default: `true`.
    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    /// Add size hint for one column/row.
    pub fn size(mut self, size: Size) -> Self {
        self.sizing.add(size);
        self
    }

    /// Add size hint for several columns/rows at once.
    pub fn sizes(mut self, size: Size, count: usize) -> Self {
        for _ in 0..count {
            self.sizing.add(size);
        }
        self
    }

    /// Build horizontal strip: Cells are positions from left to right.
    /// Takes the available horizontal width, so there can't be anything right of the strip or the container will grow slowly!
    ///
    /// Returns a `[egui::Response]` for hover events.
    pub fn horizontal<F>(self, strip: F) -> Response
    where
        F: for<'b> FnOnce(Strip<'a, 'b>),
    {
        let widths = self.sizing.to_lengths(
            self.ui.available_rect_before_wrap().width() - self.ui.spacing().item_spacing.x,
            self.ui.spacing().item_spacing.x,
        );
        let mut layout = StripLayout::new(self.ui, CellDirection::Horizontal, self.clip);
        strip(Strip {
            layout: &mut layout,
            direction: CellDirection::Horizontal,
            sizes: &widths,
        });
        layout.allocate_rect()
    }

    /// Build vertical strip: Cells are positions from top to bottom.
    /// Takes the full available vertical height, so there can't be anything below of the strip or the container will grow slowly!
    ///
    /// Returns a `[egui::Response]` for hover events.
    pub fn vertical<F>(self, strip: F) -> Response
    where
        F: for<'b> FnOnce(Strip<'a, 'b>),
    {
        let heights = self.sizing.to_lengths(
            self.ui.available_rect_before_wrap().height() - self.ui.spacing().item_spacing.y,
            self.ui.spacing().item_spacing.y,
        );
        let mut layout = StripLayout::new(self.ui, CellDirection::Vertical, self.clip);
        strip(Strip {
            layout: &mut layout,
            direction: CellDirection::Vertical,
            sizes: &heights,
        });
        layout.allocate_rect()
    }
}

/// A Strip of cells which go in one direction. Each cell has a fixed size.
/// In contrast to normal egui behavior, strip cells do *not* grow with its children!
pub struct Strip<'a, 'b> {
    layout: &'b mut StripLayout<'a>,
    direction: CellDirection,
    sizes: &'b [f32],
}

impl<'a, 'b> Strip<'a, 'b> {
    fn next_cell_size(&mut self) -> (CellSize, CellSize) {
        assert!(
            !self.sizes.is_empty(),
            "Tried using more strip cells than available."
        );
        let size = self.sizes[0];
        self.sizes = &self.sizes[1..];

        match self.direction {
            CellDirection::Horizontal => (CellSize::Absolute(size), CellSize::Remainder),
            CellDirection::Vertical => (CellSize::Remainder, CellSize::Absolute(size)),
        }
    }

    /// Add empty cell
    pub fn empty(&mut self) {
        let (width, height) = self.next_cell_size();
        self.layout.empty(width, height);
    }

    /// Add cell contents.
    pub fn cell(&mut self, add_contents: impl FnOnce(&mut Ui)) {
        let (width, height) = self.next_cell_size();
        self.layout.add(width, height, add_contents);
    }

    /// Add strip as cell
    pub fn strip(&mut self, strip_builder: impl FnOnce(StripBuilder<'_>)) {
        let clip = self.layout.clip;
        self.cell(|ui| {
            strip_builder(StripBuilder::new(ui).clip(clip));
        });
    }
}

impl<'a, 'b> Drop for Strip<'a, 'b> {
    fn drop(&mut self) {
        while !self.sizes.is_empty() {
            self.empty();
        }
    }
}
