use egui::{Pos2, Rect, Response, Sense, Ui};

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
}

impl<'l> StripLayout<'l> {
    pub(crate) fn new(ui: &'l mut Ui, direction: CellDirection, cell_layout: egui::Layout) -> Self {
        let rect = ui.available_rect_before_wrap();
        let pos = rect.left_top();

        Self {
            ui,
            direction,
            rect,
            cursor: pos,
            max: pos,
            cell_layout,
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
        clip: bool,
        striped: bool,
        width: CellSize,
        height: CellSize,
        add_cell_contents: impl FnOnce(&mut Ui),
    ) -> (Rect, Response) {
        let max_rect = self.cell_rect(&width, &height);

        if striped {
            // Make sure we don't have a gap in the stripe background:
            let stripe_rect = max_rect.expand2(0.5 * self.ui.spacing().item_spacing);

            self.ui
                .painter()
                .rect_filled(stripe_rect, 0.0, self.ui.visuals().faint_bg_color);
        }

        let used_rect = self.cell(clip, max_rect, add_cell_contents);

        self.set_pos(max_rect);

        let allocation_rect = if clip {
            max_rect
        } else {
            max_rect.union(used_rect)
        };

        let response = self.ui.allocate_rect(allocation_rect, Sense::hover());

        (used_rect, response)
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
        self.ui.allocate_rect(rect, Sense::hover());
    }

    fn cell(&mut self, clip: bool, rect: Rect, add_cell_contents: impl FnOnce(&mut Ui)) -> Rect {
        let mut child_ui = self.ui.child_ui(rect, self.cell_layout);

        if clip {
            let margin = egui::Vec2::splat(self.ui.visuals().clip_rect_margin);
            let margin = margin.min(0.5 * self.ui.spacing().item_spacing);
            let clip_rect = rect.expand2(margin);
            child_ui.set_clip_rect(clip_rect.intersect(child_ui.clip_rect()));
        }

        add_cell_contents(&mut child_ui);
        child_ui.min_rect()
    }

    /// Allocate the rect in [`Self::ui`] so that the scrollview knows about our size
    pub fn allocate_rect(&mut self) -> Response {
        let mut rect = self.rect;
        rect.set_right(self.max.x);
        rect.set_bottom(self.max.y);

        self.ui.allocate_rect(rect, Sense::hover())
    }
}
