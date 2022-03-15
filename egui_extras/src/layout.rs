use egui::{Pos2, Rect, Response, Rgba, Sense, Ui};

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
/// In a horizontal strip, a `[Layout]` with horizontal `[CellDirection]` is used.
/// Its cells go from left to right inside this `[Layout]`.
///
/// In a table there's a `[Layout]` for each table row with a horizonal `[CellDirection]`.
/// Its cells go from left to right. And the lines go from top to bottom.
pub(crate) enum CellDirection {
    /// Cells go from left to right
    Horizontal,
    /// Cells go fromtop to bottom
    Vertical,
}

/// Positions cells in `[CellDirection]` and starts a new line on `[Layout::end_line]`
pub struct Layout<'l> {
    ui: &'l mut Ui,
    direction: CellDirection,
    rect: Rect,
    pos: Pos2,
    max: Pos2,
}

impl<'l> Layout<'l> {
    pub(crate) fn new(ui: &'l mut Ui, direction: CellDirection) -> Self {
        let rect = ui.available_rect_before_wrap();
        let pos = rect.left_top();

        Self {
            ui,
            rect,
            pos,
            max: pos,
            direction,
        }
    }

    pub fn current_y(&self) -> f32 {
        self.rect.top()
    }

    fn cell_rect(&self, width: &CellSize, height: &CellSize) -> Rect {
        Rect {
            min: self.pos,
            max: Pos2 {
                x: match width {
                    CellSize::Absolute(width) => self.pos.x + width,
                    CellSize::Remainder => self.rect.right() - self.ui.spacing().item_spacing.x,
                },
                y: match height {
                    CellSize::Absolute(height) => self.pos.y + height,
                    CellSize::Remainder => self.rect.bottom() - self.ui.spacing().item_spacing.y,
                },
            },
        }
    }

    fn set_pos(&mut self, rect: Rect) {
        match self.direction {
            CellDirection::Horizontal => {
                self.pos.x = rect.right() + self.ui.spacing().item_spacing.x;
            }
            CellDirection::Vertical => {
                self.pos.y = rect.bottom() + self.ui.spacing().item_spacing.y;
            }
        }

        self.max.x = self
            .max
            .x
            .max(rect.right() + self.ui.spacing().item_spacing.x);
        self.max.y = self
            .max
            .y
            .max(rect.bottom() + self.ui.spacing().item_spacing.y);
    }

    pub(crate) fn empty(&mut self, width: CellSize, height: CellSize) {
        self.set_pos(self.cell_rect(&width, &height));
    }

    pub(crate) fn add(
        &mut self,
        width: CellSize,
        height: CellSize,
        clip: bool,
        add_contents: impl FnOnce(&mut Ui),
    ) -> Response {
        let rect = self.cell_rect(&width, &height);
        self.cell(rect, clip, add_contents);
        self.set_pos(rect);

        self.ui.allocate_rect(rect, Sense::click())
    }

    pub(crate) fn add_striped(
        &mut self,
        width: CellSize,
        height: CellSize,
        clip: bool,
        add_contents: impl FnOnce(&mut Ui),
    ) -> Response {
        let mut rect = self.cell_rect(&width, &height);
        // Make sure we don't have a gap in the stripe background
        *rect.top_mut() -= self.ui.spacing().item_spacing.y;
        *rect.left_mut() -= self.ui.spacing().item_spacing.x;

        let text_color: Rgba = self.ui.visuals().text_color().into();
        self.ui
            .painter()
            .rect_filled(rect, 0.0, text_color.multiply(0.2));

        self.add(width, height, clip, add_contents)
    }

    /// only needed for layouts with multiple lines, like Table
    pub fn end_line(&mut self) {
        match self.direction {
            CellDirection::Horizontal => {
                self.pos.y = self.max.y;
                self.pos.x = self.rect.left();
            }
            CellDirection::Vertical => {
                self.pos.x = self.max.x;
                self.pos.y = self.rect.top();
            }
        }
    }

    fn cell(&mut self, rect: Rect, clip: bool, add_contents: impl FnOnce(&mut Ui)) {
        let mut child_ui = self.ui.child_ui(rect, *self.ui.layout());

        if clip {
            let mut clip_rect = child_ui.clip_rect();
            clip_rect.min = clip_rect.min.max(rect.min);
            clip_rect.max = clip_rect.max.min(rect.max);
            child_ui.set_clip_rect(clip_rect);
        }

        add_contents(&mut child_ui);
    }

    /// Set the rect so that the scrollview knows about our size
    pub fn set_rect(&mut self) -> Response {
        let mut rect = self.rect;
        rect.set_right(self.max.x);
        rect.set_bottom(self.max.y);

        self.ui.allocate_rect(rect, Sense::hover())
    }
}
